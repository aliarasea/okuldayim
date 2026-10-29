# Redis İhtiyaç Analizi

28 Ekim 2026 · Sürüm 1.1 · Faz 1a · Sorumlu: backend ve altyapı

Kaynaklar: [ortak sözleşmeler](README.md), [redis.conf](../../infra/redis/redis.conf), [Compose ACL](../../infra/docker-compose.yml), [backend §3.3, §3.9](../backend-requirements-analysis.md).

## Amaç ve veri sınırı

OTP challenge/deneme sayaçları, hız sınırlama, kısa süreli üyelik ipuçları ve olay Pub/Sub dağıtımı sağlar. Kalıcı iş, rıza, oturum iptali veya teslim kaydı sahibi değildir. Kimlikler, HMAC indeksleri ve IP tabanlı sayaçlar takma kimlikli veri olabilir; erişim ve TTL kuralları uygulanır.

## Gereksinimler

| Kimlik | Gereksinim | Kabul kanıtı |
| --- | --- | --- |
| RD01 | OTP kontrolü, deneme artışı ve tek kullanımlık tüketim Lua/atomik işlemle yapılır; kod 180 sn | Paralel doğrulamada tek tüketim; deneme sayısı kaybolmaz |
| RD02 | Numara/IP rate-limit pencereleri ve TTL atomik; Redis restart/eviction sonrası pencere sıfırlanarak aşılmaz | C03; restart sonrası OTP kabulü kısıtlanır veya güvenilir DB sayaçları kullanılır |
| RD03 | Oturum ipucu 30 sn, üyelik ipucu 60 sn; PostgreSQL son erişim kontrolünü yapar | Cache'te eski izin tutulsa da iptal edilmiş oturum veri alamaz |
| RD04 | Pub/Sub mesajı yalnızca olay türü, `eventId`, kaynak `version` ve kaynak kimliği taşır; alan adları REST DTO'larıyla aynıdır, PIN/ad/sağlık yoktur. Outbox olay tekrar yayını ve REST uzlaştırması vardır | C04/C10; abone kopması, duplicate ve bağlantı açıkken kayıp |
| RD05 | ACL; varsayılan kullanıcı kapalı, komut/anahtar/kanal kapsamı dar, TLS ve özel ağ | İlgisiz key/channel/admin komutları reddedilir |
| RD06 | Güvenlik sayaçları normal cache eviction'a maruz kalmaz; üretim temel tercihi `noeviction` ve sınırlı cache büyümesi | Memory dolduğunda izin verme yerine kontrollü hata |

## Kesinti davranışı

Redis hatasında PostgreSQL'le doğrulanabilen iş okumaları ve teslim uzlaştırması devam eder. Yeni OTP/verifikasyon/hız sınırı gerektiren işlemler, güvenilir yedek sayaç yoksa 503 döner; limitsiz çalışma yapılmaz. Redis kalıcılıksızsa yeni instance önceki 10 dakikalık limit penceresini bilmez: DB sayaç yedeği veya pencere boyunca auth kabulünü kısıtlayan başlangıç modu gerekir. Mevcut oturumlar PostgreSQL'den doğrulandığından bütün kullanıcıları çıkarmak gerekmez.

Pub/Sub son kullanıcı teslim kanıtı değildir. Worker PostgreSQL outbox'tan yayınlar; publish başarılı olsa da abone olayı kaçırabilir. API eventId/version kullanır; aktif teslim ekranı en çok 5 sn, diğer açık ekranlar 60 sn'de REST ile uzlaşır. Güvenlik iptalinde WS bağlantısı yeniden yetkilendirilir.

## Kapasite ve işletim

Key başına byte × eşzamanlı challenge/cache sayısı + Pub/Sub bağlantı tamponları ölçülür. `maxmemory`, container belleğinin altında ve istemci tamponları için rezervli tutulur. Cache/OTP verilerine ayrı prefix, sınırlı TTL ve kota konur; cache trafiği güvenlik sayaçlarını tehlikeye atarsa ayrı instance kararı verilir.

Üretimde failover/topoloji tercihi barındırma sağlayıcısına göre kanıtlanır; uçuşta komut kaybı ve replica gecikmesi test edilir. OTP/PIN açık değeri, sağlık içeriği, refresh token veya telefon düz metni saklanmaz. Redis yedeği iş verisini kurtarma kaynağı değildir; restart kontrollü yeniden kurulumdur.

Metrikler: memory, eviction (güvenlik prefix'lerinde hedef 0), reddedilen yazma, bağlantı sayısı, Pub/Sub kopması, command p95, auth kısıtlı çalışma durumu. Redis parolası Vault'tan alınır; rotation yeni bağlantıyla test edilir.

## Mevcut fark ve kabul

Yerel yapılandırma 256 MB `noeviction` belleği, 384 MB container sınırı ve kapalı kalıcılık kullanır; bellek dolunca yazmalar reddedilir ve uygulamanın fail-closed davranışı gerekir. API Redis parolasını Vault KV'den alır ve readiness için `PING` yapar; Redis kesintisi `degraded` olarak raporlanır. OTP/rate-limit/oturum cache'i ve Pub/Sub ürün davranışları henüz uygulanmadı. ACL varsayılanı kapatır ve yönetim komutlarını engeller, ancak henüz uygulama key/channel ad alanı tanımlanmadığından `+@all` tabanlı izin yalnızca geliştirme içindir. TLS, sayaç kaybı başlangıç politikası ve üretim topolojisi sağlanmış sayılmaz. RD01–RD06 ile C03/C04/C06/C09/C10/C13 geçmelidir.
