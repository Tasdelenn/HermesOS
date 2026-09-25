//! Integration tests: start a real Brain WebSocket + HTTP server, connect simulated
//! workers, and verify the enrollment, hello, heartbeat, and HTTP task dispatch flows end-to-end.

use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use hermes_protocol::{BrainMessage, WorkerMessage};
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;

use hermes_brain::enrollment::EnrollmentService;
use hermes_brain::http::{create_router, AppState, HealthResponse, WorkerInfo};
use hermes_brain::registry::WorkerRegistry;
use hermes_brain::tasks::{TaskRecord, TaskRegistry, TaskService, TaskStatus};

/// Spin up the Brain with WS and HTTP on OS-assigned ports (127.0.0.1:0).
async fn start_brain() -> (
    String,
    String,
    EnrollmentService,
    WorkerRegistry,
    TaskService,
) {
    let ws_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let ws_addr = ws_listener.local_addr().unwrap().to_string();

    let http_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let http_addr = http_listener.local_addr().unwrap().to_string();

    let enrollment = EnrollmentService::new();
    let worker_registry = WorkerRegistry::new();
    let task_registry = TaskRegistry::new();
    let task_service = TaskService::new(task_registry, worker_registry.clone());

    let e = enrollment.clone();
    let r = worker_registry.clone();
    let ts = task_service.clone();

    // Spawn HTTP API Server
    let app_state = AppState {
        task_service: task_service.clone(),
        started_at: Instant::now(),
    };
    let app = create_router(app_state);
    tokio::spawn(async move {
        let _ = axum::serve(http_listener, app).await;
    });

    // Spawn WebSocket Server
    tokio::spawn(async move {
        loop {
            let (stream, peer) = match ws_listener.accept().await {
                Ok(conn) => conn,
                Err(_) => break,
            };
            let e2 = e.clone();
            let r2 = r.clone();
            let ts2 = ts.clone();
            tokio::spawn(async move {
                let _ = hermes_brain::handle_connection(stream, peer, e2, r2, ts2).await;
            });
        }
    });

    // Give servers a moment to initialize
    tokio::time::sleep(Duration::from_millis(50)).await;

    (
        ws_addr,
        http_addr,
        enrollment,
        worker_registry,
        task_service,
    )
}

