# Yedekleme ve Felaket Kurtarma İhtiyaç Analizi

28 Ekim 2026 · Sürüm 1.1 · Gerçek veri öncesi · Sorumlu: altyapı; imha kontrolü: KVKK sorumlusu

Kaynaklar: [ortak sözleşmeler](README.md), [ürün §5, §6.7](../../school-parent-communication-requirements.md), [PostgreSQL](postgresql-requirements.md), [Vault](vault-requirements.md), [S3](object-storage-requirements.md).

## Amaç ve kurtarma birimi

Tam sistem RPO ≤ 15 dk, RTO ≤ 4 saat ve 35 günlük yedek rotasyonu kaynak ürün hedefleridir. RPO son geri yüklenebilir tutarlı durumun yaşıdır; RTO arızadan yetkili iş akışları kullanılabilir olana dek geçen süredir. Normal request/worker retry'de veri kaybetmeme, felakette sıfır kayıp garantisi anlamına gelmez. Tamamlanmış imha ve başarıyla onaylanmış erişim/rıza/oturum iptalleri bu genel RPO'nun dışında bağımsız kalıcı manifestle korunur; eski yedek bunları geri açamaz.

| Veri | Yedek gereksinimi |
| --- | --- |
| PostgreSQL | Tam yedek + sürekli WAL/PITR; iş/olay/idempotency/rıza/oturum dahil |
| Vault | Raft snapshot ve güvenli seal/auto-unseal kurtarma malzemesi; PG ciphertext/HMAC sürümlerini açabilen anahtarlar |
| Yayın S3 ve hazır dökümler | Kullanılacak backup/replica politikasında silme süreleri korunur; karantina ham içerik kalıcı arşivlenmez |
| İmha/iptal manifesti | Son yedekten sonraki silme, rıza, oturum, üyelik ve kişi yetkisi iptalleri; ayrı hata alanında kalıcı, sıralı ve bütünlüğü doğrulanabilen güncel kayıt |
| Yapılandırma | Dağıtım/IaC, schema sürümü, image digest, gerekli kimlik/sertifika kurtarma prosedürü |
| Denetim kayıtları | Kategori saklama süresine bağlı, erişimi sınırlı ve bütünlük doğrulanabilir kopya; günlük imzalı zincir checkpoint'i bağımsız depoda |

## Gereksinimler

| Kimlik | Gereksinim | Kabul kanıtı |
| --- | --- | --- |
| BK01 | Yedekler Türkiye'de ve ana sistemden ayrı hata/yetki alanında, şifreli, rutin app kimliğine kapalı | Bölge, erişim ve şifreleme envanteri |
| BK02 | Backup zinciri ve WAL arşivleme sürekli izlenir; 15 dk RPO'yu aşma riski oluşmadan alarm | Arşiv kesintisi ve tam restore testi |
| BK03 | Yeni anahtar/rotasyon PG yazılarıyla uyumlu kurtarılabilir duruma getirilir | Son Vault snapshot sonrası yeni ciphertext senaryosu |
| BK04 | İmha edilmiş anahtar, object ve kayıt eski yedekten tekrar hizmete açılmaz | C08/C12; restore sonrası imha manifesti uygulanır |
| BK05 | Retention/immutability kilidi kişisel verinin silme yükümlülüğüyle uyumludur; süre ve hukuki muhafaza kayıtlıdır | 35 gün rotasyon ve kategori silmesi kanıtı |
| BK06 | En az üç ayda bir tam kurtarma ve büyük mimari değişiklikte tekrar tatbikat; sorumlu/kanıt belli | RPO/RTO ölçüm raporu |
| BK07 | Fiziksel imha tamamlanmadan veya iptal API'si başarı döndürmeden önce bağımsız manifest kalıcı yazma onayı verir; en güncel head/checkpoint doğrulanmadan restore hizmete açılmaz | İptal onayından hemen sonra ana alan kaybı; manifest yazarken crash/timeout; eksik/eski manifest ve yinelenen işlem testleri |

