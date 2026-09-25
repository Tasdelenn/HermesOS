use std::env;
use std::io::Write;
use std::net::SocketAddr;
use std::time::Instant;

use hermes_brain::credentials::CredentialStore;
use hermes_brain::enrollment::EnrollmentService;
use hermes_brain::http::{create_router, AppState};
use hermes_brain::registry::WorkerRegistry;
use hermes_brain::tasks::{TaskRegistry, TaskService};
use tokio::net::TcpListener;

const DEFAULT_CREDENTIALS_FILE: &str = "config/brain_credentials.json";

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

    // Flags after the two positional addresses:
    //   --token <t>               seed a single-use registration token (repeatable)
    //   --credentials-file <path> where per-worker credential digests are kept
    //                             (default: config/brain_credentials.json, git-ignored)
    //   --no-credentials-file     keep credentials in memory only (workers must
    //                             re-enroll after every Brain restart)
    let mut credentials_path = Some(String::from(DEFAULT_CREDENTIALS_FILE));
    let mut args = env::args().skip(3);
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--token" => {
                if let Some(token) = args.next() {
                    enrollment.add_token(token).await;
                }
            }
            "--credentials-file" => match args.next() {
                Some(path) => credentials_path = Some(path),
                None => {
                    eprintln!("hermes-brain: --credentials-file requires a path");
                    std::process::exit(2);
                }
            },
            "--no-credentials-file" => credentials_path = None,
            other => {
                eprintln!("hermes-brain: unknown argument: {other}");
                std::process::exit(2);
            }
        }
    }

    let credentials = match &credentials_path {
        Some(path) => CredentialStore::with_file(path).unwrap_or_else(|e| {
            eprintln!("hermes-brain: failed to load credentials: {e}");
            std::process::exit(1);
        }),
        None => CredentialStore::in_memory(),
    };

    let token_count = enrollment.token_count().await;
    eprintln!("hermes-brain: Starting services...");
    eprintln!("hermes-brain: WebSocket listening on ws://{ws_addr}");
    eprintln!("hermes-brain: HTTP API listening on http://{http_addr}");
    eprintln!("hermes-brain: {token_count} registration token(s) loaded");
    match &credentials_path {
        Some(path) => eprintln!(
            "hermes-brain: {} enrolled worker credential(s) loaded from {path}",
            credentials.len().await
        ),
        None => eprintln!(
            "hermes-brain: credentials are in-memory only; workers must re-enroll after restart"
        ),
    }
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
        let credentials = credentials.clone();
        let task_service = task_service.clone();

        tokio::spawn(async move {
            if let Err(e) = hermes_brain::handle_connection(
                stream,
                peer,
                enrollment,
                credentials,
                registry,
                task_service,
            )
            .await
            {
                eprintln!("hermes-brain: [{peer}] connection error: {e}");
            }
        });
    }

    let _ = http_task.await;
}
