//! UI-independent state for the Ultramarine Server local OOBE.
//!
//! This module deliberately contains only serializable state and persistence.
//! The local web service can expose it over a narrow API, while libtaidan's
//! privileged operation implementations remain separate from the UI layer.

/// Canonical local port for the Ultramarine Server OOBE web service.
pub const DEFAULT_OOBE_PORT: u16 = 3972;

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::err::{Err, Res};

/// Ordered setup stages shown by a local OOBE client.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OobeStep {
    Welcome,
    Network,
    Administrator,
    LocalEnvironment,
    Tetra,
    Fyra,
    Complete,
}

impl Default for OobeStep {
    fn default() -> Self {
        Self::Welcome
    }
}

impl OobeStep {
    pub const ALL: [Self; 7] = [
        Self::Welcome,
        Self::Network,
        Self::Administrator,
        Self::LocalEnvironment,
        Self::Tetra,
        Self::Fyra,
        Self::Complete,
    ];
}

/// Lifecycle state for one OOBE stage.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "state", content = "message")]
pub enum StepState {
    #[default]
    Pending,
    Active,
    Complete,
    RequiresAuthentication,
    Blocked(String),
    Failed(String),
}

/// Connectivity state reported for the local Tetra agent.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct TetraState {
    pub status: ConnectionStatus,
    pub endpoint: Option<String>,
    pub host_fingerprint: Option<String>,
    pub last_error: Option<String>,
}

/// Connectivity state reported for the Fyra control plane.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct FyraState {
    pub status: ConnectionStatus,
    pub account_hint: Option<String>,
    pub environment_id: Option<String>,
    pub last_error: Option<String>,
}

/// A deliberately small status vocabulary shared by UI and backend adapters.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionStatus {
    #[default]
    Unknown,
    Checking,
    Offline,
    Online,
    Paired,
    RequiresPairing,
}

/// Persistent, non-secret state consumed by the local OOBE UI.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct OobeState {
    pub version: u32,
    pub completed: bool,
    pub active_step: OobeStep,
    pub steps: Vec<StepProgress>,
    pub hostname: Option<String>,
    pub tetra: TetraState,
    pub fyra: FyraState,
}

impl OobeState {
    pub const CURRENT_VERSION: u32 = 1;

    pub fn new() -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            active_step: OobeStep::Welcome,
            steps: OobeStep::ALL
                .into_iter()
                .map(|step| StepProgress { step, state: StepState::Pending })
                .collect(),
            ..Self::default()
        }
    }

    /// Update a stage without allowing the UI to manufacture arbitrary stages.
    pub fn set_step_state(&mut self, step: OobeStep, state: StepState) {
        if let Some(progress) = self.steps.iter_mut().find(|item| item.step == step) {
            progress.state = state;
        }
        self.active_step = step;
    }

    pub fn mark_complete(&mut self) {
        self.completed = true;
        self.set_step_state(OobeStep::Complete, StepState::Complete);
    }
}

/// State for one ordered OOBE stage.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StepProgress {
    pub step: OobeStep,
    pub state: StepState,
}

/// Atomic JSON persistence for mutable OOBE state.
///
/// Secrets and enrollment tokens must not be placed in this file. Those belong
/// in separately protected credential storage owned by the relevant backend.
pub struct OobeStateStore {
    path: PathBuf,
}

impl OobeStateStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> Res<OobeState> {
        if !self.path.exists() {
            return Ok(OobeState::new());
        }
        let contents = fs::read_to_string(&self.path).map_err(Err::Io)?;
        let state = serde_json::from_str(&contents)?;
        Ok(state)
    }

    /// Write state through a restricted temporary file and atomic rename.
    pub fn save(&self, state: &OobeState) -> Res<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(Err::Io)?;
        }
        let contents = serde_json::to_vec_pretty(state)?;
        let temporary = self.path.with_extension("json.tmp");
        let mut file = fs::File::create(&temporary).map_err(Err::Io)?;
        file.write_all(&contents).map_err(Err::Io)?;
        file.write_all(b"\n").map_err(Err::Io)?;
        file.sync_all().map_err(Err::Io)?;
        fs::rename(temporary, &self.path).map_err(Err::Io)
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn new_state_contains_all_ordered_steps() {
        let state = OobeState::new();
        assert_eq!(state.version, OobeState::CURRENT_VERSION);
        assert_eq!(state.steps.len(), OobeStep::ALL.len());
        assert_eq!(state.active_step, OobeStep::Welcome);
    }

    #[test]
    fn state_round_trips_atomically() {
        let directory = tempdir().unwrap();
        let store = OobeStateStore::new(directory.path().join("state/oobe.json"));
        let mut state = OobeState::new();
        state.hostname = Some("server.example".into());
        state.set_step_state(OobeStep::Tetra, StepState::RequiresAuthentication);
        store.save(&state).unwrap();

        assert_eq!(store.load().unwrap(), state);
        assert!(!store.path().with_extension("json.tmp").exists());
    }

    #[test]
    fn missing_state_starts_a_new_setup() {
        let directory = tempdir().unwrap();
        let store = OobeStateStore::new(directory.path().join("missing.json"));
        assert_eq!(store.load().unwrap(), OobeState::new());
    }
}
