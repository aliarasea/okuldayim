# Medya İşleme İhtiyaç Analizi

28 Ekim 2026 · Sürüm 1.1 · Faz 2 · Sorumlu: medya/backend ekibi

Kaynaklar: [ortak sözleşme](README.md), [S3](object-storage-requirements.md), [ClamAV](malware-scanner-requirements.md), [runtime](application-runtime-requirements.md).

## Amaç ve iş hattı

Karantina içeriğini doğrulanmış, EXIF/konumdan arındırılmış, rıza kapsamına uygun türeve dönüştürür. Yüz tespiti yüz tanıma değildir; çocuğu model otomatik teşhis etmez. Çocuk–yüz etiketi öğretmen doğrulaması ve belirsizlikte manuel onay gerektirir. İşleme hattı bugün uygulanmadı.

Sıra: upload intent/HEAD/checksum → format ve kaynak sınırı → tarama → decode/yeniden kodlama → yüz/etiket kontrolü → rıza ve kurum onayı kontrolü → özel türev yazma → DB yayın sürümü → ham nesne silme. Her aşama failure durumunda yayın kapalı kalır.

| Kimlik | Gereksinim | Kabul |
| --- | --- | --- |
| MED01 | Byte/MIME/decode tutarlılığı, maksimum piksel/süre/dosya sınırı | Sahte tür, decompression bomb, bozuk/video dosyası reddi |
| MED02 | EXIF/konum ve diğer metadata temizliği; video container/codec hattı ayrıca doğrulanır | Çıktı metadata taraması; yalnız image crate video kabulü sağlamaz |
| MED03 | Etiketli tüm çocuklar, güncel rıza ve yönetici onayı yayın öncesi kontrol edilir. Çocuğun velisine türev yalnız `FOTO_VELI` etkinse üretilir; diğer velilerin türevinde çocuk yalnız `FOTO_VELI` ve `FOTO_GRUP` birlikte etkinse net görünür | Rıza iş sırasında değişirse türev yayınlanmaz; hiçbir etiketli çocukta `FOTO_VELI` yoksa yükleme reddedilir |
| MED04 | Rızası olmayan yüz belirsizse otomatik paylaşım yok; manuel kalite kabulü | Farklı ışık/örtülü/yandan yüz ve çoklu çocuk test seti |
| MED05 | Grup videosunda gereken rıza yoksa yayın engellenir; başlangıçta video yüz bulanıklaştırma yok | Ret durumunda metadata/thumbnail da erişilemez |
| MED06 | Idempotent sürümlü türev, S3/DB telafi, hash/kaynak envanteri ve kesin ham cleanup | C04/C07/C12; iş tekrarında türev sızıntısı yok |

## Kaynak ve güvenlik sınırları

CPU/bellek/disk/task süre sınırı, ağ kısıtı ve read-only uygulama image'i olan ayrı worker kapasitesi kullanılır. Geçici çalışma alanı kurumlar arası paylaşılmaz ve iş sonunda temizlenir. Shell'e istemci dosya adı/argümanı geçirilmez. Model/codec sürümü, hash'i, lisansı ve test fixture'ları sabitlenir.

Rıza iptalinde eski türev yeni taleplere anında kapanır; yeniden üretim bitene kadar açılmaz. Döndürülmüş private GET bağlantısı da geçitte güncel yetki ister. Daha önce kullanıcıya iletilmiş byte'ların geri alınamayacağı sınırdır.

Ham nesne yayından sonra silindiği için yeniden üretim mevcut türevden ve medya kaydındaki yüz kutularından yapılır. Bu yolla yalnız bulanıklaştırma eklenebilir; sonradan verilen rıza eski bulanık türevleri açmaz, yeni medyaya uygulanır. Yüz kutusu koordinattır, biyometrik şablon değildir ve medyayla birlikte imha edilir. İndirme (MED-06) görüntüleme bağlantısından ayrı bir bağlantıdır; kurum ayarı ve fotoğraftaki bütün etiketli çocukların `FOTO_GRUP` rızası olmadan verilmez.

Kapasite: en büyük dosya × paralel iş × decode bellek çarpanı ve günlük iş/fan-out tahmin edilir. İş latency, queue yaşı, retry/manual-approval/ret sayısı ve cleanup gecikmesi izlenir. MED01–MED06, C05/C07/C09/C10/C12; kota, codec ve kalite eşikleri Faz 2 kabulünden önce kesinleşir.