Manifest kaydı işlem kimliği, kurum/hedefin asgari kimliği, karar türü, yürürlük zamanı ve artan sürüm taşır; hedef kimlikleri kişisel veri olarak korunur. İptalde PostgreSQL yeni erişimi hemen kapatır, kalıcı manifest onayı alınana kadar işlem beklemede kalır; `200/204` ve idempotency başarı replay'i verilmez. İmha worker'ı manifest onayından önce object/anahtar/kayıt silmez. Manifest kesintisinde iptal güvenli kısıtlı kalır, imha yeniden denenir; başarı varsayılmaz. Bu bağımlılık hassas işlem yolunun süre/503 politikasına ve alarmlarına dahildir.

Manifest, asıl DB/S3/Vault ile aynı kayıp alanındaki ikinci kopya olamaz; ayrı Türkiye konumlu dayanıklı depo seçimi ve doğrulanmış güncel head kurtarma prosedürüne girer. Yeni izin verme de eski iptal kaydını sessizce silemez; yeniden yetkilendirme daha yeni onaylı sürümle kayıt altına alınır. Silme tombstone'ları, etkilenen son backup/replica kopyası sona erene kadar tutulur; imha tutanağının en az 3 yıllık saklaması ayrıdır. Kurtarmada güncel manifest alınamıyor veya bütünlük kanıtlanamıyorsa trafik kapalı tutulur. Aynı bağımsız depo denetim zincirinin günlük imzalı checkpoint'lerini de tutar ([backend §3.13](../backend-requirements-analysis.md)); restore sonrası zincir son checkpoint'e karşı doğrulanır.

## Kurtarma sırası

1. Dış trafik ve worker yazmaları kapatılır; olay zaman çizelgesi ile bağımsız depodaki güncel manifest head/checkpoint'i ve bütünlüğü doğrulanır; eski snapshot manifesti yeterli değildir.
2. İzole özel ağda uyumlu Vault ve seal anahtarları hazırlanır. Kurum anahtarı imha edilmişse geçmiş snapshot'tan kullanılabilir hâle getirilmez; güncel imha manifesti uygulanır.
3. PostgreSQL seçilen PITR noktasına yüklenir; şema/uygulama sürümü ve Vault key version uyumu doğrulanır.
4. Yayın S3 envanteri ile DB object referansları uzlaştırılır; eksik/bozuk türevler erişime kapatılır ve güvenli kaynak varsa yeniden işlenir.
5. Güncel imha/erişim kısıtlama kayıtları uygulanır; oturumlar ve hassas bağlantılar gerekirse yeniden geçersizleştirilir. Redis boş ve kontrollü auth başlangıç moduyla açılır.
6. RLS, rıza, teslim ve decrypt smoke testleri yapılır. Süresi geçmiş OTP/PIN/bildirim işler yeniden gönderilmez; outbox tekrarları idempotent çalışır. Restore noktasında `YOLDA`/`HAZIRLANIYOR` görünen teslim talepleri `SURESI_DOLDU` yapılır ve PIN kopyaları silinir; veliler yeni talep açar.
7. Trafik kademeli açılır, RPO/RTO ve veri farkı raporlanır; kuruma/KVKK sorumlusuna etkisi bildirilir.

## Kapasite ve işletim

Tam + WAL + S3 backup + Vault snapshot + 35 günlük rotasyon maliyeti hesaplanır. Volume/aynı diskte ikinci kopya bağımsız yedek değildir. Şifre çözme anahtarlarına ulaşamayan veya sadece PostgreSQL'i geri yükleyen test başarılı sayılmaz.

İzlenenler: son başarılı backup ve restore zamanı, WAL/Vault snapshot yaşı, backup byte büyümesi, checksum hataları, manifest kalıcı onay süresi/head farkı, bekleyen iptal/imha ve erişim ihlalleri. Backup çıktıları gerçek kişisel veri olarak korunur; geliştirme ortamına aktarılmaz. BK01–BK07 ve C08/C10/C12 gerçek veri kapısının zorunlu koşuludur.
