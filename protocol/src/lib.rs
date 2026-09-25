use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Wire protocol version.
///
/// v2 (ADR-014): `enrollment_accepted` carries a per-worker `worker_secret`,
/// `hello` must present it, and the Brain answers with `hello_accepted`.
pub const PROTOCOL_VERSION: u16 = 2;

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
    /// Identity announcement on reconnect. Only accepted for a previously
    /// enrolled `worker_id` presenting the credential issued at enrollment.
    Hello {
        protocol_version: u16,
        worker_id: String,
        worker_secret: String,
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
    /// Confirms a successful enrollment and issues the per-worker credential
    /// the worker must persist and present in every later `hello`.
    EnrollmentAccepted {
        worker_id: String,
        worker_secret: String,
    },
    /// Confirms that a `hello` was authenticated.
    HelloAccepted { worker_id: String },
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

    pub fn hello(
        worker_id: String,
        worker_secret: String,
        role: String,
        capabilities: Vec<String>,
    ) -> Self {
        Self::Hello {
            protocol_version: PROTOCOL_VERSION,
            worker_id,
            worker_secret,
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
    pub fn enrollment_accepted(worker_id: String, worker_secret: String) -> Self {
        Self::EnrollmentAccepted {
            worker_id,
            worker_secret,
        }
    }

    pub fn hello_accepted(worker_id: String) -> Self {
        Self::HelloAccepted { worker_id }
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
        assert_eq!(v["protocol_version"], PROTOCOL_VERSION);
    }

    #[test]
    fn serializes_hello_message() {
        let msg = WorkerMessage::hello(
            "win-dev-01".into(),
            "s3cret".into(),
            "development".into(),
            vec!["system.info".into()],
        );
        let v = serde_json::to_value(&msg).unwrap();
        assert_eq!(v["type"], "hello");
        assert_eq!(v["worker_id"], "win-dev-01");
        assert_eq!(v["worker_secret"], "s3cret");
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
            "worker_id": "win-dev-01",
            "worker_secret": "abc123"
        }))
        .unwrap();
        assert_eq!(
            msg,
            BrainMessage::EnrollmentAccepted {
                worker_id: "win-dev-01".into(),
                worker_secret: "abc123".into(),
            }
        );
    }

    #[test]
    fn deserializes_hello_accepted() {
        let msg: BrainMessage = serde_json::from_value(json!({
            "type": "hello_accepted",
            "worker_id": "win-dev-01"
        }))
        .unwrap();
        assert_eq!(msg, BrainMessage::hello_accepted("win-dev-01".into()));
    }

    #[test]
    fn hello_without_secret_is_rejected_by_parser() {
        let parsed = serde_json::from_value::<WorkerMessage>(json!({
            "type": "hello",
            "protocol_version": PROTOCOL_VERSION,
            "worker_id": "w",
            "role": "dev",
            "capabilities": []
        }));
        assert!(parsed.is_err());
    }

    #[test]
    fn round_trips_heartbeat() {
        let msg = WorkerMessage::heartbeat();
        let json_str = serde_json::to_string(&msg).unwrap();
        let parsed: WorkerMessage = serde_json::from_str(&json_str).unwrap();
        assert!(matches!(parsed, WorkerMessage::Heartbeat { .. }));
    }
}
