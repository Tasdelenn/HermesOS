# Hermes Worker Core

The first HermesOS Worker prototype loads its local identity and allowed capabilities from a configuration file. It deliberately does not open a network connection or execute capabilities yet.

## Run

From the repository root on Windows:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" run --manifest-path workers/agent-core/Cargo.toml -- --config config/worker.conf
```

Copy `config/worker.example.conf` to `config/worker.conf` before running it. `worker.conf` is intentionally ignored by Git because a real worker identity must remain local to the device.

## Configuration format

The configuration uses simple `key=value` lines. Supported keys are `worker_id`, `role`, and the comma-separated `capabilities` allowlist.
