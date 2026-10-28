# S3 Uyumlu Nesne Depolama İhtiyaç Analizi

28 Ekim 2026 · Sürüm 1.1 · Destekleyici kurulum 1a, içerik hattı 2 · Sorumlu: altyapı ve medya/backend

Kaynaklar: [ortak sözleşmeler](README.md), [bucket başlangıcı](../../infra/object-storage/init-buckets.sh), [lifecycle](../../infra/object-storage/lifecycle-1-day.json), [Compose RustFS](../../infra/docker-compose.yml).

## Amaç, bucket ve erişim sınırı

Medya ikililerini ve veri dökümünü saklar; nesnenin kurumunu, rızasını, işleme durumunu ve saklama kategorisini PostgreSQL belirler. Prefix tek başına yetkilendirme değildir. RustFS mevcut geliştirme seçimidir; üretim S3 sağlayıcısı SSE, IAM, checksum, Range, lifecycle ve yedek yetenekleriyle doğrulanır.

| Bucket | İçerik | İzin ve ömür |
| --- | --- | --- |
| `okuldayim-media-quarantine` | Henüz taranmamış/işlenmemiş yükleme | İstemci scoped PUT; worker read/delete; son kullanıcı GET yok; worker temizliği, günlük lifecycle yedeği |
| `okuldayim-media` | Onaylı ve rıza kapsamlı türevler/ekler | Worker write, medya geçidi read; kişiye açık bucket yok; kategori bazlı imha |
| `okuldayim-exports` | Hazır JSON/medya arşivi | Döküm worker write/delete, yetkili geçit read; hazır olduktan sonra en çok 24 saat |

## Gereksinimler

| Kimlik | Gereksinim | Kabul kanıtı |
| --- | --- | --- |
| S301 | Public erişim ve list kapalı, rol/bucket/prefix bazlı IAM; API imza yetkisi sınırlı | Başka bucket/kurum anahtarına PUT/GET reddi |
| S302 | Şifreli disk/SSE ve yedek; anahtar ve veri konumu Türkiye | Sağlayıcı konfigürasyonu ve kurtarma testinde şifreleme kanıtı |
| S303 | PUT yalnız sunucunun ürettiği benzersiz object key'e; süre ≤ 5 dk; boyut/tür/checksum upload intent ve HEAD ile doğrulanır | Büyük, eksik, yanlış türde ve tekrar overwrite yüklemeleri yayınlanmaz |
| S304 | İstemciye ham S3 GET imzası verilmez. Vault KV `media_link_key` ile imzalı, ≤ 5 dk geçerli uygulama bağlantısı geçitte her GET/Range'de oturum, kurum, rıza ve yayın sürümüyle yeniden doğrulanır | C05; önceden verilmiş bağlantı iptal sonrası reddedilir |
| S305 | Tamamlanmamış multipart, yetim object, geçersiz türev ve süresi dolan arşiv temizlenir | C12; lifecycle çalışmasa da worker süreyi uygular |
| S306 | Versioning/replica varsa noncurrent sürümler ve delete marker temizliği saklama politikasına bağlıdır | Silinen verinin eski version veya replica üzerinden okunamaması |
| S307 | Object yazma + DB yayın işareti atomik olmadığından telafi/reconciliation işi vardır | Yazma/commit arasında kapanma yayın sızıntısı veya kalıcı yetim üretmez |

## Bağlantı ve boyutlandırma

Worker S3 özel endpoint'ine gider. PUT imzası dış hostname/path ile üretilir; ingress Host/path/query'yi imzayı bozacak şekilde değiştirmez. GET geçidi stream/Range ile belleğe tüm dosyayı yüklemeden içerik verir; bağlantı query'si loglanmaz.

CSV/XLSX içe aktarma ve dayanak belgeleri de checksum/karantina/tarama hattını kullanır; binary içerik JSON API'ye gönderilmez. Upload intent amacı ve kurum/hedef kapsamı sabittir; temiz sonucu farklı amaçla yayın veya sağlık/fotoğraf erişim hakkı vermez. İçe aktarma dosyaları işlem/iptal sonrası, artık dosyalar en geç 24 saat içinde silinir; dayanak belgesi kendi hukuki saklama kategorisinde özel erişimle tutulur.

Ürün tahmini 50 bin çocuk × yılda 1–2 GB = 50–100 TB/yıl medya tabanıdır. Türev sayısı, video, replikasyon, yedek ve geçici alan buna eklenir. İlk gün gerçek kapasite tahsisi bu ölçekle aynı olmak zorunda değildir; büyüme/bütçe/kota alarmı gerekir. Maksimum dosya boyutu, video süresi, multipart eşikleri ve kurum kotaları Faz 2 kabulünden önce netleştirilir.

Checksum semantiği sağlayıcıya göre doğrulanır; multipart/SSE nesnesinde ETag'in MD5 olduğu varsayılmaz. Bucket init tekrar çalışabilir, fakat başarısız lifecycle kurulumunu sadece uyarıyla geçmesi üretim kabulü değildir.

## İşletim ve kabul

İzlenenler: kapasite/kota, PUT/GET hata ve latency, yetim sayısı, lifecycle gecikmesi, worker temizleme yaşı, replikasyon ve restore bütünlüğü. 503/timeout durumunda aynı intent ile güvenli retry uygulanır; yeni object key kontrollü üretilir. Depolama geri gelene kadar iş `published` olmaz.

Backup ayrı hata alanına yapılır; karantinadaki ham dosya kalıcı backup'a taşınmaz. S301–S307, C01/C05/C07/C08/C09/C10/C12 geçmelidir. Mevcut Compose'da üretim IAM/SSE/HA/backup ve GET geçidi uygulanmadığından private bucket kurulmuş olması bu koşulları tamamlamaz.
