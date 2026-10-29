# Mobil–Backend Kullanım Senaryoları

30 Ekim 2026 · Kaynak: [Backend ihtiyaç analizi, §6.2–6.5](backend-requirements-analysis.md) ve mevcut mobil ekran/API sözleşmeleri.

Bu belge mobildeki mevcut kullanıcı akışlarını İngilizce `/v1` REST sözleşmesiyle eşler. İşlev kapsamı mobilde görünse bile ihtiyaç analizindeki faz önceliği değişmez. Mobil istemci `EXPO_PUBLIC_API_MODE=http` ile HTTP taşımacısını kullanabilir; aksi halde sentetik bellek içi mock varsayılandır.

## Kullanım senaryoları

| ID | Aktör / gereksinim | Başarılı akış ve endpointler | Önemli kabul / hata yolu | Faz |
| --- | --- | --- | --- | --- |
| UC-01 Oturum açma ve kapatma | Veli/öğretmen · AUTH-01/02/07 | `POST /v1/auth/otp` → `POST /v1/auth/otp/verify` → `GET /v1/me`; yenileme `POST /v1/auth/refresh`; çıkış `POST /v1/auth/logout` | Kayıtlı olmayan telefon aynı OTP kabul yanıtını alır; OTP süre/deneme sınırı vardır; refresh anahtarı tek kullanımlıdır; replay oturum ailesini iptal eder. Erişim anahtarı bellekte, refresh anahtarı cihazın güvenli deposundadır. | 1a |
| UC-02 Aydınlatma ve rıza | Veli · KVKK-01…04 | `GET /v1/me` → `GET /v1/organizations/{organizationId}/privacy-notice` → `POST …/{version}/acknowledgement` → çocuk başına `GET/PUT /v1/children/{childId}/consents` | Aydınlatma okuması rıza kaydından ayrıdır. Geçerli metin sürümü zorunludur; karar güncellemesi etkin rıza durumunu değiştirir. Çoklu veli çatışması ayrı bir E2E senaryosudur. | 1a |
| UC-03 Günlük görünüm | Veli/öğretmen · ACT | Veli: `GET /v1/children/{childId}/feed?date=YYYY-MM-DD`; öğretmen: `GET /v1/classes/{classId}/today` | Çocuk/sınıf kapsamı doğrulanır. Öğretmen için sağlık rızası etkin değilse çocuk DTO’sunda `allergens: null` olmalı; hassas alan hiç gönderilmemelidir. | 1a |
| UC-04 Öğün kaydı ve çevrimdışı uzlaştırma | Öğretmen · MEAL-01…05 | `GET /v1/classes/{classId}/today` → outbox ile `POST /v1/classes/{classId}/meal-records` | Sınıf/öğün/çocuk kapsamı doğrulanır. Her kuyruk öğesi `Idempotency-Key` ile tek kez uygulanır; yeniden deneme aynı kaydı çoğaltmaz. `recordedAt` ve geç gelen kayıt korunur. | 1a |
| UC-05 Teslim talebi ve PIN ile teslim | Veli + öğretmen · PICK-01…07 | Veli: `GET /v1/children/{childId}/authorized-pickup-people`, `GET /v1/children/{childId}/pickup-requests/active`, `POST /v1/pickup-requests`; öğretmen: `GET /v1/classes/{classId}/pickup-queue`, `POST /v1/pickup-requests/{requestId}/preparing`, `POST /v1/pickup-requests/{requestId}/handover` | Oluşturma `Idempotency-Key` ister; yalnız geçerli/onaylı kişi seçilir; açık talep çakışması `409`; PIN yalnız talep sahibi veliye görünür; öğretmen DTO’sunda PIN `null`; üç yanlış PIN `423 pickup_blocked`; doğru PIN tek kullanılır. | 1a |
| UC-06 KVKK başvurusu ve veri dökümü | Veli · KVKK-05/06 | `GET/POST /v1/privacy/requests`; `POST /v1/privacy/data-exports` | Kurum/çocuk yetkisi gerekir; açıklama ve hata içeriği loglanmaz; döküm isteği `202` alır ve süreli indirme bağlantısı worker tamamlayınca oluşturulur. | Başvuru 1b; döküm 2 |
| UC-07 Hassas ekran denetimi | Oturum açmış kullanıcı · KVKK-15 | Outbox üzerinden `POST /v1/audit/events` | `Idempotency-Key` zorunludur; olay payload’ı OTP, PIN, sağlık metni veya erişim token’ı içermez. | 1a |

## Uç nokta eşleme durumu

