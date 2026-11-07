//! LG webOS SSAP registration and transport primitives.
//!
//! Protocol behavior inspected in the MIT-licensed upstream project:
//! https://github.com/JPersson77/LGTVCompanion
//!
//! This module implements connection + registration parsing only. Power
//! operations remain unavailable until the complete state machine is tested.

use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
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

/// Build the SSAP registration frame. A missing key requests TV-side pairing.
pub fn registration_message(session_key: Option<&str>) -> Value {
    let mut payload = json!({
        "forcePairing": false,
        "pairingType": "PROMPT",
        "manifest": {
            "manifestVersion": 1,
            "permissions": [
                "LAUNCH", "LAUNCH_WEBAPP", "APP_TO_APP", "CLOSE",
                "TEST_OPEN", "TEST_PROTECTED", "CONTROL_AUDIO",
                "CONTROL_DISPLAY", "CONTROL_INPUT_JOYSTICK",
                "CONTROL_INPUT_MEDIA_RECORDING", "CONTROL_INPUT_MEDIA_PLAYBACK",
                "CONTROL_INPUT_TV", "CONTROL_POWER", "CONTROL_TV_SCREEN",
                "READ_APP_STATUS", "READ_CURRENT_CHANNEL",
                "READ_INPUT_DEVICE_LIST", "READ_NETWORK_STATE",
                "READ_RUNNING_APPS", "READ_TV_CHANNEL_LIST",
                "WRITE_NOTIFICATION_TOAST", "READ_POWER_STATE",
                "READ_COUNTRY_INFO", "READ_SETTINGS", "CONTROL_INPUT_TEXT",
                "CONTROL_MOUSE_AND_KEYBOARD", "WRITE_SETTINGS",
                "WRITE_NOTIFICATION_ALERT", "READ_INSTALLED_APPS",
                "READ_UPDATE_INFO"
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

/// Connect and submit registration. Caller must persist any returned key safely.
pub async fn register(endpoint: &WebOsEndpoint, session_key: Option<&str>) -> Result<RegistrationResponse> {
    let url = endpoint.url();
    let (mut socket, _) = connect_async(&url)
        .await
        .with_context(|| format!("webOS WebSocket connection failed: {url}"))?;

    socket.send(Message::Text(registration_message(session_key).to_string().into())).await?;
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
                tracing::debug!("ignoring non-registration webOS frame during pairing");
            }
            Message::Ping(data) => socket.send(Message::Pong(data)).await?,
            Message::Close(_) => anyhow::bail!("webOS closed connection during registration"),
            _ => {}
        }
    }
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
        let frame = registration_message(Some("secret-session-key"));
        assert_eq!(frame["payload"]["client-key"], "secret-session-key");
    }

    #[test]
    fn parses_registered_response_and_key() {
        let response: RegistrationResponse = serde_json::from_value(json!({
            "type": "registered", "id": REGISTER_ID,
            "payload": {"client-key": "abc123"}
        })).unwrap();
        assert!(response.is_registration_reply());
        assert_eq!(response.client_key(), Some("abc123"));
    }

    #[test]
    fn detects_registration_error() {
        let response: RegistrationResponse = serde_json::from_value(json!({
            "type": "error", "id": REGISTER_ID,
            "error": "pairing rejected", "payload": {}
        })).unwrap();
        assert!(response.is_registration_reply());
        assert_eq!(response.response_type, "error");
    }
}