/// Connect a plain WebSocket client to the brain.
async fn connect_ws(
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
// WebSocket & Protocol Tests
// -----------------------------------------------------------------------

#[tokio::test]
async fn enrollment_happy_path() {
    let (ws_addr, _http_addr, enrollment, registry, _ts) = start_brain().await;
    enrollment.add_token("tok-secret-123".into()).await;

    let mut ws = connect_ws(&ws_addr).await;

    let enroll = WorkerMessage::enroll(
        "tok-secret-123".into(),
        "test-worker".into(),
        "development".into(),
        vec!["system.info".into()],
    );
    ws.send(send_json(&enroll)).await.unwrap();

    let reply = ws.next().await.unwrap().unwrap();
    let brain_msg = parse_brain(reply);
    assert_eq!(
        brain_msg,
        BrainMessage::enrollment_accepted("test-worker".into())
    );

    let workers = registry.list().await;
    assert_eq!(workers.len(), 1);
    assert_eq!(workers[0].worker_id, "test-worker");

    assert_eq!(enrollment.token_count().await, 0);
}

#[tokio::test]
async fn enrollment_rejects_bad_token() {
    let (ws_addr, _http_addr, _enrollment, _registry, _ts) = start_brain().await;

    let mut ws = connect_ws(&ws_addr).await;

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
async fn hello_registers_worker() {
    let (ws_addr, _http_addr, _enrollment, registry, _ts) = start_brain().await;

    let mut ws = connect_ws(&ws_addr).await;

    let hello = WorkerMessage::hello(
        "my-worker".into(),
        "iot".into(),
        vec!["homeassistant.control".into()],
    );
    ws.send(send_json(&hello)).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let workers = registry.list().await;
    assert_eq!(workers.len(), 1);
    assert_eq!(workers[0].worker_id, "my-worker");
}

#[tokio::test]
async fn heartbeat_updates_registry() {
    let (ws_addr, _http_addr, _enrollment, registry, _ts) = start_brain().await;

    let mut ws = connect_ws(&ws_addr).await;

    ws.send(send_json(&WorkerMessage::hello(
        "hb-worker".into(),
        "dev".into(),
        vec![],
    )))
    .await
    .unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    ws.send(send_json(&WorkerMessage::heartbeat()))
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let w = registry.get("hb-worker").await;
    assert!(w.is_some());
}

#[tokio::test]
async fn disconnect_removes_worker() {
    let (ws_addr, _http_addr, _enrollment, registry, _ts) = start_brain().await;

    let mut ws = connect_ws(&ws_addr).await;

    ws.send(send_json(&WorkerMessage::hello(
        "temp-worker".into(),
        "dev".into(),
        vec![],
    )))
    .await
    .unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(registry.list().await.len(), 1);

    ws.close(None).await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;

    assert_eq!(registry.list().await.len(), 0);
}

// -----------------------------------------------------------------------
// HTTP API & End-to-End Task Execution Tests
// -----------------------------------------------------------------------

#[tokio::test]
async fn http_health_endpoint() {
    let (_ws_addr, http_addr, _enrollment, _registry, _ts) = start_brain().await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{http_addr}/health"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), reqwest::StatusCode::OK);
    let health: HealthResponse = resp.json().await.unwrap();
    assert_eq!(health.status, "ok");
    assert_eq!(health.version, "0.1.0");
    assert_eq!(health.workers_connected, 0);
}

#[tokio::test]
async fn http_list_workers_endpoint() {
    let (ws_addr, http_addr, _enrollment, _registry, _ts) = start_brain().await;

    let mut ws = connect_ws(&ws_addr).await;
    ws.send(send_json(&WorkerMessage::hello(
        "http-worker".into(),
        "development".into(),
        vec!["system.info".into(), "files.read".into()],
    )))
    .await
    .unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{http_addr}/workers"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), reqwest::StatusCode::OK);
    let workers: Vec<WorkerInfo> = resp.json().await.unwrap();
    assert_eq!(workers.len(), 1);
    assert_eq!(workers[0].worker_id, "http-worker");
    assert_eq!(workers[0].role, "development");
    assert_eq!(workers[0].capabilities, vec!["system.info", "files.read"]);
}

#[tokio::test]
async fn http_task_dispatch_and_execution_loop() {
    let (ws_addr, http_addr, _enrollment, _registry, _ts) = start_brain().await;

    // 1. Connect simulated worker with system.info capability
    let mut ws = connect_ws(&ws_addr).await;
    ws.send(send_json(&WorkerMessage::hello(
        "executor-node-01".into(),
        "runner".into(),
        vec!["system.info".into()],
    )))
    .await
    .unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // 2. Dispatch task via HTTP POST /tasks
    let client = reqwest::Client::new();
    let post_resp = client
        .post(format!("http://{http_addr}/tasks"))
        .json(&json!({
            "capability": "system.info",
            "input": {}
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(post_resp.status(), reqwest::StatusCode::ACCEPTED);
    let task_record: TaskRecord = post_resp.json().await.unwrap();
    assert_eq!(task_record.worker_id, Some("executor-node-01".to_string()));
    assert_eq!(task_record.capability, "system.info");
    let task_id = task_record.task_id;

    // 3. Worker receives task via WebSocket
    let ws_msg = ws.next().await.unwrap().unwrap();
    let brain_msg = parse_brain(ws_msg);

    match brain_msg {
        BrainMessage::Task {
            task_id: rec_id,
            capability,
            input: _,
        } => {
            assert_eq!(rec_id, task_id);
            assert_eq!(capability, "system.info");

            // 4. Worker executes and sends back TaskResult
            let result_msg = WorkerMessage::TaskResult {
                task_id: rec_id,
                success: true,
                output: json!({
                    "operating_system": "windows",
                    "architecture": "x86_64"
                }),
            };
            ws.send(send_json(&result_msg)).await.unwrap();
        }
        other => panic!("expected Task message, got {other:?}"),
    }

    // 5. Client polls HTTP GET /tasks/:id
    tokio::time::sleep(Duration::from_millis(100)).await;

    let get_resp = client
        .get(format!("http://{http_addr}/tasks/{task_id}"))
        .send()
        .await
        .unwrap();

    assert_eq!(get_resp.status(), reqwest::StatusCode::OK);
    let completed_task: TaskRecord = get_resp.json().await.unwrap();

    assert_eq!(completed_task.task_id, task_id);
    assert_eq!(completed_task.status, TaskStatus::Completed);
    assert_eq!(
        completed_task.result,
        Some(json!({
            "operating_system": "windows",
            "architecture": "x86_64"
        }))
    );
}

#[tokio::test]
async fn http_task_dispatch_fails_when_no_worker_available() {
    let (_ws_addr, http_addr, _enrollment, _registry, _ts) = start_brain().await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{http_addr}/tasks"))
        .json(&json!({
            "capability": "non.existent.capability",
            "input": {}
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), reqwest::StatusCode::BAD_REQUEST);
    let body: Value = resp.json().await.unwrap();
    assert!(body["error"]
        .as_str()
        .unwrap()
        .contains("no worker available with capability"));
}

#[tokio::test]
async fn task_idempotency_ignores_duplicate_results() {
    let (ws_addr, http_addr, _enrollment, _registry, _ts) = start_brain().await;

    let mut ws = connect_ws(&ws_addr).await;
    ws.send(send_json(&WorkerMessage::hello(
        "idempotent-worker".into(),
        "runner".into(),
        vec!["system.info".into()],
    )))
    .await
    .unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let post_resp = client
        .post(format!("http://{http_addr}/tasks"))
        .json(&json!({
            "capability": "system.info",
            "input": {}
        }))
        .send()
        .await
        .unwrap();

    let task_record: TaskRecord = post_resp.json().await.unwrap();
    let task_id = task_record.task_id;

    // Worker receives task
    let _ = ws.next().await.unwrap().unwrap();

    // Send first TaskResult
    ws.send(send_json(&WorkerMessage::TaskResult {
        task_id: task_id.clone(),
        success: true,
        output: json!({"attempt": 1}),
    }))
    .await
    .unwrap();

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Send duplicate TaskResult
    ws.send(send_json(&WorkerMessage::TaskResult {
        task_id: task_id.clone(),
        success: true,
        output: json!({"attempt": 2}),
    }))
    .await
    .unwrap();

    tokio::time::sleep(Duration::from_millis(50)).await;

    let get_resp = client
        .get(format!("http://{http_addr}/tasks/{task_id}"))
        .send()
        .await
        .unwrap();

    let task: TaskRecord = get_resp.json().await.unwrap();
    assert_eq!(task.status, TaskStatus::Completed);
    // Should preserve first result
    assert_eq!(task.result, Some(json!({"attempt": 1})));
}
