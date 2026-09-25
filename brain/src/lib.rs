pub mod credentials;
pub mod enrollment;
pub mod http;
pub mod registry;
pub mod tasks;

use std::net::SocketAddr;

use futures_util::stream::SplitSink;
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

use hermes_protocol::{BrainMessage, WorkerMessage, PROTOCOL_VERSION};

use crate::credentials::{CredentialStore, VerifyResult};
use crate::enrollment::EnrollmentService;
use crate::registry::WorkerRegistry;
use crate::tasks::TaskService;

/// Generic error returned to a worker whose `hello` fails authentication. The
/// specific reason (unknown id vs. wrong secret) is only logged server-side so
/// the endpoint cannot be used to enumerate enrolled worker ids.
pub const HELLO_AUTH_FAILED: &str = "authentication failed: unknown worker or invalid credential";

/// Mask a token/secret for logging: keep at most the first 4 characters
/// (char-boundary safe, so non-ASCII input never panics).
pub fn mask_secret(secret: &str) -> String {
    if secret.chars().count() > 4 {
        let prefix: String = secret.chars().take(4).collect();
        format!("{prefix}****")
    } else {
        "****".to_string()
    }
}

type WsSink = SplitSink<WebSocketStream<TcpStream>, Message>;

async fn send_msg(sink: &mut WsSink, msg: &BrainMessage) {
    match serde_json::to_string(msg) {
        Ok(json) => {
            let _ = sink.send(Message::Text(json.into())).await;
        }
        Err(e) => eprintln!("hermes-brain: failed to serialize message: {e}"),
    }
}

