use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const PROTOCOL_VERSION: u16 = 1;

/// Messages sent by a worker to the Hermes Brain over a WebSocket frame.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkerMessage {
    Hello {
        protocol_version: u16,
        worker_id: String,
        role: String,
        capabilities: Vec<String>,
    },
    Heartbeat {
        timestamp_ms: u64,
    },
    TaskResult {
        task_id: String,
        success: bool,
        output: Value,
    },
}

/// Messages sent by the Hermes Brain to a connected worker.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BrainMessage {
    Task {
        task_id: String,
        capability: String,
        input: Value,
    },
}

impl WorkerMessage {
    pub fn hello(worker_id: String, role: String, capabilities: Vec<String>) -> Self {
        Self::Hello {
            protocol_version: PROTOCOL_VERSION,
            worker_id,
            role,
            capabilities,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn serializes_hello_message() {
        let message = WorkerMessage::hello(
            "windows-development-01".to_string(),
            "development".to_string(),
            vec!["system.info".to_string()],
        );

        assert_eq!(
            serde_json::to_value(message).unwrap(),
            json!({
                "type": "hello",
                "protocol_version": 1,
                "worker_id": "windows-development-01",
                "role": "development",
                "capabilities": ["system.info"]
            })
        );
    }

    #[test]
    fn deserializes_task_message() {
        let message: BrainMessage = serde_json::from_value(json!({
            "type": "task",
            "task_id": "task-123",
            "capability": "system.info",
            "input": { "detail": "basic" }
        }))
        .unwrap();

        assert_eq!(
            message,
            BrainMessage::Task {
                task_id: "task-123".to_string(),
                capability: "system.info".to_string(),
                input: json!({ "detail": "basic" }),
            }
        );
    }
}
