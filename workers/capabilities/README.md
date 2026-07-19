# HermesOS Worker Capabilities

Capabilities are explicit, locally enforced actions that a Worker may perform.
The Worker only registers implementations whose identifiers appear in its local
`capabilities` allowlist; a task cannot enable a capability by naming it.

## Implemented prototype capability

- `system.info`: returns the operating-system name and CPU architecture.

Future capabilities must validate their inputs and have focused permissions.
