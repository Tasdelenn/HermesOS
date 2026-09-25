pub mod enrollment;
pub mod http;
pub mod registry;
pub mod tasks;

use std::net::SocketAddr;

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

use hermes_protocol::{BrainMessage, WorkerMessage, PROTOCOL_VERSION};

use crate::enrollment::EnrollmentService;
use crate::registry::WorkerRegistry;
use crate::tasks::TaskService;

/// Handle a single worker WebSocket connection. Exposed as public so
/// integration tests can call it without starting the full binary.
pub async fn handle_connection(
    stream: TcpStream,
    peer: SocketAddr,
    enrollment: EnrollmentService,
    registry: WorkerRegistry,
    task_service: TaskService,
) -> Result<(), String> {
    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .map_err(|e| format!("websocket handshake failed: {e}"))?;

    println!("hermes-brain: [{peer}] connected");

    let (mut sink, mut stream) = ws_stream.split();
    let mut worker_id: Option<String> = None;

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
                        let err = BrainMessage::error(format!("invalid message: {e}"));
                        let _ = sink
                            .send(Message::Text(serde_json::to_string(&err).unwrap().into()))
                            .await;
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
                            let err = BrainMessage::error(format!(
                                "unsupported protocol version: {protocol_version}"
                            ));
                            let _ = sink
                                .send(Message::Text(serde_json::to_string(&err).unwrap().into()))
                                .await;
                            break;
                        }

                        let masked = if registration_token.len() > 4 {
                            format!("{}****", &registration_token[..4])
                        } else {
                            "****".to_string()
                        };

                        if !enrollment.consume_token(&registration_token).await {
                            eprintln!(
                                "hermes-brain: [{peer}] enrollment rejected (invalid token {masked})"
                            );
                            let err = BrainMessage::error("invalid or expired registration token");
                            let _ = sink
                                .send(Message::Text(serde_json::to_string(&err).unwrap().into()))
                                .await;
                            break;
                        }

                        println!("hermes-brain: [{peer}] enrolled worker '{wid}' (token {masked})");
                        registry.register(wid.clone(), role, capabilities, task_tx.clone()).await;
                        worker_id = Some(wid.clone());

                        let reply = BrainMessage::enrollment_accepted(wid);
                        let _ = sink
                            .send(Message::Text(serde_json::to_string(&reply).unwrap().into()))
                            .await;
                    }

                    WorkerMessage::Hello {
                        protocol_version,
                        worker_id: wid,
                        role,
                        capabilities,
                    } => {
                        if protocol_version != PROTOCOL_VERSION {
                            let err = BrainMessage::error(format!(
                                "unsupported protocol version: {protocol_version}"
                            ));
                            let _ = sink
                                .send(Message::Text(serde_json::to_string(&err).unwrap().into()))
                                .await;
                            break;
                        }

                        println!(
                            "hermes-brain: [{peer}] hello from '{wid}' role={role} caps={capabilities:?}"
                        );
                        registry.register(wid.clone(), role, capabilities, task_tx.clone()).await;
                        worker_id = Some(wid);
                    }

                    WorkerMessage::Heartbeat { timestamp_ms } => {
                        if let Some(ref wid) = worker_id {
                            registry.update_heartbeat(wid).await;
                            println!("hermes-brain: [{peer}] heartbeat from '{wid}' ts={timestamp_ms}");
                        }
                    }

                    WorkerMessage::TaskResult {
                        task_id,
                        success,
                        output,
                    } => {
                        let wid = worker_id.as_deref().unwrap_or("unknown");
                        if success {
                            println!(
                                "hermes-brain: [{peer}] task '{task_id}' from '{wid}' succeeded: {output}"
                            );
                        } else {
                            eprintln!(
                                "hermes-brain: [{peer}] task '{task_id}' from '{wid}' failed: {output}"
                            );
                        }

                        task_service.handle_worker_result(&task_id, success, output).await;
                    }
                }
            }

            // Outgoing tasks from brain to this worker
            task = task_rx.recv() => {
                match task {
                    Some(brain_msg) => {
                        let json = serde_json::to_string(&brain_msg).unwrap();
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

    if let Some(wid) = &worker_id {
        println!("hermes-brain: [{peer}] worker '{wid}' disconnected");
        registry.remove(wid).await;
    } else {
        println!("hermes-brain: [{peer}] disconnected (unauthenticated)");
    }

    Ok(())
}
