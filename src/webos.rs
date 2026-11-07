//! LG webOS SSAP registration and transport primitives.
//! Protocol behavior inspected in the MIT-licensed upstream project:
//! https://github.com/JPersson77/LGTVCompanion

use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{fs, os::unix::fs::{OpenOptionsExt, PermissionsExt}, path::{Path, PathBuf}};
use tokio_tungstenite::{connect_async, tungstenite::Message};

const REGISTER_ID: &str = "register_0";

#[derive(Debug, Clone)]
pub struct WebOsEndpoint {
    pub host: String,
    pub secure: bool,
    pub port: u16,
}

impl WebOsEndpoint {
    pub fn url(&self) -> String {
        let scheme = if self.secure { "wss" } else { "ws" };
        let port = if self.secure && self.port == 3000 { 3001 } else { self.port };
        format!("{scheme}://{}:{port}/", self.host)
    }
}

pub fn registration_message(session_key: Option<&str>) -> Value {
    let mut payload = json!({
        "forcePairing": false,
        "pairingType": "PROMPT",
        "manifest": {
            "manifestVersion": 1,
            "permissions": [
                "LAUNCH", "LAUNCH_WEBAPP", "APP_TO_APP", "CLOSE", "TEST_OPEN",
                "TEST_PROTECTED", "CONTROL_AUDIO", "CONTROL_DISPLAY",
                "CONTROL_INPUT_JOYSTICK", "CONTROL_INPUT_MEDIA_RECORDING",
                "CONTROL_INPUT_MEDIA_PLAYBACK", "CONTROL_INPUT_TV", "CONTROL_POWER",
                "CONTROL_TV_SCREEN", "READ_APP_STATUS", "READ_CURRENT_CHANNEL",
                "READ_INPUT_DEVICE_LIST", "READ_NETWORK_STATE", "READ_RUNNING_APPS",
                "READ_TV_CHANNEL_LIST", "WRITE_NOTIFICATION_TOAST", "READ_POWER_STATE",
                "READ_COUNTRY_INFO", "READ_SETTINGS", "CONTROL_INPUT_TEXT",
                "CONTROL_MOUSE_AND_KEYBOARD", "WRITE_SETTINGS",
                "WRITE_NOTIFICATION_ALERT", "READ_INSTALLED_APPS", "READ_UPDATE_INFO"
            ]
        }
    });
    if let Some(key) = session_key.filter(|key| !key.is_empty()) {
        payload["client-key"] = Value::String(key.to_owned());
    }
    json!({"type": "register", "id": REGISTER_ID, "payload": payload})
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RegistrationResponse {
    #[serde(rename = "type")]
    pub response_type: String,
    pub id: Option<Value>,
    #[serde(default)]
    pub payload: Value,
    pub error: Option<String>,
}

impl RegistrationResponse {
    pub fn is_registration_reply(&self) -> bool {
        self.id.as_ref().and_then(Value::as_str) == Some(REGISTER_ID)
    }
    pub fn client_key(&self) -> Option<&str> {
        self.payload.get("client-key").and_then(Value::as_str)
    }
}

async fn exchange(endpoint: &WebOsEndpoint, key: Option<&str>) -> Result<RegistrationResponse> {
    let url = endpoint.url();
    let (mut socket, _) = connect_async(&url)
        .await.with_context(|| format!("webOS WebSocket connection failed: {url}"))?;
    socket.send(Message::Text(registration_message(key).to_string())).await?;
    loop {
        let message = socket.next().await.context("webOS closed before registration response")??;
        match message {
            Message::Text(text) => {
                let response: RegistrationResponse =
                    serde_json::from_str(&text).context("invalid webOS registration JSON")?;
                if response.is_registration_reply() {
                    socket.close(None).await.ok();
                    return Ok(response);
                }
            }
            Message::Ping(data) => socket.send(Message::Pong(data)).await?,
            Message::Close(_) => anyhow::bail!("webOS closed connection during registration"),
            _ => {}
        }
    }
}

/// Register with the TV and persist its client key in the system state directory.
/// Key files are created with mode 0600 and replaced atomically.
pub async fn register_and_store(endpoint: &WebOsEndpoint, state_dir: &Path) -> Result<RegistrationResponse> {
    tokio::fs::create_dir_all(state_dir).await?;
    tokio::fs::set_permissions(state_dir, fs::Permissions::from_mode(0o700)).await?;
    let key_path = state_dir.join("webos-client-key");
    let stored_key = tokio::fs::read_to_string(&key_path).await.ok();
    if stored_key.is_some() {
        tokio::fs::set_permissions(&key_path, fs::Permissions::from_mode(0o600)).await?;
    }
    let mut response = exchange(endpoint, stored_key.as_deref()).await?;

    // An invalid stored key is commonly answered with a registration error.
    // Retry exactly once without it; never spin on a rejected pairing prompt.
    if response.response_type == "error" && stored_key.is_some() {
        response = exchange(endpoint, None).await?;
    }
    if response.response_type == "registered" {
        if let Some(key) = response.client_key() {
            persist_key(&key_path, key)?;
        } else if stored_key.is_none() {
            anyhow::bail!("TV registered without returning a client key");
        }
    } else {
        anyhow::bail!("webOS registration failed: {}", response.error.as_deref().unwrap_or("pairing rejected or unsupported response"));
    }
    Ok(response)
}

/// Register and query the current webOS power state. This is diagnostic only:
/// it never sends a power-toggle or screen-off command.
pub async fn query_power_state(endpoint: &WebOsEndpoint, state_dir: &Path) -> Result<String> {
    register_and_store(endpoint, state_dir).await?;
    let key_path = state_dir.join("webos-client-key");
    let key = tokio::fs::read_to_string(&key_path).await?;
    let url = endpoint.url();
    let (mut socket, _) = connect_async(&url)
        .await.with_context(|| format!("webOS WebSocket connection failed: {url}"))?;
    socket.send(Message::Text(registration_message(Some(key.trim())).to_string())).await?;

    loop {
        let message = socket.next().await.context("webOS closed during power-state query")??;
        match message {
            Message::Text(text) => {
                let value: Value = serde_json::from_str(&text).context("invalid webOS JSON response")?;
                let id = value.get("id").and_then(Value::as_str).unwrap_or_default();
                let kind = value.get("type").and_then(Value::as_str).unwrap_or_default();
                if id == REGISTER_ID && kind == "registered" {
                    let request = json!({
                        "type": "request",
                        "id": "getPowerState",
                        "uri": "ssap://com.webos.service.tvpower/power/getPowerState",
                        "payload": {}
                    });
                    socket.send(Message::Text(request.to_string())).await?;
                } else if id == REGISTER_ID && kind == "error" {
                    anyhow::bail!("webOS rejected the stored pairing key");
                } else if id == "getPowerState" && kind == "response" {
                    let state = value.get("payload").and_then(|p| p.get("state"))
                        .and_then(Value::as_str).context("webOS response has no power state")?;
                    let state = state.to_owned();
                    socket.close(None).await.ok();
                    return Ok(state);
                }
            }
            Message::Ping(data) => socket.send(Message::Pong(data)).await?,
            Message::Close(_) => anyhow::bail!("webOS closed connection before returning power state"),
            _ => {}
        }
    }
}

fn persist_key(path: &Path, key: &str) -> Result<()> {
    let tmp: PathBuf = path.with_extension("tmp");
    let mut file = fs::OpenOptions::new().write(true).create(true).truncate(true)
        .mode(0o600).open(&tmp)?;
    use std::io::Write;
    file.set_permissions(fs::Permissions::from_mode(0o600))?;
    file.write_all(key.as_bytes())?;
    file.sync_all()?;
    fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unpaired_registration_requests_prompt_without_key() {
        let frame = registration_message(None);
        assert_eq!(frame["type"], "register");
        assert_eq!(frame["id"], REGISTER_ID);
        assert_eq!(frame["payload"]["pairingType"], "PROMPT");
        assert_eq!(frame["payload"]["forcePairing"], false);
        assert!(frame["payload"].get("client-key").is_none());
    }
    #[test]
    fn paired_registration_includes_client_key() {
        assert_eq!(registration_message(Some("key"))["payload"]["client-key"], "key");
    }
    #[test]
    fn parses_registered_response_and_key() {
        let response: RegistrationResponse = serde_json::from_value(json!({
            "type": "registered", "id": REGISTER_ID, "payload": {"client-key": "abc"}
        })).unwrap();
        assert!(response.is_registration_reply());
        assert_eq!(response.client_key(), Some("abc"));
    }
    #[test]
    fn detects_registration_error() {
        let response: RegistrationResponse = serde_json::from_value(json!({
            "type": "error", "id": REGISTER_ID, "error": "rejected", "payload": {}
        })).unwrap();
        assert!(response.is_registration_reply());
        assert_eq!(response.response_type, "error");
    }
}
