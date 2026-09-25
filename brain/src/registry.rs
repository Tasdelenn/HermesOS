use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
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
    /// Unique id of the WebSocket connection that owns this entry. Used so a
    /// stale connection can never remove a newer entry for the same worker.
    pub connection_id: u64,
    /// Channel to send tasks to this worker's WebSocket handler
    pub task_tx: mpsc::UnboundedSender<BrainMessage>,
}

/// Thread-safe in-memory registry of connected workers.
#[derive(Debug, Clone, Default)]
pub struct WorkerRegistry {
    workers: Arc<Mutex<HashMap<String, WorkerEntry>>>,
    next_connection_id: Arc<AtomicU64>,
}

/// Returned when a worker id already has a live connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlreadyConnected;

impl WorkerRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a live connection for `worker_id`. Fails if the id already has
    /// a live connection. On success returns the new connection id, which must
    /// be passed to [`remove_connection`](Self::remove_connection) on disconnect.
    pub async fn register(
        &self,
        worker_id: String,
        role: String,
        capabilities: Vec<String>,
        task_tx: mpsc::UnboundedSender<BrainMessage>,
    ) -> Result<u64, AlreadyConnected> {
        let mut workers = self.workers.lock().await;
        if workers.contains_key(&worker_id) {
            return Err(AlreadyConnected);
        }
        let connection_id = self.next_connection_id.fetch_add(1, Ordering::Relaxed) + 1;
        let now = Instant::now();
        let entry = WorkerEntry {
            worker_id: worker_id.clone(),
            role,
            capabilities,
            connected_at: now,
            last_heartbeat: now,
            connection_id,
            task_tx,
        };
        workers.insert(worker_id, entry);
        Ok(connection_id)
    }

    pub async fn is_connected(&self, worker_id: &str) -> bool {
        self.workers.lock().await.contains_key(worker_id)
    }

    /// Remove `worker_id` only if the entry still belongs to `connection_id`.
    /// Returns true if an entry was removed.
    pub async fn remove_connection(&self, worker_id: &str, connection_id: u64) -> bool {
        let mut workers = self.workers.lock().await;
        match workers.get(worker_id) {
            Some(entry) if entry.connection_id == connection_id => {
                workers.remove(worker_id);
                true
            }
            _ => false,
        }
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
            .await
            .unwrap();
        let list = reg.list().await;
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].worker_id, "w1");
    }

    #[tokio::test]
    async fn heartbeat_updates() {
        let reg = WorkerRegistry::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        reg.register("w1".into(), "dev".into(), vec![], tx)
            .await
            .unwrap();
        assert!(reg.update_heartbeat("w1").await);
        assert!(!reg.update_heartbeat("unknown").await);
    }

    #[tokio::test]
    async fn find_by_capability() {
        let reg = WorkerRegistry::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        reg.register("w1".into(), "dev".into(), vec!["system.info".into()], tx1)
            .await
            .unwrap();
        reg.register(
            "w2".into(),
            "iot".into(),
            vec!["homeassistant.control".into()],
            tx2,
        )
        .await
        .unwrap();
        let found = reg.find_by_capability("system.info").await;
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].worker_id, "w1");
    }

    #[tokio::test]
    async fn duplicate_live_worker_id_is_rejected() {
        let reg = WorkerRegistry::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        reg.register("w1".into(), "dev".into(), vec![], tx1)
            .await
            .unwrap();
        assert_eq!(
            reg.register("w1".into(), "dev".into(), vec![], tx2).await,
            Err(AlreadyConnected)
        );
    }

    #[tokio::test]
    async fn stale_connection_cannot_remove_newer_entry() {
        let reg = WorkerRegistry::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        let old = reg
            .register("w1".into(), "dev".into(), vec![], tx1)
            .await
            .unwrap();
        assert!(reg.remove_connection("w1", old).await);
        let new = reg
            .register("w1".into(), "dev".into(), vec![], tx2)
            .await
            .unwrap();
        assert_ne!(old, new);
        // A late cleanup from the old connection must be a no-op.
        assert!(!reg.remove_connection("w1", old).await);
        assert!(reg.get("w1").await.is_some());
    }

    #[tokio::test]
    async fn remove_worker() {
        let reg = WorkerRegistry::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        reg.register("w1".into(), "dev".into(), vec![], tx)
            .await
            .unwrap();
        reg.remove("w1").await;
        assert!(reg.list().await.is_empty());
    }
}
