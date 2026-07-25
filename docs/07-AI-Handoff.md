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
- ADR kayıtları başladı (ADR-001 ~ ADR-010)
- Device Independence kararı alındı
- Worker mimarisi oluşturuldu
- Rust Worker prototipi (config, protocol, capability, secrets)
- Worker enrollment WebSocket istemcisi
- Bootstrap scriptleri (Windows + Linux + macOS)

## Henüz yapılmayanlar

- Hermes Brain servisi kurulmadı
- Worker WebSocket tam döngüsü (hello -> heartbeat -> task loop) yok
- mTLS geçişi yapılmadı
- Telegram entegrasyonu yok
- Aktif hafıza sistemi yok
- CI/CD pipeline yok

## Bir sonraki adım

Hermes Brain tarafında registration token doğrulama ve kayıt servisini kurmak.
Worker'ın tam WebSocket döngüsünü tamamlamak (hello, heartbeat, task alma).
Normal oturumlar için hedef mTLS'tir.

---

## AI Ajanları İçin Kurallar

1. **ADR'ler merkez kaynaktır.** Mimari kararlar yalnızca ADR'lerde geçerlidir. Bir karar ADR'de yoksa, yoktur.
2. **Var olan kararları değiştirmeden devam et.** Büyük mimari değişiklikler yeni ADR gerektirir.
3. **Önce dokümantasyonu güncel tut.** Kod yazmadan önce planı/dokümanı güncelle.
4. **Her ajan kendi branch'inde çalışır.** Doğrudan master'a push yasaktır.
5. **Katkılarını bu dokümanda kaydet.** Her merge sonrası bu handoff'u güncelle.
6. **Branch adlandırma: `agent/<ajan-adı>`** formatını kullan.
