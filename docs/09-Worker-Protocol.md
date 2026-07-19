# HermesOS Worker Protocol v1

This document defines the initial JSON messages exchanged over the WebSocket
connection in ADR-008. The protocol is intentionally small: it establishes a
worker identity, advertises allowed capabilities, accepts a task, and returns a
result.

## Versioning

Every `hello` message carries `protocol_version`. The initial value is `1`.
The Brain must reject unsupported versions before it sends a task.

## Worker to Brain

### Hello

```json
{
  "type": "hello",
  "protocol_version": 1,
  "worker_id": "windows-development-01",
  "role": "development",
  "capabilities": ["system.info"]
}
```

### Heartbeat

```json
{ "type": "heartbeat", "timestamp_ms": 0 }
```

### Task result

```json
{
  "type": "task_result",
  "task_id": "task-123",
  "success": true,
  "output": { "hostname": "example" }
}
```

## Brain to Worker

### Task

```json
{
  "type": "task",
  "task_id": "task-123",
  "capability": "system.info",
  "input": { "detail": "basic" }
}
```

The Worker must reject a task whose `capability` is not in its local
configuration. It reports that rejection with `success: false` in a
`task_result` message. Authentication, transport encryption, task
authorization, and replay protection are prerequisites for any production
connection and are not defined by this prototype.
