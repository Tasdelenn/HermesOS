# ADR-013: Task Registry and HTTP API Architecture

## Status

Accepted

## Date

21.08.2026

---

## Context

ADR-011 Hermes Brain'in WebSocket tabanlı worker orkestrasyonunu tanımlamış, ADR-012 ise Windows ortamında Axum HTTP API listener başlatılırken karşılaşılan bind sorununu ve task dispatch mekanizmasının eksikliğini dokümante etmiştir.

HermesOS'un dış dünyadan (HTTP istemcileri, CLI, Telegram botu, gelecekteki UI arayüzleri) asenkron görev alabilmesi, uygun worker'a yönlendirebilmesi ve görev sonuçlarını takip edebilmesi için:
1. HTTP REST API sunucusunun WebSocket sunucusuyla eşzamanlı ve stabil çalışması,
2. Görev yaşam döngüsünü (`Pending`, `Running`, `Completed`, `Failed`, `Timeout`, `Cancelled`) yöneten bir `TaskRegistry` ve `TaskService` katmanının bulunması,
3. Bu yapının donanım/ortam bağımsız (RPi vs. VPS) ve gelecekteki kalıcı veritabanı geçişine hazır (`TaskStore` trait soyutlaması) olması gerekmektedir.

## Decision

Hermes Brain servisine bağımsız bir **Task Yönetim ve HTTP API Mimarisi** entegre edilmiştir.

### 1. Katmanlı Mimari ve Sorumluluk Ayrımı

```
HTTP REST Router (Axum :9001)
       │
       ▼
  TaskService
  ┌────┴──────────────────────────┐
  ▼                               ▼
TaskRegistry (TaskStore trait)   WorkerRegistry
  │                               │
  ▼                               ▼
InMemoryTaskStore (Future DB)   WebSocket Transport (:9000)
                                  │
                                  ▼
                                Worker
```

- **HTTP Router (`brain/src/http.rs`)**: Yalnızca HTTP isteklerini karşılar, yetkilendirme ve serileştirmeyi yapar. Worker seçimi veya dispatch kararlarını bilmez.
- **TaskService (`brain/src/tasks.rs`)**: Görev oluşturma, worker seçimi/eşleştirme ve sonuç çözümleme orkestrasyonunu yönetir.
- **TaskStore / TaskRegistry**: Persistence-independent (kalıcılık bağımsız) trait arayüzü ile görev durumlarını saklar. İlk aşamada `InMemoryTaskStore` kullanılır; ileride `SqliteTaskStore` veya `PostgresTaskStore` ile genişletilebilir.
- **WorkerRegistry (`brain/src/registry.rs`)**: Yalnızca bağlı worker'ların canlı durumunu ve capability allowlist'lerini yönetir.

### 2. HTTP REST Uç Noktaları

| Metot | Yol | Açıklama |
|---|---|---|
| `GET` | `/health` | Servis sağlık durumu, çalışma süresi, bağlı worker ve görev sayısı |
| `GET` | `/workers` | Aktif worker listesi, roller, capability'ler ve son heartbeat zamanı |
| `POST` | `/tasks` | Asenkron görev dispatch (`capability`, `input`, opsiyonel `worker_id`) |
| `GET` | `/tasks` | Son görevlerin listesi (varsayılan limit: 50) |
| `GET` | `/tasks/:id` | Belirli bir görevin detaylı durumu (`status`, `result`, `error`, zaman damgaları) |

### 3. Asenkron Task Dispatch ve Idempotency

- `POST /tasks` çağrısı yapıldığında Brain görevi `Pending`/`Running` durumunda kaydeder, worker'ın WebSocket kanalına iletir ve istemciye `202 Accepted` ile `task_id` ve atanan `worker_id` döner.
- Worker görevi çalıştırıp `WorkerMessage::TaskResult` döndüğünde `TaskService` sonucu `TaskRegistry` üzerinde `Completed` veya `Failed` olarak günceller.
- **Idempotency**: `TaskRegistry`, tamamlanmış (`terminal`) bir görevin yinelenen bir sonuç mesajıyla ezilmesini engeller.

## Consequences

- ADR-012'de belirtilen HTTP API bind sorunu çözüldü (`tokio::spawn` ile Axum sunucusu WebSocket döngüsüyle paralel çalışmaktadır).
- Hermes Brain artık harici HTTP istemcileri ve CLI araçları tarafından programatik olarak yönetilebilir.
- Task saklama katmanı `TaskStore` trait'i ile soyutlandığı için VPS/RPi geçişlerinde ve SQLite entegrasyonunda API katmanı etkilenmeyecektir.

## Related Documents

- [[ADR-011-Brain-Service-Architecture|Brain Service Architecture]]
- [[ADR-012-HTTP-API-Bind-Issue|HTTP API Bind Issue (Resolved)]]
- [[05-Current-State|Current State]]
- [[07-AI-Handoff|AI Handoff]]
