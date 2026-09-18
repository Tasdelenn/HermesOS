# HermesOS Project Rules

## Architecture Decision Records (ADR)

ADR'ler bu projenin merkez mimari kaynaklarıdır.

- Mimari bir karar yalnızca `docs/adr/` altında ADR dosyası varsa geçerlidir.
- Kod veya dokümanlarda geçen ama ADR'de olmayan mimari varsayımlar resmi değildir.
- Yeni mimari karar almadan önce mevcut ADR'leri oku.
- Mevcut bir kararı değiştirmek istiyorsan yeni bir ADR oluştur, eskisini "Superseded" yap.

## Branch Kuralları

- Doğrudan master'a push yasaktır.
- Her AI ajanı `agent/<ajan-adı>` branch'inde çalışır.
- Değişiklikler Pull Request ile merge edilir.
- Detay: `docs/08-Branch-Strategy.md`

## Dokümantasyon

- Kod yazmadan önce planı/dokümanı güncelle.
- Her merge sonrası `docs/07-AI-Handoff.md` güncellenir.
- Her merge sonrası `docs/05-Current-State.md` güncellenir.

## Güvenlik

- Secrets asla commit edilmez (.gitignore ile korunur).
- Registration token'lar log çıktılarında maskelenir.
- Worker yalnızca wss:// bağlantı kabul eder.

## Kod Kalitesi

- Rust: `cargo build` ve `cargo test` geçmeli.
- Commit mesajı formatı: `<tip>: <açıklama>` (feat/fix/docs/chore/adr/test)
