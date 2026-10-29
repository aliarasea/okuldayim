# Okuldayım — yerel geliştirme altyapısı

Bu dizin, ihtiyaç analizlerindeki Faz 1a backend bağımlılıklarını yerelde çalıştırmak için Docker Compose yapılandırmasını, servis başlangıç betiklerini ve yerel ayarları içerir. Bu bir üretim dağıtımı değildir. Rust API runtime Compose ile çalıştırılır; ürün uç noktaları, worker ve üretim dağıtım bileşenleri henüz tamamlanmamıştır.

Sistem gereksinimleri ve mevcut faz durumu: [backend sistem ihtiyaç analizleri](../docs/systems/README.md) ve [backend gereksinim analizi](../docs/backend-requirements-analysis.md).

## Gereksinimler ve başlatma

- Docker Engine veya Docker Desktop; Docker Compose v2 (`docker compose` komutu).
- POSIX uyumlu shell; `generate-dev-env.sh` için `tr` ve `sed`.
- İlk çalıştırmada container imajlarını indirebilmek için registry erişimi ve servisler için yeterli disk/RAM.

```sh
cd infra
./scripts/generate-dev-env.sh   # infra/.env'i üretir; varsa üzerine yazmaz
docker compose config -q        # Compose sözdizimini ve zorunlu değişkenleri doğrular
docker compose up -d
docker compose ps
```

PostgreSQL, Redis, Vault, RustFS, Mailpit ve API servisleri sağlıklı duruma gelmeli; `vault-bootstrap`, `vault-approle-volume-init` ve `storage-init` tek seferlik servisleri `Exited (0)` göstermelidir. `docker compose logs vault-bootstrap storage-init api` ile başlangıç hataları incelenebilir. Başarısız bir init servisini tekrar çalıştırmak için `docker compose run --rm vault-bootstrap` veya `docker compose run --rm storage-init` kullanın.

PostgreSQL uygulama şemasını hazırlamak için servis sağlıklı olduktan sonra migration aracını ayrıca çalıştırın:

```sh
docker compose --profile tools run --rm postgres-migrate
```

Bu işlem `infra/postgres/migrations/` içindeki SQL dosyalarını sırayla transaction içinde uygular, `app.schema_migrations` tablosuna kaydeder ve RLS/yetki kontrollerini yapar. Migration geçmişi olmayan uygulama tabloları bulunursa durur; mevcut tabloları veya volume'ları silmez.

`okuldayim_app` runtime rolü `organization` kayıtlarını yalnızca okuyabilir; kurum oluşturma geniş bir RLS `INSERT` politikasıyla açılmaz. Kurum onboarding'i, ayrıca tanımlanacak onaylı provisioning akışı/kimliği üzerinden yapılmalıdır.

API operasyon uç noktaları `http://127.0.0.1:8080/healthz`, `/readyz` ve `/openapi.json` adreslerindedir. Liveness yalnızca prosesin çalıştığını bildirir; readiness DB lease/pool durumunu zorunlu tutar, Redis kesintisinde `degraded` ile `200` döndürür. API image'ı Compose ile `--build` seçeneği kullanılarak oluşturulur.

## Servisler ve neden kullanıldıkları

