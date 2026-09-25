use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use hermes_protocol::BrainMessage;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::registry::WorkerRegistry;

fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Lifecycle states of a task in HermesOS.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Timeout,
    Cancelled,
}

impl TaskStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            TaskStatus::Completed
                | TaskStatus::Failed
                | TaskStatus::Timeout
                | TaskStatus::Cancelled
        )
    }
}

/// Detailed representation of a task record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskRecord {
    pub task_id: String,
    pub worker_id: Option<String>,
    pub capability: String,
    pub input: Value,
    pub status: TaskStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

/// Persistence-independent trait for storing and querying tasks.
pub trait TaskStore: Send + Sync {
    fn create(&self, task: TaskRecord) -> impl std::future::Future<Output = TaskRecord> + Send;
    fn get(&self, task_id: &str) -> impl std::future::Future<Output = Option<TaskRecord>> + Send;
    fn list(&self, limit: usize) -> impl std::future::Future<Output = Vec<TaskRecord>> + Send;
    fn update_status(
        &self,
        task_id: &str,
        status: TaskStatus,
        result: Option<Value>,
        error: Option<String>,
    ) -> impl std::future::Future<Output = bool> + Send;
}

/// In-memory implementation of TaskStore.
#[derive(Debug, Clone, Default)]
pub struct InMemoryTaskStore {
    tasks: Arc<Mutex<HashMap<String, TaskRecord>>>,
}

impl InMemoryTaskStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl TaskStore for InMemoryTaskStore {
    async fn create(&self, task: TaskRecord) -> TaskRecord {
        let mut map = self.tasks.lock().await;
        map.insert(task.task_id.clone(), task.clone());
        task
    }

    async fn get(&self, task_id: &str) -> Option<TaskRecord> {
        let map = self.tasks.lock().await;
        map.get(task_id).cloned()
    }

    async fn list(&self, limit: usize) -> Vec<TaskRecord> {
        let map = self.tasks.lock().await;
        let mut list: Vec<TaskRecord> = map.values().cloned().collect();
        // Sort descending by created_at_ms
        list.sort_by(|a, b| b.created_at_ms.cmp(&a.created_at_ms));
        if limit > 0 && list.len() > limit {
            list.truncate(limit);
        }
        list
    }

    async fn update_status(
        &self,
        task_id: &str,
        status: TaskStatus,
        result: Option<Value>,
        error: Option<String>,
    ) -> bool {
        let mut map = self.tasks.lock().await;
        if let Some(record) = map.get_mut(task_id) {
            // Idempotency: do not overwrite terminal state with another state
            if record.status.is_terminal() {
                return false;
            }
            record.status = status;
            record.result = result;
            record.error = error;
            record.updated_at_ms = current_timestamp_ms();
            true
        } else {
            false
        }
    }
}

/// TaskRegistry manages task persistence and status querying.
#[derive(Clone)]
pub struct TaskRegistry {
    store: Arc<InMemoryTaskStore>,
}

impl Default for TaskRegistry {
    fn default() -> Self {
        Self {
            store: Arc::new(InMemoryTaskStore::new()),
        }
    }
}

impl TaskRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn create_task(
        &self,
        task_id: String,
        worker_id: Option<String>,
        capability: String,
        input: Value,
    ) -> TaskRecord {
        let now = current_timestamp_ms();
        let record = TaskRecord {
            task_id,
            worker_id,
            capability,
            input,
            status: TaskStatus::Pending,
            result: None,
            error: None,
            created_at_ms: now,
            updated_at_ms: now,
        };
        self.store.create(record).await
    }

    pub async fn get_task(&self, task_id: &str) -> Option<TaskRecord> {
        self.store.get(task_id).await
    }

    pub async fn list_tasks(&self, limit: usize) -> Vec<TaskRecord> {
        self.store.list(limit).await
    }

    pub async fn complete_task(&self, task_id: &str, result: Value) -> bool {
        self.store
            .update_status(task_id, TaskStatus::Completed, Some(result), None)
            .await
    }

    pub async fn fail_task(&self, task_id: &str, error: String) -> bool {
        self.store
            .update_status(task_id, TaskStatus::Failed, None, Some(error))
            .await
    }
}

/// TaskService encapsulates task dispatch orchestration, resolving target workers
/// and maintaining task lifecycle state cleanly decoupled from the HTTP router.
#[derive(Clone)]
pub struct TaskService {
    task_registry: TaskRegistry,
    worker_registry: WorkerRegistry,
}

