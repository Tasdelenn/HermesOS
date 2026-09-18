# ADR-012: HTTP API Bind Issue on Windows

## Status
Resolved (Superseded by ADR-013)

## Date
07.08.2026

---

## Context
Hermes Brain servisinde Axum tabanlı HTTP API (port 9001) `tokio::spawn` içinde başlatılıyor. WebSocket server (port 9000) ana loop'ta çalışırken, HTTP server ayrı bir task'te spawn ediliyor.

Local testlerde (Windows 10, MSYS2/bash):
- WebSocket port 9000: ✅ Çalışıyor, `netstat` gösteriyor, worker bağlanıyor
- HTTP port 9001: ❌ `netstat` göstermiyor, curl "Connection refused"

## Decision
Bu bilinen bir sorun olarak dokümante edilir. Root cause araştırılana kadar workaround olarak:
1. HTTP API yerine WebSocket üzerinden task dispatch yapılabilir (protocol zaten destekliyor)
2. Veya HTTP server ayrı process olarak çalıştırılabilir
3. Veya `0.0.0.0:9001` yerine `127.0.0.1:9001` bind denenebilir (denendi, çalışmadı)

## Technical Details

### Kod Yapısı (brain/src/main.rs:146-155)
```rust
let http_listener = TcpListener::bind(http_addr).await.unwrap_or_else(|e| {
    eprintln!("hermes-brain: failed to bind HTTP {http_addr}: {e}");
    std::process::exit(1);
});

tokio::spawn(async move {
    if let Err(e) = axum::serve(http_listener, app).await {
        eprintln!("hermes-brain: HTTP server error: {e}");
    }
});
```

### Olası Nedenler
1. **Windows loopback binding** - `127.0.0.1` vs `0.0.0.0` davranışı
2. **tokio::spawn task lifetime** - Main loop WebSocket accept'te bloklu, spawn edilen task scheduler'a veriliyor ama Windows'ta Immediately executed olmayabilir
3. **Firewall / Hyper-V / WSL2 interference** - Port 9001 rezerve edilmiş olabilir
4. **Axum/Tower version uyumsuzluğu** - `axum 0.7` + `tokio 1.53` Windows'ta edge case

### Test Edilenler
- `127.0.0.1:9001` - çalışmadı
- `0.0.0.0:9001` - çalışmadı  
- `netstat -an` port 9001 hiç görünmüyor (LISTENING değil)
- WebSocket port 9000 aynı kod yapısıyla çalışıyor

## Consequences
- **Kısa vadeli**: Task dispatch HTTP API yerine WebSocket `Task` mesajıyla test edilebilir (protocol zaten var)
- **Orta vadeli**: Root cause bulunup fix edilmeli (logging ekleyip debug build ile)
- **Uzun vadeli**: Production'da reverse proxy (nginx/caddy) TLS termination yapacak, o zaman HTTP API internal network'te çalışacak

## Workaround
Worker testleri için HTTP API `/tasks` endpoint yerine Brain'e doğrudan WebSocket üzerinden `BrainMessage::Task` gönderilebilir. `WorkerRegistry::send_task()` ve `dispatch_task()` zaten implement edilmiş.

## Related Documents
- [[ADR-011-Brain-Service-Architecture|Brain Service Architecture]]
- [[05-Current-State|Current State Checkpoint]]
- [[09-Worker-Protocol|Worker Protocol v1]]