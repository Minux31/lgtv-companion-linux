//! LG webOS connection layer.
//!
//! This module is intentionally conservative: the initial transport is not yet
//! a complete implementation of the upstream pairing/state machine. Do not send
//! power commands until pairing, request correlation, reconnect and state
//! handling have been implemented and tested against real hardware.

use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio_tungstenite::{connect_async, tungstenite::Message};

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

/// Opens a WebSocket and performs a protocol-level registration request.
/// Pairing prompt handling and client-key persistence are still TODO.
pub async fn probe(endpoint: &WebOsEndpoint) -> Result<()> {
    let url = endpoint.url();
    let (mut socket, _) = connect_async(&url)
        .await
        .with_context(|| format!("webOS WebSocket connection failed: {url}"))?;

    let registration: Value = json!({
        "type": "register",
        "id": "lgtv-companion-linux-register",
        "payload": {
            " pairingType": "PROMPT",
            "manifest": {
                "manifestVersion": 1,
                "appVersion": "0.1.0",
                "signed": {
                    "created": "20260101000000",
                    "appId": "com.minux31.lgtv-companion-linux",
                    "vendorId": "Minux31",
                    "localizedAppNames": { "": "LGTV Companion Linux" },
                    "localized事Names": { "": "LGTV Companion Linux" },
                    "permissions": [
                        "CONTROL_POWER", "READ_POWER_STATE",
                        "CONTROL_INPUT_TEXT", "CONTROL_AUDIO"
                    ],
                    "serial": "lgtv-companion-linux"
                },
                "permissions": ["CONTROL_POWER", "READ_POWER_STATE"]
            }
        }
    });
    socket.send(Message::Text(registration.to_string().into())).await?;
    if let Some(message) = socket.next().await {
        let message = message?;
        if let Message::Text(text) = message {
            let parsed: Value = serde_json::from_str(&text).context("invalid webOS JSON response")?;
            tracing::debug!(response_type = ?parsed.get("type"), "received webOS registration response");
        }
    }
    socket.close(None).await.ok();
    Ok(())
}
