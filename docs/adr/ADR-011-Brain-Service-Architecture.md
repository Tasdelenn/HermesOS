# ADR-011: Hermes Brain Service Architecture

## Status

Accepted

## Date

25.07.2026

---

## Context

ADR-008 ve ADR-009 Worker'ların Hermes Brain'e pull-based WebSocket bağlantısı
kuracağını ve ilk kayıtta registration token kullanılacağını belirler. Ancak
Brain servisinin kendisi henüz tanımlanmamıştı.

Worker prototipi (workers/agent-core) çalışır durumda: config okur, hello/
heartbeat/task_result mesajları üretir, capability allowlist'e göre görevleri
kabul veya reddeder. Şimdi bu Worker'ın bağlanacağı Brain servisine ihtiyaç
vardır.

## Decision

Hermes Brain ilk prototipte tek process Rust WebSocket sunucusu olarak
geliştirilecektir. Aynı repository içinde `brain/` klasörü altında yaşayacaktır.

### İlk Prototip Kapsamı

1. **WebSocket Listener** - Worker bağlantılarını kabul eder (wss:// için TLS
   termination reverse proxy arkasında, geliştirme ortamında ws://).
2. **Enrollment Endpoint** - Registration token doğrulama ve tek kullanım
   iptali.
3. **Worker Registry** - Bağlı Worker'ların kimlik, rol, capability ve
   bağlantı durumunu bellekte tutar.
4. **Protocol v1 Handler** - Hello, heartbeat, task_result mesajlarını işler;
   Task mesajı gönderir.
5. **Shared Protocol Crate** - Worker ve Brain arasında ortak kullanılan
   protocol tipleri `protocol/` crate'ine taşınır.

### İlk Prototipte Yok

- Kalıcı depolama (SQLite/Postgres) - bellekte tutulur
- LLM entegrasyonu - task'lar manuel veya API üzerinden oluşturulur
- Telegram/Web arayüzü
- mTLS - geliştirilme aşamasında ws://, production'da reverse proxy TLS

### Dizin Yapısı

```
HermesOS/
├── brain/                  # Hermes Brain servisi
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs         # WebSocket server + CLI
│       ├── registry.rs     # Worker registry (in-memory)
│       └── enrollment.rs   # Token doğrulama
├── protocol/               # Paylaşılan protocol crate
│   ├── Cargo.toml
│   └── src/lib.rs          # WorkerMessage, BrainMessage, Enroll...
└── workers/agent-core/     # Mevcut Worker (protocol/ crate'ini kullanacak)
```

## Consequences

- Worker ve Brain aynı protocol tiplerini paylaşır, uyumsuzluk riski azalır.
- İlk prototip lokal test edilebilir (ws://localhost).
- Brain stateless bellekte çalışır, restart'ta worker bilgileri kaybolur
  (kabul edilebilir - worker'lar tekrar bağlanır).
- Production deployment için TLS termination (nginx/caddy) gerekir.

## Related Documents

- [[ADR-008-Worker-Communication-Model|Worker Communication Model]]
- [[ADR-009-Worker-Enrollment-and-mTLS-Migration|Worker Enrollment and mTLS]]
- [[09-Worker-Protocol|Worker Protocol v1]]
