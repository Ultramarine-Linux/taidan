//! Server setup orchestration for Ultramarine Server OOBE.
//!
//! This module provides typed operations that map to Tetra agent commands.
//! It does not reimplement host operations; it constructs command envelopes
//! and dispatches them through the `tetra` binary.

use std::process::Stdio;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::err::{Err, Res};

/// A command envelope for the Tetra agent.
#[derive(Debug, Clone, Serialize)]
pub struct AgentCommand {
    pub id: String,
    pub module: String,
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

/// Response from the Tetra agent.
#[derive(Debug, Clone, Deserialize)]
pub struct AgentResponse {
    pub id: String,
    pub ok: bool,
    #[serde(default)]
    pub payload: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<String>,
}

/// Dispatch a command to the local Tetra agent via `tetra agent-dispatch`.
///
/// # Errors
/// - Tetra binary is not found
/// - Tetra exits with non-zero status
/// - Response JSON cannot be parsed
pub async fn dispatch_tetra(command: &AgentCommand) -> Res<AgentResponse> {
    let json_cmd = serde_json::to_vec(command)?;

    let mut child = tokio::process::Command::new("tetra")
        .arg("agent-dispatch")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| Err::FailToRunProgram("tetra", e))?;

    let mut stdin = child.stdin.take().expect("piped stdin");
    tokio::io::AsyncWriteExt::write_all(&mut stdin, &json_cmd)
        .await
        .map_err(Err::Io)?;

    let output = child.wait_with_output().await.map_err(|e| Err::FailToRunProgram("tetra", e))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Err::Message(format!("tetra failed: {stderr}")));
    }

    let response: AgentResponse = serde_json::from_slice(&output.stdout)?;
    Ok(response)
}

/// Set the system hostname via Tetra.
pub async fn set_hostname(hostname: impl Into<String>) -> Res<AgentResponse> {
    dispatch_tetra(&AgentCommand {
        id: "oobe-set-hostname".into(),
        module: "settings".into(),
        action: "set_hostname".into(),
        payload: Some(json!({ "hostname": hostname.into() })),
        signature: None,
    })
    .await
}

/// Check network status via Tetra.
pub async fn check_network() -> Res<AgentResponse> {
    dispatch_tetra(&AgentCommand {
        id: "oobe-network-status".into(),
        module: "network".into(),
        action: "status".into(),
        payload: Some(json!({})),
        signature: None,
    })
    .await
}

/// Create a local user via Tetra.
pub async fn create_user(name: impl Into<String>, shell: Option<String>, home: Option<String>) -> Res<AgentResponse> {
    let mut payload = serde_json::Map::new();
    payload.insert("name".into(), json!(name.into()));
    if let Some(shell) = shell {
        payload.insert("shell".into(), json!(shell));
    }
    if let Some(home) = home {
        payload.insert("home".into(), json!(home));
    }

    dispatch_tetra(&AgentCommand {
        id: "oobe-create-user".into(),
        module: "users".into(),
        action: "create".into(),
        payload: Some(serde_json::Value::Object(payload)),
        signature: None,
    })
    .await
}

/// Set a user's password hash via Tetra.
pub async fn set_password(name: impl Into<String>, password_hash: impl Into<String>) -> Res<AgentResponse> {
    dispatch_tetra(&AgentCommand {
        id: "oobe-set-password".into(),
        module: "users".into(),
        action: "set_password".into(),
        payload: Some(json!({
            "name": name.into(),
            "password_hash": password_hash.into(),
        })),
        signature: None,
    })
    .await
}

/// Query Tetra capabilities (also useful as an existence check).
pub async fn tetra_capabilities() -> Res<AgentResponse> {
    dispatch_tetra(&AgentCommand {
        id: "oobe-capabilities".into(),
        module: "agent".into(),
        action: "capabilities".into(),
        payload: Some(json!({})),
        signature: None,
    })
    .await
}

/// Query a systemd service status via Tetra.
pub async fn service_status(service: impl Into<String>) -> Res<AgentResponse> {
    dispatch_tetra(&AgentCommand {
        id: "oobe-service-status".into(),
        module: "services".into(),
        action: "status".into(),
        payload: Some(json!({ "service": service.into() })),
        signature: None,
    })
    .await
}

/// Start a systemd service via Tetra.
pub async fn service_start(service: impl Into<String>) -> Res<AgentResponse> {
    dispatch_tetra(&AgentCommand {
        id: "oobe-service-start".into(),
        module: "services".into(),
        action: "start".into(),
        payload: Some(json!({ "service": service.into() })),
        signature: None,
    })
    .await
}

/// Enable a systemd service via Tetra.
pub async fn service_enable(service: impl Into<String>) -> Res<AgentResponse> {
    dispatch_tetra(&AgentCommand {
        id: "oobe-service-enable".into(),
        module: "services".into(),
        action: "enable".into(),
        payload: Some(json!({ "service": service.into() })),
        signature: None,
    })
    .await
}
