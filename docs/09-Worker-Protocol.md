# HermesOS Worker Protocol v2

This document defines the initial JSON messages exchanged over the WebSocket
connection in ADR-008. The protocol is intentionally small: it establishes a
worker identity, advertises allowed capabilities, accepts a task, and returns a
result.

## Versioning

Every `enroll` and `hello` message carries `protocol_version`. The current
value is `2` (v2 added the per-worker credential, see
[[adr/ADR-014-Worker-Hello-Authentication|ADR-014]]). The Brain rejects
unsupported versions before it sends a task.

## Authentication flow

1. First start: the worker sends `enroll` with a single-use
   `registration_token`. The Brain answers `enrollment_accepted` with a random
   `worker_secret`, which the worker stores in its git-ignored state file.
2. Every later start: the worker sends `hello` with `worker_secret`. The Brain
   answers `hello_accepted`, or `error` and closes the connection if the id is
   unknown, the secret is wrong, or the id already has a live connection.

## Worker to Brain

### Enroll

```json
{
  "type": "enroll",
  "protocol_version": 2,
  "registration_token": "<single-use token>",
  "worker_id": "windows-development-01",
  "role": "development",
  "capabilities": ["system.info"]
}
```

### Hello

```json
{
  "type": "hello",
  "protocol_version": 2,
  "worker_id": "windows-development-01",
  "worker_secret": "<secret from enrollment_accepted>",
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

### Enrollment accepted / hello accepted

```json
{ "type": "enrollment_accepted", "worker_id": "windows-development-01", "worker_secret": "<64 hex chars>" }
{ "type": "hello_accepted", "worker_id": "windows-development-01" }
```

### Error

```json
{ "type": "error", "message": "authentication failed: unknown worker or invalid credential" }
```

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
`task_result` message. The Brain only accepts `task_result` from an
authenticated connection for tasks assigned to that worker. Transport
encryption (`wss://`/mTLS, ADR-009) and replay protection are still
prerequisites for any production connection.
