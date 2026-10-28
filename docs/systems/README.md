# Backend Sistemleri İhtiyaç Analizleri

28 Ekim 2026 · Sürüm 1.1 · Durum: uygulanacak tasarım ve kabul koşulları

Bu dizin [backend analizini](../backend-requirements-analysis.md), [ürün gereksinimlerini](../../school-parent-communication-requirements.md) ve [mevcut yerel altyapıyı](../../infra/README.md) birlikte ele alır. Her sistemin ayrı belgesi vardır; ortak sınırlar ve sistemler arası kabul koşulları burada tanımlanır. Bu belgeler sistemlerin kurulduğuna veya yük testlerinden geçtiğine dair kanıt değildir.

## 1. Sistem envanteri ve mevcut durum

Depoda Rust backend kodu, üretim dağıtımı ve sözleşme testi komutu henüz bulunmuyor. `infra/docker-compose.yml` yalnızca geliştirme ortamını tanımlar. İmaj etiketleri yapılandırma tercihleridir; erişilebilirlik, destek süresi ve güvenlik güncelliği ayrıca doğrulanmalıdır.

| Sistem | Ayrı ihtiyaç analizi | Mevcut durum | Gerekli faz |
| --- | --- | --- | --- |
| PostgreSQL | [postgresql-requirements.md](postgresql-requirements.md) | Compose, rol başlangıç betiği; uygulama şeması yok | 1a |
| Redis | [redis-requirements.md](redis-requirements.md) | Compose, ACL ve kalıcılıksız geliştirme yapılandırması | 1a |
| Vault | [vault-requirements.md](vault-requirements.md) | Compose, Transit/KV/Database/AppRole bootstrap; `kullanici-pii`, `denetim-checkpoint`, `pickup_pepper`, `media_link_key` ve üretim yetki ayrımı eksik | 1a |
| S3 uyumlu nesne depolama | [object-storage-requirements.md](object-storage-requirements.md) | RustFS, bucket başlangıcı ve geçici dosya yaşam döngüsü | 2; destekleyici kurulum 1a |
| Ingress, DNS ve özel ağ | [ingress-network-requirements.md](ingress-network-requirements.md) | Üretim yapılandırması yok | Staging/üretim öncesi |
| API, worker, migration ve dağıtım | [application-runtime-requirements.md](application-runtime-requirements.md) | Tasarım var; Rust workspace ve CI yok | 1a |
| SMS sağlayıcı | [sms-provider-requirements.md](sms-provider-requirements.md) | Sağlayıcı seçilmedi; geliştirmede Mailpit adaptörü planlanıyor | 1a |
| FCM/APNs push | [push-delivery-requirements.md](push-delivery-requirements.md) | Backend ve mobil cihaz kaydı henüz uygulanmadı | 1b |
| E-posta/SMTP ve Mailpit | [email-delivery-requirements.md](email-delivery-requirements.md) | Mailpit var; gerçek e-posta sağlayıcı seçilmedi | 1a (AUTH-01 alternatifi); sağlayıcı kabulü pilot kapısında |
| Medya işleme | [media-processing-requirements.md](media-processing-requirements.md) | Worker/model/codec uygulaması yok | 2 |
| ClamAV zararlı dosya tarama | [malware-scanner-requirements.md](malware-scanner-requirements.md) | Servis yok | 2 |
| OpenTelemetry Collector | [opentelemetry-requirements.md](opentelemetry-requirements.md) | Collector ve iz deposu yok | Staging/üretim öncesi |
| Sentry | [sentry-requirements.md](sentry-requirements.md) | Yurt içi hata izleme kurulumu yok | Staging/üretim öncesi |
| Metrik, log ve alarm sistemi | [monitoring-logging-requirements.md](monitoring-logging-requirements.md) | Merkezi depo ve alarm kanalı yok | Staging/üretim öncesi |
| Yedekleme ve felaket kurtarma | [backup-recovery-requirements.md](backup-recovery-requirements.md) | Yerel volume var; bağımsız yedek, iptal/imha manifest deposu ve kurtarma kanıtı yok | Gerçek veri öncesi |

Worker iş kuyruğu PostgreSQL'dedir. Redis Pub/Sub geçici dağıtım kanalıdır; ayrı Kafka/RabbitMQ servisi gerekmiyor. İptal/imha manifest deposu ayrı bir ürün olmak zorunda değildir: ana DB/S3/Vault'tan ayrı hata ve yetki alanında olması yeterlidir; yedekleme analizinde tanımlanır ve denetim zinciri checkpoint'lerini de tutar. Yeni ürün/sistem ihtiyacı doğarsa bu envanter güncellenir.

