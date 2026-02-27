//! WebSocket transport to AIVory backend.

use crate::capture::ExceptionCapture;
use crate::config::Config;
use futures_util::{SinkExt, StreamExt};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};

/// WebSocket connection to the AIVory backend.
pub struct Connection {
    sender: RwLock<Option<mpsc::UnboundedSender<String>>>,
    connected: RwLock<bool>,
}

#[derive(Serialize)]
struct OutgoingMessage<T: Serialize> {
    #[serde(rename = "type")]
    msg_type: String,
    payload: T,
    timestamp: i64,
}

#[derive(Deserialize)]
struct IncomingMessage {
    #[serde(rename = "type")]
    msg_type: String,
    #[serde(default)]
    payload: serde_json::Value,
}

impl Connection {
    /// Creates a new connection.
    pub fn new() -> Self {
        Connection {
            sender: RwLock::new(None),
            connected: RwLock::new(false),
        }
    }

    /// Connects to the backend.
    pub async fn connect(&self, config: &Config) {
        let url = match url::Url::parse(&config.backend_url) {
            Ok(u) => u,
            Err(e) => {
                eprintln!("[AIVory Monitor] Invalid backend URL: {}", e);
                return;
            }
        };

        let config = config.clone();
        let sender_slot = Arc::new(RwLock::new(None::<mpsc::UnboundedSender<String>>));
        let connected = Arc::new(RwLock::new(false));

        let sender_slot_clone = sender_slot.clone();
        let connected_clone = connected.clone();

        tokio::spawn(async move {
            let mut reconnect_attempts = 0;
            let max_reconnect_attempts = 10;

            loop {
                match Self::connect_once(&url, &config, sender_slot_clone.clone(), connected_clone.clone()).await {
                    Ok(_) => {
                        reconnect_attempts = 0;
                    }
                    Err(e) => {
                        if config.debug {
                            eprintln!("[AIVory Monitor] Connection error: {}", e);
                        }
                    }
                }

                *connected_clone.write() = false;
                *sender_slot_clone.write() = None;

                reconnect_attempts += 1;
                if reconnect_attempts > max_reconnect_attempts {
                    eprintln!("[AIVory Monitor] Max reconnect attempts reached");
                    break;
                }

                let delay = Duration::from_secs(2u64.pow(reconnect_attempts.min(6)));
                if config.debug {
                    eprintln!(
                        "[AIVory Monitor] Reconnecting in {:?} (attempt {})",
                        delay, reconnect_attempts
                    );
                }
                tokio::time::sleep(delay).await;
            }
        });

        // Store references
        // Note: In a real implementation, we'd need better synchronization
    }

    async fn connect_once(
        url: &url::Url,
        config: &Config,
        sender_slot: Arc<RwLock<Option<mpsc::UnboundedSender<String>>>>,
        connected: Arc<RwLock<bool>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if config.debug {
            println!("[AIVory Monitor] Connecting to {}", url);
        }

        let (ws_stream, _) = connect_async(url.as_str()).await?;
        let (mut write, mut read) = ws_stream.split();

        if config.debug {
            println!("[AIVory Monitor] WebSocket connected");
        }

        // Create message channel
        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        *sender_slot.write() = Some(tx.clone());
        *connected.write() = true;

        // Send registration
        let register_msg = OutgoingMessage {
            msg_type: "register".to_string(),
            payload: serde_json::json!({
                "api_key": config.api_key,
                "agent_id": config.agent_id,
                "hostname": config.hostname,
                "environment": config.environment,
                "agent_version": "1.0.1",
                "runtime": "rust",
                "runtime_version": env!("CARGO_PKG_VERSION"),
                "platform": std::env::consts::OS,
                "arch": std::env::consts::ARCH,
            }),
            timestamp: chrono::Utc::now().timestamp_millis(),
        };

        let msg_json = serde_json::to_string(&register_msg)?;
        write.send(WsMessage::Text(msg_json)).await?;

        // Message handling loop
        let debug = config.debug;

        // Spawn sender task
        let mut write = write;
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if write.send(WsMessage::Text(msg)).await.is_err() {
                    break;
                }
            }
        });

        // Heartbeat
        let tx_heartbeat = tx.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                let heartbeat = OutgoingMessage {
                    msg_type: "heartbeat".to_string(),
                    payload: serde_json::json!({
                        "timestamp": chrono::Utc::now().timestamp_millis()
                    }),
                    timestamp: chrono::Utc::now().timestamp_millis(),
                };
                if let Ok(json) = serde_json::to_string(&heartbeat) {
                    if tx_heartbeat.send(json).is_err() {
                        break;
                    }
                }
            }
        });

        // Read messages
        while let Some(msg) = read.next().await {
            match msg {
                Ok(WsMessage::Text(text)) => {
                    if let Ok(incoming) = serde_json::from_str::<IncomingMessage>(&text) {
                        if debug {
                            println!("[AIVory Monitor] Received: {}", incoming.msg_type);
                        }

                        match incoming.msg_type.as_str() {
                            "registered" => {
                                if debug {
                                    println!("[AIVory Monitor] Agent registered");
                                }
                            }
                            "error" => {
                                let code = incoming.payload.get("code")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("unknown");
                                let message = incoming.payload.get("message")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("Unknown error");
                                eprintln!("[AIVory Monitor] Backend error: {} - {}", code, message);

                                if code == "auth_error" || code == "invalid_api_key" {
                                    eprintln!("[AIVory Monitor] Authentication failed");
                                    return Err("Authentication failed".into());
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Ok(WsMessage::Close(_)) => break,
                Err(e) => {
                    if debug {
                        eprintln!("[AIVory Monitor] WebSocket error: {}", e);
                    }
                    break;
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Disconnects from the backend.
    pub async fn disconnect(&self) {
        *self.sender.write() = None;
        *self.connected.write() = false;
    }

    /// Sends an exception capture.
    pub fn send_exception(&self, capture: ExceptionCapture) {
        let sender = self.sender.read();
        if let Some(tx) = sender.as_ref() {
            let msg = OutgoingMessage {
                msg_type: "exception".to_string(),
                payload: capture,
                timestamp: chrono::Utc::now().timestamp_millis(),
            };

            if let Ok(json) = serde_json::to_string(&msg) {
                let _ = tx.send(json);
            }
        }
    }

    /// Returns true if connected.
    pub fn is_connected(&self) -> bool {
        *self.connected.read()
    }
}

impl Default for Connection {
    fn default() -> Self {
        Self::new()
    }
}
