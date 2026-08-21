# HermesOS Branch Strategy

## Genel Bakış

HermesOS birden fazla AI ajanı ve insan geliştiriciler tarafından paralel
geliştirilmektedir. Conflict riski minimize edilmeli ve her değişikliğin
kaynağı izlenebilir olmalıdır.

---

## Branch Yapısı

```
master                       <- kararlı, merge edilmiş kod
  |
  +-- agent/antigravity      <- Antigravity Agent
  +-- agent/hermes           <- Hermes Agent (Nous Research)
  +-- agent/gemini           <- Gemini
  +-- agent/worker-*         <- eski ajan branch'leri
  +-- feature/<ozellik-adi>  <- insan geliştirici feature'ları
  +-- fix/<hata-adi>         <- hata düzeltmeleri
```

## Kurallar

### 1. Doğrudan master'a push yasaktır
Tüm değişiklikler Pull Request ile merge edilir.

### 2. Her ajan kendi branch'inde çalışır
- Branch adı: `agent/<ajan-adı>`
- Her ajan yalnızca kendi branch'ini değiştirir
- Başka ajanın branch'ine müdahale etmez

### 3. Merge öncesi kontrol
- Kod derlenir mi? (cargo build, script syntax)
- Testler geçer mi? (cargo test)
- Dokümantasyon güncel mi?
- ADR gerekiyorsa eklendi mi?

### 4. Conflict çözümü
- Her ajan merge öncesi master'dan rebase/merge alır
- Conflict varsa ajan kendi branch'inde çözer
- Ortak dosyalarda (README, Handoff) dikkatli olunur

### 5. Branch temizliği
- Merge edilen branch'ler silinir
- Uzun süre aktif olmayan branch'ler gözden geçirilir

---

## Ortak Dosyalar (dikkat gerektiren)

Bu dosyalar birden fazla ajan tarafından düzenlenebilir.
Conflict riski yüksektir, değişiklik yaparken master ile sync olun.

- `README.md`
- `docs/05-Current-State.md`
- `docs/07-AI-Handoff.md`
- `.gitignore`

---

## Commit Mesajı Formatı

```
<tip>: <açıklama>

Örnekler:
feat: add Hermes Brain WebSocket server
fix: correct heartbeat interval calculation
docs: update AI handoff with Hermes agent contributions
chore: add tools/ to gitignore
adr: add ADR-011 for brain service architecture
test: add enrollment flow integration tests
```