| Servis | Yerel adres | Kullanım | Veri / sınır |
| --- | --- | --- | --- |
| PostgreSQL 18 | `127.0.0.1:5432` | Kalıcı uygulama verisi; owner, migrator, runtime grubu ve Vault dinamik DB rolü için başlangıç rolleri | `postgres-data` volume'u. Uygulama şeması `postgres-migrate` ile ayrıca kurulur; healthcheck yalnız bağlantı kabulünü doğrular. |
| Rust API | `127.0.0.1:8080` | Operasyon HTTP yüzeyi; Vault AppRole, dinamik PostgreSQL kimliği/pool rotasyonu ve Redis readiness bağlantısı | Non-root container. Yalnız AppRole dosyası içeren `vault-backend-approle` volume'unu salt okunur bağlar; Vault root token/unseal anahtarını içeren geliştirme dizinini bağlamaz. Henüz ürün API akışlarını sunmaz. |
| Redis 8 | `127.0.0.1:6379` | Kısa ömürlü sayaçlar, hız sınırlama ve Pub/Sub | Kalıcılık kapalı; `default` kapalı ve `app` ACL kullanıcısı parolalıdır. `noeviction`, 256 MB Redis bütçesi ve 384 MB container limiti bellek baskısında güvenlik sayaçlarının sessizce atılmasını önler; uygulama yazma hatasında doğrulamayı fail-closed yapmalıdır. |
| HashiCorp Vault | `http://127.0.0.1:8200` | Transit anahtarları, KV sırları ve kısa ömürlü PostgreSQL kimlik bilgileri | `vault-data` ve `vault-logs` volume'ları. Tek düğüm/TLS'siz geliştirme kurulumu; her yeniden başlatmada mühürlü açılabilir. |
| RustFS (S3 uyumlu) | API `127.0.0.1:9000`, konsol `127.0.0.1:9001` | Yerel medya ve veri dökümü bucket'ları | `object-storage-data` volume'u. Bucket'lar özel; init betiği karantina ve export bucket'larına 1 günlük asenkron lifecycle uygular. Kesin silme worker sorumluluğudur. |
| Mailpit | SMTP `127.0.0.1:1025`, arayüz `http://127.0.0.1:8025` | Geliştirme e-postalarını yakalamak ve OTP akışını incelemek | Dışarı e-posta göndermez. UI ve SMTP yalnız yerel makineden erişilir. SMS adaptörü Jasmin'e gider ve Mailpit'e yönlendirilmez. |
| Jasmin SMS Gateway | Harici `OKD_SMS_GATEWAY_URL` (`/send`) | İsteğe bağlı açık kaynak SMS gateway adaptörü | Gateway bu Compose'a eklenmez; gerçek SMS için ayrı Jasmin kurulumu, SMSC/operatör bağlantısı, kullanıcı/route ve kredi gerekir. Jasmin parolası API'ye değil Vault KV'ye aktarılır. |

Tüm host port eşlemeleri `127.0.0.1`'e bağlanır. Servislerin container ağı içinde dinlemesi, host ağından veya üretim ağından erişim izni anlamına gelmez. Bu portları tünel/proxy ile dışarı açmayın.

## Yapılandırma dosyaları

