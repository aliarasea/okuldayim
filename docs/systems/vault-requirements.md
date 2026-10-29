# Vault İhtiyaç Analizi

28 Ekim 2026 · Sürüm 1.1 · Faz 1a · Sorumlu: altyapı, backend ve anahtar operasyonu yetkilileri

Kaynaklar: [ortak sözleşme](README.md), [bootstrap](../../infra/vault/scripts/bootstrap.sh), [backend politikası](../../infra/vault/policies/okuldayim-backend.hcl), [imha politikası](../../infra/vault/policies/kvkk-ops.hcl), [konfigürasyon](../../infra/vault/config/vault.hcl).

## Amaç ve anahtar envanteri

Sırların, alan şifreleme anahtarlarının, JWT imzasının ve kısa ömürlü veritabanı kimliklerinin sahibi Vault'tur. API ve worker'ın uyguladığı kurum yetkisini Vault'un kurum wildcard politikası tek başına sağlamaz. Veritabanı ile ayrı erişim alanında Türkiye'de çalışır.

| Kaynak | Kullanım | Yaşam döngüsü |
| --- | --- | --- |
| `transit/keys/kurum-<uuid>` | Sağlık ve kurum kapsamlı hassas alanlar | Yıllık sürüm rotasyonu; eski sürüm kontrollü rewrap; hukuki muhafaza bitmeden silinmez |
| `transit/keys/kullanici-pii` | Telefon, e-posta, davet hedefi, push belirteci ve TOTP sırrı | Platform kapsamı; kullanıcı silmesi nesne/kayıt silmesidir, ortak anahtar silmesi değildir |
| `transit/keys/blind-index` | Telefon ve e-posta HMAC araması (alan türü ön ekli) | HMAC sürümü kayıtta tutulur; dual-read/reindex planı olmadan rotasyon yapılmaz |
| `transit/keys/jwt-signing` | ES256 erişim imzası | 90 gün rotasyon; eski açık anahtarlar token ömrü ve saat sapması bitene dek doğrular |
| `transit/keys/denetim-checkpoint` | Günlük denetim zinciri başı imzası | JWT anahtarından ayrı; rotasyonda eski açık anahtarlar checkpoint saklama süresince doğrular |
| `secret/okuldayim/backend` | Redis/S3 erişimi, `otp_pepper`, yeni `pickup_pepper` ve `media_link_key`, sağlayıcı sırları | Sürüm/rotation ve erişim sahipleri belirli. `media_link_key` rotasyonunda eski sürüm en çok bağlantı ömrü (5 dk) kadar doğrular; sırlar loglanmaz |
| Database `okuldayim-app` | RLS'e tabi runtime DB kimliği | Mevcut default 1 saat/max 24 saat; token lease sınırlarıyla birlikte yenilenir |
| Retention/provisioning kimlikleri | Sınırlı imha ve kurum anahtarı kurulumu | Ayrı rol/politika; sıradan API kimliğine imha yetkisi verilmez |

## Gereksinimler ve arayüz

| Kimlik | Gereksinim | Kabul kanıtı |
| --- | --- | --- |
| VT01 | Auth yöntemini barındırma seçimi belirler: Kubernetes varsa service account, aksi durumda kısa ömürlü workload kimliği/AppRole güvenli dağıtımı | Root token veya kalıcı SecretID uygulama imajında yok |
| VT02 | Ciphertext ve HMAC sürümleri saklanır; batch decrypt yalnızca yetkili satırlara uygulanır | C01/C06; anahtar rotasyonu sonrası eski/yeni veriler okunur |
| VT03 | Transit ES256 imzası `marshaling_algorithm=jws` ile alınır; `vault:vN:` sarmalı ayrılır ve JWT kütüphanesinin JWS raw imza beklentisi doğrulanır | Yerel JWT doğrulama entegrasyon testi |
| VT04 | App token yenileme ve yeniden auth, DB lease yenileme/yeniden üretimi ayrı durum makineleridir | Token max TTL/lease expire testi; bağlantı geçişi |
| VT05 | İmha politikası onaylanan tek kurum UUID'sine daralır; JWT/kör indeks/platform anahtarları kapsam dışında | Yetkisiz anahtar silme/trim/config reddi ve iki kişi operasyon kanıtı |
| VT06 | Denetim cihazları ve alarmı çalışır; audit yazılamadığında işlem davranışı fail-closed kabul edilir | Disk doluluğu/collector kesinti testi |
| VT07 | Raft snapshot, seal/auto-unseal anahtarı ve geri yükleme prosedürü birlikte korunur | C08; yalnız DB yedeğiyle başarılı sayılmaz |

## Kapasite ve kesinti

Decrypt/sign/HMAC çağrı sayısı ve p95 ölçülür; batch kullanılır, bağlantı yeniden kullanılır. Transit CPU ve disk I/O ile 10× yük denenir. Vault erişilemezse yeni oturum imzası ve decrypt gerektiren işlemler 503 verir; güncel DB yetkisi ve önbellekteki geçerli açık anahtarlarla hassas çözme gerektirmeyen işlemler devam edebilir. Tanınmayan `kid` yenilemesi sınırlanır; her hatalı token için Vault çağrısı açılmaz.

Üretimde en az üç Raft düğümü, özel ağ, TLS ve doğrulanmış Türkiye konumlu HSM/KMS auto-unseal gerekir. Auto-unseal harici KMS'e bağımlıysa konumu ve kesinti etkisi veri akışı envanterine girer. Anahtarlar normal operatöre dışa aktarılamaz; root token rutin iş kimliği değildir.

Yeni DB kimliği/havuzu hazırlanır, trafik geçirilir, eski havuz lease dolmadan boşaltılır. Yalnız lease renew'e güvenilmez; DB `VALID UNTIL`, rol iptali ve açık bağlantı davranışı test edilir. Eski rol iptal edilemiyorsa alarm/cleanup işi gerekir.

## Mevcut farklar ve teslim

Yerel Compose/bootstrap `kullanici-pii`, `denetim-checkpoint`, `pickup_pepper`, `media_link_key` ve DB lease yolunu sağlar. API AppRole ile token alıp yeniler, KV'den Redis parolasını okur ve 1 saatlik Vault DB kimliğiyle PostgreSQL pool'u kurup yeni kimliğe rotasyon yapar; API henüz Transit ürün işlemlerini kullanmaz. AppRole dosyası API'ye root token/unseal dosyalarından ayrı named volume üzerinden salt okunur verilir. Runtime politikasındaki kurum anahtarı oluşturma/rewrap erişimi üretimde ayrı provisioning/rotation rolleriyle daraltılmalıdır. `kvkk-ops` mevcut wildcard'ı tüm Transit anahtarlarına ulaşabilir; yorumdaki iki kişi onayı gerçek teknik iş akışı değildir. Yerel bootstrap'ın tek AppRole kullanması üretim yetki ayrımı yerine geçmez.

Yerel tek düğüm/TLS'siz/root-token diskte yaklaşımı üretim değildir. Bootstrap yeniden çalıştırılınca yeni AppRole SecretID üretilir; eski kimliklerin iptali ve backend yeniden yüklemesi yönetilir. İlk kurulum/kurtarma sırasında sırlar çıktıya verilmez. VT01–VT07, C01/C06/C08/C10/C12/C13 geçmeden üretim anahtarları açılmaz.
