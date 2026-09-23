//! Typed server OOBE orchestration primitives.
//!
//! This module deliberately contains policy and validation only. Host mutations
//! are executed by Tetra through its typed agent command interface.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StepStatus {
    Pending,
    Running,
    Complete,
    RequiresAuthentication,
    Blocked,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupStep {
    pub id: String,
    pub status: StepStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupState {
    pub version: u32,
    pub completed: bool,
    pub active_step: String,
    pub steps: Vec<SetupStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupOperation {
    pub step: String,
    pub operation: String,
    #[serde(default)]
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    InvalidStep,
    InvalidOperation,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidStep => f.write_str("unknown setup step"),
            Self::InvalidOperation => f.write_str("operation is not allowed during setup"),
        }
    }
}

impl std::error::Error for ValidationError {}

/// Validate the narrow operation vocabulary exposed by the local OOBE API.
/// Arbitrary commands, executable paths, and argument vectors are rejected.
pub fn validate_operation(operation: &SetupOperation) -> Result<(), ValidationError> {
    const STEPS: &[&str] = &[
        "welcome", "devicename", "whoareyou", "password", "internet", "tweaks",
        "ssh-key", "dashboard", "complete",
    ];
    const OPERATIONS: &[&str] = &[
        "state.patch", "state.reset", "hostname.apply", "keyboard.apply", "user.create",
        "password.set", "network.interfaces", "network.scan_wifi", "network.connect_wifi",
        "network.set_config", "network.check", "tetra.detect", "tetra.start", "ssh-key.apply",
        "tweaks.apply", "fyra.begin", "system.reboot", "system.poweroff", "dashboard.install",
        "dashboard.handoff", "cloudflare.install",
    ];
    if !STEPS.contains(&operation.step.as_str()) {
        return Err(ValidationError::InvalidStep);
    }
    if !OPERATIONS.contains(&operation.operation.as_str()) {
        return Err(ValidationError::InvalidOperation);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_arbitrary_commands() {
        let operation = SetupOperation {
            step: "internet".into(),
            operation: "shell.exec".into(),
            payload: serde_json::Value::Null,
        };
        assert_eq!(validate_operation(&operation), Err(ValidationError::InvalidOperation));
    }
}
