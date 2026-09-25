# ADR-014: Worker Hello Authentication (Per-Worker Credential)

## Status

Accepted

## Date

25.09.2026

---

## Context

[[ADR-009-Worker-Enrollment-and-mTLS-Migration|ADR-009]] ilk kaydı tek
kullanımlık registration token'a bağlar. Ancak Rust Brain'de `hello` mesajı
hiçbir kimlik kontrolü yapmadan kabul ediliyordu: Brain'in WebSocket portuna
erişebilen herkes, token olmadan istediği `worker_id` ve rol ile kayıt olup
görev alabiliyordu. Ayrıca:

- Aynı `worker_id` ile ikinci bir bağlantı ilk kaydın üzerine yazıyor, eski
  bağlantı kapanınca yeni kaydı da siliyordu.
- Kimliği doğrulanmamış bir bağlantı `task_result` gönderip herhangi bir görevi
  tamamlanmış gösterebiliyordu.
- Token maskeleme (`[..4]` byte dilimleme) ASCII olmayan token'larda panic
  üretiyordu.

## Decision

### 1. Enrollment sırasında worker'a özel credential

- Başarılı `enroll` sonrasında Brain rastgele 256-bit bir `worker_secret`
  üretir ve bunu **yalnızca bir kez** `enrollment_accepted` mesajında döner.
- Brain secret'ın kendisini saklamaz; yalnızca SHA-256 özetini `worker_id`
  anahtarıyla saklar. Karşılaştırma sabit zamanlıdır (`subtle`).
  Secret yüksek entropili olduğu için yavaş parola hash'i (argon2 vb.) gerekmez.
- Aynı `worker_id` için yeni bir token ile tekrar enroll edilirse credential
  döndürülür (eski secret geçersiz olur).

### 2. `hello` yalnızca kayıtlı kimlik için

- `hello` artık `worker_secret` alanını zorunlu taşır.
- Bilinmeyen `worker_id` ve yanlış secret aynı genel hata ile reddedilir
  (`authentication failed: unknown worker or invalid credential`), bağlantı
  kapatılır. Ayrıntılı sebep yalnızca Brain loglarına yazılır (id taraması
  engellenir).
- Başarılı `hello` sonrası Brain `hello_accepted` döner.
- Protokol sürümü `2`'ye yükseltildi; v1 `hello` mesajları reddedilir.

### 3. Bağlantı sahipliği

- Canlı bağlantısı olan bir `worker_id` için ikinci `hello`/`enroll` reddedilir
  (enroll durumunda token harcanmadan önce).
- Her bağlantı benzersiz bir `connection_id` alır; bağlantı kapanırken kayıt
  yalnızca hâlâ o bağlantıya aitse silinir.
- `task_result` yalnızca kimliği doğrulanmış bağlantıdan ve yalnızca o worker'a
  atanmış görevler için kabul edilir.

### 4. Kalıcılık

| Taraf | Konum | Not |
|---|---|---|
| Brain | `config/brain_credentials.json` (varsayılan, `--credentials-file` ile değişir) | Yalnızca SHA-256 özetleri; `0600`, atomik yazım; `.gitignore`'da |
| Brain | `--no-credentials-file` | Yalnızca bellek; Brain yeniden başlayınca tüm worker'lar yeniden enroll olmalı |
| Worker | `<config-adı>.state.json` (config'in yanında, `state_file=` ile değişir) | Secret düz metin; `0600`, atomik yazım; `.gitignore`'da |

JSON dosyası seçildi çünkü: Brain yeniden başlatıldığında (systemd) worker'ların
her seferinde yeni token ile yeniden kayıt olması pratik değildir; veritabanı ise
bu aşama için gereksiz bağımlılıktır. Dosya yalnızca özet içerdiği için
sızması durumunda worker taklidine izin vermez.

## Consequences

- **Breaking change:** mevcut worker'lar yeni bir registration token ile bir kez
  yeniden enroll olmalıdır (`hermes-brain ... --token <yeni>`, worker
  config'ine `registration_token=<yeni>`). Sonraki başlatmalarda worker
  state dosyasındaki credential ile `hello` gönderir.
- Python prototipi (`scripts/brain-server.py`) bu protokolle uyumlu değildir
  ve hâlâ `0.0.0.0` üzerinde açık enrollment ile dinler; bu ADR kapsamında
  değiştirilmedi.
- Transport hâlâ `ws://`; secret ağda şifresiz gider. `wss://`/mTLS geçişi
  (ADR-009) production öncesi zorunludur.
- Takılı kalmış (yarı açık) bir TCP bağlantısı, Brain onu kapatana kadar aynı
  `worker_id` ile yeniden bağlanmayı engelleyebilir; heartbeat zaman aşımı ile
  kayıt temizleme sonraki adımdır.
- Enrollment'ta bildirilen rol saklanır ancak `hello`'da henüz zorlanmaz.
