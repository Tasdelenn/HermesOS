# HermesOS Architecture Decisions

Bu dosya HermesOS mimarisindeki önemli kararları ve nedenlerini kayıt altında tutar.

---

# ADR-001: Dağıtık Agent Mimarisi

## Karar

Hermes doğrudan cihazları yönetmeyecek.

Arada Worker katmanı bulunacak.

## Sebep

- Güvenlik
- Yetki kontrolü
- Farklı işletim sistemlerini destekleme
- Hata izolasyonu

## Mimari

```
Hermes Brain

      |

      v

Worker Agent

      |

      v

Device
```

---

# ADR-002: Cloud Brain + Local Worker

## Karar

LLM merkezi olarak VPS üzerinde çalışacak.

Yerel cihazlar görev uygulayacak.

## Sebep

Yerel cihazların:

- GPU gücü sınırlı
- RAM kapasitesi düşük
- sürekli çalışmaya uygun değil

olması.

---

# ADR-003: Hafıza Sistemi

## Karar

Hermes hafızası katmanlı olacak.

Katmanlar:

1. Working Memory

Geçici görev bilgileri.

2. Long Term Memory

Veritabanı ve vektör hafıza.

3. Crystal Memory

Obsidian Markdown bilgi bankası.

---

# ADR-004: Rust Kullanımı

## Karar

Performans kritik servislerde Rust kullanılacak.

Örnek:

- Worker Agent
- Network servisleri
- Edge daemon
- CLI araçları

## Sebep

- Düşük RAM kullanımı
- Tek binary dağıtım
- Raspberry Pi uyumluluğu
- Güvenli sistem programlama

---

# ADR-005: Açık Standartlar

## Karar

Sistem mümkün olduğunca:

- MQTT
- REST API
- WebSocket
- Markdown
- Git

gibi açık standartları kullanacak.

---

# ADR-006: Device Independence

## Karar

HermesOS fiziksel cihazlara bağlı olmayacaktır.

Cihazlar değiştirilebilir worker node olarak kabul edilir.

## Sebep

Donanım yaşam döngüsü yazılım yaşam döngüsünden farklıdır.

Bir cihazın kaybı veya değişimi sistem mimarisini bozmamalıdır.

## Uygulama

- Cihazlar rol ile tanımlanır.
- Kimlikler merkezi yönetilir.
- Hafıza merkezi tutulur.
- Capability sistemi kullanılır.
- Yeni cihazlar kolayca sisteme dahil edilir.

---

# ADR-009: Worker Enrollment ve mTLS Geçişi

## Karar

İlk Worker kaydı, cihaz başına benzersiz, süreli ve tek kullanımlık registration
token ile yapılacaktır. Normal Worker bağlantıları için uzun vadeli hedef mTLS'tir.

## Sebep

İlk prototipin kurulumu basit olmalı; buna karşılık kalıcı token tabanlı
kimlik doğrulama uzun vadede yeterli güvenlik sağlamaz.

Detay: [[adr/ADR-009-Worker-Enrollment-and-mTLS-Migration|ADR-009]].

---

# ADR-014: Worker Hello Kimlik Doğrulaması

## Karar

Enrollment sırasında Brain her worker'a özel rastgele bir credential üretir;
`hello` yalnızca kayıtlı `worker_id` ve doğru credential ile kabul edilir.
Brain yalnızca credential'ın SHA-256 özetini saklar.

## Sebep

Önceden `hello` kimlik doğrulamasız kabul ediliyordu; token tabanlı enrollment
bu yüzden fiilen atlanabiliyordu.

Detay: [[adr/ADR-014-Worker-Hello-Authentication|ADR-014]].

