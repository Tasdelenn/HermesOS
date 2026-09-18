# HermesOS — Systemd Servis Kurulumu

HermesOS'un Raspberry Pi üzerinde kalıcı çalışması için systemd servisleri.

## Servisler

| Servis | Ne yapar? | Port |
|---|---|---|
| `hermes-brain.service` | Rust Brain — WebSocket + HTTP API | ws://127.0.0.1:9000, http://127.0.0.1:9001 |
| `hermes-brain-ws.service` | Python Brain Prototip — WebSocket | ws://0.0.0.0:8765 |
| `hermes-worker.service` | Rust Worker — Brain'e bağlanır | — (client) |

## Kurulum

```bash
# Servis dosyalarını sistem dizinine kopyala
sudo cp systemd/hermes-brain.service systemd/hermes-brain-ws.service systemd/hermes-worker.service /etc/systemd/system/

# systemd'i tazele
sudo systemctl daemon-reload

# Servisleri etkinleştir ve başlat
sudo systemctl enable --now hermes-brain hermes-brain-ws hermes-worker
```

## Durum kontrolü

```bash
systemctl status hermes-brain hermes-brain-ws hermes-worker
systemctl is-active hermes-brain hermes-brain-ws hermes-worker
```

## Loglar

```bash
journalctl -u hermes-brain -f
journalctl -u hermes-brain-ws -f
journalctl -u hermes-worker -f
```

## HTTP API Doğrulama

```bash
curl http://127.0.0.1:9001/health    # {"status":"ok","workers_connected":1,...}
curl http://127.0.0.1:9001/workers   # bağlı worker listesi
curl http://127.0.0.1:9001/tasks     # görev listesi
```

## Notlar

- `hermes-worker.service`, `hermes-brain.service`'e bağımlıdır
  (`Requires=`, `After=`), brain'den sonra başlar.
- `config/worker.conf` git-ignored'dır; `worker.example.conf`'dan kopyalanır.
- Rust brain varsayılan adreslerle `127.0.0.1:9000 127.0.0.1:9001` olarak
  çalışır.
- Python prototip `.venv/bin/python` ile çalışır (venv git-ignored).