## 2. Ortak sözleşmeler

Bu değerler bütün sistem belgelerinin ortak referansıdır. Değişiklikte backend analizi, ilgili sistem analizi ve kabul matrisi birlikte güncellenir.

| Konu | Ortak gereksinim |
| --- | --- |
| Öncelik | Ürün MoSCoW'u kaynaktır. M maddeleri pilot öncesi fazlardadır; C maddeleri (NOT-03 SMS yedeği, PICK-09 "yaklaştı" vb.) pilot kapısını bekletmez ([backend §4.1](../backend-requirements-analysis.md)). |
| Veri konumu | İçerik, veritabanı, nesneler, anahtarlar, yedekler ve gözlemlenebilirlik depoları Türkiye'de. FCM/APNs cihaz belirteçleri ve teknik metaverileri için ayrı aktarım değerlendirmesi gerekir. |
| İletim | Staging/üretimde TLS 1.2+, tercih TLS 1.3; özel ağ ve servis kimlik doğrulaması. TLS'siz bağlantı yalnızca izole yerel geliştirmede. |
| API SLO | Senkron JSON iş isteklerinde sunucu tarafı p95 < 300 ms; medya aktarımı ve uzun işler ayrı ölçülür. Sağlayıcı bekleme süresi senkron API'ye eklenmez. |
| Erişilebilirlik | Europe/Istanbul 07:00–19:00 penceresinde, aylık başarılı iş isteği oranı ≥ %99,9; planlı kesinti de hata bütçesine dahil. Bağımlılıkların ortak etkisi ölçülür. |
| İstek süreleri | Backend toplam süre sınırı 10 sn; ingress JSON upstream sınırı 12 sn; mevcut mobil taşıyıcı 15 sn. Medya ve WebSocket ayrı süre/payload sınırlarına sahip. |
| Canlı olay | Çevrimiçi ve yetkili öğretmen için commit → güncelleme p95 < 5 sn. WebSocket ilk auth ≤ 5 sn, ping 25 sn, ingress boşta bağlantı sınırı ≥ 90 sn. Olay alanları REST DTO adlarıyla aynıdır ve yalnız tür, `eventId`, `version` ve kimlik taşır. Push teslim süresi bu garantiye dahil değil. |
| Yakalama sorgusu | Aktif teslim ekranında en çok 5 sn; diğer açık ekranlarda en çok 60 sn. Pub/Sub kaçırması bağlantı açıkken de olabildiği için sürüm kontrolü ve dönemsel REST uzlaştırması gerekir. |
| Yetki ve rıza | Yeni isteklerde güncel oturum, üyelik, kapsam ve rıza PostgreSQL'den doğrulanır. Redis önbelleği tek başına erişim kararı vermez. Medya GET geçidi aynı kuralı uygular. |
| Kimlik ve teslim süreleri | Erişim JWT 15 dk; yenileme anahtarı 30 gün ve her kullanımda döner; davet 7 gün ve tek kullanımlık; OTP 180 sn ve 5 deneme. Teslim PIN'i 4 hane ve 3 deneme; yalnız talep sahibine ve talep açıkken gösterilir, talep kapanınca veya eğitim günü sonunda silinir. |
| İş teslimi | Transactional outbox, en az bir kez çalıştırma, idempotent handler, sınırlı tekrar, hatalı işler için operatör incelemesi. Harici sağlayıcılarda tam bir kez teslim garantisi yok. Idempotency yalnız commit edilen `2xx` sonucu replay eder. |
| Zamanlanmış işler | `Europe/Istanbul` takvimiyle tek örnekte çalışır (lease satırı veya transaction kilidi; session advisory lock değil). Çalışma kaydı tutulur, kaçırılan çalışma bir kez telafi edilir. Düğüm saatleri NTP ile senkrondur. |
| Kurtarma | Tam sistem RPO ≤ 15 dk, RTO ≤ 4 saat. PG tek düğüm HA: failover ≤ 60 sn, onaylı commit kaybı yok ve tek yazıcı/fencing. Tamamlanmış imha/onaylanmış iptaller genel RPO dışında bağımsız kalıcı manifestle korunur. |
| Saklama | Yedekler 35 gün; operasyon log/izleri en çok 90 gün; iş denetimi ve güvenlik denetim kayıtları (Vault audit, pgaudit dahil) 2 yıl; imha tutanakları en az 3 yıl. Diğer kategoriler ürün belgesi §6.7 ve hukukça kesinleştirilen politikalardan gelir. |
| Geçici veriler | OTP 180 sn; Redis oturum ipucu 30 sn, üyelik ipucu 60 sn; idempotency metaverisi 7 gün. OTP/PIN sırları bu 7 günlük genel sürenin dışında, kendi geçerlilik süresinde imha edilir. |
| Medya bağlantıları | PUT S3 imzası en çok 5 dk. Özel medya ve döküm GET'i, Vault KV `media_link_key` ile imzalı, en çok 5 dk geçerli uygulama bağlantısı ve her istekte güncel yetki kontrolüyle ingress/API geçidinden akar. Ham S3 GET imzası istemciye verilmez. |
| Veri dökümü | Bağlantı ve arşiv hazırlama tamamlandığında başlayan en çok 24 saatlik erişim; worker kesin süreli silme yapar. S3 `Days: 1` kuralı asenkron güvenlik ağıdır. |
| KVKK başvurusu | Operasyon hedefi 15 takvim günü; yasal son tarih 30 takvim günü. Başvuru saatinden hesaplanır; kurum onayı olmadan teknik ekip hukuk politikasını değiştirmez. |
| Yoğunluk | 500 kurum, 50 bin çocuk, 100 bin veli kayıt ölçeği. Çevrimiçi oturum sayısı, ortalama RPS ve dosya boyutları ayrıca ölçülür; kabul yükü ölçülmüş ortalamanın 10 katıdır. |