| Dosya | Neden var / ne zaman çalışır |
| --- | --- |
| `docker-compose.yml` | Yerel servisleri, portları, volume'ları, healthcheck'leri ve init sırasını tanımlar. `docker compose up -d` ile container'ları oluşturur/başlatır. |
| `.env.example` → `.env` | Yalnız yerel geliştirme parolaları ve host portları. `.env` Git'e alınmaz; `generate-dev-env.sh` rastgele değer üretir ve mevcut dosyayı değiştirmez. Jasmin URL/user/password opsiyoneldir; operatör hesabı olmadan boş bırakılır. |
| `scripts/generate-dev-env.sh` | İlk çalıştırmada `.env` üretir; geliştirme parolaları Vault/S3/Redis/PostgreSQL container'larına aktarılır. Üretim sırrı üretmek için kullanılmaz. |
| `postgres/initdb/01-roles.sh` | PostgreSQL data volume'u ilk kez boşken çalışır; owner/migrator/app/Vault rollerini ve `app` şemasını kurar. Var olan volume'da tekrar çalışmaz. |
| `postgres/migrations/0000_migration_ledger.sql` | Migration geçmişi tablosunu idempotent biçimde kurar; uygulama tablolarına dokunmaz. |
| `postgres/migrations/0001_initial_schema.sql` | İngilizce adlandırılmış normalize tabloları, tenant RLS politikalarını, erişim indekslerini ve runtime yetkilerini transaction içinde kurar. |
| `postgres/migrate.sh` | Migration'ları sırayla uygular, geçmişi denetler; geçmişsiz tablolar varsa durur ve RLS ile runtime rolünü doğrular. |
| `redis/redis.conf` | Yerel kalıcılıksız Redis'i ve bellek sınırını ayarlar. ACL parolası Compose başlangıcında geçici ACL dosyasına yazılır. |
| `vault/config/vault.hcl` | Yerel tek düğümlü Raft, UI ve TLS'siz listener ayarı. Üretim ayarı değildir. |
| `vault/scripts/bootstrap.sh` | Vault'u ilk kez başlatır/mührünü açar; audit, Transit/KV/Database motorlarını, anahtarları, sırları, DB rolünü, AppRole'u ve politikaları idempotent biçimde kurar. Yerel root token ile çalışır. |
| `vault-backend-approle` named volume ve `vault-approle-volume-init` | Backend AppRole dosyasını Vault bootstrap dosyalarından ayırır; API'ye en az sır erişimiyle salt okunur verilir. |
| `vault/policies/*.hcl` | Backend AppRole ve KVKK operasyonlarının Vault erişim sınırlarını tanımlar. Üretimde provisioning/retention rolleri ayrıca ayrılmalıdır. |
| `object-storage/init-buckets.sh` ve `lifecycle-1-day.json` | Özel medya/quarantine/export bucket'larını ve geçici nesneler için asenkron lifecycle kuralını oluşturur. Betik tekrar çalıştırılabilir. |
| `.gitignore` | `.env` ve `.local/` içindeki yerel parolaların, Vault root token/unseal anahtarının ve AppRole dosyasının repoya girmesini engeller. |

### Vault'ta hazırlanan yerel kaynaklar

`vault-bootstrap` şu bileşenleri oluşturur:

- Transit: `blind-index`, `kullanici-pii`, `jwt-signing` (90 günde rotasyon) ve ayrı `denetim-checkpoint` anahtarları. Kurum başına anahtarlar kurum provisioning akışında oluşturulur.
- KV v2: `secret/okuldayim/backend` altında OTP pepper'ı, Redis/S3 yerel kimlik bilgileri, teslim PIN'i için `pickup_pepper` ve Faz 2 medya bağlantıları için `media_link_key`. Pepper/link anahtarları sadece eksik olduklarında üretilir; her bootstrap'ta değiştirilmez.
- Database secrets engine: `database/creds/okuldayim-app` üzerinden 1 saatlik, `okuldayim_app` grubuna bağlı geçici PostgreSQL rolü.
- AppRole ve politika: `okuldayim-backend` için `.local/vault/backend-approle.env`; Transit PII/checkpoint işlemleri ve DB lease yenileme izinleri dahildir. `kvkk-ops` yerel politika dosyasıdır.

Vault ilk açılışta root token ve unseal anahtarını `infra/.local/vault/` altına düz metin yazar. Bu yalnızca sentetik yerel geliştirme içindir. Bootstrap betiğini yeniden çalıştırmak yeni AppRole SecretID üretip dosyayı yeniler; bağlı backend yeniden başlatılmalı ve eski kimliklerin geçerliliği ayrıca yönetilmelidir. Vault yeniden mühürlenmişse `docker compose run --rm vault-bootstrap` ile açıp yapılandırabilirsiniz.

## Uygulama bağlantıları

API runtime bugün Vault, PostgreSQL ve Redis'e bağlanır; isteğe bağlı Jasmin transport client'ı da `/readyz` üzerinde yapılandırma durumunu bildirir. Aşağıdaki ürün ve yardımcı süreçlerin bir kısmı henüz uygulanmamıştır. Tam veri akışı tasarımı [backend analizindedir](../docs/backend-requirements-analysis.md):

