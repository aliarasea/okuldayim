# OpenTelemetry Collector İhtiyaç Analizi

28 Ekim 2026 · Sürüm 1.0 · Staging/üretim öncesi · Sorumlu: backend ve altyapı

Kaynaklar: [ortak sözleşmeler](README.md), [log/metrik/iz depoları](monitoring-logging-requirements.md), [backend §3.13](../backend-requirements-analysis.md).

## Amaç ve veri akışı

API→DB/Vault ve outbox→worker→sağlayıcı iz ilişkisini OTLP ile toplar; tek başına kalıcı iz deposu veya alarm ürünü değildir. Collector ve seçilecek Türkiye konumlu iz deposu mevcut Compose'da yoktur. Kaynak SDK'da ayıklama Collector'da ikinci filtreyle desteklenir.

| Kimlik | Gereksinim | Kabul |
| --- | --- | --- |
| OT01 | OTLP endpoint özel ağ, auth/TLS ve tenant-independent servis kimliği | Yetkisiz exporter reddi |
| OT02 | Authorization/cookie/query/telefon/OTP/PIN/sağlık alanları export öncesi silinir | C10; sentetik sır span/resource/event alanlarında yok |
| OT03 | Span adı route şablonu; kullanıcı/çocuk/kurum UUID'si metric label yapılmaz | Cardinality sınırı ve örnek payload incelemesi |
| OT04 | Async outbox trace context/link taşır; uzun job yeni span olur | API commit ile worker/send ilişkisi izlenir |
| OT05 | Bounded queue/batch/memory ve sampling; Collector kesintisi iş isteğini bloke etmez | Collector kapalıyken API çalışır; drop/failure alarmı |
| OT06 | Exporter yalnız onaylı yurt içi iz/metrik deposuna gider | Konum/receiver/exporter envanteri |

## İşletim ve kabul

Sampling, hata/gecikme analizini koruyacak bütçeyle seçilir; her trace'i tutma zorunlu değildir. Capture edilmiş güvenlik metaverisi kişisel veri olabileceği için erişim/kategori/saklama sınırı uygulanır. Operasyon trace retention en çok 90 gündür; gereksiz veri daha erken silinir.

Collector CPU/memory, batch queue/drop, exporter retry ve disk spool seçildiyse şifreleme/cleanup izlenir. Sampling/saklama byte bütçesi request/job hacminden hesaplanır. OT01–OT06 ve C09/C10 kanıtı staging smoke/yük testine girer; collector kurulumu iz deposunun kurulduğu anlamına gelmez.