## 3. Birlikte rafine edilen kararlar

| Çelişki/eksiklik | Birleşik karar | Etkilenen sistemler |
| --- | --- | --- |
| Redis önbelleği anında iptal garantisi vermiyordu | PostgreSQL son erişim kontrolünü yapar; Pub/Sub iptali hızlandırır | API, PostgreSQL, Redis, ingress |
| Olay yalnızca commit sonrası yayınlanıyordu | İş/olay aynı transaction'da kaydedilir; yeniden denenen yayın + REST uzlaştırması | PostgreSQL, worker, Redis, mobil |
| Tuzlu SHA-256 küçük PIN uzayını korumuyordu | Talep kimliğine bağlı HMAC ve Vault pepper; denemeler atomik | API, PostgreSQL, Vault |
| İmzalı S3 GET, rıza geri alındığında hâlâ çalışabiliyordu | Yetki denetleyen uygulama medya geçidi; yeniden üretimden önce eski türevi erişime kapatma | API, ingress, S3, worker |
| S3 günlük lifecycle kesin silme saati gibi anlatılıyordu | Zaman sınırını worker uygular; lifecycle artık dosyalar için yedek tedbir | S3, worker, izleme |
| Append-only kayıtları ortak app rolü silemiyordu | Ayrı ve kurum kapsamlı retention kimliği/yordamı, denetimli son süre silmesi | PostgreSQL, Vault, worker |
| KVKK hedefi ile son tarihi aynı sanılıyordu | 15 günlük hedef, 30 günlük son tarih ve iki ayrı alarm | API, worker, izleme |
| Sır/lease rotasyonu mevcut havuzlarla açıklanmamıştı | Yeni kimlik ve havuz hazırlama, kademeli geçiş, süre dolmadan boşaltma | Vault, API, PostgreSQL |
| Yalnızca PG yedeği yeterli görünüyordu | PG + Vault + S3 + bağımsız imha listesi birlikte kurtarılır | Yedek, Vault, S3, runtime |
| Push metaverisi kişisel veri değil sayılıyordu | Takma kimlikli veri kabulü, asgari içerik ve ayrı aktarım incelemesi | Push, Vault, izleme |
| İmaj/kütüphane sürümleri doğrulanmış gibi yazılmıştı | Lockfile/digest, uyum ve güvenlik doğrulaması uygulama aşamasında kanıtlanır | Runtime ve bütün sağlayıcılar |
| PIN yalnız ilk yanıt/replay'de dönüyordu; mobil veli ekranı PIN'i 10 sn'de bir aktif talep GET'inden okuyor | PIN talep sahibine talep açıkken kurum anahtarıyla şifreli süreli kopyadan döner; diğer herkese `null` | API, PostgreSQL, Vault, mobil |
| Teslim durumlarında `ENGELLENDI` ve açık talep için çıkış yoktu; çocuğun sonraki teslimleri bloke oluyordu | İptal, öğretmen engeli (PICK-07), yalnız yöneticinin gerekçeli çözümü ve gün sonu süre aşımı | API, PostgreSQL, worker, mobil |
| Gecikmeli çevrimdışı öğün kaydı yanlış güne yazılabilir veya reddedilip kaybolabilirdi | `date` esas alınır; zaman tutarsızlığı reddedilmez, işaretlenip incelemeye düşer; çakışmada daha yeni olay zamanı kazanır | API, PostgreSQL, mobil |
| WebSocket olay alanları REST DTO adlarından farklıydı | İngilizce camelCase alanlar, `eventId`/`version`; üyelik değişimi için `uyelik.degisti` | API, Redis, mobil |
| Zamanlanmış işler session advisory lock'a dayanıyordu | Lease satırı veya transaction kilidi, çalışma kaydı ve kaçırılan çalışmanın telafisi | Runtime, PostgreSQL |
| Uygulama medya bağlantısının imza anahtarı tanımsızdı | Vault KV `media_link_key`, sürümlü rotasyon | Vault, API, ingress, S3 |
| Denetim zincirinin sıralaması ve dış anchor'ı belirsizdi | Kurum başına zincir; günlük `denetim-checkpoint` imzası bağımsız depoda | PostgreSQL, Vault, yedek, izleme |
| Sağlık rızası geri alınınca "derhal" silme gece işine kalıyordu | Anında erişim kapatma ve aynı gün imha işi | API, PostgreSQL, worker |
| C önceliği maddeler (NOT-03, PICK-09) pilot zorunlu fazlardaydı | Ürün MoSCoW'u kaynak; pilot sonrası | SMS, push, API |

