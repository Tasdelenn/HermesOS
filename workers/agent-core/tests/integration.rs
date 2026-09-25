//! Integration tests: start a real Brain WebSocket server, connect a simulated
//! worker, and verify enrollment + heartbeat flows end-to-end. These tests
//! exercise the worker's protocol understanding against a real Brain.

use futures_util::{SinkExt, StreamExt};
use hermes_protocol::{BrainMessage, WorkerMessage};
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;

/// Spin up the brain on an OS-assigned port and return (addr, enrollment, registry).
async fn start_brain() -> (
    String,
    hermes_brain::enrollment::EnrollmentService,
    hermes_brain::registry::WorkerRegistry,
) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    let enrollment = hermes_brain::enrollment::EnrollmentService::new();
    let credentials = hermes_brain::credentials::CredentialStore::in_memory();
    let registry = hermes_brain::registry::WorkerRegistry::new();
    let task_registry = hermes_brain::tasks::TaskRegistry::new();
    let task_service = hermes_brain::tasks::TaskService::new(task_registry, registry.clone());

    let e = enrollment.clone();
    let c = credentials.clone();
    let r = registry.clone();
    let ts = task_service.clone();

    tokio::spawn(async move {
        loop {
            let (stream, peer) = listener.accept().await.unwrap();
            let e2 = e.clone();
            let c2 = c.clone();
            let r2 = r.clone();
            let ts2 = ts.clone();
            tokio::spawn(async move {
                let _ = hermes_brain::handle_connection(stream, peer, e2, c2, r2, ts2).await;
            });
        }
    });

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    (addr, enrollment, registry)
}

/// Connect a plain WebSocket client to the brain.
async fn connect(
    addr: &str,
) -> tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>> {
    let (ws, _) = tokio_tungstenite::connect_async(format!("ws://{addr}"))
        .await
        .unwrap();
    ws
}

fn send_json(msg: &WorkerMessage) -> Message {
    Message::Text(serde_json::to_string(msg).unwrap().into())
}

fn parse_brain(msg: Message) -> BrainMessage {
    match msg {
        Message::Text(t) => serde_json::from_str(&t).unwrap(),
        other => panic!("expected text, got {other:?}"),
    }
}

// -----------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------

#[tokio::test]
async fn worker_enrolls_with_valid_token() {
    let (addr, enrollment, registry) = start_brain().await;
    enrollment.add_token("tok-worker-test".into()).await;

    let mut ws = connect(&addr).await;

    // Worker sends Enroll with the right protocol version
    let enroll = WorkerMessage::enroll(
        "tok-worker-test".into(),
        "worker-test-01".into(),
        "development".into(),
        vec!["system.info".into()],
    );
    ws.send(send_json(&enroll)).await.unwrap();

    // Expect enrollment_accepted carrying the per-worker credential
    let reply = ws.next().await.unwrap().unwrap();
    match parse_brain(reply) {
        BrainMessage::EnrollmentAccepted {
            worker_id,
            worker_secret,
        } => {
            assert_eq!(worker_id, "worker-test-01");
            assert!(!worker_secret.is_empty());
        }
        other => panic!("expected enrollment_accepted, got {other:?}"),
    }

    // Worker should be in registry
    let workers = registry.list().await;
    assert_eq!(workers.len(), 1);
    assert_eq!(workers[0].worker_id, "worker-test-01");

    // Token should be consumed
    assert_eq!(enrollment.token_count().await, 0);
}

#[tokio::test]
async fn worker_enrollment_rejected_with_bad_token() {
    let (addr, _enrollment, _registry) = start_brain().await;

    let mut ws = connect(&addr).await;

    let enroll = WorkerMessage::enroll(
        "wrong-token".into(),
        "bad-worker".into(),
        "development".into(),
        vec![],
    );
    ws.send(send_json(&enroll)).await.unwrap();

    let reply = ws.next().await.unwrap().unwrap();
    let brain_msg = parse_brain(reply);
    assert!(matches!(brain_msg, BrainMessage::Error { .. }));
}

#[tokio::test]
async fn worker_hello_without_enrollment_is_rejected() {
    let (addr, _enrollment, registry) = start_brain().await;

    let mut ws = connect(&addr).await;

    let hello = WorkerMessage::hello(
        "hello-worker".into(),
        "not-a-real-secret".into(),
        "iot".into(),
        vec!["homeassistant.control".into()],
    );
    ws.send(send_json(&hello)).await.unwrap();

    let reply = parse_brain(ws.next().await.unwrap().unwrap());
    assert!(matches!(reply, BrainMessage::Error { .. }));

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert!(registry.list().await.is_empty());
}

#[tokio::test]
async fn worker_reconnects_with_issued_secret() {
    let (addr, enrollment, registry) = start_brain().await;
    enrollment.add_token("tok-reconnect".into()).await;

    let mut ws = connect(&addr).await;
    ws.send(send_json(&WorkerMessage::enroll(
        "tok-reconnect".into(),
        "reconnect-worker".into(),
        "dev".into(),
        vec!["system.info".into()],
    )))
    .await
    .unwrap();
    let secret = match parse_brain(ws.next().await.unwrap().unwrap()) {
        BrainMessage::EnrollmentAccepted { worker_secret, .. } => worker_secret,
        other => panic!("expected enrollment_accepted, got {other:?}"),
    };
    ws.close(None).await.unwrap();
    for _ in 0..50 {
        if registry.get("reconnect-worker").await.is_none() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }

    let mut ws = connect(&addr).await;
    ws.send(send_json(&WorkerMessage::hello(
        "reconnect-worker".into(),
        secret,
        "dev".into(),
        vec!["system.info".into()],
    )))
    .await
    .unwrap();
    assert_eq!(
        parse_brain(ws.next().await.unwrap().unwrap()),
        BrainMessage::hello_accepted("reconnect-worker".into())
    );
    assert!(registry.get("reconnect-worker").await.is_some());
}

#[tokio::test]
async fn worker_sends_heartbeat_after_enrollment() {
    let (addr, enrollment, registry) = start_brain().await;
    enrollment.add_token("tok-hb".into()).await;

    let mut ws = connect(&addr).await;

    // Enroll first
    ws.send(send_json(&WorkerMessage::enroll(
        "tok-hb".into(),
        "hb-worker".into(),
        "dev".into(),
        vec![],
    )))
    .await
    .unwrap();

    // Consume enrollment_accepted
    let _ = ws.next().await.unwrap().unwrap();

    // Send heartbeat
    ws.send(send_json(&WorkerMessage::heartbeat()))
        .await
        .unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Worker still registered
    let w = registry.get("hb-worker").await;
    assert!(w.is_some());
}

#[tokio::test]
async fn worker_disconnect_cleans_up() {
    let (addr, enrollment, registry) = start_brain().await;
    enrollment.add_token("tok-temp".into()).await;

    let mut ws = connect(&addr).await;

    ws.send(send_json(&WorkerMessage::enroll(
        "tok-temp".into(),
        "temp-worker".into(),
        "dev".into(),
        vec![],
    )))
    .await
    .unwrap();

    let _ = ws.next().await.unwrap().unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    assert_eq!(registry.list().await.len(), 1);

    // Close connection
    ws.close(None).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    assert_eq!(registry.list().await.len(), 0);
}
