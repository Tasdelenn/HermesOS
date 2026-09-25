---
# Checkpoint - 21.08.2026
## Tamamlananlar
- HermesOS proje kökü oluşturuldu.
- Git repository başlatıldı.
- Vision dokümanı oluşturuldu.
- Architecture dokümanı oluşturuldu.
- Architecture Decision Records başladı (ADR-001 ~ ADR-013).
- Device Independence prensibi kabul edildi.
- Worker klasör mimarisi oluşturuldu.
- HermesOS ana klasörünün Obsidian Vault olarak kullanılması kararlaştırıldı.

## Mevcut Mimari Karar
Fiziksel cihazlar geçici worker node olarak kabul edilir.

Kalıcı olan:
- kimlik
- rol
- capability
- hafıza
- konfigürasyon

bilgileridir.

## Worker Prototype
Rust Worker prototipi `workers/agent-core` altında hazır:

- Yerel kimlik, rol ve capability allowlist'i okur.
- Worker Protocol v1 JSON mesajlarını tanımlar (`protocol/` crate).
- `system.info` capability'sini yalnızca yerel allowlist izin verirse çalıştırır.
- Capability dışındaki görevleri başarısız `task_result` ile reddeder.
- Heartbeat ve Task alma döngüsünü WebSocket istemcisiyle yönetir.

## Brain Service & Task Mimarisi (21.08.2026)
Hermes Brain servisi `brain/` klasöründe geliştirildi ve tam döngü doğrulandı:

- **WebSocket listener (port 9000)** - Worker bağlantıları, hello, heartbeat, task dispatch ve task result yönetimi.
- **Enrollment endpoint** - Registration token doğrulama (tek kullanım, loglarda maskeli).
- **Worker Registry (`registry.rs`)** - Bellekte bağlı worker'ların kimlik, rol, capability, durum ve WebSocket kanal referansları.
- **Task Registry & Task Store (`tasks.rs`)** - Persistence-independent `TaskStore` trait soyutlaması, `InMemoryTaskStore`, `TaskStatus` yaşam döngüsü (`Pending`, `Running`, `Completed`, `Failed`, `Timeout`, `Cancelled`) ve idempotency koruması.
- **Task Service (`tasks.rs`)** - HTTP router ile Worker/Task registry arasındaki orkestrasyon katmanı.
- **HTTP REST API (port 9001 - `http.rs`)**:
  - `GET /health` - Sağlık durumu, uptime, worker ve görev sayıları
  - `GET /workers` - Bağlı worker listesi ve metrikleri
  - `POST /tasks` - Asenkron task dispatch
  - `GET /tasks` - Son görev listesi
  - `GET /tasks/:id` - Görev detay sorgulama

## Entegrasyon Testleri (21.08.2026)
Workspace genelinde 37 test başarıyla geçmektedir:

✅ Worker enrollment token ile başarılı ve token tüketimi  
✅ Heartbeat alınıyor ve registry güncelleniyor  
✅ Token loglarda maskeleniyor (güvenlik)  
✅ HTTP API port 9001 bind ve concurrency çözüldü (ADR-013)  
✅ HTTP API üzerinden `POST /tasks` ile `system.info` görevi atandı  
✅ Worker WebSocket üzerinden görevi aldı, çalıştırdı ve `TaskResult` döndü  
✅ `GET /tasks/:id` ile sonuç JSON çıktısı başarıyla doğrulandı  
✅ Worker offline/yokken hata yönetimi doğrulandı (400 Bad Request)  
✅ Yinelenen `TaskResult` durumunda idempotency koruması doğrulandı  

## Sonraki Teknik Adım
1. Normal oturumlar ve production iletişimi için mTLS geçişi ([[adr/ADR-009-Worker-Enrollment-and-mTLS-Migration|ADR-009]]).
2. Harici kullanıcı etkileşimi için Telegram Bot arayüzü entegrasyonu.
3. Uzun süreli hafıza ve bilgi tabanı (Obsidian Vault) entegrasyonu.
---

# Checkpoint - 20.09.2026 (Servis Kurulumu)

## Tamamlananlar
- **`agent/antigravity` → `master` merge edildi** — Rust brain (WS 9000 + HTTP 9001), protocol crate, worker WebSocket client master'da.
- **Python brain prototipi** korundu (`scripts/brain-server.py`, WS 8765) — `scripts/README-BrainServer.md` ile belgelendi.
- **Systemd servisleri kuruldu ve çalışıyor**:
  - `hermes-brain.service` → Rust brain (127.0.0.1:9000 + 9001)
  - `hermes-brain-ws.service` → Python prototip (0.0.0.0:8765)
  - `hermes-worker.service` → Rust worker (`rpi-test-worker`, brain'e bağlı, heartbeat 30s)
- Kurulum rehberi: [`systemd/README.md`](../systemd/README.md)

## Doğrulananlar
- Rust workspace testleri yeşil (37 test, 0 hata)
- `GET /health` → `workers_connected: 1`
- `GET /workers` → `rpi-test-worker` bağlı, heartbeat güncel
- Worker brain'e bağlandı, hello gönderdi, heartbeat döngüsü aktif

## Aktif Mimari
```
hermes-brain.service (Rust, :9000 WS + :9001 HTTP)
        ↑ bağlantı (hello/heartbeat/task)
hermes-worker.service (Rust worker)
hermes-brain-ws.service (Python prototip, :8765, ayrı)
```

## Sonraki Adımlar
- Telegram bot arayüzü entegrasyonu
- mTLS geçişi (ADR-009)
- Uzun süreli hafıza / Obsidian entegrasyonu

---

# Checkpoint - 25.09.2026 (Hello Kimlik Doğrulaması + CI)

## Tamamlananlar
- **`hello` kimlik doğrulaması** ([[adr/ADR-014-Worker-Hello-Authentication|ADR-014]]):
  enrollment sırasında Brain worker'a özel `worker_secret` üretir, yalnızca
  SHA-256 özetini saklar (`config/brain_credentials.json`). Worker secret'ı
  `<config>.state.json` dosyasına yazar ve sonraki başlatmalarda `hello` ile
  gönderir. Kayıtsız / yanlış secret'lı `hello` reddedilir.
- Aynı `worker_id` ile ikinci canlı bağlantı reddedilir; eski bağlantı yeni
  kaydı silemez. `task_result` yalnızca doğrulanmış ve görevin atandığı
  worker'dan kabul edilir. ASCII olmayan token maskeleme panic'i düzeltildi.
- Protokol sürümü **2**.
- `config/worker_registry.json` artık Git'te takip edilmiyor (`.gitignore`);
  örnek: `config/worker_registry.example.json`.
- CI: `.github/workflows/ci.yml` (fmt, clippy `-D warnings`, test) ve
  `rust-toolchain.toml` (Rust 1.98.1).

## Doğrulananlar
- Workspace testleri yeşil: 57 test (brain 17 unit + 14 entegrasyon,
  protocol 7, worker 13 unit + 6 entegrasyon).

## Geçiş Notu (Breaking)
- Mevcut worker'lar bir kez yeniden enroll olmalı:
  `hermes-brain 127.0.0.1:9000 127.0.0.1:9001 --token <yeni-token>` ve worker
  config'ine `registration_token=<yeni-token>`. İlk başarılı enroll'dan sonra
  token config'ten silinebilir.
- Python prototipi (`hermes-brain-ws.service`) yeni protokolle uyumlu değildir.