## 4. Sistemler arası kabul matrisi

| Kimlik | Senaryo ve geçme koşulu | Sorumlu |
| --- | --- | --- |
| C01 | Farklı kurum/sınıf/çocuk kimliğiyle API, worker ve medya GET denemeleri veri döndürmez; havuz tekrar kullanımında kapsam taşınmaz | Backend + güvenlik |
| C02 | Aynı idempotency anahtarıyla eşzamanlı istekler tek iş kaydı üretir; commit sonrası yanıt kaybolunca aynı sonuç güvenle döner. Gecikmeli çevrimdışı öğün kaydı reddedilmeden `date` gününe işaretli yazılır ve daha yeni kaydı ezmez | Backend + mobil |
| C03 | Redis restart/kesinti/anahtar kaybında aktif oturum iptali aşılamaz, OTP kısıtları sıfırlanarak kullanılamaz; teslim REST ile uzlaşır | Backend + altyapı |
| C04 | Worker publish öncesi/sonrası kapanır; olay/iş kaybolmaz, tekrarlar durum değişikliğini çoğaltmaz; canlı hedef normal koşullarda sağlanır. Zamanlanmış iş birden çok worker'da tek çalışır, kapalıyken kaçırılan çalışma bir kez telafi edilir | Backend |
| C05 | Rıza veya velayet erişimi geri alınınca daha önce verilen medya bağlantısıyla yeni GET reddedilir; eski türev yeniden işlenene dek kapalıdır | Backend + medya |
| C06 | Vault ve DB kimliği döner; bağlantılar geçişte çalışır, eski lease kapanır, sırlar loglarda görünmez | Altyapı + backend |
| C07 | Zararlı/bozuk/aşırı büyük medya, tarayıcı kesintisi ve model belirsizliği hiçbir yayın türevi oluşturmaz | Medya + güvenlik |
| C08 | Tam kurtarmada PG/Vault/S3 uyumlu, RPO ≤ 15 dk ve RTO ≤ 4 saat; imha/onaylı iptaller geri açılmaz. BK07 anlık kayıp/eksik manifest testleri ve PG09 primary kaybı/ağ bölünmesi/eski primary dönüşünde tek yazıcı, commit korunması ve ≤ 60 sn HA kanıtı ayrı geçer | Altyapı + KVKK sorumlusu |
| C09 | Test öncesi sabitlenen iş/WS/fan-out/dosya profiliyle en az 30 dk 10× ortalama yük: JSON API p95 < 300 ms, çevrimiçi canlı p95 < 5 sn, onaylı pool/memory/task sınırları aşılmaz. Kararlı son 15 dk'da iş çıkış hızı girişten az olmaz; yük bitince backlog 5 dk içinde başlangıç seviyesine iner | Altyapı + backend |
| C10 | Test telefonu, OTP, PIN, token ve sağlık metni uygulama/ingress/DB/Vault/OTel/Sentry loglarında, WebSocket olaylarında ve push/operasyon alarmında bulunmaz. Onaylı SMS/e-posta kanalında hedef, OTP, tek kullanımlı davet veya üçüncü kişi aydınlatma bağlantısı yalnız ilgili akış için gönderilir; PIN/sağlık aktarılmaz. İzinli takma kimlikler erişim ve saklama kurallarına tabidir | Güvenlik + altyapı |
| C11 | Sağlayıcı 429/5xx/timeout ve yinelenen webhook gönderir; durumlar doğru, harcama/tekrar sınırları çalışır; provider kabulü kullanıcı okuması sayılmaz | Bildirim ekibi |
| C12 | Başvuru hedef/son tarihleri, arşiv 24 saat silmesi, kategori silmesi, sağlık rızası düşünce aynı gün imha ve yedek rotasyonu zaman kontrollü testte doğrulanır | Backend + KVKK sorumlusu |
| C13 | Teslim güvenliği: aynı talebe eşzamanlı PIN denemeleri en çok üç hatada `ENGELLENDI` olur, doğru PIN bir kez tüketilir. `ENGELLENDI` yeni talep açarak atlanamaz ve yalnız kurum yöneticisinin denetimli çözümüyle kapanır. PIN yalnız talep sahibine ve talep açıkken görünür; log, push, WebSocket ve SMS'te bulunmaz. İptal ve gün sonu süre aşımı PIN kopyasını siler. Redis/WS kaybında öğretmen kuyruğu en çok 5 sn'de REST ile uzlaşır | Backend + mobil + güvenlik |