impl TaskService {
    pub fn new(task_registry: TaskRegistry, worker_registry: WorkerRegistry) -> Self {
        Self {
            task_registry,
            worker_registry,
        }
    }

    pub fn task_registry(&self) -> &TaskRegistry {
        &self.task_registry
    }

    pub fn worker_registry(&self) -> &WorkerRegistry {
        &self.worker_registry
    }

    /// Dispatch a task to a target worker or any worker providing the capability.
    pub async fn dispatch(
        &self,
        capability: String,
        input: Value,
        target_worker_id: Option<String>,
    ) -> Result<TaskRecord, String> {
        let task_id = Uuid::new_v4().to_string();
        let task_msg = BrainMessage::task(task_id.clone(), capability.clone(), input.clone());

        let chosen_worker = if let Some(ref wid) = target_worker_id {
            if self.worker_registry.send_task(wid, task_msg).await {
                wid.clone()
            } else {
                return Err(format!("worker '{}' not found or offline", wid));
            }
        } else {
            match self
                .worker_registry
                .dispatch_task(&capability, task_msg)
                .await
            {
                Some(wid) => wid,
                None => {
                    return Err(format!(
                        "no worker available with capability: {}",
                        capability
                    ));
                }
            }
        };

        let mut record = self
            .task_registry
            .create_task(task_id, Some(chosen_worker), capability, input)
            .await;

        record.status = TaskStatus::Running;
        let _ = self
            .task_registry
            .store
            .update_status(&record.task_id, TaskStatus::Running, None, None)
            .await;

        Ok(record)
    }

    /// Process a TaskResult received over the WebSocket transport.
    pub async fn handle_worker_result(&self, task_id: &str, success: bool, output: Value) -> bool {
        if success {
            self.task_registry.complete_task(task_id, output).await
        } else {
            let error_msg = if let Some(err_str) = output.get("error").and_then(Value::as_str) {
                err_str.to_string()
            } else {
                output.to_string()
            };
            self.task_registry.fail_task(task_id, error_msg).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_task_registry_crud_and_idempotency() {
        let registry = TaskRegistry::new();
        let task = registry
            .create_task(
                "task-1".into(),
                Some("worker-1".into()),
                "system.info".into(),
                json!({}),
            )
            .await;

        assert_eq!(task.task_id, "task-1");
        assert_eq!(task.status, TaskStatus::Pending);

        // Complete task
        assert!(
            registry
                .complete_task("task-1", json!({"os": "linux"}))
                .await
        );
        let fetched = registry.get_task("task-1").await.unwrap();
        assert_eq!(fetched.status, TaskStatus::Completed);
        assert_eq!(fetched.result, Some(json!({"os": "linux"})));

        // Idempotency: subsequent duplicate complete should return false and not overwrite
        assert!(
            !registry
                .complete_task("task-1", json!({"os": "windows"}))
                .await
        );
        let fetched_after = registry.get_task("task-1").await.unwrap();
        assert_eq!(fetched_after.result, Some(json!({"os": "linux"})));
    }

    #[tokio::test]
    async fn test_task_service_dispatch_and_result() {
        let task_reg = TaskRegistry::new();
        let worker_reg = WorkerRegistry::new();
        let service = TaskService::new(task_reg.clone(), worker_reg.clone());

        let (tx, mut rx) = mpsc::unbounded_channel();
        worker_reg
            .register(
                "worker-a".into(),
                "dev".into(),
                vec!["system.info".into()],
                tx,
            )
            .await;

        let record = service
            .dispatch("system.info".into(), json!({}), None)
            .await
            .unwrap();

        assert_eq!(record.worker_id, Some("worker-a".to_string()));

        // Worker receives task over channel
        let received = rx.recv().await.unwrap();
        match received {
            BrainMessage::Task {
                task_id,
                capability,
                ..
            } => {
                assert_eq!(task_id, record.task_id);
                assert_eq!(capability, "system.info");

                // Handle worker result
                let updated = service
                    .handle_worker_result(&task_id, true, json!({"status": "ok"}))
                    .await;
                assert!(updated);
            }
            _ => panic!("unexpected message"),
        }

        let completed = task_reg.get_task(&record.task_id).await.unwrap();
        assert_eq!(completed.status, TaskStatus::Completed);
    }
}
