# Brain Server (Python Prototip)

> ADR-008/009 uyumlu, pull-based worker iletişimi için **geçici prototip**.
> Asıl hedef Rust brain'dir (bkz. `brain/`), bu prototip worker-beyin akışını doğrulamak ve archlin/RPi worker senaryosunu hızlı test etmek içindir.

## Kurulum

```bash
python3 -m venv .venv
.venv/bin/pip install websockets
```

## Çalıştırma

```bash
.venv/bin/python scripts/brain-server.py --port 8765
```

## API (WebSocket)

- `hello` → yeni worker kaydı / welcome
- `heartbeat` → canlılık sinyali
- `task` → brain → worker görev dağıtımı
- `task_result` → worker → brain sonuç

Worker kayıtları `config/worker_registry.json` içinde tutulur (ilk aşamada open enrollment).

## Neden Python?

- Prototip için hızlı geri bildirim
- Rust brain'e geçmeden önce protokol/akış doğrulaması
- Archlin worker'ının Python tabanlı olması durumunda düşük bağımlılık

## Geçiş Notu

Rust brain (`brain/`) production hedefidir. Bu prototip, Rust tarafındaki
`Task Registry + HTTP API + E2E loop` (ADR-013) ile aynı işi yapar; ancak
WebSocket tabanlıdır (HTTP REST değil). İleride iki taraf da aynı `protocol`
crate'ini kullanacak şekilde birleştirilebilir.