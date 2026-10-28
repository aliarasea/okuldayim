# Sentry Hata İzleme İhtiyaç Analizi

28 Ekim 2026 · Sürüm 1.0 · Staging/üretim öncesi · Sorumlu: altyapı, backend ve mobil

Kaynaklar: [ortak sözleşmeler](README.md), [telemetry](opentelemetry-requirements.md), [gözlemlenebilirlik](monitoring-logging-requirements.md).

## Amaç ve mevcut durum

Backend/mobil hata ve çökme olaylarını release ve source map sembolleriyle gruplar. Türkiye'de barındırılan kurulum istenir; SaaS eşdeğeri varsayılmaz. Mevcut backend ve mobil bağımlılıklarında çalışan Sentry entegrasyonu yoktur.

| Kimlik | Gereksinim | Kabul |
| --- | --- | --- |
| SEN01 | Uygulama/collector verisi yurt içi sunucuya; staging/üretim projeleri ayrı | DSN/endpoint ve ağ trafiği incelemesi |
| SEN02 | `send_default_pii=false` + before-send/allowlist; breadcrumb/attachment/URL/user alanları da temizlenir | C10; genel ayar tek başına kanıt sayılmaz |
| SEN03 | Source map/debug symbol yalnız yetkili CI kimliğiyle upload; erişim kısıtlı | Release sembol çözümü ve secret taraması |
| SEN04 | Replay, ekran görüntüsü, request body ve benzeri içerik toplama varsayılan kapalı | Çocuk/sağlık içeriği hata olayına eklenmez |
| SEN05 | Kota/sampling/retention (en çok 90 gün), silme ve rol bazlı erişim | Event hacmi ve retention testi |
| SEN06 | Sunucu/SDK hatası uygulama iş akışını bloke etmez; başarısız gönderim ölçülür | Sentry kapalıyken API/mobil işlevleri çalışır |

## Bağımlılık ve işletim

Self-hosted Sentry seçilen destekli sürümün kendi depolama/kuyruk/worker bileşenlerini gerektirir; bu bileşenler uygulamanın PostgreSQL/Redis yetki/veri alanını paylaşmaz. Tam destekli kurulum, lisans, disk/bellek, yedek ve upgrade bütçesi sağlayıcı/sürüm seçiminde doğrulanır. Bir Sentry container'ı yeterli varsayılmaz.

DSN yazma erişiminin istismarına karşı ingress/kota önlemleri; kullanıcı projeleri ve yönetici erişimi MFA; source map release eşleşmesi izlenir. İzinli takma kimlikler sınırlı saklanır; gerçek kullanıcı profili veya sağlık açıklaması serbest exception metnine girmez.

SEN01–SEN06/C10, sentetik backend hata ve gerçek cihaz test çökmesinin sembollü/ayıklanmış görünmesiyle kabul edilir. Başarı yalnız SDK paketinin eklenmesiyle değerlendirilmez.
