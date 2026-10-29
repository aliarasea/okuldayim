# Okuldayım — mobil uygulama

React Native 0.88 · Expo SDK 58 · React 19.3 · TypeScript 6 · Expo Router

Veli ve öğretmen rolleri tek uygulamada. Rust backend Faz 1a kimlik doğrulama ve ürün `/v1` uçlarını sunar.
Uygulama varsayılan olarak bellek içi **mock sunucu** ile çalışır; HTTP backend'i `EXPO_PUBLIC_API_MODE=http`
ile seçilir. Mock sentetik verisi yalnız demo ve mobil UI/domain testleri içindir.
Uç nokta eşlemesi ve kabul koşulları [mobil-backend use case belgesindedir](../docs/mobile-backend-use-cases.md).

## Çalıştırma

```sh
npm install
npm run ios        # veya: npm run android
```

| Mock demo numara | Rol | Not |
| --- | --- | --- |
| 555 000 00 01 | Veli (Ada ve Can) | Ada'nın diğer velisi grup fotoğrafına itiraz etmiş durumda |
| 555 000 00 02 | Öğretmen (Kelebekler) | Mert'in velisi sağlık rızası vermediği için alerjisi görünmez |
| 555 000 00 03 | Öğretmen + veli | Profil ekranından rol değiştirilebilir |

Mock doğrulama kodu `123456`'dır. Gerçek backend için iOS Simulator'da:

```sh
EXPO_PUBLIC_API_MODE=http EXPO_PUBLIC_API_URL=http://127.0.0.1:8080 npm run ios
```

Android Emulator/cihazda loopback yönlendirmesi için önce `adb reverse tcp:8080 tcp:8080` çalıştırıp aynı
`EXPO_PUBLIC_API_*` değişkenleriyle `npm run android` başlatın. Gerçek cihazda `EXPO_PUBLIC_API_URL` için
geliştirme makinesinin LAN adresini kullanın. API Compose ile `127.0.0.1:8080` üzerinde başlatılır.

Backend geliştirme profilinde SMS gateway ayarlanmamışsa OTP gönderimi kapalıdır; test OTP'si normal
development/production API yanıtlarında bulunmaz. Gerçek mobil giriş için davetli/kayıtlı kullanıcı ve
Jasmin bağlantısı gerekir. Test kodu yalnızca yalıtılmış `OKD_ENV=test` E2E çalışmasında döndürülür.
Üretim derlemesinde `https://` dışındaki API adresleri reddedilir. Backend E2E'si gerçek DB/Redis/Vault
üzerinde API HTTP router'ını test eder; gerçek iOS/Android cihaz akışı değildir.

## Komutlar

```sh
npm test            # Jest (jest-expo, Testing Library)
npm run typecheck   # tsc --noEmit
npm run lint        # expo lint
npx expo-doctor     # bağımlılık ve yapılandırma kontrolü
```

## Yapı

```
src/
  app/        Expo Router ekranları. (auth), (onboarding), (app) grupları Stack.Protected ile korunur
  core/       api (istemci, mock sunucu), auth (token), session, offline (şifreli kuyruk),
              consent, rbac, privacy, i18n
  features/   auth, consent, feed, meals, pickup, privacy
  ui/         tema ve ortak bileşenler
```

## Uygulanan gereksinimler

- **Kimlik (AUTH-01/02/03/07):** SMS OTP; erişim anahtarı yalnızca bellekte, yenileme anahtarı Keychain/Keystore'da
  (`WHEN_UNLOCKED_THIS_DEVICE_ONLY`). Yenileme tek seferde yapılır. Sunucu oturumu iptal ederse cihazdaki
  tüm veriler silinir. Çoklu rol ve çocuk arasında geçiş yapılabilir.
- **Aydınlatma ve rıza (KVKK-01…04):** Aydınlatma, rızadan ayrı ve önce gösterilir. Rızalar çocuk bazında
  altı türdür ve hiçbiri önceden seçili gelmez; vermemek uygulamanın geri kalanını kilitlemez. İki velide
  en kısıtlayıcı rıza uygulanır ve bu durum ekranda açıkça gösterilir.
- **Öğün (MEAL-02…05):** "Herkes tamamını yedi" ile toplu giriş ve tek dokunuşla istisna düzeltme.
  Menü alerjen uyarısı yalnızca sağlık rızası olan çocuklarda çalışır.
- **Teslim (PICK-01…07):** Veli "yola çıktım" bildirimi gönderir ve yetkili kişiyi seçer; süresi dolmuş ya
  da onaylanmamış kişi seçilemez. Öğretmen kuyruğu görür ve tek kullanımlık kodla teslim eder. Üç hatalı
  denemede teslim durur.
- **Çevrimdışı:** Öğretmen girişleri önce SQLCipher ile şifreli yerel kuyruğa yazılır. Gönderim sırası
  korunur, `Idempotency-Key` kullanılır ve hata durumunda üstel bekleme uygulanır. Oturum bilgisi
  şifreli önbellekten açılabilir.
- **KVKK başvurusu ve gizlilik merkezi (KVKK-05/06/10, AUTH-04):** Uygulama içinden m.11 başvurusu,
  hesap silme talebi ve veri dökümü isteği yapılabilir.
- **Cihaz güvenliği (KVKK-13/14/15):** Hassas ekranlarda ekran görüntüsü engellenir (Android); iOS'ta
  alındığında denetim kaydı oluşur. iOS görev değiştiricide içerik bulanıklaşır. Android yedeklemesi
  kapalıdır. Loglarda kişisel veri maskelenir ve üretimde konsola yazılmaz. Üçüncü taraf analitik SDK'sı yoktur.

## Bilinen sınırlar

- **SQLCipher:** Expo Go'da çalışmaz. Geliştirmede uyarı verilir ve veritabanı şifresiz açılır; üretim
  derlemesi SQLCipher olmadan açılmayı reddeder. Şifrelemeyi denemek için development build gerekir:
  `npx expo run:ios`.
- **Canlı bildirim:** Teslim durumu şimdilik 10 saniyede bir sorgulanıyor. WebSocket ve push bildirimleri
  Rust backend ile birlikte eklenecek; push içeriğinde kişisel veri olmayacak. Bağlantı ayrıntıları:
  [`docs/backend-requirements-analysis.md`](../docs/backend-requirements-analysis.md).
- **Kapsam dışı ekranlar:** Medya, duyuru, mesajlaşma, yoklama ve takvim henüz yok. Yetkili teslim alan ve
  kurum yöneticisi rollerinin mobil ekranları da sonraki adımlarda yapılacak.
