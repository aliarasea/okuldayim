# SMS Gateway İhtiyaç Analizi

29 Ekim 2026 · Sürüm 1.2 · Jasmin HTTP adapter seçildi; OTP/davet Faz 1a, üçüncü kişi aydınlatma bağlantısı Faz 2, kritik yedek (NOT-03, C) pilot sonrası · Sorumlu: backend, ürün ve hukuk

Kaynaklar: [ortak sözleşmeler](README.md), [backend OTP/bildirim](../backend-requirements-analysis.md), [e-posta geliştirme adaptörü](email-delivery-requirements.md).

## Amaç ve arayüz

Seçilen açık kaynak SMS gateway'i [Jasmin SMS Gateway](https://github.com/jookies/jasmin)'dir. Backend'de HTTP gönderim adaptörü uygulanmıştır. Jasmin bir gateway yazılımıdır, telekom operatörü/SMSC veya SMS teslim hizmeti değildir: gerçek teslim için operatör bağlantısı, yetkili sender ID, Jasmin kullanıcı hesabı ve kredi gerekir. Depoda bu harici hesap/connector bilgileri yoktur; bu nedenle yerel kurulum kendiliğinden gerçek SMS göndermez.

Hedef akış OTP, davet, yetkili/acil kişiye aydınlatma ve fotoğraf onay bağlantısı (KVKK-07), ihlal bildirimi (ürün §6.8) ve pilot sonrası kritik bildirim yedeğidir. Telefon + OTP/davet metni kişisel veri/sırdır; push'a veya operasyon loguna kopyalanmaz. SMS iş kuyruğu, OTP/auth uçları, otomatik yeniden deneme, delivery receipt callback'i ve e-posta gönderimi henüz uygulanmamıştır.

## Jasmin HTTP sözleşmesi ve yapılandırma

Adaptör, [Jasmin HTTP API `/send` sözleşmesini](https://docs.jasminsms.com/en/latest/apis/http/index.html) izler: form-urlencoded POST ile tek E.164 hedefi (`to`), metin (`content`), `username`, `password` ve Unicode için `coding=8` gönderir. Kimlik bilgileri URL/query string'e konmaz. İstek 5 saniye timeout'a ve redirect kapatmaya sahiptir. Üretimde gateway URL'si HTTPS olmalıdır.

Jasmin `200` + `Success "<message-id>"` yanıtı mesajın gateway kuyruğuna alındığını bildirir; telefona teslim edildiğini değil. 4xx yanıtı reddedilmiş istek sayılır. Ağ hatası, 5xx veya bozuk `200` cevabı sonucu belirsizdir; `/send` için belgelenmiş idempotency anahtarı olmadığından otomatik tekrar yapılmamalıdır. Uygulama gateway cevabının gövdesini loglamaz; yalnızca kimliksiz/sanitized hata türü döndürür.

API'ye `OKD_SMS_GATEWAY_URL` ile `.../send` adresi verilir. Jasmin `username` ve `password` Vault KV v2 `secret/okuldayim/backend` içine `jasmin_username` / `jasmin_password` alanlarıyla konur; yerel kullanımda `infra/.env` değerlerini `vault-bootstrap` aktarır. API parolayı environment variable'dan almaz. URL boşsa SMS adaptörü devre dışıdır; URL verilip Vault kimlik bilgileri eksikse API güvenli şekilde başlamaz. `/readyz` yalnızca `sms_provider: configured|disabled` bilgisini gösterir, gateway erişilebilirliğini veya teslimatı test etmez.

Gateway tarafında bir Jasmin HTTP API hesabı, `DefaultRoute` ve SMSC'ye bağlı `SmppClientConnector` ayrıca provision edilmelidir. Gerçek operatör bağlantısı ve test numarası sağlanana kadar yalnızca mock HTTP testleri yapılabilir. Sender ID, Türkiye'deki veri işleme/alt işleyen konumu, operatör sözleşmesi, kota, maliyet, teslimat raporu ve saklama koşulları pilot öncesi ayrıca onaylanmalıdır.

| Kimlik | Gereksinim | Kabul |
| --- | --- | --- |
| SMS01 | E.164 normalizasyonu, kayıtlı/davetli hedef ve sabit dış yanıt | Kayıt varlığı sızdıran yanıt farkı yok |
| SMS02 | OTP gönderimi 180 sn içinde, bağlantı içeren ileti bağlantının geçerliliği içinde; geciken iş expire olur | Süresi dolmuş kod veya bağlantı gönderilmez; queue yaşı alarmı |
| SMS03 | HTTPS, Vault sağlayıcı sırrı, timeout ve sağlayıcı kimlikli receipt | Sır rotation ve 429/5xx/timeout testi |
| SMS04 | Sağlayıcı idempotency/dedup destekliyorsa kullanılır; timeout sonrası belirsiz kabul ayrı tutulur | Mükerrer gönderim/hacim sınırı ve operatör incelemesi |
| SMS05 | Webhook imza/kimlik doğrulaması, timestamp/replay koruması, tekrar idempotency | Sahte/tekrarlı callback durumu bozamaz |
| SMS06 | Gönderim kabulü/teslim/okuma ayrımı; maliyet, kullanıcı/IP/kurum kotası | Sağlayıcı kabulü kullanıcı onayı sayılmaz; harcama alarmı |

## İşletim, gizlilik ve seçim

Rate-limit kaynağı Redis + kontrollü yedek politikasıdır; restart sonrası limitsiz OTP gönderilmez. OTP aynı challenge içinde tek kullanımlıktır. Queue'daki gönderilecek kod kısa süreli şifreli tutulur ve expire sonunda silinir. Kullanıcı varlığını eşit dış yanıtla gizlemek sağlayıcı masrafını anonim numaralara açmak anlamına gelmez.

Kritik SMS yedeği (NOT-03) açıldığında kullanıcı okuma kaydına göre tetiklenir; bekleme ve tekrar sayısı kategori bazında tanımlanır. Pazarlama ile hizmet bildirimi ayrımı hukuk/sözleşme değerlendirmesidir; yeni pazarlama akışı bu ihtiyaç kapsamına dahil değildir.

Seçim kararı Jasmin gateway yazılımı düzeyindedir; SMS taşıyıcısı/operatör seçimi açık kalır. Veri konumu/alt işleyen, API kotaları, sender ID, receipt/webhook doğrulaması, maliyet, SLA, saklama ve silme süreçleri operatör seçimiyle tamamlanmalıdır. Giriş SMS'e bağlı olduğu için SMS01–SMS06, C03/C04/C10/C11 ve OTP p95 gecikme ölçümü pilot kapısında geçmiş olmalıdır.
