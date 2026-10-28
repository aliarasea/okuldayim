# SMS Sağlayıcı İhtiyaç Analizi

28 Ekim 2026 · Sürüm 1.1 · OTP/davet Faz 1a, üçüncü kişi aydınlatma bağlantısı Faz 2, kritik yedek (NOT-03, C) pilot sonrası · Sorumlu: backend, ürün ve hukuk

Kaynaklar: [ortak sözleşmeler](README.md), [backend OTP/bildirim](../backend-requirements-analysis.md), [e-posta geliştirme adaptörü](email-delivery-requirements.md).

## Amaç ve arayüz

Türkiye'de hizmet/veri işleme konumu doğrulanmış sağlayıcıya OTP, davet, yetkili/acil kişiye aydınlatma ve fotoğraf onay bağlantısı (KVKK-07), ihlal bildirimi (ürün §6.8) ve pilot sonrası kritik bildirim yedeği gönderilir. Sağlayıcı seçilmedi; geliştirmenin SMS→Mailpit adaptörü de backend uygulaması olarak henüz yoktur. Telefon + OTP/davet metni kişisel veri/sırdır; push'a veya operasyon loguna kopyalanmaz.

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

Seçim kanıtı: veri konumu/alt işleyen, API kotaları, sender ID, receipt/webhook doğrulaması, maliyet, SLA, saklama ve silme süreçleri. Giriş SMS'e bağlı olduğu için SMS01–SMS06, C03/C04/C10/C11 ve OTP p95 gecikme ölçümü pilot kapısında geçmiş olmalıdır.
