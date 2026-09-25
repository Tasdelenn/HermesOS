use std::path::PathBuf;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

use hermes_protocol::{BrainMessage, WorkerMessage};

use crate::capability::CapabilityRegistry;
use crate::state::{self, WorkerState};
use crate::WorkerConfig;

/// WebSocket connection to the HermesOS Brain.
pub struct WorkerClient {
    config: WorkerConfig,
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
    capabilities: CapabilityRegistry,
    worker_id: String,
    state_path: PathBuf,
}

/// Short, secret-free description of a Brain message for error output.
fn describe(msg: &BrainMessage) -> &'static str {
    match msg {
        BrainMessage::EnrollmentAccepted { .. } => "enrollment_accepted",
        BrainMessage::HelloAccepted { .. } => "hello_accepted",
        BrainMessage::Task { .. } => "task",
        BrainMessage::Error { .. } => "error",
    }
}

impl WorkerClient {
    /// Connect to the Brain, authenticate (enroll or hello), and return a
    /// ready-to-run client.
    pub async fn connect(config: WorkerConfig, state_path: PathBuf) -> Result<Self, String> {
        let capabilities = CapabilityRegistry::from_allowlist(&config.capabilities);

        let (ws, _) = connect_async(&config.brain_url)
            .await
            .map_err(|e| format!("failed to connect to {}: {e}", config.brain_url))?;

        let mut client = Self {
            config,
            ws,
            capabilities,
            worker_id: String::new(),
            state_path,
        };

        client.authenticate().await?;

        Ok(client)
    }

    /// Authenticate with the Brain (ADR-014):
    /// - if a state file with a stored credential exists, send `hello` with it;
    /// - otherwise, if a registration token is configured, `enroll` and persist
    ///   the issued credential;
    /// - otherwise fail with instructions.
    async fn authenticate(&mut self) -> Result<(), String> {
        let stored = state::load(&self.state_path)?;

        if let Some(stored) = stored {
            if stored.worker_id != self.config.worker_id {
                return Err(format!(
                    "state file {} belongs to worker '{}', but config says '{}'; delete the state file to re-enroll",
                    self.state_path.display(),
                    stored.worker_id,
                    self.config.worker_id
                ));
            }
            if self.config.registration_token.is_some() {
                println!(
                    "hermes-worker: stored credential found; ignoring registration_token (it can be removed from the config)"
                );
            }
            return self.hello(stored).await;
        }

        match self.config.registration_token.clone() {
            Some(token) => self.enroll(token).await,
            None => Err(format!(
                "worker is not enrolled: no credential at {} and no registration_token in config",
                self.state_path.display()
            )),
        }
    }

    async fn enroll(&mut self, token: String) -> Result<(), String> {
        let enroll = WorkerMessage::enroll(
            token,
            self.config.worker_id.clone(),
            self.config.role.clone(),
            self.config.capabilities.clone(),
        );
        self.send(&enroll).await?;

        match self.recv_brain().await? {
            BrainMessage::EnrollmentAccepted {
                worker_id,
                worker_secret,
            } => {
                if worker_id != self.config.worker_id {
                    return Err(format!("brain enrolled unexpected worker id '{worker_id}'"));
                }
                state::save(
                    &self.state_path,
                    &WorkerState {
                        worker_id: worker_id.clone(),
                        worker_secret,
                    },
                )
                .map_err(|e| {
                    format!("enrolled, but could not persist credential ({e}); a new registration token will be needed")
                })?;
                println!(
                    "hermes-worker: enrolled as '{worker_id}', credential saved to {}",
                    self.state_path.display()
                );
                self.worker_id = worker_id;
                Ok(())
            }
            BrainMessage::Error { message } => Err(format!("enrollment rejected: {message}")),
            other => Err(format!(
                "unexpected response to enroll: {}",
                describe(&other)
            )),
        }
    }

    async fn hello(&mut self, stored: WorkerState) -> Result<(), String> {
        let hello = WorkerMessage::hello(
            stored.worker_id,
            stored.worker_secret,
            self.config.role.clone(),
            self.config.capabilities.clone(),
        );
        self.send(&hello).await?;

        match self.recv_brain().await? {
            BrainMessage::HelloAccepted { worker_id } => {
                println!("hermes-worker: authenticated as '{worker_id}'");
                self.worker_id = worker_id;
                Ok(())
            }
            BrainMessage::Error { message } => Err(format!(
                "hello rejected: {message}. If this worker must re-enroll, delete {} and set a new registration_token",
                self.state_path.display()
            )),
            other => Err(format!(
                "unexpected response to hello: {}",
                describe(&other)
            )),
        }
    }

    /// Main run loop: spawn heartbeat and handle incoming tasks.
    pub async fn run(&mut self) -> Result<(), String> {
        let worker_id = self.worker_id.clone();
        let interval = Duration::from_secs(self.config.heartbeat_interval_secs);

        println!(
            "hermes-worker: connected as '{}', heartbeat every {}s",
            worker_id, self.config.heartbeat_interval_secs
        );

        loop {
            tokio::select! {
                msg = self.ws.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            match serde_json::from_str::<BrainMessage>(&text) {
                                Ok(brain_msg) => {
                                    self.handle_brain_message(brain_msg).await;
                                }
                                Err(e) => {
                                    eprintln!("hermes-worker: failed to parse brain message: {e}");
                                }
                            }
                        }
                        Some(Ok(Message::Close(_))) => {
                            println!("hermes-worker: brain closed connection");
                            return Ok(());
                        }
                        Some(Err(e)) => {
                            return Err(format!("websocket read error: {e}"));
                        }
                        None => {
                            println!("hermes-worker: websocket stream ended");
                            return Ok(());
                        }
                        _ => {} // Ignore binary/ping/pong
                    }
                }
                _ = tokio::time::sleep(interval) => {
                    let hb = WorkerMessage::heartbeat();
                    if let Err(e) = self.send(&hb).await {
                        return Err(format!("heartbeat send failed: {e}"));
                    }
                }
            }
        }
    }

    async fn handle_brain_message(&mut self, msg: BrainMessage) {
        match msg {
            BrainMessage::Task { .. } => {
                if let Some(response) = self.capabilities.execute_task(msg) {
                    if let Err(e) = self.send(&response).await {
                        eprintln!("hermes-worker: failed to send task result: {e}");
                    }
                }
            }
            BrainMessage::EnrollmentAccepted { .. } | BrainMessage::HelloAccepted { .. } => {
                // Already authenticated; ignore duplicate.
            }
            BrainMessage::Error { message } => {
                eprintln!("hermes-worker: brain error: {message}");
            }
        }
    }

    async fn send(&mut self, msg: &WorkerMessage) -> Result<(), String> {
        let json = serde_json::to_string(msg).map_err(|e| format!("serialize: {e}"))?;
        self.ws
            .send(Message::Text(json.into()))
            .await
            .map_err(|e| format!("send: {e}"))
    }

    async fn recv_brain(&mut self) -> Result<BrainMessage, String> {
        match self.ws.next().await {
            Some(Ok(Message::Text(text))) => {
                serde_json::from_str(&text).map_err(|e| format!("deserialize: {e}"))
            }
            Some(Ok(Message::Close(_))) => Err("brain closed connection".to_string()),
            Some(Err(e)) => Err(format!("websocket read error: {e}")),
            None => Err("websocket stream ended".to_string()),
            _ => Err("unexpected message type".to_string()),
        }
    }
}
