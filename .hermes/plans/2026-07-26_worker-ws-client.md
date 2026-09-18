# Worker WebSocket Client Implementation Plan

> **For Hermes:** Implement task-by-task with TDD (test-driven-development skill).

**Goal:** Worker agent-core'u Brain'e WebSocket ile bağlanan tam teşekküllü bir client haline getir.

**Architecture:** 
- Worker `hermes-protocol` crate'ini kullanarak Brain'e bağlanır
- Enroll veya Hello ile kimlik doğrular
- Heartbeat gönderir, Task alır, CapabilityRegistry ile çalıştırır, TaskResult döner
- Brain ile aynı tokio-tungstenite stack'ini kullanır

**Tech Stack:** Rust, tokio, tokio-tungstenite, futures-util, hermes-protocol

---

## Task 1: Worker'ı temizle — stale protocol.rs'i kaldır, hermes-protocol'e geç

**Objective:** Worker'daki eski `protocol.rs` kopyasını sil, `capability.rs`'i ortak crate'e geçir, `Cargo.toml`'a gerekli deps'leri ekle.

**Files:**
- Delete: `workers/agent-core/src/protocol.rs`
- Modify: `workers/agent-core/src/capability.rs` — `use hermes_protocol::...`
- Modify: `workers/agent-core/Cargo.toml` — add deps

**Deps to add:**
```toml
hermes-protocol = { path = "../../protocol" }
tokio = { version = "1", features = ["macros", "net", "rt-multi-thread", "sync", "time"] }
tokio-tungstenite = "0.28"
futures-util = "0.3"
```

## Task 2: WorkerConfig genişlet — brain_url ve registration_token ekle

**Objective:** Config parser'a yeni alanlar: `brain_url`, `registration_token`, `heartbeat_interval_secs`.

**Files:**
- Modify: `workers/agent-core/src/main.rs`

**Yeni config formatı:**
```
worker_id=windows-dev
role=development
capabilities=system.info
brain_url=ws://127.0.0.1:9000
registration_token=tok-secret
heartbeat_interval_secs=30
```

## Task 3: WorkerClient struct — WebSocket bağlantı yönetimi

**Objective:** Brain'e bağlanan, enroll/hello yapan, heartbeat gönderen, task dinleyen client.

**Files:**
- Create: `workers/agent-core/src/client.rs`

**WorkerClient API:**
```rust
pub struct WorkerClient { ... }

impl WorkerClient {
    pub async fn connect(config: WorkerConfig) -> Result<Self, String>;
    pub async fn run(&mut self) -> Result<(), String>;
}
```

**Akış:**
1. `tokio_tungstenite::connect_async` ile Brain'e bağlan
2. Registration token varsa `Enroll`, yoksa `Hello` gönder
3. Brain'den `EnrollmentAccepted` bekle (veya Error'da çık)
4. Heartbeat loop (ayrı task)
5. Brain'den Task mesajlarını dinle
6. CapabilityRegistry ile execute et
7. TaskResult gönder
8. Bağlantı koparsa reconnect

## Task 4: main.rs güncelle — tokio async main

**Objective:** main'i async yap, WorkerClient oluşturup çalıştır.

**Files:**
- Modify: `workers/agent-core/src/main.rs`

## Task 5: Integration test — Brain ile karşılıklı test

**Objective:** Worker'ın Brain'e bağlanıp enroll+heartbeat+task akışını test et.

**Files:**
- Create: `workers/agent-core/tests/integration.rs`

---

## Verification

```bash
# Brain testleri
cd /f/HermesOS/brain && cargo test

# Protocol testleri
cd /f/HermesOS/protocol && cargo test

# Worker testleri + build
cd /f/HermesOS/workers/agent-core && cargo test && cargo build
```