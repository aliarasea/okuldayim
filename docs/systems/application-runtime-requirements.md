# API, Worker, Migration ve Dağıtım İhtiyaç Analizi

28 Ekim 2026 · Sürüm 1.1 · Faz 1a · Sorumlu: backend ve altyapı

Kaynaklar: [ortak sözleşmeler](README.md), [backend workspace/akış tasarımı](../backend-requirements-analysis.md), [PostgreSQL](postgresql-requirements.md), [Vault](vault-requirements.md).

## Amaç ve süreçler

`okuldayim-api`, `okuldayim-worker`, tek seferlik `okuldayim-migrate` ve sentetik `okuldayim-seed` ayrı süreçlerdir. Aynı Rust workspace alan modüllerini paylaşır; CPU yoğun medya işi web runtime thread'lerini bloke etmez. Mevcut durumda API runtime/lockfile ve yerel Compose servisi vardır; worker, migration/seed ikilileri, ürün uç noktaları ve backend CI henüz yoktur.

## Gereksinimler

| Kimlik | Gereksinim | Kabul |
| --- | --- | --- |
| RUN01 | Toolchain/lockfile/image digest, uyum/MSRV/lisans ve bağımlılık taraması sabitlenir | Tekrarlanabilir CI build ve artifact kaydı |
| RUN02 | API istek süresi 10 sn, gövde 1 MB; kaynak/bağlantı/task sayısı sınırlı | C09 ve aşırı büyük input kontrollü hatası |
| RUN03 | TenantTx son yetki/rıza kontrolü + iş/audit/outbox transaction'ı ortak | C01/C02 |
| RUN04 | Worker claim kısa DB transaction'ı; lease, idempotent handler, retry/expiry ve failed-item listesi | C04; publish/call öncesi ve sonrası crash |
| RUN05 | Shutdown istek/WS/claim kabulünü durdurur; iş güvenle tamamlanır veya lease ile geri alınır | Rolling deploy ve worker kill testi |
| RUN06 | Liveness process kontrolüdür; readiness PostgreSQL/güncel yetki ve geçerli DB havuzuyla asgari güvenli hizmet kapasitesini raporlar | Vault kesintisinde geçerli lease/doğrulama anahtarıyla decrypt/imza istemeyen yollar çalışır; ilgili yollar 503; güvenli kapasite/lease biterse readiness 503 |
| RUN07 | Migration tek sahipli/advisory-lock; eklemeli şema ve eski-yeni sürüm birlikte çalışır | İki migrate süreci ve rollout/rollback testi |
| RUN08 | Demo sabit OTP/seed gerçek üretim derlemesinde yok; runtime ortam değişkeni tek savunma değildir | Üretim artifact testinde demo özelliği reddi |
| RUN09 | Zamanlanmış işler (`imha`, `teslim-gun-sonu`, `gun-sonu-raporu` vb.) lease satırı veya transaction kilidiyle tek çalışır; çalışma kaydı tutulur, kaçırılan çalışma bir kez telafi edilir; düğüm saatleri NTP ile senkrondur | İki worker'da tek çalışma; worker kapalıyken kaçan 02:00 çalışmasının telafisi; JWT/TOTP/OTP sürelerini bozan saat sapması alarmı |

## Veri, sır ve hata sözleşmesi

Provider/medya işi senkron HTTP yanıtını uzun süre bekletmez; 202 ve durum kaydı döner. Claim/inflight/failed durumları operatöre görünür; hata payload'larında token/OTP/sağlık içeriği yoktur. İdempotency aynı method/kurum/hedef kullanıcı scope'una bağlıdır; yetkisi geri alınmış kullanıcıya eski başarılı gövde tekrar verilmez. Yalnız commit edilen `2xx` sonuç replay edilir. Aynı anahtarla eşzamanlı istek ilk transaction'ı `lock_timeout` sınırında bekler; süre aşılırsa `503` + `Retry-After` döner, çünkü mobil kuyruk `409`'u kalıcı hata sayar.

Başlangıçta Vault'tan gerekli kimlik ve doğrulama anahtarları edinilmeden ready olunmaz. Sonradan Vault, Redis, S3 veya push kesintisi için her yolun zorunlu capability'si belirtilir; ilgisiz yollar gereksiz kapanmaz. Readiness `ready/degraded/unavailable` nedenini iç ağda raporlar; hiçbir degraded durum eski yetki veya rıza önbelleğiyle erişim vermez. Lease süre sonu, PG kesintisi ve dependency geri gelişi birlikte test edilir.

İmha worker'ı normal app rolünün append-only kısıtını aşmak için genel owner/BYPASSRLS kullanmaz; ayrı, kapsamlı retention kimliği gerekir. Outbox'taki geçici OTP payload'ı şifreli ve 180 sn sınırında silinir. Teslim PIN'inin şifreli gösterim kopyası talep son duruma geçtiği transaction'da silinir.

## Dağıtım ve kapasite

Kubernetes zorunlu teknoloji seçimi değildir; Türkiye konumlu managed runtime da özel ağ, workload auth, HA, graceful shutdown ve kaynak sınırlarıyla kabul edilebilir. API en az iki örnekle tek örnek kaybı testi yapar; worker paralelliği DB pool/S3/CPU bütçesiyle sınırlıdır. Migration ve iki Vault havuzunun geçici bağlantıları toplam bütçeye dahildir.

CI: format/clippy/test, migration/RLS, OpenAPI farkı ve mobil contract harness, SBOM/imaj tarama, staging smoke, kontrollü rollout. `test:contract`/codegen henüz mobilde yoktur; setup teslimi gerekir. Yapılandırma non-secret env ile, sırlar Vault/workload kimliğiyle verilir.

RUN01–RUN09 ve C01/C02/C03/C04/C06/C08/C09/C10/C12/C13 geçmelidir. Nöbetçi, başarısız job tekrar çalıştırma yetkisi, rollback sahibi ve saat/durum izleme altyapı kabulüne girer.
