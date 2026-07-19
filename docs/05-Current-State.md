---

# Checkpoint - 19.07.2026

## Tamamlananlar

- HermesOS proje kökü oluşturuldu.
- Git repository başlatıldı.
- Vision dokümanı oluşturuldu.
- Architecture dokümanı oluşturuldu.
- Architecture Decision Records başladı.
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

İlk Rust Worker prototipi `workers/agent-core` altında oluşturuldu.

- Yerel kimlik, rol ve capability allowlist'i okur.
- Worker Protocol v1 JSON mesajlarını tanımlar.
- `system.info` capability'sini yalnızca yerel allowlist izin verirse çalıştırır.
- Capability dışındaki görevleri başarısız `task_result` ile reddeder.

## Sonraki Teknik Adım

Worker ile Hermes Brain arasındaki kayıt ve kimlik doğrulama modeli belirlendi:
cihaz başına tek kullanımlık registration token ile ilk kayıt; hedef durumda
mTLS. Sonraki uygulama adımı bu akışa uygun WebSocket istemcisidir.

Detay: [[adr/ADR-009-Worker-Enrollment-and-mTLS-Migration|ADR-009]].
