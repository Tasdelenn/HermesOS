use std::collections::HashMap;

use serde_json::{json, Value};

use hermes_protocol::{BrainMessage, WorkerMessage};

pub const SYSTEM_INFO: &str = "system.info";

pub trait Capability {
    fn execute(&self, input: &Value) -> Result<Value, String>;
}

pub struct CapabilityRegistry {
    capabilities: HashMap<String, Box<dyn Capability>>,
}

impl CapabilityRegistry {
    pub fn from_allowlist(allowed: &[String]) -> Self {
        let mut capabilities: HashMap<String, Box<dyn Capability>> = HashMap::new();

        if allowed.iter().any(|capability| capability == SYSTEM_INFO) {
            capabilities.insert(SYSTEM_INFO.to_string(), Box::new(SystemInfo));
        }

        Self { capabilities }
    }

    pub fn execute(&self, capability: &str, input: &Value) -> Result<Value, String> {
        let handler = self
            .capabilities
            .get(capability)
            .ok_or_else(|| format!("capability is not allowed: {capability}"))?;

        handler.execute(input)
    }

    pub fn execute_task(&self, message: BrainMessage) -> Option<WorkerMessage> {
        match message {
            BrainMessage::Task {
                task_id,
                capability,
                input,
            } => match self.execute(&capability, &input) {
                Ok(output) => Some(WorkerMessage::TaskResult {
                    task_id,
                    success: true,
                    output,
                }),
                Err(error) => Some(WorkerMessage::TaskResult {
                    task_id,
                    success: false,
                    output: json!({ "error": error }),
                }),
            },
            _ => None,
        }
    }
}

struct SystemInfo;

impl Capability for SystemInfo {
    fn execute(&self, _input: &Value) -> Result<Value, String> {
        Ok(json!({
            "operating_system": std::env::consts::OS,
            "architecture": std::env::consts::ARCH,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn executes_an_allowlisted_capability() {
        let registry = CapabilityRegistry::from_allowlist(&[SYSTEM_INFO.to_string()]);
        let output = registry.execute(SYSTEM_INFO, &Value::Null).unwrap();

        assert_eq!(output["operating_system"], std::env::consts::OS);
        assert_eq!(output["architecture"], std::env::consts::ARCH);
    }

    #[test]
    fn rejects_a_capability_outside_the_allowlist() {
        let registry = CapabilityRegistry::from_allowlist(&[]);
        let error = registry.execute(SYSTEM_INFO, &Value::Null).unwrap_err();

        assert_eq!(error, "capability is not allowed: system.info");
    }

    #[test]
    fn turns_a_rejected_task_into_a_failed_result() {
        let registry = CapabilityRegistry::from_allowlist(&[]);
        let result = registry.execute_task(BrainMessage::Task {
            task_id: "task-123".to_string(),
            capability: SYSTEM_INFO.to_string(),
            input: Value::Null,
        });

        assert_eq!(
            result,
            Some(WorkerMessage::TaskResult {
                task_id: "task-123".to_string(),
                success: false,
                output: json!({ "error": "capability is not allowed: system.info" }),
            })
        );
    }

    #[test]
    fn non_task_messages_return_none() {
        let registry = CapabilityRegistry::from_allowlist(&[SYSTEM_INFO.to_string()]);
        let result = registry.execute_task(BrainMessage::hello_accepted("w1".into()));
        assert_eq!(result, None);
    }
}