- Migration: `okuldayim_migrator`; migration bağlantısında `SET ROLE okuldayim_owner`.
- API: Vault AppRole ile giriş, `database/creds/okuldayim-app` üzerinden geçici DB kimliği ve lease'in %70'inde yeni pool'a rotasyon. Üretimde AppRole yerine barındırmaya uygun workload kimliği gerekir.
- PostgreSQL: `127.0.0.1:5432`, database `okuldayim`, schema `app`; uygulama tabloları `postgres-migrate` çalıştırılınca hazırlanır.
- Redis: `redis://app@127.0.0.1:6379`; parola Vault KV'den.
- SMS: `OKD_SMS_GATEWAY_URL=https://<jasmin-host>/send`; `JASMIN_USERNAME` / `JASMIN_PASSWORD` yalnızca yerel bootstrap girdileridir ve Vault KV'ye yazılır. Gateway/SMSC bu Compose tarafından kurulmaz.
- S3: `http://127.0.0.1:9000`; bucket'lar `okuldayim-media-quarantine`, `okuldayim-media`, `okuldayim-exports`.
- SMTP: `127.0.0.1:1025`; yakalanan e-posta `http://127.0.0.1:8025` arayüzünde görünür.

Host'tan `cargo run -p okuldayim-api` ile başlatmak için `OKD_VAULT_APPROLE_FILE=../infra/.local/vault/backend-approle.env` kullanılabilir. Bu dosya yalnızca sentetik geliştirme içindir; üretim sırrı veya image içine kopyalanmamalıdır.

Kiracı verisi eklenmeden önce migration'lar RLS `USING`/`WITH CHECK` ve `FORCE`, append-only kayıt yetkileri ve doğru rol sınırlarını kurmalıdır. Yerel servis healthcheck'leri bu veritabanı kabul testlerinin yerine geçmez.

## Kapatma ve veri ömrü

```sh
docker compose stop       # container'ları durdurur, veriyi korur
docker compose down       # container/ağı kaldırır, adlandırılmış volume'ları korur
```

`docker compose down -v` PostgreSQL, Vault ve RustFS volume verisini kalıcı olarak siler. Vault volume'u silinirse `.local/vault` içindeki eski root/unseal/AppRole dosyaları da yeni Vault ile eşleşmeyeceğinden, sıfırlama öncesi bunları yedekleyip/ayıklayıp yeni kurulum oluşturun. Bu işlem veritabanı ve anahtarları geri döndürülemez biçimde kaybettirir; normal kapatma için kullanmayın.

## Kapsam dışı olanlar ve üretim kapısı

İhtiyaç analizindeki Faz 2 ve staging/üretim sistemleri bu yerel compose'a bilerek eklenmedi:

- ClamAV ve medya worker'ı Faz 2; uygulama tarama entegrasyonu, dosya limitleri ve fail-closed testi olmadan tek başına scanner container'ı güvenlik sağlamaz.
- OpenTelemetry Collector, Sentry, metrik/log/iz depoları ve alarm sistemi staging/üretim öncesi; Türkiye'de barındırma, erişim, saklama ve ürün seçimleri netleşmeden yerel servis eklemek bu koşulları karşılamaz.
- Ingress/DNS/TLS, PostgreSQL HA/fencing, Vault 3 düğüm + KMS/HSM, bağımsız imha manifesti, PITR/WAL ve yedekler gerçek veri öncesi üretim gereksinimleridir.
- Jasmin açık kaynak gateway'i seçilmiş ve HTTP transport adaptörü vardır; ancak SMS operatörü/SMSC hesabı ve Jasmin kurulumu haricidir. E-posta için Mailpit yalnızca yerel SMTP yakalayıcısıdır. OTP uçları, SMS işçisi, DLR callback'i, push ve ürün API'si henüz mevcut değildir.

İmaj etiketleri yerel geliştirme tercihleridir; imaj digest'i, destek süresi, güvenlik taraması, yedekleme/restore ve C01–C13 kabul kanıtları doğrulanmadan üretimde kullanılamaz. Bu ortamda gerçek çocuk, veli veya sağlık verisi kullanmayın.
