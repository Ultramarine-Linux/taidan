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
        "welcome", "devicename", "whoareyou", "password", "ssh-key", "internet", "updates",
        "remote-access", "dashboard", "complete",
    ];
    const OPERATIONS: &[&str] = &[
        "state.patch", "state.reset", "hostname.apply", "keyboard.apply", "user.create",
        "password.set", "network.interfaces", "network.scan_wifi", "network.connect_wifi",
        "network.configure", "network.confirm", "network.check", "ssh-key.apply",
        "updates.set_policy", "remote_access.status", "remote_access.tailscale_up",
        "remote_access.cloudflared_set_token", "access.set", "system.reboot", "system.poweroff",
        "setup.complete",
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

    #[test]
    fn rejects_removed_enrollment() {
        let operation = SetupOperation {
            step: "dashboard".into(),
            operation: "fyra.begin".into(),
            payload: serde_json::Value::Null,
        };
        assert_eq!(validate_operation(&operation), Err(ValidationError::InvalidOperation));
    }

    #[test]
    fn rejects_removed_host_operations() {
        for removed in [
            "dashboard.install",
            "dashboard.handoff",
            "cloudflare.install",
            "tweaks.apply",
            "tetra.detect",
            "tetra.start",
        ] {
            let operation = SetupOperation {
                step: "dashboard".into(),
                operation: removed.into(),
                payload: serde_json::Value::Null,
            };
            assert_eq!(
                validate_operation(&operation),
                Err(ValidationError::InvalidOperation),
                "{removed}"
            );
        }
    }

    #[test]
    fn network_uses_structured_configuration() {
        let operation = |name: &str| SetupOperation {
            step: "internet".into(),
            operation: name.into(),
            payload: serde_json::Value::Null,
        };
        // Raw keyfile writes are not a setup operation; Tetra refuses them too.
        assert_eq!(
            validate_operation(&operation("network.set_config")),
            Err(ValidationError::InvalidOperation)
        );
        for allowed in ["network.configure", "network.confirm", "network.connect_wifi"] {
            assert_eq!(validate_operation(&operation(allowed)), Ok(()), "{allowed}");
        }
    }

    #[test]
    fn accepts_new_setup_steps() {
        for (step, operation) in [
            ("updates", "updates.set_policy"),
            ("remote-access", "remote_access.tailscale_up"),
            ("remote-access", "remote_access.cloudflared_set_token"),
            ("dashboard", "access.set"),
        ] {
            let operation = SetupOperation {
                step: step.into(),
                operation: operation.into(),
                payload: serde_json::Value::Null,
            };
            assert_eq!(validate_operation(&operation), Ok(()));
        }
    }

    #[test]
    fn accepts_setup_complete() {
        let operation = SetupOperation {
            step: "complete".into(),
            operation: "setup.complete".into(),
            payload: serde_json::Value::Null,
        };
        assert_eq!(validate_operation(&operation), Ok(()));
    }
}