## 5. Teslim sırası ve açık kararlar

1. **1a:** PG/Vault/Redis yerel gereksinim farklarını kapatma, runtime ve migration, SMS/e-posta adaptörleri, kurum kapsamı, kimlik, mevcut mobil işlevler, teslim durum makinesi, güvenilir outbox. Demo/test özelliklerinin üretim derlemesinden çıkarılması.
2. **1b:** WebSocket/push/cihaz yönetimi, iptal/erişim kısıtlaması, KVKK iş akışı, saklama ve imha.
3. **2:** S3 yayın geçidi, medya/ClamAV, iletişim ve yönetim; döküm durumu/indirme. Kurum yönetiminde MFA ve yetki matrisi.
4. **Gerçek veri kapısı:** TLS/ağ izolasyonu, tüm gözlemlenebilirlik ve yedek sistemleri, bağımsız manifest deposu, C01–C13 kanıtları, SMS/e-posta sağlayıcı kabulü, push aktarım kararı, hukuk metinleri ve bağımsız sızma testi. Faz numarası bu kapıyı ertelemez.

| Açık karar | Karar sahibi | Son nokta |
| --- | --- | --- |
| Türkiye lokasyonu, HA hizmetleri ve auto-unseal sağlayıcısının konumu | Altyapı sorumlusu | Staging kurulumu |
| Bağımsız iptal/imha manifest deposunun ürünü, konumu ve erişim modeli | Altyapı + KVKK sorumlusu | Gerçek veri kapısı |
| Ortalama RPS, eşzamanlı WS, en büyük sınıf ve medya yükleme kotaları | Ürün + backend | Yük testi planı |
| SMS/SMTP sözleşmesi, push aktarım mekanizması ve işletim giderleri | Ürün + hukuk | Pilot kapısı |
| Saklama süreleri, sözleşme bitimi ve hukuki muhafaza istisnaları | Kurum + hukuk | Gerçek veri kapısı |
| Tarayıcı/codec/model sürümleri, lisansları ve görüntü işleme kalite eşiği | Medya sorumlusu | Faz 2 kabulü |
| Üretim nöbetçisi, alarm kanalı, yedek maliyeti ve kurtarma tatbikatı sahipleri | Altyapı + ürün | Gerçek veri kapısı |

## 6. Kanıt ve bakım

Her gereksinim için uygulama/config referansı, test sonucu, tarih ve sorumlu kaydedilir. Mevcut durum envanteri, uygulama tamamlandığında ayrı güncellenir. Yeni SDK/sağlayıcı, saklama süresi, endpoint sözleşmesi veya ağ yolu değişirse C01–C13'ten etkilenen kontroller tekrar değerlendirilir. Yalnızca belge bağlantısı ve tutarlılık kontrolü bu çalışma kapsamında kanıttır; çalışma zamanı kabulü sonraki uygulama teslimine aittir.
