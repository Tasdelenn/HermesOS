use std::time::Instant;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tower_http::cors::{Any, CorsLayer};

use crate::tasks::{TaskRecord, TaskService};

#[derive(Clone)]
pub struct AppState {
    pub task_service: TaskService,
    pub started_at: Instant,
}

#[derive(Debug, Deserialize)]
pub struct SendTaskRequest {
    pub capability: String,
    #[serde(default)]
    pub input: Value,
    #[serde(default)]
    pub worker_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkerInfo {
    pub worker_id: String,
    pub role: String,
    pub capabilities: Vec<String>,
    pub connected_at_ms: u64,
    pub last_heartbeat_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_secs: u64,
    pub workers_connected: usize,
    pub tasks_count: usize,
}

#[derive(Debug, Deserialize)]
pub struct ListTasksQuery {
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    50
}

pub fn create_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/health", get(health_handler))
        .route("/workers", get(list_workers_handler))
        .route("/tasks", post(send_task_handler).get(list_tasks_handler))
        .route("/tasks/:id", get(get_task_handler))
        .layer(cors)
        .with_state(state)
}

async fn health_handler(State(state): State<AppState>) -> Json<HealthResponse> {
    let workers = state.task_service.worker_registry().list().await;
    let tasks = state.task_service.task_registry().list_tasks(0).await;
    let uptime = state.started_at.elapsed().as_secs();

    Json(HealthResponse {
        status: "ok".to_string(),
        version: "0.1.0".to_string(),
        uptime_secs: uptime,
        workers_connected: workers.len(),
        tasks_count: tasks.len(),
    })
}

async fn list_workers_handler(State(state): State<AppState>) -> Json<Vec<WorkerInfo>> {
    let workers = state.task_service.worker_registry().list().await;
    let now = Instant::now();

    Json(
        workers
            .into_iter()
            .map(|w| WorkerInfo {
                worker_id: w.worker_id,
                role: w.role,
                capabilities: w.capabilities,
                connected_at_ms: now.saturating_duration_since(w.connected_at).as_millis() as u64,
                last_heartbeat_ms: now.saturating_duration_since(w.last_heartbeat).as_millis()
                    as u64,
            })
            .collect(),
    )
}

async fn send_task_handler(
    State(state): State<AppState>,
    Json(req): Json<SendTaskRequest>,
) -> Result<(StatusCode, Json<TaskRecord>), (StatusCode, Json<Value>)> {
    match state
        .task_service
        .dispatch(req.capability, req.input, req.worker_id)
        .await
    {
        Ok(record) => Ok((StatusCode::ACCEPTED, Json(record))),
        Err(err) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": err
            })),
        )),
    }
}

async fn list_tasks_handler(
    State(state): State<AppState>,
    Query(query): Query<ListTasksQuery>,
) -> Json<Vec<TaskRecord>> {
    let tasks = state
        .task_service
        .task_registry()
        .list_tasks(query.limit)
        .await;
    Json(tasks)
}

async fn get_task_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<TaskRecord>, (StatusCode, Json<Value>)> {
    match state.task_service.task_registry().get_task(&id).await {
        Some(record) => Ok(Json(record)),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": format!("task '{}' not found", id)
            })),
        )),
    }
}
