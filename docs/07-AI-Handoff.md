# HermesOS AI Handoff Document

## Proje

HermesOS

## Amaç

Kişisel AI işletim sistemi altyapısı.

## Kullanıcı

Hakan

## Şu ana kadar yapılanlar

- Proje klasörü oluşturuldu
- Git başlatıldı
- Vision yazıldı
- Architecture yazıldı
- ADR kayıtları başladı
- Device Independence kararı alındı
- Worker mimarisi oluşturuldu

## Mevcut dosya yapısı

...

## Mimari prensipler

...

## Kesin kararlar

...

## Henüz yapılmayanlar

- VPS seçilmedi
- Hermes Brain kurulmadı
- Worker WebSocket bağlantısı uygulanmadı
- Telegram bağlanmadı

## Bir sonraki adım

Registration token ile ilk kayıt akışına uygun Worker WebSocket bağlantısını
uygulamak. Normal oturumlar için hedef mTLS'tir.

Karar kaydı: [[adr/ADR-009-Worker-Enrollment-and-mTLS-Migration|ADR-009]].

## AI'dan beklenen davranış

- Var olan kararları değiştirmeden devam et.
- Önce dokümantasyonu güncel tut.
- Büyük mimari değişiklikleri ADR olarak kaydet.
