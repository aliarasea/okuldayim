# Ingress, DNS ve Özel Ağ İhtiyaç Analizi

28 Ekim 2026 · Sürüm 1.1 · Staging/üretim öncesi · Sorumlu: altyapı

Kaynaklar: [ortak süre/veri sözleşmeleri](README.md), [backend topolojisi](../backend-requirements-analysis.md), [S3](object-storage-requirements.md).

## Amaç ve trafik sözleşmesi

Mobil/web trafiğinin TLS sonlandırma, yönlendirme, kimlikli medya erişimi ve ağ sınırlarını sağlar. Mevcut Compose host portları 127.0.0.1'e açıktır; üretim ingress/DNS/sertifika kurulumu yoktur.

| Yol | Hedef ve kural |
| --- | --- |
| `api.<alan>/v1/*` | API; JSON gövde en çok 1 MB, upstream süre sınırı 12 sn |
| `api.<alan>/v1/ws` | API WebSocket upgrade; auth 5 sn, ping 25 sn, idle sınırı en az 90 sn |
| `medya.<alan>` PUT | Scoped S3 imzası; Host/path/query korunur; dosya boyut/süre kotası JSON sınırından ayrıdır |
| `medya.<alan>` GET/Range | API medya geçidi; `media_link_key` imzalı ≤ 5 dk uygulama bağlantısı, her istekte oturum/kapsam/rıza kontrolü ve stream |
| `panel.<alan>` | Kurum web paneli statik içeriği (Faz 2); CSP ve HSTS. API'ye credential'lı CORS yalnız bu kaynaktan, çerez `HttpOnly; Secure; SameSite=Strict` |
| Yönetim/health/metrics | Özel ağ/VPN veya ayrı yetkilendirme; altyapı console'ları internete açılmaz |

## Gereksinimler

| Kimlik | Gereksinim | Kabul |
| --- | --- | --- |
| NET01 | TLS 1.2+, tercih 1.3; sertifika otomatik yenileme, HSTS ve DNS sahipliği | Staging sertifika/renewal testi; süresi dolmadan alarm |
| NET02 | PostgreSQL/Redis/Vault/S3 yönetimi özel ağ; servisler arası en az erişim ve doğrulanmış TLS | Yetkisiz subnet/kimlik bağlantısı reddi |
| NET03 | Proxy IP/Forwarded bilgisi yalnız güvenilen hop'tan; istemci spoof'u rate-limit'i aşamaz | Sahte `X-Forwarded-For` testi |
| NET04 | İmzalı PUT canonical Host/path/query bozulmaz; GET doğrudan S3'e bypass edilemez | C01/C05; imza ve eski link iptal testleri |
| NET05 | CORS yalnız web panel kaynağı; credentials/CSRF politikası eşleşir | Başka Origin'den credential'lı işlem reddi |
| NET06 | HA/fencing ve rolling drain; uzun WS/stream kontrollü kapanır | Bir ingress/API örneği kaybında yeniden bağlantı |

## Kapasite, işletim ve açık kararlar

RPS, eşzamanlı WS ve medya byte/s ayrı ölçülür; stream tüm dosyayı RAM'e almaz. WS buffer ve bağlantı/user/kurum kotaları tanımlanır. S3 imza/uygulama bağlantısı içeren URL query'leri ve Authorization/cookie loglanmaz; request-id doğrulanır.

Metrikler: TLS/DNS hata, HTTP p95/5xx/429, active WS, reset/drain, medya throughput, upstream timeout ve sertifika kalan ömrü. Bölge/ingress ürünü/kota seçimleri kapasite planıyla kesinleşir. Alarm ve yönetim erişimi Türkiye konumu kuralına tabidir.

NET01–NET06, C01/C05/C09/C10 gerçek veri kapısında geçmelidir. Genel CDN/private medya cache'i başlangıç gereksinimi değildir; eklenecekse rıza iptali ve yurt içi veri akışı tekrar değerlendirilir.
