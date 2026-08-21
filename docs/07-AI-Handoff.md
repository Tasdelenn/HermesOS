# HermesOS AI Handoff Document
## Proje
HermesOS
## Amaç
Kişisel AI işletim sistemi altyapısı.
## Kullanıcı
Hakan
---
## Aktif AI Ajanları
Bu proje birden fazla AI ajanı tarafından paralel geliştirilmektedir.
Her ajan kendi branch'inde çalışır ve katkılarını burada kayıt altına alır.

### Antigravity
- Branch: `agent/antigravity`
- Odak: Task Registry, HTTP REST API, Worker Task Dispatch Loop, Idempotency, E2E Integration Testing
- Katkılar:
  - ADR-013 mimari kaydı oluşturuldu (Task Registry & HTTP API Mimarisi).
  - ADR-012 bind sorunu çözüldü ve "Resolved" olarak güncellendi.
  - `brain/src/tasks.rs`: Persistence-independent `TaskStore` trait, `InMemoryTaskStore`, `TaskRegistry` ve `TaskService` orkestrasyon katmanı geliştirildi.
  - `brain/src/http.rs`: Axum REST API uç noktaları (`GET /health`, `GET /workers`, `POST /tasks`, `GET /tasks`, `GET /tasks/:id`) eklendi.
  - `brain/src/main.rs`: Axum HTTP ve WebSocket sunucuları eşzamanlı çalışacak şekilde yapılandırıldı.
  - `brain/tests/integration.rs`: HTTP üzerinden asenkron task dispatch, Worker üzerinden çalıştırma (`system.info`), sonuç sorgulama, hata yönetimi ve idempotency testleri dahil 10 entegrasyon testi yazıldı ve doğrulandı.

### Hermes Agent (Nous Research)
- Branch: `agent/hermes`
- Odak: Brain servisi, altyapı iyileştirme, dokümantasyon, CI/CD
- Katkılar:
  - .gitignore temizliği (tools/, npm-cache)
  - AI-Handoff dokümanı yeniden yapılandırma
  - Branch stratejisi dokümanı
  - ADR merkez kaynak kuralı

### Gemini
- Branch: `agent/worker-websocket-client`
- Odak: Worker WebSocket istemcisi, enrollment akışı
- Katkılar:
  - WebSocket enrollment client
  - Secrets modülü
  - Bootstrap architecture ADR

### Diğer Ajanlar
- Branch: `agent/worker-prototype-and-enrollment`
- Katkılar:
  - İlk Worker prototipi
  - Enrollment planı

---
## Şu ana kadar yapılanlar
- Proje klasörü oluşturuldu
- Git başlatıldı
- Vision yazıldı
- Architecture yazıldı
- ADR kayıtları yapıldı (ADR-001 ~ ADR-013)
- Device Independence kararı alındı
- Worker mimarisi oluşturuldu
- Rust Worker prototipi (config, protocol, capability, secrets)
- Worker enrollment WebSocket istemcisi
- Bootstrap scriptleri (Windows + Linux + macOS)
- Hermes Brain servisi (WebSocket, enrollment, registry, protocol v1)
- Brain + Worker entegrasyon testi (enrollment, heartbeat başarılı)
- HTTP API port 9001 bind sorunu çözüldü (ADR-013)
- Asenkron Task Registry ve HTTP REST API implementasyonu tamamlandı
- Brain $\leftrightarrow$ Worker tam döngü (HTTP dispatch $\rightarrow$ WS execution $\rightarrow$ TaskResult $\rightarrow$ HTTP status check) doğrulandı
- Idempotency ve worker yokken hata yönetimi test edildi

## Henüz yapılmayanlar
- mTLS geçişi yapılmadı (hedef durum)
- Telegram entegrasyonu yok
- Aktif hafıza ve Obsidian bilgi tabanı bağlantısı yok
- CI/CD pipeline yok

## Bir sonraki adım
1. Normal oturumlar ve production iletişimi için mTLS geçişi ([[adr/ADR-009-Worker-Enrollment-and-mTLS-Migration|ADR-009]]).
2. Harici kullanıcı etkileşimi için Telegram Bot arayüzü entegrasyonu.
3. Uzun süreli hafıza ve bilgi tabanı (Obsidian Vault) entegrasyonu.

---
## AI Ajanları İçin Kurallar
1. **ADR'ler merkez kaynaktır.** Mimari kararlar yalnızca ADR'lerde geçerlidir. Bir karar ADR'de yoksa, yoktur.
2. **Var olan kararları değiştirmeden devam et.** Büyük mimari değişiklikler yeni ADR gerektirir.
3. **Önce dokümantasyonu güncel tut.** Kod yazmadan önce planı/dokümanı güncelle.
4. **Her ajan kendi branch'inde çalışır.** Doğrudan master'a push yasaktır.
5. **Katkılarını bu dokümanda kaydet.** Her merge sonrası bu handoff'u güncelle.
6. **Branch adlandırma: `agent/<ajan-adı>`** formatını kullan.