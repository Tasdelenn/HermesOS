use std::env;
use std::io::Write;
use std::net::SocketAddr;
use std::time::Instant;

use hermes_brain::enrollment::EnrollmentService;
use hermes_brain::http::{create_router, AppState};
use hermes_brain::registry::WorkerRegistry;
use hermes_brain::tasks::{TaskRegistry, TaskService};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let ws_addr: SocketAddr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:9000".to_string())
        .parse()
        .expect("invalid websocket address");

    let http_addr: SocketAddr = env::args()
        .nth(2)
        .unwrap_or_else(|| "127.0.0.1:9001".to_string())
        .parse()
        .expect("invalid http address");

    let enrollment = EnrollmentService::new();
    let worker_registry = WorkerRegistry::new();
    let task_registry = TaskRegistry::new();
    let task_service = TaskService::new(task_registry, worker_registry.clone());

    // Seed tokens from CLI: --token abc --token def
    let mut args = env::args().skip(3);
    while let Some(flag) = args.next() {
        if flag == "--token" {
            if let Some(token) = args.next() {
                enrollment.add_token(token).await;
            }
        }
    }

    let token_count = enrollment.token_count().await;
    eprintln!("hermes-brain: Starting services...");
    eprintln!("hermes-brain: WebSocket listening on ws://{ws_addr}");
    eprintln!("hermes-brain: HTTP API listening on http://{http_addr}");
    eprintln!("hermes-brain: {token_count} registration token(s) loaded");
    std::io::stderr().flush().ok();

    // Bind HTTP listener
    let http_listener = TcpListener::bind(http_addr).await.unwrap_or_else(|e| {
        eprintln!("hermes-brain: failed to bind HTTP {http_addr}: {e}");
        std::process::exit(1);
    });

    let app_state = AppState {
        task_service: task_service.clone(),
        started_at: Instant::now(),
    };
    let app = create_router(app_state);

    // Spawn HTTP API server
    let http_task = tokio::spawn(async move {
        if let Err(e) = axum::serve(http_listener, app).await {
            eprintln!("hermes-brain: HTTP server error: {e}");
        }
    });

    // Bind WebSocket listener
    let ws_listener = TcpListener::bind(ws_addr).await.unwrap_or_else(|e| {
        eprintln!("hermes-brain: failed to bind WS {ws_addr}: {e}");
        std::process::exit(1);
    });

    eprintln!("hermes-brain: Services initialized successfully");
    std::io::stderr().flush().ok();

    // WS accept loop
    while let Ok((stream, peer)) = ws_listener.accept().await {
        let enrollment = enrollment.clone();
        let registry = worker_registry.clone();
        let task_service = task_service.clone();

        tokio::spawn(async move {
            if let Err(e) =
                hermes_brain::handle_connection(stream, peer, enrollment, registry, task_service)
                    .await
            {
                eprintln!("hermes-brain: [{peer}] connection error: {e}");
            }
        });
    }

    let _ = http_task.await;
}