/// Handle a single worker WebSocket connection. Exposed as public so
/// integration tests can call it without starting the full binary.
pub async fn handle_connection(
    stream: TcpStream,
    peer: SocketAddr,
    enrollment: EnrollmentService,
    credentials: CredentialStore,
    registry: WorkerRegistry,
    task_service: TaskService,
) -> Result<(), String> {
    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .map_err(|e| format!("websocket handshake failed: {e}"))?;

    println!("hermes-brain: [{peer}] connected");

    let (mut sink, mut stream) = ws_stream.split();
    // Set once this connection has authenticated (enroll or hello) and owns a
    // registry entry: (worker_id, connection_id).
    let mut session: Option<(String, u64)> = None;

    // Create a channel for sending tasks to this worker
    let (task_tx, mut task_rx) = mpsc::unbounded_channel::<BrainMessage>();

    loop {
        tokio::select! {
            // Incoming messages from worker
            msg = stream.next() => {
                let msg = match msg {
                    Some(Ok(Message::Text(text))) => text,
                    Some(Ok(Message::Close(_))) => break,
                    Some(Ok(_)) => continue,
                    Some(Err(e)) => {
                        eprintln!("hermes-brain: [{peer}] read error: {e}");
                        break;
                    }
                    None => break,
                };

                let worker_msg: WorkerMessage = match serde_json::from_str(&msg) {
                    Ok(m) => m,
                    Err(e) => {
                        send_msg(&mut sink, &BrainMessage::error(format!("invalid message: {e}"))).await;
                        continue;
                    }
                };

                match worker_msg {
                    WorkerMessage::Enroll {
                        protocol_version,
                        registration_token,
                        worker_id: wid,
                        role,
                        capabilities,
                    } => {
                        if protocol_version != PROTOCOL_VERSION {
                            send_msg(&mut sink, &BrainMessage::error(format!(
                                "unsupported protocol version: {protocol_version} (expected {PROTOCOL_VERSION})"
                            ))).await;
                            break;
                        }
                        if session.is_some() {
                            send_msg(&mut sink, &BrainMessage::error("connection already authenticated")).await;
                            continue;
                        }
                        if wid.trim().is_empty() {
                            send_msg(&mut sink, &BrainMessage::error("worker_id must not be empty")).await;
                            break;
                        }

                        let masked = mask_secret(&registration_token);

                        // Cheap pre-check so a duplicate does not burn a token.
                        if registry.is_connected(&wid).await {
                            eprintln!("hermes-brain: [{peer}] enrollment rejected: '{wid}' already connected");
                            send_msg(&mut sink, &BrainMessage::error(format!("worker '{wid}' is already connected"))).await;
                            break;
                        }

                        if !enrollment.consume_token(&registration_token).await {
                            eprintln!(
                                "hermes-brain: [{peer}] enrollment rejected (invalid token {masked})"
                            );
                            send_msg(&mut sink, &BrainMessage::error("invalid or expired registration token")).await;
                            break;
                        }

                        let connection_id = match registry
                            .register(wid.clone(), role.clone(), capabilities, task_tx.clone())
                            .await
                        {
                            Ok(id) => id,
                            Err(_) => {
                                eprintln!("hermes-brain: [{peer}] enrollment rejected: '{wid}' already connected");
                                send_msg(&mut sink, &BrainMessage::error(format!("worker '{wid}' is already connected"))).await;
                                break;
                            }
                        };

                        let secret = match credentials.issue(&wid, &role).await {
                            Ok(secret) => secret,
                            Err(e) => {
                                eprintln!("hermes-brain: [{peer}] failed to store credential for '{wid}': {e}");
                                registry.remove_connection(&wid, connection_id).await;
                                send_msg(&mut sink, &BrainMessage::error("internal error: could not store worker credential")).await;
                                break;
                            }
                        };

                        println!("hermes-brain: [{peer}] enrolled worker '{wid}' (token {masked})");
                        session = Some((wid.clone(), connection_id));
                        send_msg(&mut sink, &BrainMessage::enrollment_accepted(wid, secret)).await;
                    }

                    WorkerMessage::Hello {
                        protocol_version,
                        worker_id: wid,
                        worker_secret,
                        role,
                        capabilities,
                    } => {
                        if protocol_version != PROTOCOL_VERSION {
                            send_msg(&mut sink, &BrainMessage::error(format!(
                                "unsupported protocol version: {protocol_version} (expected {PROTOCOL_VERSION})"
                            ))).await;
                            break;
                        }
                        if session.is_some() {
                            send_msg(&mut sink, &BrainMessage::error("connection already authenticated")).await;
                            continue;
                        }

                        match credentials.verify(&wid, &worker_secret).await {
                            VerifyResult::Ok => {}
                            reason => {
                                eprintln!(
                                    "hermes-brain: [{peer}] hello rejected for '{wid}': {reason:?}"
                                );
                                send_msg(&mut sink, &BrainMessage::error(HELLO_AUTH_FAILED)).await;
                                break;
                            }
                        }

                        let connection_id = match registry
                            .register(wid.clone(), role.clone(), capabilities.clone(), task_tx.clone())
                            .await
                        {
                            Ok(id) => id,
                            Err(_) => {
                                eprintln!("hermes-brain: [{peer}] hello rejected: '{wid}' already connected");
                                send_msg(&mut sink, &BrainMessage::error(format!("worker '{wid}' is already connected"))).await;
                                break;
                            }
                        };

                        println!(
                            "hermes-brain: [{peer}] hello from '{wid}' role={role} caps={capabilities:?}"
                        );
                        session = Some((wid.clone(), connection_id));
                        send_msg(&mut sink, &BrainMessage::hello_accepted(wid)).await;
                    }

                    WorkerMessage::Heartbeat { timestamp_ms } => {
                        if let Some((ref wid, _)) = session {
                            registry.update_heartbeat(wid).await;
                            println!("hermes-brain: [{peer}] heartbeat from '{wid}' ts={timestamp_ms}");
                        }
                    }

                    WorkerMessage::TaskResult {
                        task_id,
                        success,
                        output,
                    } => {
                        let Some((ref wid, _)) = session else {
                            send_msg(&mut sink, &BrainMessage::error("not authenticated")).await;
                            continue;
                        };
                        if success {
                            println!(
                                "hermes-brain: [{peer}] task '{task_id}' from '{wid}' succeeded: {output}"
                            );
                        } else {
                            eprintln!(
                                "hermes-brain: [{peer}] task '{task_id}' from '{wid}' failed: {output}"
                            );
                        }

                        if !task_service.handle_worker_result(wid, &task_id, success, output).await {
                            eprintln!(
                                "hermes-brain: [{peer}] ignored result for task '{task_id}' from '{wid}' (unknown, not assigned to this worker, or already final)"
                            );
                        }
                    }
                }
            }

            // Outgoing tasks from brain to this worker
            task = task_rx.recv() => {
                match task {
                    Some(brain_msg) => {
                        let json = match serde_json::to_string(&brain_msg) {
                            Ok(json) => json,
                            Err(e) => {
                                eprintln!("hermes-brain: [{peer}] failed to serialize task: {e}");
                                continue;
                            }
                        };
                        if let Err(e) = sink.send(Message::Text(json.into())).await {
                            eprintln!("hermes-brain: [{peer}] failed to send task: {e}");
                            break;
                        }
                    }
                    None => {
                        // Channel closed, worker done
                        break;
                    }
                }
            }
        }
    }

    if let Some((wid, connection_id)) = &session {
        println!("hermes-brain: [{peer}] worker '{wid}' disconnected");
        // Only removes the entry if it still belongs to this connection.
        registry.remove_connection(wid, *connection_id).await;
    } else {
        println!("hermes-brain: [{peer}] disconnected (unauthenticated)");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::mask_secret;

    #[test]
    fn mask_keeps_four_chars() {
        assert_eq!(mask_secret("abcdefgh"), "abcd****");
        assert_eq!(mask_secret("abcd"), "****");
        assert_eq!(mask_secret(""), "****");
    }

    #[test]
    fn mask_is_safe_for_non_ascii() {
        // Byte-slicing `[..4]` would panic here (multi-byte chars).
        assert_eq!(mask_secret("ğüşiöç-token"), "ğüşi****");
        assert_eq!(mask_secret("🔑🔑🔑🔑🔑"), "🔑🔑🔑🔑****");
    }
}
