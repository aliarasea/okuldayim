# Okuldayım — yerel geliştirme altyapısı

Backend'in ihtiyaç duyduğu sunucuları Docker Compose ile ayağa kaldırır. Yalnızca geliştirme içindir:
bu ortama üretim verisi girmez, tüm portlar yalnızca `127.0.0.1`'e açılır.

Sistem bazında mevcut farklar, üretim gereksinimleri ve ortak kabul koşulları: [sistem ihtiyaç analizleri](../docs/systems/README.md). Yerel volume ve container healthcheck, HA/yedekleme veya üretim hazır olma kanıtı değildir.

| Servis | Adres | Amaç | KVKK notu |
| --- | --- | --- | --- |
| PostgreSQL 18 | `127.0.0.1:5432` | Ana veritabanı | Uygulama rolü tablo sahibi değil, `BYPASSRLS` yok; kimlikler Vault'tan kısa ömürlü |
| Redis 8 | `127.0.0.1:6379` | Önbellek, OTP sayaçları, pub/sub | `default` kullanıcı kapalı; `app` kullanıcısı yönetim komutlarını çalıştıramaz; kalıcılık kapalı |
| Vault 2.2 | `http://127.0.0.1:8200` | Sırlar, alan şifreleme, JWT imzalama, dinamik DB kimlikleri | Denetim logu açık; backend anahtar silemez, dışa aktaramaz |
| RustFS 1.0 (S3) | API `127.0.0.1:9000`, konsol `127.0.0.1:9001` | Medya depolama | Bucket'lar özel; geçici bucket'larda asenkron `Days: 1` lifecycle, kesin süre için worker gerekir |
| Mailpit | SMTP `127.0.0.1:1025`, arayüz `http://127.0.0.1:8025` | E-posta OTP'lerini yakalar | Dışarıya e-posta göndermez |

## Başlatma

```sh
cd infra
./scripts/generate-dev-env.sh   # .env'i rastgele parolalarla üretir (varsa dokunmaz)
docker compose up -d
docker compose ps               # vault-bootstrap ve storage-init "Exited (0)" olmalı
```

`vault-bootstrap` ilk çalıştırmada Vault'u başlatır ve şunları kurar:

- **Transit:** `blind-index` (telefon gibi şifreli alanlarda arama için HMAC), `jwt-signing` (ECDSA P-256, 90 günde bir döner; özel anahtar Vault'tan çıkmaz). Kurum anahtarları (`kurum-<uuid>`) backend tarafından kurum açılırken oluşturulur.
- **KV v2:** `secret/okuldayim/backend` (OTP pepper, Redis ve S3 erişim bilgileri).
- **Database:** `database/creds/okuldayim-app` her okumada 1 saatlik bir PostgreSQL kullanıcısı üretir.
- **AppRole:** `okuldayim-backend` rolü; bilgileri `infra/.local/vault/backend-approle.env` dosyasına yazılır.
- **Politikalar:** `okuldayim-backend` (en az yetki) ve `kvkk-ops` (kriptografik imha).

Vault mühürlü yeniden açılabilir; tamamlanmış `vault-bootstrap` container'ının sıradan `up -d` ile her defasında tekrar çalışacağı garanti değildir. Yerel ortamda gerekirse `docker compose run --rm vault-bootstrap` açıkça çalıştırılır; bu betik yeni AppRole SecretID üretip dosyayı günceller. Backend kimliği yeniden yüklenmeli, eski kimliklerin yaşam döngüsü yönetilmelidir. Root/unseal dosyaları yalnız geliştirme içindir.

Vault arayüzü için root token: `infra/.local/vault/root.token`. Bu dosya ve unseal anahtarı yalnızca geliştirme kolaylığı içindir.

## Backend bağlantısı

Rust backend'in tasarımı ve mobil istemciyle bağlantısı: [`docs/backend-requirements-analysis.md`](../docs/backend-requirements-analysis.md).

- **Migration'lar:** `okuldayim-migrate` ikilisi `okuldayim_migrator` kullanıcısıyla bağlanır (parola `.env` içinde) ve bağlantı açılınca `SET ROLE okuldayim_owner` çalıştırır; böylece nesnelerin sahibi `okuldayim_owner` olur. Varsayılan şema `app`.
- **Uygulama:** `okuldayim-api` ve `okuldayim-worker`, `vaultrs` ile AppRole girişi yapıp `database/creds/okuldayim-app` üzerinden kimlik alır. Bu kullanıcı yalnızca `okuldayim_app` grubunun yetkilerine sahiptir, tablo oluşturamaz ve RLS'ten muaf değildir.
- **Kiracı izolasyonu:** Kiracı tablolarında RLS açılır ve zorlanır; sunucuda güncel üyelikle doğrulanmış tek kurum transaction-local yazılır. Global kimlik/oturum ve worker claim erişimi ayrı kapsamlı yordamlarla yönetilir:

```sql
ALTER TABLE app.cocuk ENABLE ROW LEVEL SECURITY;
ALTER TABLE app.cocuk FORCE ROW LEVEL SECURITY;
CREATE POLICY kiraci ON app.cocuk
  USING (kurum_id = NULLIF(current_setting('app.kurum_id', true), '')::uuid)
  WITH CHECK (kurum_id = NULLIF(current_setting('app.kurum_id', true), '')::uuid);

-- Backend, her transaction başında (parametreli, enjeksiyona kapalı):
SELECT set_config('app.kurum_id', $1, true);
```

- **Denetim ve rıza tabloları** yalnızca eklenebilir olmalı: migration'da `REVOKE UPDATE, DELETE ON app.denetim_kaydi FROM okuldayim_app;`.

## Medya bucket'ları

| Bucket | İçerik | Yaşam döngüsü |
| --- | --- | --- |
| `okuldayim-media-quarantine` | Mobilin imzalı URL ile yüklediği ham dosyalar | 1 gün |
| `okuldayim-media` | EXIF'i silinmiş, gerekli yüzleri bulanıklaştırılmış yayın kopyaları | Saklama motoru siler |
| `okuldayim-exports` | KVKK-06 veri dökümü paketleri | 1 gün |

`Days: 1` lifecycle asenkrondur; kesin 24 saatlik silmeyi worker uygulamalıdır. Mevcut bucket init, üretim IAM/SSE/replica/backup ve her GET'te yetki denetleyen medya geçidini içermez. Güncel Redis `volatile-lru` güvenlik sayaçlarını da tahliye edebilir; üretim politikası [Redis analizinde](../docs/systems/redis-requirements.md) ayrı tanımlıdır.

## Sıfırlama

```sh
docker compose down -v && rm -rf .local   # tüm veriler, Vault anahtarları dahil silinir
```

## Üretimde farklı olması gerekenler

- Tüm servislerde TLS; Vault için 3+ düğümlü raft ve HSM/KMS ile otomatik unseal. Root token ilk kurulumdan sonra iptal edilir, unseal anahtarları paylaştırılarak saklanır.
- `vault write -f database/rotate-root/okuldayim` ile `vault_admin` parolasını yalnızca Vault bilir hale getirin.
- PostgreSQL için disk şifreleme, şifreli ve 35 gün rotasyonlu yedekler, `pgaudit` ile erişim denetimi.
- Redis için TLS ve ağ izolasyonu; S3 için sunucu tarafı şifreleme ve Türkiye'deki veri merkezi.
- Vault denetim logu merkezi log sistemine aktarılır ve 2 yıl saklanır.
