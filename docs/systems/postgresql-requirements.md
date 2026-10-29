# PostgreSQL İhtiyaç Analizi

28 Ekim 2026 · Sürüm 1.1 · Faz 1a · Sorumlu: backend ve veritabanı/altyapı ekibi

Kaynaklar: [ortak sistem sözleşmesi](README.md), [backend §3.4, §5](../backend-requirements-analysis.md), [rol betiği](../../infra/postgres/initdb/01-roles.sh), [Compose](../../infra/docker-compose.yml).

## Amaç ve sınırlar

Kurum, kullanıcı, rıza, günlük kayıt, teslim, mesaj, denetim, idempotency ve worker/outbox verilerinin kalıcı kaynağıdır. Redis ve bildirim sağlayıcıları bu verinin sahibi değildir. PostgreSQL 18 Compose servisi, rol başlangıcı ve İngilizce adlandırılmış uygulama migration'ı yerel şemayı kurar; üretim HA kurulumu bu kapsamda değildir.

## Fonksiyonel gereksinimler

| Kimlik | Gereksinim | Kabul kanıtı |
| --- | --- | --- |
| PG01 | Kiracı tablolarında `organization_id`, RLS `USING`/`WITH CHECK` ve `FORCE`; seçili kurum `app.organization_id` transaction-local ayarlanır | C01; eksik/boş/yanlış bağlam ve havuz tekrar kullanım testi |
| PG02 | Kurum referansları birleşik anahtar/FK ile korunur; çocuğun başka kurum sınıfına bağlanması engellenir | Çapraz kurum INSERT/UPDATE reddi |
| PG03 | Migration rolü owner'a geçer; runtime tablo sahibi/BYPASSRLS/DDL yetkili değildir | Rol yetki testi ve temiz DB'de migration |
| PG04 | `(organization_id, user_id, http_method, route_template, idempotency_key)` benzersiz; durum değişimi, outbox ve yanıt kaydı atomik | C02; paralel aynı anahtar/commit sonrası timeout |
| PG05 | Teslimde çocuk başına tek açık talep (`YOLDA`/`HAZIRLANIYOR`/`ENGELLENDI`); PIN denemesi ve durum geçişleri satır kilidi/koşullu UPDATE ile atomik; PIN gösterim kopyası son durumla aynı transaction'da silinir | Aynı PIN'e paralel denemeler en fazla üç başarısız girişten sonra engeller; `ENGELLENDI` iken yeni talep reddedilir; C13 |
| PG06 | Outbox claim kısa transaction'da yapılır; harici işlem boyunca DB kilidi tutulmaz; claim süresi ve yeniden alma vardır | C04; worker kapanma ve tekrar testleri |
| PG07 | Rıza/denetim kayıtlarında sıradan UPDATE/DELETE yok; süre sonu silme ayrı kurum kapsamlı retention kimliği/yordamıyla yapılır | App rolü silmeyi yapamaz, retention yalnızca uygun kayıtları siler |
| PG08 | Global kimlik/oturum erişimi kullanıcıya bağlı doğrulama/yordamla sınırlıdır; worker global queue'dan yalnızca kurum/id metaverisi alır | Kullanıcı veya worker kontrol akışı global kişisel veri listeleyemez |
| PG09 | Üretimde tek yazıcı, quorum/fencing ve ayrı hata alanında senkron kalıcı replica; güvenli primary belirlenemeyince yazma kapalı | Primary kaybı, ağ bölünmesi ve eski primary dönüşünde tek yazıcı; onaylanmış commit kaybı yok; tek düğüm failover ≤ 60 sn; eski primary yalnız yeniden eşitlenerek katılır |
| PG10 | Denetim zinciri kurum başına tutulur, zincir başı satır kilidiyle sıralanır. Zamanlanmış iş tekliği lease satırı veya `pg_try_advisory_xact_lock` ile sağlanır; session advisory lock'a dayanılmaz | Eşzamanlı denetim yazımında zincir bozulmaz, kurumlar arası global kilit yoktur; transaction pooling ve havuz rotasyonunda iş tek çalışır |

## Veri ve bağlantı sözleşmesi

