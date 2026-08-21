use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const PROTOCOL_VERSION: u16 = 1;

// ---------------------------------------------------------------------------
// Worker -> Brain messages
// ---------------------------------------------------------------------------

/// Messages sent by a worker to the Hermes Brain over a WebSocket frame.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkerMessage {
    /// First-time enrollment with a single-use registration token.
    Enroll {
        protocol_version: u16,
        registration_token: String,
        worker_id: String,
        role: String,
        capabilities: Vec<String>,
    },
    /// Identity announcement after enrollment or reconnect.
    Hello {
        protocol_version: u16,
        worker_id: String,
        role: String,
        capabilities: Vec<String>,
    },
    /// Periodic liveness signal.
    Heartbeat { timestamp_ms: u64 },
    /// Result of a previously assigned task.
    TaskResult {
        task_id: String,
        success: bool,
        output: Value,
    },
}

// ---------------------------------------------------------------------------
// Brain -> Worker messages
// ---------------------------------------------------------------------------

/// Messages sent by the Hermes Brain to a connected worker.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BrainMessage {
    /// Confirms a successful enrollment.
    EnrollmentAccepted { worker_id: String },
    /// Assigns a task to the worker.
    Task {
        task_id: String,
        capability: String,
        input: Value,
    },
    /// Generic error sent to the worker.
    Error { message: String },
}

// ---------------------------------------------------------------------------
// Convenience constructors
// ---------------------------------------------------------------------------

impl WorkerMessage {
    pub fn enroll(
        registration_token: String,
        worker_id: String,
        role: String,
        capabilities: Vec<String>,
    ) -> Self {
        Self::Enroll {
            protocol_version: PROTOCOL_VERSION,
            registration_token,
            worker_id,
            role,
            capabilities,
        }
    }

    pub fn hello(worker_id: String, role: String, capabilities: Vec<String>) -> Self {
        Self::Hello {
            protocol_version: PROTOCOL_VERSION,
            worker_id,
            role,
            capabilities,
        }
    }

    pub fn heartbeat() -> Self {
        let timestamp_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        Self::Heartbeat { timestamp_ms }
    }
}

impl BrainMessage {
    pub fn enrollment_accepted(worker_id: String) -> Self {
        Self::EnrollmentAccepted { worker_id }
    }

    pub fn task(task_id: String, capability: String, input: Value) -> Self {
        Self::Task {
            task_id,
            capability,
            input,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self::Error {
            message: message.into(),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn serializes_enroll_message() {
        let msg = WorkerMessage::enroll(
            "tok-abc".into(),
            "win-dev-01".into(),
            "development".into(),
            vec!["system.info".into()],
        );
        let v = serde_json::to_value(&msg).unwrap();
        assert_eq!(v["type"], "enroll");
        assert_eq!(v["registration_token"], "tok-abc");
        assert_eq!(v["protocol_version"], 1);
    }

    #[test]
    fn serializes_hello_message() {
        let msg = WorkerMessage::hello(
            "win-dev-01".into(),
            "development".into(),
            vec!["system.info".into()],
        );
        let v = serde_json::to_value(&msg).unwrap();
        assert_eq!(v["type"], "hello");
        assert_eq!(v["worker_id"], "win-dev-01");
    }

    #[test]
    fn deserializes_task_message() {
        let msg: BrainMessage = serde_json::from_value(json!({
            "type": "task",
            "task_id": "task-123",
            "capability": "system.info",
            "input": { "detail": "basic" }
        }))
        .unwrap();
        assert_eq!(
            msg,
            BrainMessage::Task {
                task_id: "task-123".into(),
                capability: "system.info".into(),
                input: json!({ "detail": "basic" }),
            }
        );
    }

    #[test]
    fn deserializes_enrollment_accepted() {
        let msg: BrainMessage = serde_json::from_value(json!({
            "type": "enrollment_accepted",
            "worker_id": "win-dev-01"
        }))
        .unwrap();
        assert_eq!(
            msg,
            BrainMessage::EnrollmentAccepted {
                worker_id: "win-dev-01".into()
            }
        );
    }

    #[test]
    fn round_trips_heartbeat() {
        let msg = WorkerMessage::heartbeat();
        let json_str = serde_json::to_string(&msg).unwrap();
        let parsed: WorkerMessage = serde_json::from_str(&json_str).unwrap();
        assert!(matches!(parsed, WorkerMessage::Heartbeat { .. }));
    }
}
