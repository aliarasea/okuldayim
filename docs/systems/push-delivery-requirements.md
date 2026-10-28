# FCM/APNs Push İhtiyaç Analizi

28 Ekim 2026 · Sürüm 1.1 · Faz 1b · Sorumlu: backend/mobil, ürün ve hukuk

Kaynaklar: [ortak sözleşmeler](README.md), [backend §3.10](../backend-requirements-analysis.md), [Vault](vault-requirements.md), [SMS yedeği](sms-provider-requirements.md).

## Amaç ve veri akışı

Arka plandaki uygulamaya genel bildirim gönderir. `getDevicePushTokenAsync` Android'de FCM kayıt token'ı, iOS'ta APNs device token'ı verir. Worker Android'e FCM HTTP v1, iOS'a doğrudan APNs HTTP/2 ile gönderir; native APNs token'ı FCM'ye verilmez. Expo cihaz API'sinden native token alınması Expo Push servisi kullanımını gerektirmez. Mevcut mobilde `expo-notifications` ve gerçek backend cihaz kaydı henüz yoktur.

Token ve opak referans anonim değildir; teknik SDK/ağ metaverisi yurt dışı aktarım değerlendirmesine girer. İçerikte çocuk/veli adı, sağlık, fotoğraf, telefon, OTP/PIN bulunmaz. Genel başlık + tür + opak referans gönderilir; açılışta ayrıntı güncel yetkiyle REST'ten çekilir.

| Kimlik | Gereksinim | Kabul |
| --- | --- | --- |
| PUSH01 | Platform/device/provider/token türü ve ortam kaydı, yenileme, oturumla bağlama ve iptal | Android FCM ve iOS APNs token'ı doğru adaptöre gider; topic/ortam uyuşmazlığı reddedilir; logout hedefi temizler |
| PUSH02 | Token Vault `kullanici-pii` ile şifreli; servis/APNs kimliği Vault'ta | Plain token/crash/log taraması C10 |
| PUSH03 | TTL, retry/backoff, kota, invalid-token temizliği, provider error sınıfları | C11; expire olmuş olay sonradan gönderilmez |
| PUSH04 | Kategori tercihleri ve kritik kategoriler; OS izni kapalı durum açıkça işlenir | Sunucu tercihi OS engelini aşabildiğini varsaymaz |
| PUSH05 | Duplicate bildirimi idempotent açma/okuma; kabul/teslim/okuma ayrı | Sağlayıcı success okundu sayılmaz |
| PUSH06 | Hukuki aktarım mekanizması ve SDK veri akışı doğrulanmadan üretim push açılmaz | Sağlayıcı/SDK envanteri ve onay kaydı |

## Kapasite ve kesinti

Olay başına hedef cihaz sayısı, fan-out, sağlayıcı kota/harcama ve en eski queue işi ölçülür. Worker paralelliği quota'yı aşmaz; provider kabulü timeout'ta belirsiz olabilir. Push best-effort'tur; canlı < 5 sn hedefi WebSocket/REST yolundan ölçülür. Sağlayıcı kesintisi kalıcı teslim kaydını etkilemez. Kritik bildirimin SMS yedeği (NOT-03, C önceliği) pilot sonrasıdır; açıldığında kategori bazlı ve kontrollü çalışır.

Sır rotation, APNs `.p8`/team/key/topic bilgileri, sandbox/production uç noktaları, OAuth yetkileri ve cihaz OS izin senaryoları test edilir. APNs imzalı provider token'ı süre/rotation kurallarına uyar; HTTP/2 bağlantı havuzu bounded'dır. PUSH01–PUSH06 ile C04/C10/C11; push entitlement içeren build ve gerçek cihazda Android/iOS arka plan/ön plan/kapalı izin senaryoları Faz 1b kabulüdür.