Alan şifrelemesi Vault ciphertext'ini sürümüyle saklar; OTP, PIN, refresh sırları açık tutulmaz. İdempotency yanıtında kişisel veri varsa şifrelenir; idempotency kaydına PIN yazılmaz. Teslim PIN'inin talep sahibine gösterim kopyası kurum anahtarıyla şifrelidir ve talep son duruma geçince silinir. Saklama kategorisi, zaman başlangıcı ve varsa hukuki muhafaza ayrı sütunlardır.

`TenantTx` yalnızca sunucunun doğruladığı kurum kimliğini kullanır. RLS uygulama/sınıf/çocuk yetkilendirmesinin ek savunmasıdır; DB istemci girdisinden güvenlik bağlamı üretmez. `user_account`, `session` gibi çok kuruma ortak tablolar için özel politikalar/yordamlar gerekir. Güvenlik tanımlı yordamlar sabit `search_path`, dar EXECUTE izinleri ve açık kapsam kontrolüyle denetlenir; varsayılan PUBLIC EXECUTE kaldırılır.

Vault kısa ömürlü kimliği API/worker havuzlarına verir. Yeni kimlik havuzu hazır olmadan eskisi kapatılmaz. Toplam bağlantı bütçesi API örnekleri + worker + migration + izleme + en az %20 işletim rezervini kapsar; ikinci havuzun rotasyon sırasındaki geçici bağlantıları da sayılır. Transaction pooling seçilirse session varsayımlarına dayanılmaz.

## Kapasite, süreler ve işletim

Kurum/tarih/sınıf/çocuk bazlı indeksler ve cursor sorguları 10× yükte EXPLAIN/latency ile doğrulanır. Audit/outbox büyümesi, autovacuum, bloat ve disk rezervi izlenir; tablo partition kararı ölçümle verilir. `statement_timeout` API'nin 10 sn toplam sınırından kısa, `lock_timeout` daha kısa olacak şekilde tanımlanır; uzun retention/döküm işleri ayrı sınırlarla küçük batch çalışır.

Üretimde disk şifreleme, TLS, özel ağ, WAL arşivi ve PG09 otomatik failover/fencing gerekir. Yeni primary, eski yazıcının güvenli dışlandığı ve onaylı WAL'ın kalıcı olduğu kanıtlanmadan yazma açmaz; yönetilen hizmet de aynı test kanıtını sunar. Tek düğüm HA'da ≤ 60 sn ve onaylı commit kaybı yok hedefi, bölgesel felaketin RPO ≤ 15 dk / RTO ≤ 4 saat hedefinden ayrıdır. HA tek başına yedek değildir. Şifre çözme için uyumlu Vault yedeği zorunludur.

İzlenen değerler: pool doluluğu, bağlantı/lease hataları, yavaş sorgu, lock bekleme, replication/WAL gecikmesi, disk ve queue yaşı. SQL parametreleri, parolalı DDL ve sağlık verisi loglanmaz; pgaudit kuralları da aynı içerik kontrolünden geçer.

## Mevcut farklar ve teslim

Rol betiği geniş varsayılan CRUD/EXECUTE verir; migration her tablonun gerçek yetkisini daraltır. API runtime Vault'tan geçici DB kimliği alır, yeni pool'u doğruladıktan sonra lease'in %70'inde pool'u değiştirir ve tenant transaction'ında `app.user_id`/`app.organization_id` değerlerini transaction-local yazar. Kurum oluşturma henüz runtime'a açılmamıştır; onaylı provisioning akışı/kimliği, ayrı retention rolü, global kimlik erişim kısıtları, üretim replikasyonu ve yedekleme ayrıca tamamlanmalıdır. Lease sona ermesi, eski rol iptali ve açık bağlantı davranışının C06 entegrasyon testi henüz yapılmadı.

PG01–PG10, C01/C02/C04/C06/C08/C09/C10/C12/C13 geçmeden gerçek veri kullanılmaz. Migration geri alma stratejisi: eklemeli şema, eski uygulamayla uyum, kontrollü veri geri doldurma; veri kaybettiren down migration varsayılan rollback değildir.
