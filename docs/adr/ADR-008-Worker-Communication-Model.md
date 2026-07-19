# ADR-008: Worker Communication Model

## Status

Accepted

## Date

19.07.2026

---

# Context

HermesOS farklı ağlarda bulunan geçici cihazları yönetebilmelidir.

Worker cihazları:

- ev ağı
- mobil ağ
- farklı internet bağlantıları
- değişen IP adresleri

üzerinde çalışabilir.

---

# Decision

HermesOS pull-based worker communication model kullanacaktır.

Worker node'lar merkezi Hermes Brain'e kendileri bağlanacaktır.

---

# Architecture
Hermes Brain
  |
  |
Secure Channel
  |
  |
Worker Agent
  |
Capabilities
  |
Device

---

# Initial Technology

İlk implementasyon:

- Rust Worker
- WebSocket communication
- JSON message protocol

---

# Future Extensions

Desteklenebilir:

- MQTT
- NATS
- Event streaming

özellikleri eklenecektir.

---

# Reason

Bu yaklaşım:

- NAT problemi azaltır
- güvenliği artırır
- mobil cihaz desteği sağlar
- cihaz değişimini kolaylaştırır

---

# Security Principle

Hermes hiçbir cihaza doğrudan erişmez.

Worker:

- kimliğini doğrular
- yeteneklerini bildirir
- izin verilen görevleri çalıştırır
