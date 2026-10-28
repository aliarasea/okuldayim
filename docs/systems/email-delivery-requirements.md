# E-posta, SMTP ve Mailpit İhtiyaç Analizi

28 Ekim 2026 · Sürüm 1.1 · Geliştirme/e-posta OTP Faz 1a · Sorumlu: backend ve altyapı; sağlayıcı seçimi: ürün/hukuk

Kaynaklar: [ortak sözleşmeler](README.md), [Compose Mailpit](../../infra/docker-compose.yml), [SMS adaptörü](sms-provider-requirements.md).

## Amaç ve mevcut durum

AUTH-01 e-posta OTP alternatifini ve kurum hizmet iletilerini destekler. Mailpit yerel SMTP yakalayıcısıdır; üretim e-posta sağlayıcısı değildir. Mevcut portları 1025/8025, en çok 500 mesaj ve insecure geliştirme auth seçenekleri yalnız sentetik veri içindir. SMS'i e-posta olarak yakalama adaptörü backend'de henüz uygulanmadı.

| Kimlik | Gereksinim | Kabul |
| --- | --- | --- |
| MAIL01 | Doğrulanmış e-posta kimliği/OTP hedefi; Transit `blind-index` ile alan türü ön ekli arama; 180 sn ve aynı atomik deneme/tüketim | Telefon ve e-posta kimliği yalnız ikisi de doğrulanmışsa aynı hesapta birleşir |
| MAIL02 | Üretim SMTP/API TLS ve kimlik doğrulama; sır Vault'ta, hedef sağlayıcı Türkiye'de doğrulanır | Konum/sözleşme ve kimlik rotation testi |
| MAIL03 | Worker queue, timeout/expiry, retry ve bounce/receipt sınıflama | C11; expire OTP gönderilmez, kabul okuma sayılmaz |
| MAIL04 | SPF/DKIM/DMARC, doğrulanmış gönderici ve kota | Staging test adresinde teslim/authentication raporu |
| MAIL05 | OTP/PIN/sağlık loglara veya herkese açık Mailpit UI'a çıkmaz | C10; Mailpit yalnız özel yerel erişim |
| MAIL06 | Yerel e-posta/OTP kayıtlarına süreli cleanup; 500 mesaj sınırı tek başına retention değildir | OTP expire sonrası silme veya izole test fixture temizliği |

## İşletim ve açık seçimler

SMTP accept ile son kullanıcı teslim/okuması farklıdır; güvenilir callback varsa imza/replay kontrolü yapılır. Sınırlı retry sırasında yeni challenge oluşturulmaz. Üretim hizmeti aktif değilse özelliği var saymak yerine API/istemci capability açıkça bildirir.

Seçim konuları: veri ve destek erişiminin konumu, saklama, gönderici domain'i, maliyet/kota, bounce kontrolü ve OTP teslim gecikmesi. Mailpit geliştirme hata ayıklamada sır tutar; test verileri dışarı relay edilmez.

AUTH-01 e-posta alternatifi M önceliğinde olduğundan MAIL01–MAIL06 ve C04/C10/C11 pilot kapısında geçmiş olmalıdır. SMS geliştirme adaptörünün Mailpit'e ulaşması SMS sağlayıcı entegrasyonu kabulü yerine geçmez.