Backend ihtiyaç analizindeki yollar İngilizcedir. Mobil `createEndpoints`, mock sunucu ve Rust handler’ları aynı `/v1` yollarını kullanır; JSON alan adları mevcut DTO sözleşmesiyle uyumludur. Teslim talebi ekranı her ayrı gövde için bir UUID idempotency anahtarı üretir ve hata sonrası aynı gövdeyi yeniden gönderirken aynı anahtarı kullanır. HTTP modu `EXPO_PUBLIC_API_MODE=http` ile seçilir.

| Katman | Bugünkü durum | Kanıt / sınır |
| --- | --- | --- |
| Mobil HTTP istemcisi | `fetch`, JSON gövdesi, timeout, auth/refresh ve HTTP/mock API seçimi var | `mobile/src/core/api/httpTransport.ts`, `client.ts`, `tokens.ts`; `endpoints-test.ts` teslim idempotency başlığını doğrular. |
| Mobil akışlar | UI/domain testleri mock backend kullanır; gerçek API E2E’si olarak sayılmaz | `mobile/src/__tests__/app-flow-test.tsx`, `mobile/src/core/api/mock/__tests__/mockServer-test.ts`. |
| Rust API | Faz 1a auth, feed/class, consent, meal, pickup, privacy request ve audit uçları implement edildi; `/openapi.json` bunları listeler | `backend/apps/api/src/auth.rs`, `product.rs`, `http.rs`. |
| Gerçek HTTP E2E | Geçti: sentetik fixture ile gerçek Rust router’ı, PostgreSQL, Redis ve Vault kullanır; SMS gateway çağrılmaz | `backend/apps/api/src/phase1a_e2e.rs`; test yalnız `OKD_RUN_E2E=1` ve `OKD_ENV=test` ile etkinleşir. |

Gerçek E2E; bilinmeyen telefon için genel OTP yanıtı, hatalı OTP denemeleri, refresh replay’de aile iptali, tenant/yetki kontrolleri, sağlık rızası geri çekilince alerjenin gizlenmesi, öğün ve pickup idempotency, PIN sızıntısı/kilidi, KVKK kayıtları, davetli telefon kaydı ve audit replay’ini kapsar. Test OTP kodu yalnız `OKD_ENV=test` yanıtında bulunur; üretim/geliştirme için sabit OTP yoktur. Veri dışa aktarımı yalnız kabul edilir ve kaydedilir; worker ile indirme dosyası üretimi bu E2E’nin ve Faz 1a’nın dışındadır. SMS/SMTP, S3 medya yükleme/indirme ve gerçek cihazda mobil HTTP akışı da burada test edilmez.

## E2E kabul matrisi

1. UC-01: OTP isteği → yanlış kod sınırı → doğru OTP → `/me`; bilinmeyen telefon genel yanıtı alır; refresh replay sonrası oturum ailesi iptal edilir.
2. UC-02: veli aydınlatmayı okur/onaylar; `SAGLIK` kararını reddedince etkin rıza `false` olur.
3. UC-03: etkin sağlık rızası varken öğretmen alerjeni görür; rıza geri çekilince aynı sınıf cevabında `allergens: null` döner; yetkisiz çocuk `403` alır.
4. UC-04: çevrimdışı bir öğün isteği idempotency anahtarıyla gönderilip aynı gövde/anahtarla tekrar edilir; tek kalıcı sonuç vardır.
5. UC-05: veli talep açar → öğretmen kuyruğu ve hazırlanma durumunu görür → doğru PIN ile teslim; ayrıca üç yanlış PIN’de kilitlenme ve PIN alanı sızıntısı kontrol edilir.
6. UC-06/07: başvuru/döküm istekleri doğru yetkiyle kabul edilir; denetim outbox replay’i çoğaltılmaz ve hassas değerler loga düşmez.

E2E sabit UUID’li ve adıyla işaretlenmiş sentetik tenant kullanır; cleanup bu marker doğrulanmadan silme yapmaz ve tenant üyeliğinden dinamik davetli kullanıcıyı da bulur. Telefon numaraları koşu başına sentetik üretilir. Yerel E2E, `infra/.env` içindeki süper kullanıcı parolasını ve host AppRole dosyasını kullanır; gerçek telefon/SMS hesabı veya üretim veritabanı kullanmaz.

Yerel koşu (komut `backend/` dizininden çalıştırılır):

```sh
set -a
. ../infra/.env
set +a
OKD_RUN_E2E=1 OKD_ENV=test \
  OKD_VAULT_APPROLE_FILE=../../../infra/.local/vault/backend-approle.env \
  cargo test -p okuldayim-api --bin okuldayim-api \
  phase1a_use_cases_over_real_http_postgres_redis_and_vault -- --nocapture
```
