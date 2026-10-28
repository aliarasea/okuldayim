# Metrik, Log, İz Deposu ve Alarm İhtiyaç Analizi

28 Ekim 2026 · Sürüm 1.1 · Staging/üretim öncesi · Sorumlu: altyapı; iş alarmları: backend/ürün

Kaynaklar: [ortak SLO/süreler](README.md), [Collector](opentelemetry-requirements.md), [Sentry](sentry-requirements.md), [kurtarma](backup-recovery-requirements.md).

## Amaç ve sistem sınırı

Metrik/log/izlerin kalıcı yurt içi depolarını, dashboard ve alarm yönlendirmesini sağlar. OTel taşıma katmanı ve Sentry hata gruplaması bu ihtiyacı tek başına karşılamaz. Prometheus uyumlu metrik deposu, log ve trace deposu, dashboard/alarm bileşenleri seçilmelidir; mevcut Compose'da yoktur.

| Kimlik | Gereksinim | Kabul |
| --- | --- | --- |
| OBS01 | Tüm altyapı/API/worker collector'ları ve üretim hataları izlenir; erişim özel ağ/MFA/RBAC | Görünmeyen temel bağımlılık yok; yetkisiz log erişimi reddi |
| OBS02 | API p95/başarılı iş oranı, çevrimiçi canlı p95, queue yaşı ve sağlayıcı kabulü ayrı | Ortak SLO dashboard ve ayırt edilmiş failure senaryosu |
| OBS03 | PG/WAL, Vault lease/audit/seal, Redis memory/eviction, S3 cleanup/capacity, ClamAV signature yaşı, sertifika, backup alarmları; zamanlanmış iş ve manifest onay gecikmesi, denetim checkpoint'i, `ENGELLENDI` teslim | Bir kontrollü failure her ilgili nöbetçiye ulaşır |
| OBS04 | Kaynakta sır/içerik filtreleme, route template logları, sınırlı cardinality | C10 ve seri/byte bütçesi testi |
| OBS05 | Operasyon log/iz ≤90 gün; iş audit ve güvenlik denetimi (Vault audit, pgaudit) 2 yıl, imha kaydı ≥3 yıl farklı erişim/depo politikaları | Kategori bazlı expire/erişim ve silme testi |
| OBS06 | Alarm kanalı ve nöbetçi/eskalasyon/runbook; alert mesajında okul/sağlık içeriği yok | Tatbikat, ack ve çözüm süresi kaydı |

## İşletim ve kapasite

Request-id/trace-id ile korelasyon yapılır; metrik label'ına kullanıcı, çocuk, token veya sınırsız hedef id eklenmez. Operasyon trace/log yüksek hacimde bounded sampling uygular; zorunlu iş denetimi sampling ile atılmaz. Denetim zinciri kurum başına tutulur; zincir başı her gün Vault `denetim-checkpoint` anahtarıyla imzalanıp bağımsız depoya yazılır. Checkpoint gecikmesi ve doğrulama hatası alarm üretir. Log arşivi anahtar/sır kopyası değildir.

Seri sayısı × retention ve log/trace event byte × hacim hesaplanır. Disk doluluğu, collector drop, metric scrape kaybı ve alarm sisteminin kendisi bağımsız heartbeat ile izlenir. Bütün gözlem bileşenlerinin kesintisi iş verisini bozmaz; zorunlu audit yazması gerçekleşemeyen hassas işlem güvenli reddedilir.

OBS01–OBS06, C09/C10/C12 gerçek veri kapısı koşuludur. Dashboard ürünü, log/trace store, alarm kanalı ve bütçesi altyapı sahibi tarafından seçilir; seçimde yurt içi konum ve bakım/restore yetenekleri doğrulanır.
