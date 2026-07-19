# HermesOS Vision

## Proje Amacı

HermesOS, kişisel yapay zeka destekli bir işletim sistemi altyapısıdır.

Amaç:
- Bulut tabanlı yapay zeka ajanlarını,
- evdeki fiziksel cihazları,
- kişisel bilgisayarları,
- işletme altyapısını,
- bilgi yönetim sistemlerini

tek bir kontrollü ekosistem altında birleştirmektir.

---

## Temel Fikir

HermesOS bir chatbot değildir.

Bir dijital çalışma ortağıdır.

LLM sadece karar veren beyin katmanıdır.

Asıl değer:
- hafıza,
- otomasyon,
- cihaz entegrasyonu,
- güvenli görev çalıştırma,
- kişisel bilgi birikimidir.

---

## Mimari Prensipler

### 1. Dağıtık yapı

Sistem tek bir bilgisayara bağlı olmayacaktır.

Bileşenler:
- VPS
- Raspberry Pi
- Laptoplar
- Mobil cihazlar

olarak çalışacaktır.

---

### 2. Güvenlik

Hiçbir ajan sınırsız yetkiye sahip olmayacaktır.

Her cihaz:
- kendi yeteneklerini,
- kendi izinlerini,
- kendi sınırlarını

belirleyecektir.

---

### 3. Taşınabilirlik

Projeler:
- microSD
- Git
- Docker
- açık standartlar

üzerinden taşınabilir olacaktır.

---

## Ana Bileşenler

### Hermes Brain

Bulutta çalışan karar verme katmanı.

---

### Edge Workers

Yerel cihazlarda çalışan görev ajanları.

Örnek:
- Raspberry Pi
- Windows laptop
- Linux sistemler

---

### Memory System

Katmanlı hafıza:

- çalışma hafızası
- uzun süreli hafıza
- Obsidian bilgi bankası

---

### Communication

İletişim kanalları:

- Telegram
- Web
- API

---

## Uzun Vadeli Hedef

HermesOS zaman içinde:

- kişisel asistan,
- yazılım geliştirme yardımcısı,
- ev otomasyonu yöneticisi,
- işletme takip sistemi,
- bilgi yönetim sistemi

haline gelecektir.