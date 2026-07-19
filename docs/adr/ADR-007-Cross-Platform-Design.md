# ADR-007: Cross Platform Design

## Status

Accepted

## Date

19.07.2026

---

# Context

HermesOS farklı fiziksel cihazlarda ve farklı işletim sistemlerinde çalışabilmelidir.

Kullanıcı cihazları zaman içinde değişebilir:

- Raspberry Pi
- Linux bilgisayarlar
- Windows sistemler
- macOS cihazlar
- gelecekteki platformlar

Sistem belirli bir işletim sistemine bağımlı olmamalıdır.

---

# Decision

HermesOS platform bağımsız tasarlanacaktır.

Desteklenen temel platformlar:

- Linux
- macOS
- Windows

---

# Implementation

Platforma özel başlangıç ve yönetim scriptleri bulunacaktır.

## Unix Based Systems

Linux ve macOS için:

```
bash
```

tabanlı scriptler kullanılacaktır.

Örnek:

```
scripts/bootstrap.sh
```

---

## Windows

Windows için:

```
PowerShell
```

kullanılacaktır.

Örnek:

```
scripts/bootstrap.ps1
```

---

# Future Direction

İlerleyen aşamalarda platform bağımsız bir Rust tabanlı araç geliştirilebilir.

Örnek:

```
hermes-init
```

Bu araç:

- işletim sistemi algılama
- gerekli bağımlılıkları kurma
- worker kaydı
- ilk konfigürasyon

işlemlerini yönetebilir.

---

# Reason

HermesOS'un temel prensibi:

> Donanım ve işletim sistemi değişebilir, sistem kimliği ve mimari kalıcıdır.
