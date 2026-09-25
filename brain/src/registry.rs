use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::{mpsc, Mutex};

use hermes_protocol::BrainMessage;

/// Information about a connected worker kept in-memory.
#[derive(Debug, Clone)]
pub struct WorkerEntry {
    pub worker_id: String,
    pub role: String,
    pub capabilities: Vec<String>,
    pub connected_at: Instant,
    pub last_heartbeat: Instant,
    /// Channel to send tasks to this worker's WebSocket handler
    pub task_tx: mpsc::UnboundedSender<BrainMessage>,
}

/// Thread-safe in-memory registry of connected workers.
#[derive(Debug, Clone, Default)]
pub struct WorkerRegistry {
    workers: Arc<Mutex<HashMap<String, WorkerEntry>>>,
}

impl WorkerRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn register(
        &self,
        worker_id: String,
        role: String,
        capabilities: Vec<String>,
        task_tx: mpsc::UnboundedSender<BrainMessage>,
    ) {
        let now = Instant::now();
        let entry = WorkerEntry {
            worker_id: worker_id.clone(),
            role,
            capabilities,
            connected_at: now,
            last_heartbeat: now,
            task_tx,
        };
        self.workers.lock().await.insert(worker_id, entry);
    }

    pub async fn update_heartbeat(&self, worker_id: &str) -> bool {
        let mut workers = self.workers.lock().await;
        if let Some(entry) = workers.get_mut(worker_id) {
            entry.last_heartbeat = Instant::now();
            true
        } else {
            false
        }
    }

    pub async fn remove(&self, worker_id: &str) {
        self.workers.lock().await.remove(worker_id);
    }

    pub async fn get(&self, worker_id: &str) -> Option<WorkerEntry> {
        self.workers.lock().await.get(worker_id).cloned()
    }

    pub async fn list(&self) -> Vec<WorkerEntry> {
        self.workers.lock().await.values().cloned().collect()
    }

    pub async fn find_by_capability(&self, capability: &str) -> Vec<WorkerEntry> {
        self.workers
            .lock()
            .await
            .values()
            .filter(|w| w.capabilities.iter().any(|c| c == capability))
            .cloned()
            .collect()
    }

    /// Send a task to a specific worker by ID. Returns true if worker exists and task was queued.
    pub async fn send_task(&self, worker_id: &str, task: BrainMessage) -> bool {
        let workers = self.workers.lock().await;
        if let Some(entry) = workers.get(worker_id) {
            entry.task_tx.send(task).is_ok()
        } else {
            false
        }
    }

    /// Send a task to any worker with the given capability. Returns the worker_id that got the task.
    pub async fn dispatch_task(&self, capability: &str, task: BrainMessage) -> Option<String> {
        let workers = self.workers.lock().await;
        for entry in workers.values() {
            if entry.capabilities.iter().any(|c| c == capability)
                && entry.task_tx.send(task.clone()).is_ok()
            {
                return Some(entry.worker_id.clone());
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn register_and_list() {
        let reg = WorkerRegistry::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        reg.register("w1".into(), "dev".into(), vec!["system.info".into()], tx)
            .await;
        let list = reg.list().await;
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].worker_id, "w1");
    }

    #[tokio::test]
    async fn heartbeat_updates() {
        let reg = WorkerRegistry::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        reg.register("w1".into(), "dev".into(), vec![], tx).await;
        assert!(reg.update_heartbeat("w1").await);
        assert!(!reg.update_heartbeat("unknown").await);
    }

    #[tokio::test]
    async fn find_by_capability() {
        let reg = WorkerRegistry::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        reg.register("w1".into(), "dev".into(), vec!["system.info".into()], tx1)
            .await;
        reg.register(
            "w2".into(),
            "iot".into(),
            vec!["homeassistant.control".into()],
            tx2,
        )
        .await;
        let found = reg.find_by_capability("system.info").await;
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].worker_id, "w1");
    }

    #[tokio::test]
    async fn remove_worker() {
        let reg = WorkerRegistry::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        reg.register("w1".into(), "dev".into(), vec![], tx).await;
        reg.remove("w1").await;
        assert!(reg.list().await.is_empty());
    }
}
