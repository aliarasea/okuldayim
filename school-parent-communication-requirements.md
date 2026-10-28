# Okul–Veli İletişim Uygulaması: Gereksinim Analizi

Oct 3, 2026 · @Ali Aras

> **Sürüm 1.1 · 27 Ekim 2026 — KVKK revizyonu.** Belge 6698 sayılı KVKK ve ikincil mevzuata (7499 sayılı Kanunla 2024'te yapılan değişiklikler dahil) göre gözden geçirildi. Yeni gereksinimler **KVKK-xx** koduyla, değişen maddeler **[KVKK]** etiketiyle işaretlendi; tüm değişikliklerin listesi [Ek A](#ek-a-kvkk-revizyon-özeti)'da. Orijinal metin: `docs/archive/requirements-analysis-v1.0-original.md`. Bu belge hukuki görüş değildir; yayın öncesi KVKK uzmanı ve hukuk danışmanı onayı gerekir.

> **Güncel sürüm 1.3 · 28 Ekim 2026 — birleşik teknik rafinasyon.** Sürüm 1.2'de backend ve bağımlı sistem analizleriyle ortak erişim, medya, saklama, push ve kurtarma kuralları hizalandı. 1.3'te yetkili kişi verisinin saklanması, erişim kısıtlaması kapsamı, başvuru tarihleri ve pilot kapısının teknik kabulü backend 1.2 ile eşitlendi. Değişiklikler [Ek B](#ek-b-birleşik-teknik-rafinasyon)'de. Uygulama ve üretim kabulü henüz tamamlanmış değildir.

## 1. Giriş ve kapsam

Uygulama; kreş, anaokulu ve ilkokulda öğretmen–veli iletişimini tek bir güvenli mobil kanalda toplar: günlük aktivite ve öğün takibi, duyurular, fotoğraf paylaşımı, teslim alma bildirimi ve mesajlaşma.

**Problem.** Bugün bu iletişim WhatsApp grupları, kâğıt ileti defteri, telefon ve okul kapısındaki sözlü bilgilendirmeyle dağınık yürüyor. Veli çocuğunun gününü eksik öğreniyor, öğretmen aynı bilgiyi defalarca veriyor. Çocuk fotoğrafları kontrolsüz gruplarda dolaşıyor; bu ciddi bir KVKK riski.

**Vizyon.** Öğretmen için 1–2 dokunuşla kayıt; veli için çocuğunun gününü canlı izlediği bir akış; kurum için denetlenebilir, kurumsal bir iletişim kanalı.

**Hedef kurumlar ve yaş grupları.** Modüllerin önemi yaşa göre değişir; bu yüzden her modül kurum ve sınıf bazında açılıp kapatılabilir olmalı.

| Kurum tipi | Yaş | Öne çıkan modüller |
| --- | --- | --- |
| Kreş | 0–3 | Öğün, biberon, uyku, bez/tuvalet, ilaç, ihtiyaç listesi (bez, mama) |
| Anaokulu | 3–6 | Aktivite, öğün, fotoğraf, teslim alma, duyuru |
| İlkokul | 6–10 | Duyuru, yoklama/devamsızlık, ödev notu, mesajlaşma, etkinlik takvimi |

**Kapsam (ilk sürüm).**

| Dahil | Kapsam dışı (sonraki fazlar) |
| --- | --- |
| iOS ve Android uygulaması (veli + öğretmen rolleri tek uygulamada) | Aidat ve ödeme tahsilatı |
| Kurum yönetimi için web paneli | Okul servisi canlı konum takibi |
| Günlük akış: aktivite, öğün, uyku, tuvalet, sağlık notu | e-Okul / MEB entegrasyonu, not ve karne |
| Duyuru, fotoğraf/video, mesajlaşma | Canlı kamera yayını |
| Teslim alma bildirimi ve yetkili kişi yönetimi | Muhasebe, personel bordrosu |
| Yoklama, etkinlik takvimi, push bildirim | Yapay zekâ ile fotoğrafta çocuk tanıma (biyometrik veri; bilinçli olarak kapsam dışı) |
| **[KVKK]** Aydınlatma ve rıza yönetimi, ilgili kişi başvuruları, saklama ve imha motoru | Pazarlama amaçlı ileti (6563 sayılı Kanun ve İYS kapsamı) |

Brightwheel, Lillio (eski HiMama) ve Famly bu alanın yurtdışındaki referans ürünleri; özellik setini kıyaslamak için incelenebilir.

## 2. Paydaşlar ve kullanıcı rolleri

Sistemde altı rol var; veri erişimi her zaman kurum → sınıf → çocuk hiyerarşisiyle sınırlanır. Bir kullanıcı birden fazla rol taşıyabilir (ör. hem öğretmen hem başka sınıfta velisi olan personel) ve birden fazla kuruma bağlı olabilir (kardeşler farklı okullarda).

| Rol | Kim | Temel ihtiyaç | Kanal |
| --- | --- | --- | --- |
| Kurum yöneticisi | Müdür, müdür yardımcısı, idari personel | Sınıf/öğretmen/çocuk kaydı, duyuru, onay ve raporlar | Web panel + mobil |
| Öğretmen | Sınıf öğretmeni, yardımcı öğretmen, branş öğretmeni | Hızlı toplu kayıt, fotoğraf, mesaj, teslim bildirimi alma | Mobil (çoğunlukla tablet) |
| Veli | Anne, baba, yasal vasi | Çocuğun gününü izleme, mesaj, teslim alma bildirimi | Mobil |
| Yetkili teslim alan | Büyükanne/büyükbaba, bakıcı, aile dostu | Sadece teslim alma; isteğe bağlı salt-okunur akış | Mobil veya uygulamasız (QR/PIN) |
| **[KVKK]** Kurum KVKK irtibat kişisi | Kurum yöneticisi veya atanmış personel | İlgili kişi başvurularını 30 gün içinde yanıtlama, rıza ve imha raporları, ihlal sürecinde Kurul ile iletişim | Web panel |
| Platform yöneticisi | Ürün ekibi | Kurum açma, abonelik, destek, denetim. **[KVKK]** Kurum verisine varsayılan erişimi yoktur (KVKK-11) | Web admin |

**Yetki matrisi** (✓ tam yetki, S = sadece kendi sınıfı/çocuğu, O = okuma, — yetki yok):

| İşlem | Kurum yön. | Öğretmen | Veli | Yetkili teslim alan |
| --- | --- | --- | --- | --- |
| Çocuk/sınıf kaydı oluşturma | ✓ | — | — | — |
| Günlük aktivite ve öğün girme | ✓ | S | — | — |
| Günlük akışı görme | ✓ | S | S | O (veli izin verirse; sağlık bilgisi ve fotoğraf hariç) |
| Fotoğraf/video yükleme | ✓ | S | — | — |
| Kurum geneli duyuru | ✓ | — | — | — |
| Sınıf duyurusu | ✓ | S | — | — |
| Mesajlaşma | ✓ | S | S | — |
| Teslim alma bildirimi gönderme | — | — | S | S |
| Teslimi onaylama | ✓ | S | — | — |
| Yetkili kişi ekleme/silme | ✓ (onay) | — | S | — |
| Sağlık bilgisi (alerji, ilaç) düzenleme | ✓ | O¹ | S | — |
| Raporlar ve dışa aktarım | ✓ | S | Kendi çocuğu | — |
| **[KVKK]** Açık rıza verme / geri alma | Görür | — | S | — |
| **[KVKK]** Veli erişimini kısıtlama (velayet kararı) | ✓ | — | — | — |
| **[KVKK]** Denetim kaydını görme | ✓ | — | Kendi çocuğuna erişim özeti | — |
| **[KVKK]** KVKK başvurusu | Yanıtlar | Oluşturur (kendi verisi) | Oluşturur | Oluşturur (kendi verisi) |

¹ **[KVKK]** Öğretmen sağlık bilgisinden yalnızca kendi sınıfındaki çocukların alerji, diyet kısıtı ve ilaç talimatını görür; kronik durum ayrıntısı yalnızca acil durum ekranında gösterilir. Her görüntüleme denetim kaydına düşer.

İki velili ailelerde, mahkeme kararıyla velayeti kısıtlanan ebeveynin erişimi kurum yöneticisi tarafından kapatılabilmeli veya sınırlandırılabilmeli; bu, gerçek hayatta sık karşılaşılan bir gereksinim. **[KVKK]** Kısıtlamanın dayanağı (karar tarihi, sayısı, belge) kayıt altına alınır ve işlem denetim kaydına düşer. Bir velinin telefon ve adres bilgileri ile eklediği yetkili kişiler diğer veliye hiçbir koşulda gösterilmez; koruma kararı bulunan ailelerde bu kritiktir (KVKK-09).

## 3. Fonksiyonel gereksinimler

Gereksinimler 13 modüle ayrıldı ve MoSCoW ile önceliklendirildi: **M** = olmazsa olmaz (MVP), **S** = olmalı, **C** = olabilir. MVP'nin kalbi günlük akış, öğün, duyuru, fotoğraf, teslim alma ve mesajlaşma. **[KVKK]** 13. modül (3.13) KVKK revizyonuyla eklendi; M öncelikli KVKK maddeleri tamamlanmadan pilotta gerçek çocuk verisi işlenmez.

### 3.1 Kimlik ve hesap (AUTH)

| Kod | Gereksinim | Öncelik |
| --- | --- | --- |
| AUTH-01 | Telefon numarası + SMS OTP ile giriş; e-posta alternatif | M |
| AUTH-02 | Davet tabanlı kayıt: veliyi kurum davet eder (SMS/bağlantı); dışarıdan serbest kayıt yok. Davet bağlantısı tek kullanımlık ve süreli (ör. 7 gün) | M |
| AUTH-03 | Bir veli birden çok çocuğa ve kuruma bağlanabilir; çocuklar arası hızlı geçiş | M |
| AUTH-04 | Hesap silme uygulama içinden yapılabilir (App Store / Google Play zorunluluğu). **[KVKK]** Silmeden önce hangi verilerin silineceği, hangilerinin kurumun yasal saklama yükümlülüğü nedeniyle süre sonuna kadar kısıtlı erişimle tutulacağı (ilaç, kaza, teslim kayıtları) açıkça gösterilir; hesap verileri en geç 30 gün içinde silinir | M |
| AUTH-05 | Face ID / parmak izi ile hızlı giriş. **[KVKK]** Biyometrik veri cihazdan çıkmaz; uygulama yalnızca işletim sisteminin doğrulama sonucunu kullanır | S |
| AUTH-06 | Sınıfta ortak tablet modu: öğretmenler PIN ile hızlı kullanıcı değiştirir. **[KVKK]** Hareketsizlikte otomatik kilit (ör. 2 dk); ortak tablette sağlık verisi yerel önbelleğe yazılmaz | S |
| AUTH-07 | **[KVKK]** Cihaz listesi ve uzaktan oturum kapatma; oturumu kapatılan veya yetkisi alınan cihazda yerel veriler (önbellek, çevrimdışı kuyruk, medya) silinir | M |

### 3.2 Kurum, sınıf ve çocuk yönetimi (ORG)

| Kod | Gereksinim | Öncelik |
| --- | --- | --- |
| ORG-01 | Kurum profili; zincir kurumlar için şube yapısı | M |
| ORG-02 | Sınıf oluşturma, öğretmen ve çocuk atama, dönem sonunda toplu sınıf geçişi | M |
| ORG-03 | Çocuk profili: ad, doğum tarihi, fotoğraf, alerjiler, kronik durum, düzenli ilaç, diyet kısıtları, acil durum kişileri. **[KVKK]** Sağlık alanları ancak veli sağlık rızası verdikten sonra doldurulabilir (KVKK-02). Diyet dini inancı açığa çıkarabileceği için (ör. "helal") özel nitelikli veri sayılır; dini etiket yerine içerik bazlı kısıt ("domuz eti yok", "et yok") tutulur ve sağlık verisiyle aynı korumaya tabidir. TC kimlik numarası toplanmaz | M |
| ORG-04 | Fotoğraf/medya paylaşım izni çocuk bazında tutulur (veli onayı). **[KVKK]** Tek izin yerine kademeli rıza: yalnızca kendi velisi / grup fotoğrafında diğer veliler / kurum tanıtım materyali (KVKK-02) | M |
| ORG-05 | Excel/CSV ile toplu çocuk ve veli içe aktarma. **[KVKK]** Yüklenen dosya işlendikten sonra sunucudan silinir; sağlık alanları içe aktarılmaz, veli rızasıyla uygulamada girilir | S |
| ORG-06 | Eğitim yılı yönetimi ve geçmiş yılların arşivlenmesi. **[KVKK]** Arşiv, saklama sürelerine (6.7) tabidir; süresi dolan arşiv imha edilir | S |

### 3.3 Günlük akış ve aktiviteler (ACT)

| Kod | Gereksinim | Öncelik |
| --- | --- | --- |
| ACT-01 | Aktivite tipleri (oyun, sanat, müzik, açık hava, eğitsel vb.); kurum kendi tiplerini tanımlar | M |
| ACT-02 | Toplu giriş: bir aktivite tek seferde seçili çocuklara veya tüm sınıfa işlenir | M |
| ACT-03 | Çocuğa özel not ve fotoğraf ekleme | M |
| ACT-04 | Gün sonu otomatik günlük rapor ve veliye push bildirimi | M |
| ACT-05 | Yayın modu kurum ayarıdır: anlık veya gün sonu toplu | S |
| ACT-06 | Ruh hali kaydı (mutlu, yorgun, huzursuz…) | S |
| ACT-07 | Gelişim gözlemleri (dil, motor beceri, sosyal) ve dönemsel gelişim raporu. **[KVKK]** Değerlendirme niteliğinde veri: yalnızca ilgili öğretmen, yönetici ve veli görür | C |

### 3.4 Öğün takibi (MEAL)

| Kod | Gereksinim | Öncelik |
| --- | --- | --- |
| MEAL-01 | Haftalık/aylık yemek menüsünü kurum yayınlar; veli uygulamada görür | M |
| MEAL-02 | Öğün bazlı kayıt: kahvaltı, ara öğün, öğle, ikindi (kurum öğünleri tanımlar) | M |
| MEAL-03 | Yeme miktarı 5'li ölçekle girilir: Hiç / Az / Yarısı / Çoğu / Tamamı | M |
| MEAL-04 | Toplu giriş: "Herkes tamamını yedi" seçilir, istisnalar tek dokunuşla düzeltilir | M |
| MEAL-05 | Alerji uyarısı: menü içeriği çocuğun alerjisiyle eşleşirse öğretmen ve yönetici uyarılır | M |
| MEAL-06 | Su ve biberon (ml) takibi (kreş) | S |
| MEAL-07 | Haftalık beslenme özeti grafiği | C |

### 3.5 Bakım ve sağlık (CARE)

| Kod | Gereksinim | Öncelik |
| --- | --- | --- |
| CARE-01 | Uyku başlangıç/bitiş kaydı (zamanlayıcılı) | M |
| CARE-02 | Bez değişimi / tuvalet kaydı | M |
| CARE-03 | İlaç verme: veli talimatı girer (ilaç, doz, saat), öğretmen verildiğini kaydeder, veli bilgilendirilir | M |
| CARE-04 | Kaza/yaralanma ve ateş raporu: olay formu, veliye anında bildirim, veli "okudum" onayı | M |
| CARE-05 | İhtiyaç bildirimi: "bez azaldı", "yedek kıyafet getirin" | S |

### 3.6 Yoklama (ATT)

| Kod | Gereksinim | Öncelik |
| --- | --- | --- |
| ATT-01 | Giriş ve çıkış saati kaydı | M |
| ATT-02 | Veli "bugün gelmeyecek" bildirimi ve sebebi (hastalık, izin). **[KVKK]** Sebep listeden seçilir; tanı veya ayrıntılı sağlık bilgisi istenmez, serbest metin alanında bu konuda uyarı gösterilir | M |
| ATT-03 | Devamsızlık raporu (çocuk, sınıf, kurum) | S |
| ATT-04 | Kapıda QR veya kiosk tablet ile giriş-çıkış | C |

### 3.7 Teslim alma (PICK)

| Kod | Gereksinim | Öncelik |
| --- | --- | --- |
| PICK-01 | Veli "yola çıktım, \~X dk'ya oradayım" bildirimi gönderir | M |
| PICK-02 | Bildirimde teslim alacak kişi yetkili listeden seçilir | M |
| PICK-03 | Öğretmene push ve uygulamada canlı teslim kuyruğu (geliş sırasına göre) | M |
| PICK-04 | Durumlar: Yolda → Hazırlanıyor → Teslim edildi; her geçişte veliye bildirim | M |
| PICK-05 | Yetkili kişi listesi: fotoğraf, yakınlık, telefon, geçerlilik süresi (tek seferlik yetki dahil). **[KVKK]** Eklenen kişiye SMS ile aydınlatma bağlantısı gider, fotoğrafını bu bağlantıdan kendisi yükler veya onaylar (KVKK-07); süresi dolan yetkinin fotoğraf ve telefon bilgisi 30 gün sonra otomatik silinir | M |
| PICK-06 | Teslimde doğrulama: tek kullanımlık PIN veya QR; öğretmen ekranında teslim alanın fotoğrafı. **[KVKK]** Fotoğraf yalnızca doğrulama ekranında gösterilir, cihazda kalıcı saklanmaz; ekran görüntüsü engellenir (Android) | M |
| PICK-07 | Listede olmayan kişi gelirse teslim engellenir, veli ve yönetici uyarılır | M |
| PICK-08 | Geç teslim alma kaydı ve raporu | S |
| PICK-09 | Konum tabanlı otomatik "yaklaştı" bildirimi (geofence, veli açık rızasıyla). **[KVKK]** Konum yalnızca cihazda işlenir; sunucuya sadece "yaklaştı" olayı gider, konum geçmişi tutulmaz; varsayılan kapalı, tek dokunuşla kapatılabilir | C |

### 3.8 Duyurular (ANN)

| Kod | Gereksinim | Öncelik |
| --- | --- | --- |
| ANN-01 | Hedefli duyuru: kurum geneli, sınıf veya seçili veliler | M |
| ANN-02 | Ek desteği: görsel ve PDF | M |
| ANN-03 | Okundu bilgisi ve okunma oranı; okumayanlara hatırlatma | M |
| ANN-04 | Onay gerektiren duyuru (gezi izni vb.): veli "Onaylıyorum / Onaylamıyorum" der, kayıt saklanır | S |
| ANN-05 | Zamanlanmış duyuru | S |
| ANN-06 | Anket. **[KVKK]** Varsayılan anonim; kişisel veri toplayan ankette ayrı aydınlatma | C |
| ANN-07 | Acil duyuru (sessiz modu aşan kritik bildirim; iOS'ta Apple'dan özel izin gerekir) | C |

### 3.9 Fotoğraf ve medya (MED)

| Kod | Gereksinim | Öncelik |
| --- | --- | --- |
| MED-01 | Çoklu fotoğraf ve kısa video yükleme, çocuk etiketleme. **[KVKK]** Uygulama içi kamerayla çekilen medya cihaz galerisine kaydedilmez; yüklenene kadar uygulamanın korumalı alanında tutulur, yüklemeden sonra silinir. Galeriden seçim kurum ayarıyla kapatılabilir | M |
| MED-02 | Veli yalnızca kendi çocuğunun etiketli olduğu medyayı görür. **[KVKK]** Grup fotoğrafında, velisi grup rızası (`FOTO_GRUP`) vermemiş diğer çocukların yüzü bulanıklaştırılır | M |
| MED-03 | Fotoğraf izni olmayan çocuk etiketlenince öğretmen uyarılır; grup fotoğrafında yayın engellenir veya yüz bulanıklaştırılır. **[KVKK]** Bulanıklaştırma sunucuda, yayından önce yapılır (yüz *tespiti*; yüz tanıma veya biyometrik şablon üretilmez); bulanıklaştırılmamış orijinal yayından sonra silinir | M |
| MED-04 | Cihazda sıkıştırma, küçük önizleme; video süre/boyut sınırı (ör. 60 sn) | M |
| MED-05 | Kurum ayarı: yayın öncesi yönetici onayı | S |
| MED-06 | Çocuk bazlı zaman tüneli ve albüm; indirme izni kurum ayarı. **[KVKK]** İndirme ayrıca fotoğraftaki tüm çocukların grup rızasına bağlıdır | S |
| MED-07 | Yayın sonrası arka planda yükleme; zayıf bağlantıda kuyruk | M |
| MED-08 | **[KVKK]** İndirilen medyaya indiren hesabı belirten görünmez iz (filigran) eklenmesi; sızıntı kaynağının tespiti için | C |

### 3.10 Mesajlaşma (MSG)

| Kod | Gereksinim | Öncelik |
| --- | --- | --- |
| MSG-01 | Veli ↔ sınıf öğretmenleri: sınıfın tüm öğretmenlerinin gördüğü ortak konuşma | M |
| MSG-02 | Veli ↔ yönetim konuşması | M |
| MSG-03 | Veliler arası mesajlaşma bilinçli olarak yok | M |
| MSG-04 | Metin, fotoğraf, dosya; okundu bilgisi | M |
| MSG-05 | Mesajlar kurum arşivinde saklanır; silme sadece "herkesten gizle" şeklinde, iz kalır. **[KVKK]** Süresiz arşiv yok: mesajlar saklama süresi (6.7, öneri: eğitim yılı sonu + 2 yıl) dolunca imha edilir | M |
| MSG-06 | Öğretmen mesai saatleri ve sessiz saat; saat dışında otomatik yanıt | S |
| MSG-07 | Sesli mesaj | S |
| MSG-08 | Hazır cevap şablonları; otomatik çeviri (yabancı veliler). **[KVKK]** Çeviri servisi yurt dışındaysa mesaj içeriği yurt dışına aktarılır; yurt içi servis veya m.9 mekanizması kurulmadan açılmaz | C |

### 3.11 Takvim (CAL)

| Kod | Gereksinim | Öncelik |
| --- | --- | --- |
| CAL-01 | Kurum takvimi: tatiller, etkinlikler, veli toplantıları | M |
| CAL-02 | Etkinliği telefon takvimine ekleme | S |
| CAL-03 | Veli toplantısı için randevu slotu seçimi | C |

### 3.12 Bildirimler ve raporlama (NOT, REP)

| Kod | Gereksinim | Öncelik |
| --- | --- | --- |
| NOT-01 | Push bildirim ve uygulama içi bildirim merkezi. **[KVKK]** Push'ta çocuk adı, sağlık, fotoğraf ve mesaj içeriği gönderilmez; cihaz belirteci ve teknik referansların aktarımı ayrıca değerlendirilir (KVKK-13) | M |
| NOT-02 | Kategori bazlı bildirim tercihi; teslim, kaza ve ilaç bildirimleri kapatılamaz | M |
| NOT-03 | Kritik bildirim okunmazsa belirli süre sonra SMS ile yedekleme. **[KVKK]** SMS metninde çocuk adı veya sağlık bilgisi yer almaz | C |
| REP-01 | Yönetici paneli: doluluk, devamsızlık, duyuru okunma oranı, öğretmenlerin kayıt doldurma oranı. **[KVKK]** Personel göstergeleri çalışan aydınlatma metnine dayanır | S |
| REP-02 | Çocuk bazlı haftalık/aylık rapor (PDF). **[KVKK]** PDF kısa ömürlü bağlantıyla indirilir, e-posta eki olarak gönderilmez | S |
| REP-03 | Denetim kayıtlarının yönetici tarafından görüntülenmesi. **[KVKK]** Veli, çocuğunun verisine hangi rollerin ne zaman eriştiğinin özetini görebilir | M |

### 3.13 KVKK ve gizlilik (KVKK) — yeni modül

| Kod | Gereksinim | Öncelik |
| --- | --- | --- |
| KVKK-01 | Katmanlı aydınlatma: kurumun (veri sorumlusu) aydınlatma metni ilk girişte kısa özet + tam metin olarak gösterilir; sağlık formu, yetkili kişi ekleme, konum izni gibi veri toplama noktalarında bağlamsal bilgilendirme yapılır. Aydınlatma açık rıza ekranından ayrıdır; metinler sürümlenir, okuma kaydı sürümüyle saklanır | M |
| KVKK-02 | Ayrıştırılmış açık rıza, çocuk bazında ve varsayılan kapalı: (a) sağlık ve diyet verisi, (b) fotoğraf/video yalnızca kendi velisiyle, (c) grup fotoğrafında diğer velilerin görmesi, (d) kurum tanıtım materyali ve sosyal medya, (e) konum (geofence), (f) yetkili teslim alanın günlük akışı görmesi. Toplu "tümünü kabul et" yoktur; rıza vermemek uygulamanın geri kalanının kullanımını engellemez | M |
| KVKK-03 | Rıza kaydı ispat niteliğinde ve değiştirilemez: veren veli, çocuk, rıza türü, metin sürümü, zaman, kanal/cihaz; geri alma da aynı şekilde yeni kayıt olarak eklenir. Metin sürümü değişince yeniden onay istenir | M |
| KVKK-04 | İki velili ailede çocuk için en kısıtlayıcı rıza uygulanır: bir veli rızayı geri alırsa paylaşım durur ve diğer veli bilgilendirilir | M |
| KVKK-05 | İlgili kişi başvurusu (m.11): bilgi alma, düzeltme, silme, itiraz talepleri uygulama içinden oluşturulur ve kurum paneline düşer; 30 günlük yasal süre sayacı ve hatırlatma; yanıt ve gerekçe kayıt altına alınır | M |
| KVKK-06 | Veri dökümü: veli kendi ve çocuğunun verilerini makine tarafından okunabilir formatta (JSON + medya arşivi) kısa ömürlü bağlantıyla indirir | M |
| KVKK-07 | Üçüncü kişilere aydınlatma: veli yetkili teslim alan veya acil durum kişisi eklediğinde kişiye SMS ile aydınlatma bağlantısı gider; yetkili kişi fotoğrafını bu bağlantıdan kendisi yükler veya onaylar, onaylanmadan fotoğraf kullanılmaz | M |
| KVKK-08 | Saklama motoru: veri kategorisi bazında süreler (6.7) platform varsayılanı ve kurumun bu sınırlar içindeki ayarıyla uygulanır; günlük çalışan imha işi siler veya anonimleştirir ve imha tutanağı üretir | M |
| KVKK-09 | Erişim kısıtlaması: yönetici bir velinin erişimini kapatır veya salt-okunur yapar; dayanak belge saklanır; veliler birbirinin iletişim bilgilerini ve eklediği yetkili kişileri göremez | M |
| KVKK-10 | Gizlilik merkezi: rızalar, aydınlatma metinleri, aktif cihazlar ve oturumlar, bildirim tercihleri, başvurular ve veri dökümü tek ekranda | S |
| KVKK-11 | Platform personelinin kurum verisine erişimi varsayılan kapalıdır; destek için kurum yöneticisinin verdiği süreli (ör. 24 saat) ve kapsamı belirli acil erişim kullanılır; her erişim kuruma raporlanır | M |
| KVKK-12 | Kurum KVKK raporları: çocuk bazında rıza durumu, kişisel veri envanteri taslağı (6.3), erişim ve imha raporları | S |
| KVKK-13 | Bildirim gizliliği: push metninde çocuk adı, sağlık bilgisi, fotoğraf ve mesaj içeriği yer almaz; içerik uygulama açıldığında güncel yetkiyle sunucudan çekilir. FCM/APNs cihaz belirteci ve teknik referansları takma kimlikli kişisel veri kabul edilir; yurt dışı aktarım mekanizması özellik açılmadan değerlendirilir | M |
| KVKK-14 | Uygulamada reklam, reklam kimliği ve üçüncü taraf analitik/izleme SDK'sı yoktur; kullanım istatistikleri yurt içinde ve anonim toplanır | M |
| KVKK-15 | Hassas ekranlarda (sağlık, teslim doğrulama, medya) ekran görüntüsü engeli (Android) veya tespit ve kayıt (iOS); uygulama arka plana alınınca görev değiştirici önizlemesi gizlenir | S |

## 4. Temel kullanıcı senaryoları

Ürünün değeri üç senaryoda ölçülür: öğretmenin günü hızla kayda geçirmesi, çocuğun güvenle teslim edilmesi ve fotoğrafın yalnızca doğru veliye ulaşması.

### 4.1 Öğretmenin bir günü

1. Sabah yoklama: gelen çocuklar tek dokunuşla işaretlenir; "bugün gelmeyecek" bildirimleri listede hazır görünür.
2. Kahvaltı: sınıfa "herkes tamamını yedi" girilir, birkaç istisna düzeltilir; alerji uyarısı varsa ekranda çıkar.
3. Etkinlik: aktivite seçilip tüm sınıfa işlenir; fotoğraflar çekilir ve çocuklar etiketlenir.
4. Öğle yemeği ve uyku: öğün miktarı ve uyku başlangıç/bitişi kaydedilir; tuvalet/bez kayıtları gün boyu eklenir.
5. Gün içi: veli mesajları ortak sınıf gelen kutusundan yanıtlanır; ilaç saati gelince hatırlatma düşer.
6. Teslim saatleri: kuyruktaki çocuklar hazırlanır, kapıda doğrulama yapılır.
7. Gün sonu: günlük rapor otomatik derlenir ve velilere gönderilir.

### 4.2 Teslim alma

&#91;embedded content: teslim alma akışı · 6 adım, 1 karar\]

Doğrulama adımı atlanamaz: yetkili listede olmayan biri gelirse teslim sistemsel olarak durur ve denetim kaydına düşer.

### 4.3 Fotoğraf paylaşımı

1. Öğretmen fotoğrafları uygulama içi kamerayla çeker veya kurum izin veriyorsa galeriden seçer, her fotoğrafta çocukları etiketler. **[KVKK]** Uygulama içi kamerayla çekilen fotoğraf cihaz galerisine kaydedilmez.
2. Fotoğraf izni olmayan bir çocuk etiketlendiyse uygulama uyarır; öğretmen fotoğrafı çıkarır veya yüzü bulanıklaştırır. **[KVKK]** Yüz tespiti etiket sayısından fazla yüz bulursa öğretmen "etiketlenmemiş çocuk olabilir" uyarısı alır.
3. Kurum ayarı açıksa yayından önce yönetici onayı beklenir.
4. **[KVKK]** Sunucu her veli için uygun türevi üretir: veli kendi çocuğunu net görür, grup rızası olmayan diğer çocuklar bulanıklaştırılır. Bulanıklaştırılmamış orijinal yayından sonra silinir.
5. Fotoğraf yalnızca etiketli çocukların velilerine görünür ve onlara bildirim gider. **[KVKK]** Bildirim metninde çocuk adı yer almaz.

### 4.4 İlk giriş, aydınlatma ve rıza (KVKK, yeni)

1. Kurum veliyi davet eder; veli tek kullanımlık bağlantı ve SMS OTP ile giriş yapar.
2. Kurumun aydınlatma metni önce kısa özet, istenirse tam metin olarak gösterilir; okuma kaydı metin sürümüyle saklanır.
3. Açık rızalar ayrı bir ekranda, her biri ayrı bir "Onaylıyorum / Onaylamıyorum" seçimi olarak sunulur; hiçbiri önceden seçili gelmez (karar verilmemiş rıza verilmemiş sayılır). Hiçbirini vermemek uygulamaya girişi engellemez.
4. Sağlık rızası verilirse sağlık profili formu açılır; verilmezse alerji uyarısının çalışmayacağı açıkça yazılır.
5. Rızalar Profil → Gizlilik merkezinden her an değiştirilebilir; geri alma anında uygulanır.

### 4.5 Kurumdan ayrılış ve silme (KVKK, yeni)

1. Yönetici çocuğun ayrılış tarihini girer; velilere dışa aktarım (fotoğraflar, günlük raporlar) için bildirim gider.
2. Saklama motoru her veri kategorisi için süreyi başlatır (6.7).
3. Süresi dolan veriler günlük imha işiyle silinir veya anonimleştirilir; imha tutanağı oluşur.
4. Yedeklerdeki kopyalar en geç 35 günlük rotasyonla düşer; geri yükleme olursa silme listesi yeniden uygulanır.

## 5. Fonksiyonel olmayan gereksinimler

En kritik iki kalite özelliği hız (öğretmen bir sınıfa öğün girişini 30 saniyede bitirmeli) ve güvenilirliktir: kabul edilmiş teslim kaydı kalıcı tutulur, olay yeniden denenir ve cihaz güncel durumu REST ile uzlaştırır. Hedef değerler aşağıda; ilk pilot sonrası gerçek ölçümlerle güncellenmeli.

| Kategori | Gereksinim | Hedef |
| --- | --- | --- |
| Kullanılabilirlik | Öğretmenin 20 kişilik sınıfa bir öğün girişini tamamlama süresi | ≤ 30 sn |
| Kullanılabilirlik | Sık işlemler ana ekrandan en fazla 2 dokunuş uzakta; tek elle kullanım | 2 dokunuş |
| Performans | Senkron JSON iş isteklerinde sunucu tarafı API yanıt süresi (p95); medya aktarımı ve uzun işler ayrı ölçülür | < 300 ms |
| Performans | Uygulama soğuk açılış süresi (orta segment Android) | < 2,5 sn |
| Performans | Commit'ten çevrimiçi ve yetkili öğretmenin teslim ekranının güncellenmesine kadar süre (p95); push teslimi bu garantiye dahil değildir | < 5 sn |
| Güvenilirlik | Europe/Istanbul okul saatlerinde (07:00–19:00), aylık başarılı iş isteği oranı; planlı kesinti dahil | ≥ %99,9 |
| Güvenilirlik | Medya yükleme başarısızlığında doğrulanmış yükleme intent'iyle yeniden deneme; yerel dosya başarılı tamamlama onayına kadar korunur | Normal bağlantı/worker tekrarında kayıp ve çift yayın yok |
| Çevrimdışı | Güvenli günlük kayıtlar bağlantı yokken cihazda kuyruklanır, bağlantı gelince güncel yetkiyle senkronize olur | Teslim onayı, rıza ve ilaç yetkisi çevrimiçi doğrulanır |
| Ölçeklenebilirlik | İlk yıl hedef ölçek; yatay ölçeklenebilir mimari | 500 kurum, 50 bin çocuk, 100 bin veli |
| Ölçeklenebilirlik | Gün içi yoğunluk: sabah girişi ve 16:00–18:00 teslim saatleri | 10× ortalama yük |
| Depolama | Çocuk başı yıllık medya tahmini (sıkıştırılmış) | \~1–2 GB |
| Uyumluluk | Desteklenen işletim sistemleri | iOS 16+, Android 8 (API 26)+ |
| Uyumluluk | Telefon ve tablet yerleşimi (öğretmenler sık tablet kullanır) | Her ikisi |
| Yerelleştirme | Türkçe ve İngilizce; altyapı çoklu dil ve sağdan sola yazıya hazır | TR, EN |
| Erişilebilirlik | Dinamik yazı boyutu, ekran okuyucu etiketleri, yeterli kontrast | WCAG 2.1 AA |
| Bakım | Merkezi hata ve çökme izleme; çökmesiz oturum oranı. **[KVKK]** İzleme yurt içinde, kişisel veri maskeli | ≥ %99,5 |
| Dağıtım | Uygulama mağazası beklemeden yapılandırma ve özellik açma/kapatma | Uzaktan özellik bayrağı |
| **[KVKK]** Veri yerelliği | İçerik, veritabanı, medya, anahtarlar, yedekler ve gözlemlenebilirlik depoları Türkiye'de; §6.6'daki push metaverisi ve sonraki özelliklerin aktarımları ayrıca değerlendirilir | Ana veri depoları yurt içi; aktarımlar belgeli |
| **[KVKK]** Gizlilik | Log, iz (trace) ve hata raporlarında kişisel veri maskeleme | CI'da otomatik test |
| **[KVKK]** Gizlilik | Test ve geliştirme ortamlarında üretim verisi kullanımı | Yok (yalnızca sentetik veri) |
| **[KVKK]** Yedekleme | Şifreli yedek; kurtarma noktası / kurtarma süresi; rotasyon | ≤ 15 dk / ≤ 4 sa; 35 gün |
| **[KVKK]** Saklama | Periyodik imha işinin çalışma sıklığı (yönetmelik üst sınırı 6 ay) | Günlük |
| **[KVKK]** İlgili kişi başvurusu | Başvuru saatinden hesaplanan operasyon hedefi ve ayrı yasal son tarih | Hedef ≤ 15 takvim günü; yasal son tarih 30 takvim günü |

Normal yeniden denemede kayıp olmaması, bölgesel felaket için sıfır veri kaybı taahhüdü değildir. Tam sistem kurtarmasında RPO ≤ 15 dakika ve RTO ≤ 4 saat uygulanır. Çevrimdışı destek teslim onayı, rıza ve ilaç yetkisi gibi güncel doğrulama isteyen işlemleri kapsamaz; kayıt tiplerinin güvenli kuyruklama sınırları backend sözleşmesinde tanımlanır.

## 6. Güvenlik, gizlilik ve KVKK

Uygulama çocukların fotoğrafını, sağlık bilgisini (alerji, ilaç) ve kimin teslim alabileceğini tutuyor; bu yüzden güvenlik bir özellik değil, ürünün kendisi. Aşağıdaki maddeler yayın öncesi bir KVKK uzmanı ve hukuk danışmanıyla doğrulanmalı. **[KVKK]** Bu bölüm revizyonda genişletildi; hukuki sebepler ve süreler öneri niteliğindedir.

### 6.1 Mevzuat çerçevesi

| Düzenleme | Uygulamaya etkisi |
| --- | --- |
| 6698 sayılı KVKK (7499 sayılı Kanunla yapılan, 1 Haziran 2024'te yürürlüğe giren değişiklikler dahil) | Genel ilkeler (m.4), işleme şartları (m.5, m.6), silme ve imha (m.7), yurt dışı aktarım (m.9), aydınlatma (m.10), ilgili kişi hakları (m.11), veri güvenliği ve ihlal bildirimi (m.12) |
| Aydınlatma Yükümlülüğünün Yerine Getirilmesinde Uyulacak Usul ve Esaslar Hakkında Tebliğ | Aydınlatma veri elde edilirken yapılır ve açık rıza metninden ayrı sunulur; genel ifadeli metin yeterli değildir |
| Veri Sorumlusuna Başvuru Usul ve Esasları Hakkında Tebliğ | İlgili kişi başvurusu en geç 30 gün içinde, kural olarak ücretsiz sonuçlandırılır (m.13) |
| Kişisel Verilerin Silinmesi, Yok Edilmesi veya Anonim Hale Getirilmesi Hakkında Yönetmelik | Saklama ve imha politikası; periyodik imha aralığı en fazla 6 ay; imha işlemlerinin kayıtları en az 3 yıl saklanır |
| Kişisel Verilerin Yurt Dışına Aktarılmasına İlişkin Usul ve Esaslar Hakkında Yönetmelik (2024) | Yeterlilik kararı yoksa uygun güvence (ör. Kurul'un standart sözleşmesi) gerekir; standart sözleşme imzadan itibaren 5 iş günü içinde Kurum'a bildirilir |
| Kurul'un özel nitelikli kişisel veriler için yeterli önlemler kararı (2018/10) | Sağlık verisi için ayrı politika, kriptografik koruma, anahtarların ayrı ortamda tutulması, erişim logları, personel eğitimi ve gizlilik taahhüdü |
| Kurul'un veri ihlali bildirim usul ve esasları kararı (2019/10) | Kurul'a 72 saat içinde bildirim; ilgili kişilere en kısa makul sürede bildirim; ihlal kayıtlarının tutulması |
| Veri Sorumluları Sicili (VERBİS) Yönetmeliği | Kayıt yükümlülüğü veya muafiyeti kurum bazında değerlendirilir |
| 6563 sayılı Elektronik Ticaretin Düzenlenmesi Hakkında Kanun | Uygulama bildirimleri hizmet iletisidir; pazarlama amaçlı ileti ancak onay ve İYS kaydıyla gönderilir |
| 5651 sayılı Kanun | Platformun yer sağlayıcı sayılıp sayılmayacağı ve trafik bilgisi saklama yükümlülüğü hukukçuyla değerlendirilmeli |
| Cumhurbaşkanlığı 2019/12 sayılı Genelgesi ve Bilgi ve İletişim Güvenliği Rehberi | Yalnızca kamu okulları hedeflenirse: verinin yurt içinde tutulması ve ek güvenlik denetimleri |

### 6.2 Roller ve sözleşmeler

- **Kurum veri sorumlusudur:** çocuk, veli, yetkili teslim alan ve personel verileri için. VERBİS kaydı (veya muafiyet değerlendirmesi), aydınlatma metinleri ve ilgili kişi başvurularının yanıtlanması kurumun sorumluluğundadır; platform bunları kolaylaştıran araçları sağlar (3.13).
- **Platform çift rollüdür:** kurum adına işlenen tüm içerikte veri işleyen; kendi müşteri ilişkisi (kurum yetkililerinin iletişim ve fatura bilgileri), platform güvenliği ve anonim hizmet istatistikleri için veri sorumlusu. Bu işlemeler için platform ayrı bir aydınlatma metni yayımlar ve kendi VERBİS yükümlülüğünü değerlendirir.
- **Veri işleme sözleşmesi** (her kurumla) en az şunları içerir: yalnızca kurumun talimatıyla işleme; personelin gizlilik yükümlülüğü; alt veri işleyen listesi (6.6) ve değişiklikten önce bildirim ile itiraz hakkı; ihlalin kuruma gecikmeksizin, en geç 24 saat içinde bildirilmesi; kurumun denetim hakkı; teknik ve idari tedbirler eki; sözleşme bitiminde 30 gün içinde dışa aktarım, ardından 60 gün içinde imha ve imha tutanağı. KVKK m.12/2 uyarınca kurum ve platform veri güvenliğinden birlikte sorumludur.
- **Personel:** Öğretmen ve idari personele işe başlarken ve yılda bir KVKK eğitimi verilir, gizlilik taahhüdü imzalatılır. Personelin uygulama kullanım kayıtları (denetim kaydı, kayıt doldurma oranı) için çalışan aydınlatma metni hazırlanır.

### 6.3 Kişisel veri envanteri ve hukuki sebepler

Envanter, kurumun VERBİS bildiriminin ve saklama-imha politikasının temelidir; platform her kuruma doldurulmuş bir taslak sunar (KVKK-12). Hukuki sebepler öneridir, hukukçu kesinleştirir.

| Veri kategorisi | İlgili kişi | Amaç | Önerilen hukuki sebep | Alıcılar |
| --- | --- | --- | --- | --- |
| Kimlik (ad, soyad, doğum tarihi), sınıf | Çocuk | Eğitim ve bakım hizmetinin yürütülmesi | m.5/2-c sözleşmenin ifası (veli–kurum sözleşmesi) | Sınıf öğretmenleri, yönetim, veliler |
| İletişim (telefon, e-posta) | Veli, personel | Kimlik doğrulama, bildirim | m.5/2-c sözleşmenin ifası | SMS ve e-posta sağlayıcı (alt işleyen) |
| Yetkili kişi kimliği, fotoğrafı, yakınlığı, telefonu | Yetkili teslim alan, acil durum kişisi | Teslimde kimlik doğrulama, acil iletişim | m.5/2-f meşru menfaat (çocuğun güvenliği); aydınlatma KVKK-07 ile | Sınıf öğretmenleri, yönetim |
| Günlük kayıtlar (aktivite, öğün, uyku, tuvalet), yoklama, teslim kayıtları | Çocuk | Veliyi bilgilendirme, bakım, güvenli teslim | m.5/2-c sözleşmenin ifası | Öğretmen, yönetim, veli |
| Sağlık (alerji, kronik durum, ilaç, ateş ve kaza raporu) ve diyet kısıtı | Çocuk | Çocuğun güvenliği, ilaç verme, alerji uyarısı | m.6/3-a veli açık rızası; veliye ulaşılamayan acil durumda m.6/3-c | Öğretmen (gereken alanlar), yönetim, veli |
| Fotoğraf ve video | Çocuk | Veliyle paylaşım | m.5/1 açık rıza, kademeli (KVKK-02) | Etiketli çocukların velileri |
| Konum ("yaklaştı" olayı) | Veli | Teslim hazırlığı | m.5/1 açık rıza | Sınıf öğretmenleri |
| Mesajlar ve duyuru onayları | Veli, personel | İletişim; gezi izni gibi onayların ispatı | m.5/2-c sözleşmenin ifası; m.5/2-e hakkın korunması | Konuşmanın tarafları, yönetim |
| Rıza ve aydınlatma kayıtları | Veli | Rıza ve aydınlatmanın ispatı | m.5/2-ç hukuki yükümlülük | Yönetim |
| Denetim ve güvenlik kayıtları (IP, cihaz, erişim) | Tüm kullanıcılar | Veri güvenliği (m.12) | m.5/2-ç hukuki yükümlülük | Yönetim, platform güvenlik ekibi |
| Personel kullanım istatistikleri | Personel | Hizmet kalitesinin izlenmesi | m.5/2-f meşru menfaat (çalışan aydınlatmasıyla) | Yönetim |

- **Açık rıza hizmetin ön koşulu yapılmaz:** fotoğraf, konum veya sağlık rızası vermeyen veli uygulamanın geri kalanını kullanmaya devam eder. Sağlık rızası yoksa alerji eşleşmesinin çalışmayacağı veliye açıkça söylenir.
- **2024 değişikliği:** m.6 artık açık rıza dışında da özel nitelikli veri işleme şartları sayıyor. Okulun çocuk sağlık verisini işlemesine uyan başka bir şart (ör. kanunda açıkça öngörülme) hukukçu tarafından netleştirilinceye kadar veli açık rızası esas alınır.
- **Çocuk verisi:** Çocuk adına haklar ve rıza veli/vasi tarafından kullanılır. İlkokul yaş grubunda çocuğa da yaşına uygun dille kısa bilgilendirme yapılması iyi uygulamadır.

### 6.4 Rıza yönetimi

- **Ayrım:** Aydınlatma ve açık rıza ayrı ekranlarda sunulur. Her rıza belirli bir konuya ilişkindir; toplu "tümünü kabul et" yoktur, varsayılan kapalıdır.
- **Rıza türleri:** `SAGLIK` (sağlık ve diyet verisi), `FOTO_VELI` (fotoğraf yalnızca kendi velisiyle), `FOTO_GRUP` (grup fotoğrafında diğer veliler görebilir), `FOTO_TANITIM` (kurum tanıtım materyali ve sosyal medya), `KONUM` (geofence), `AKIS_YETKILI` (yetkili teslim alan akışı görebilir).
- **İki veli:** Rızayı velilerden biri verebilir; çocuk için her zaman en kısıtlayıcı rıza uygulanır. Bir veli rızayı geri alırsa paylaşım durur, diğer veli bilgilendirilir.
- **Geri alma:** Yeni erişim ve yayın isteklerinde güncel karar anında uygulanır. `FOTO_GRUP` geri alınırsa eski yayın türevleri erişime kapatılır; çocuğun yüzü bulanıklaştırılıp yeniden doğrulanınca uygun türevler açılır. `FOTO_VELI` geri alınırsa yeni medya yayınlanmaz ve veli mevcut medyanın silinmesini talep edebilir. Daha önce cihaza teslim edilmiş görüntülerin geri çekilemeyeceği teknik sınır kullanıcıya açıklanır.
- **Kurtarmada korunma:** İptal PostgreSQL'de hemen erişimi kapatır; başarı yanıtı bağımsız kalıcı iptal manifesti onayından sonra verilir. Fiziksel imha da manifest onayını bekler. Bu kararlar genel 15 dakikalık RPO nedeniyle eski yedekten geri açılamaz; eksik/güncelliği doğrulanamayan manifestle hizmet açılmaz.
- **Sürüm:** Aydınlatma veya rıza metni değişince yeniden onay istenir; onaylanana kadar eski sürümün kapsamı uygulanır, yeni amaçlar başlatılmaz.
- **İspat:** Kim, hangi çocuk için, hangi rıza türünü, hangi metin sürümüyle, ne zaman, hangi kanal ve cihazdan verdi veya geri aldı; kayıtlar yalnızca eklenebilir (append-only).

### 6.5 Teknik ve idari önlemler

| Alan | Önlem |
| --- | --- |
| Kiracı izolasyonu | Her sorgu kurum kimliğiyle filtrelenir (veritabanında satır düzeyi güvenlik); kurumlar arası veri sızıntısı otomatik testlerle denetlenir. **[KVKK]** Uygulama veritabanına tablo sahibi olmayan ve `BYPASSRLS` yetkisi bulunmayan bir rolle bağlanır; tablolarda `FORCE ROW LEVEL SECURITY` açıktır |
| Yetkilendirme | Rol + kapsam (kurum, sınıf, çocuk) bazlı erişim kontrolü, sunucu tarafında zorunlu. Güncel oturum, üyelik, erişim kısıtlaması ve rıza PostgreSQL'den doğrulanır; Redis önbelleği tek başına erişim vermez. **[KVKK]** Rıza kontrolleri (ör. fotoğraf, akış paylaşımı) de sunucu tarafında yetkilendirmenin parçasıdır |
| Şifreleme | İletimde TLS 1.2+, depoda AES-256; sağlık alanları ayrıca alan düzeyinde şifrelenir. **[KVKK]** Alan düzeyi şifreleme Vault Transit ile; kurum başına ayrı anahtar, yıllık rotasyon; anahtarlar veritabanından ayrı ortamda (Kurul 2018/10) |
| **[KVKK]** Sır yönetimi | Uygulama sırları Vault'ta tutulur; veritabanı kimlik bilgileri Vault tarafından kısa ömürlü üretilir; Vault erişimleri ayrı denetim loguna yazılır. Kaynak kodda ve imajlarda sır bulunmaz |
| **[KVKK]** Aranabilir şifreli alan | Telefon numarası gibi aranması gereken alanlar şifreli saklanır; arama HMAC tabanlı körleştirilmiş indeksle yapılır |
| **[KVKK]** Kriptografik imha | Kurum anahtarı hukuki muhafaza ve saklama yükümlülüğü sona erince dar kapsamlı, onaylı işlemle imha edilir. Medya, ortak kullanıcı anahtarıyla şifreli veriler ve eski Vault yedeklerindeki anahtar kopyaları ayrıca imha/rotasyon politikasına tabidir; yalnız canlı anahtarın silinmesi bütün yedekleri okunamaz yapmaz |
| Medya erişimi | S3 PUT imzası en çok 5 dk; özel GET, en çok 5 dk geçerli uygulama bağlantısı ve her istekte güncel yetki/rıza kontrolüyle medya geçidinden akar. Ham S3 GET imzası istemciye verilmez. **[KVKK]** Yükleme karantinaya yapılır; EXIF/konum silme, gerekli yüzlerin bulanıklaştırılması ve zararlı dosya taraması bitmeden yayınlanmaz; orijinal işlem sonrası silinir |
| Kimlik doğrulama | OTP deneme sınırı ve kilitleme; cihaz listesi ve uzaktan oturum kapatma; yönetici paneli için 2FA zorunlu |
| **[KVKK]** Mobil cihaz | Erişim anahtarı yalnız bellekte, yenileme anahtarı Keychain/Keystore'da; yerel veritabanı şifreli; oturum kapanınca veya yetki iptali öğrenilince yerel veri silinir. Çevrimdışı kayıp cihaza anında silme garanti edilemez; sunucu yeni erişimi reddeder. Uygulama içi kamerayla çekilen medya galeriye yazılmaz; arka plana alınınca görev değiştirici önizlemesi gizlenir; root/jailbreak tespitinde uyarı |
| Denetim kaydı | Kim, hangi çocuğun verisini ne zaman gördü/değiştirdi; teslim ve ilaç kayıtları değiştirilemez (yalnızca düzeltme kaydı eklenir). **[KVKK]** Kayıtlar yalnızca eklenebilir; bütünlük için zincirlenmiş özet (hash); kayıtlara erişim de kayda alınır |
| **[KVKK]** Platform personeli | Kurum verisine varsayılan erişim yok; kurum onaylı, süreli acil erişim (KVKK-11). Üretim veritabanına doğrudan erişim yalnızca iki kişi onayıyla, kayıt altında ve süreli kimlik bilgisiyle |
| **[KVKK]** Yetki gözden geçirme | Kurum ve platform yetkileri 3 ayda bir gözden geçirilir; ayrılan personelin erişimi aynı gün kapatılır |
| **[KVKK]** Ağ | Veritabanı, önbellek, Vault ve nesne depolama internete kapalı özel ağda; yönetim erişimi VPN + 2FA ile |
| **[KVKK]** Log ve hata izleme | Uygulama ve sunucu loglarında, hata raporlarında sır ve içerik ayıklanır; izinli takma kimlikli metaveriye erişim sınırlıdır. Hata izleme yurt içinde barındırılır; operasyon log/izleri en çok 90 gün tutulur; iş denetimi §6.7'ye tabidir |
| Ekran görüntüsü | Android'de hassas ekranlarda engelleme mümkün; iOS'ta yalnızca tespit edilip loglanabilir. Bu sınır kurumlara açıkça söylenmeli |
| Güvenlik testi | OWASP MASVS'ye göre mobil güvenlik kontrolü ve yayın öncesi bağımsız sızma testi. **[KVKK]** Sızma testi yılda en az bir kez ve büyük değişikliklerden sonra tekrarlanır; bağımlılık ve imaj zafiyet taraması CI'da |
| **[KVKK]** Geliştirme ortamı | Üretim verisi kullanılmaz, yalnızca sentetik veri; yerel altyapı `infra/` altındaki Docker Compose ile kurulur ve servisler yalnızca 127.0.0.1'e açılır |

### 6.6 Alt veri işleyenler ve yurt dışına aktarım

Türkiye'de barındırma esastır. 2024 değişikliğiyle m.9'da aktarım sırası şöyledir: yeterlilik kararı → uygun güvence (Kurul'un standart sözleşmesi, bağlayıcı şirket kuralları, taahhütname + Kurul izni) → yalnızca arızi aktarımlar için istisnalar (ör. riskler hakkında bilgilendirilmiş açık rıza). Push bildirim gibi sürekli aktarımlar açık rızaya dayandırılamaz.

| Hizmet | Konum | Aktarılan veri | Not |
| --- | --- | --- | --- |
| Bulut barındırma, veritabanı, nesne depolama, yedek | Türkiye | Tümü | Aktarım yok |
| SMS sağlayıcı (OTP, kritik bildirim yedeği) | Türkiye | Telefon, OTP kodu, kişisel veri içermeyen metin | |
| E-posta sağlayıcı | Türkiye | E-posta adresi, OTP kodu | |
| Push (FCM, APNs) | Yurt dışı | Cihaz belirteci, olay türü ve opak teknik referans; çocuk/sağlık/mesaj içeriği gönderilmez (KVKK-13) | Takma kimlikli kişisel veri aktarımıdır; uygun mekanizma ve standart sözleşme kullanılırsa 5 iş günü bildirim süreci hukukçuyla netleştirilir |
| Hata izleme | Türkiye (kendi sunucumuz) | Maskelenmiş hata kaydı | Crashlytics veya SaaS Sentry kullanılırsa yurt dışı aktarım doğar |
| Otomatik çeviri (MSG-08) | Belirlenecek | Mesaj içeriği | Yurt dışıysa m.9 mekanizması kurulmadan özellik açılmaz |
| Uygulama mağazaları | Yurt dışı | Mağaza hesabı (platformun kontrolü dışında) | Aydınlatma metninde belirtilir |

### 6.7 Saklama ve imha

Süreler kurumun saklama ve imha politikasına girer. Aşağıdakiler platform varsayılanıdır; kurum bu sınırlar içinde değiştirebilir. Süreler hukukçuyla kesinleştirilmeli.

| Veri kategorisi | Varsayılan saklama süresi | Süre sonunda |
| --- | --- | --- |
| Günlük kayıtlar, yoklama, öğün | Eğitim yılı sonu + 1 yıl | Silme; istatistik için anonimleştirme |
| Sağlık profili (alerji, kronik durum, diyet kısıtı) | Çocuk kurumdan ayrılınca + 30 gün; rıza geri alınınca derhal | Silme |
| İlaç verme, kaza ve ateş raporları | Olay tarihinden itibaren 10 yıl, erişim yalnızca kurum yöneticisinde (zamanaşımı süreleri gözetilerek kesinleştirilmeli) | Silme |
| Teslim kayıtları | Eğitim yılı sonu + 2 yıl | Silme |
| Yetkili kişinin fotoğrafı ve telefonu (PICK-05) | Yetkinin bitiminden + 30 gün; teslim alanın adı ve yakınlığı teslim kaydında teslim kayıtlarının süresiyle tutulur | Silme |
| Fotoğraf ve video | Çekildiği eğitim yılı sonu + 1 yıl veya çocuk ayrılınca + 90 gün, hangisi önce gelirse (veli öncesinde dışa aktarabilir) | Silme (orijinal ve tüm türevler) |
| Mesajlar, duyurular, onay kayıtları | Eğitim yılı sonu + 2 yıl | Silme |
| Rıza ve aydınlatma kayıtları | İlişkinin bitiminden + 10 yıl (yalnızca ispat kaydı, içerik değil) | Silme |
| İş denetimi ve izinli güvenlik olay kayıtları | 2 yıl | Silme |
| Operasyon logları, izler ve hata raporları | En çok 90 gün | Silme |
| Kullanıcı hesabı | Silme talebinden + 30 gün | Silme |
| Yedekler | 35 gün rotasyon | Rotasyonla düşer |
| İmha tutanakları | 3 yıl (yönetmelik alt sınırı) | Silme |

- **Periyodik imha** her gün çalışır; yönetmelikteki 6 aylık üst sınırın çok altındadır. Her çalışma tutanağa bağlanır.
- **Talep üzerine silme:** m.11 başvurusu 30 gün içinde sonuçlandırılır. Saklanması zorunlu kayıtlar gerekçesiyle bildirilir ve süre sonuna kadar erişimi kısıtlanır.
- **Anonimleştirme** geri döndürülemez yapılır: kimlik alanları silinir, doğum tarihi yaş grubuna, sınıf kurum toplamına indirgenir; küçük gruplarda (ör. 5 kişiden az) istatistik üretilmez.
- **Yedekler:** Şifreli ve 35 gün rotasyonlu; silinen veri ve eski anahtar kopyaları yedeklerden de en geç 35 gün içinde düşer. PostgreSQL, Vault ve nesneler birlikte kurtarılır; kurtarılan snapshot'tan bağımsız güncel imha listesi yeniden uygulanmadan hizmet açılmaz.
- **Sözleşme bitimi:** 30 gün dışa aktarım; ardından 60 gün içinde imha ve tutanak. Hukuki muhafaza veya saklama yükümlülüğü olan kayıtlar gerekçesi/süresiyle kısıtlı tutulur; anahtar imhası bu kayıtların yükümlülüğü bitince yapılır. İstisnalar kurum ve hukukça kayıt altına alınır.
- Veli ayrılmadan önce çocuğunun fotoğraflarını ve günlük raporlarını toplu indirebilir (KVKK-06).

### 6.8 Veri ihlali müdahalesi

1. **Tespit ve sınırlama:** Olağandışı toplu erişim, kurumlar arası sorgu denemesi, toplu medya indirme gibi durumlarda otomatik alarm; etkilenen erişim anahtarları ve oturumlar iptal edilir.
2. **Değerlendirme:** Etkilenen kurumlar, kişiler ve veri kategorileri; özel nitelikli veri içerip içermediği.
3. **Kuruma bildirim:** Platform, veri işleyen olarak etkilenen kurumlara gecikmeksizin, en geç 24 saat içinde bildirir; Kurul bildirim formu taslağını ve etkilenen kişi listesini hazırlar.
4. **Kurul'a bildirim:** Kurum (veri sorumlusu) Kurul'a 72 saat içinde bildirir; gecikmede gerekçe eklenir.
5. **İlgili kişilere bildirim:** Velilere en kısa makul sürede uygulama içi bildirim ve SMS ile.
6. **Kapanış:** Kök neden analizi, düzeltici önlemler, ihlal kayıtlarının güncellenmesi.

Plan yayından önce yazılı olmalı, sorumlular atanmış ve yılda en az bir tatbikat yapılmış olmalı.

### 6.9 Mağaza ve şeffaflık

- **Mağaza:** Uygulama veli ve öğretmenler içindir; App Store "Kids" kategorisine konmamalı, gizlilik etiketleri eksiksiz doldurulmalı.
- **[KVKK]** Uygulamada reklam, reklam kimliği ve üçüncü taraf analitik/izleme SDK'sı bulunmaz (KVKK-14); her yeni SDK yayından önce veri akışı açısından incelenir.
- **[KVKK]** Aydınlatma metinleri uygulama içinde ve web'de erişilebilir; sürüm geçmişi tutulur.

## 7. Kavramsal veri modeli

Modelin merkezinde çocuk var; günlük kayıt, medya ve teslim talebi çocuğa, çocuk sınıfa, sınıf kuruma bağlanır. Her tablo kurum kimliğini taşır; kiracı izolasyonu buna dayanır.

&#91;embedded content: kavramsal veri modeli · 10 ana varlık\]

Günlük kayıt tek bir tablo + `tip` alanı (aktivite, öğün, uyku, tuvalet, ilaç, olay) ve tipe özel JSON detay olarak tasarlanırsa yeni kayıt türleri şema değişikliği olmadan eklenir. **[KVKK]** Sağlık profili (alerji, kronik durum, ilaç talimatı, diyet kısıtı) çocuk tablosundan ayrı bir tabloda ve alan düzeyinde şifreli tutulur; günlük kayıt JSON detayına serbest metin sağlık bilgisi yazılmaz, ilaç ve olay kayıtlarının detayı ayrıca şifrelenir. Her kayıt, saklama süresinin hesaplanabilmesi için veri kategorisini taşır.

| Diğer varlık | Bağlı olduğu | Not |
| --- | --- | --- |
| Menü | Kurum | Haftalık öğünler ve içerik/alerjen listesi |
| Yoklama | Çocuk + gün | Giriş-çıkış saati, gelmeme sebebi |
| Rıza | Çocuk + veli | **[KVKK]** Türü (`SAGLIK`, `FOTO_VELI`, `FOTO_GRUP`, `FOTO_TANITIM`, `KONUM`, `AKIS_YETKILI`), durum, metin sürümü, tarih, kanal/cihaz; geri alma yeni kayıt olarak eklenir |
| **[KVKK]** Aydınlatma metni | Kurum | Sürüm, yürürlük tarihi, içerik özeti (hash); veli okuma kayıtları |
| **[KVKK]** Erişim kısıtlaması | Veli + çocuk | Kapsam (kapalı / salt-okunur; kısıtlama yoksa tam), dayanak belge, başlangıç-bitiş, işlemi yapan |
| **[KVKK]** Saklama politikası | Kurum + veri kategorisi | Süre, başlangıç olayı (ör. ayrılış, eğitim yılı sonu), süre sonu işlemi (silme / anonimleştirme) |
| **[KVKK]** İmha kaydı | Kurum | Çalışma zamanı, kategori, kayıt sayısı, yöntem; 3 yıl saklanır |
| **[KVKK]** KVKK başvurusu | Kullanıcı + kurum | Tür, açılış tarihi, 15 günlük operasyon hedefi, 30 günlük yasal son tarih, durum, yanıt ve gerekçe |
| Bildirim | Kullanıcı | Kanal, okunma durumu |
| Denetim kaydı | Her varlık | Kim, ne zaman, hangi işlem; değiştirilemez |

## 8. Teknik mimari önerisi

Başlangıç için önerim: React Native ile tek mobil uygulama ve Rust ile modüler monolit backend. Mikroservise ancak yük veya ekip büyüklüğü gerçekten gerektirdiğinde, modül sınırlarından bölünerek geçilir. Backend'in ayrıntılı ihtiyaç analizi, API sözleşmesi ve mobil istemciyle bağlantısı: [`docs/backend-requirements-analysis.md`](docs/backend-requirements-analysis.md).

Backend'in 15 bağımlı sistemi, ortak sınırları ve birlikte kabul senaryoları: [sistem ihtiyaç analizleri](docs/systems/README.md). REST/JSON mobil ve web sözleşmesidir; Protobuf/gRPC, ileride ayrı servisler için ölçülmüş bir ihtiyaç oluşursa değerlendirilir.

&#91;embedded content: önerilen sistem mimarisi · istemci, backend, veri, asenkron katman\]

Mobil uygulama S3 PUT imzasıyla ingress üzerinden karantinaya yükler; medya işleyici tarama, EXIF silme, rızaya göre bulanıklaştırma ve önizleme üretimini tamamlayıp yayın türevini yazar, ardından orijinali siler. Özel medya GET'i her istekte güncel yetki/rıza kontrol eden uygulama geçidinden sunulur. Teslim ve mesaj olayları iş kaydıyla aynı PostgreSQL transaction'ında outbox'a yazılır; worker Redis Pub/Sub ve WebSocket ile dağıtır. Geçici olay kaybı aktif teslimde en çok 5 sn, diğer açık ekranlarda en çok 60 sn REST uzlaştırmasıyla yakalanır.

| Katman | Öneri | Gerekçe |
| --- | --- | --- |
| Mobil | React Native 0.88 (Expo SDK 58), TypeScript | Tek kod tabanı; web paneliyle React bilgisi ve bileşen paylaşımı. Alternatif: Flutter |
| Çevrimdışı | Cihazda yerel veritabanı + giden işlem kuyruğu. **[KVKK]** Yerel veritabanı şifreli, oturum kapanınca silinir | Öğretmen girişleri bağlantı kopsa da kaybolmaz |
| Backend | Rust (2024 edition), axum + tokio, sqlx; Cargo workspace ile modüler monolit (`okuldayim-api` + `okuldayim-worker`) | Bellek güvenliği, düşük gecikme ve kaynak kullanımı; modül sınırları crate'lerle denetlenir, ileride servis olarak ayrılabilir |
| Veritabanı | PostgreSQL, satır düzeyi güvenlik. **[KVKK]** Uygulama, Vault'un ürettiği kısa ömürlü ve `BYPASSRLS` yetkisi olmayan rolle bağlanır | İlişkisel model + kiracı izolasyonu; günlük kayıt detayı için JSONB |
| **[KVKK]** Anahtar ve sır yönetimi | HashiCorp Vault: Transit (alan şifreleme, HMAC), KV v2, Database secrets engine, denetim logu; üretimde HA + HSM/KMS ile otomatik unseal | Anahtarların veriden ayrı tutulması, kısa ömürlü kimlik bilgileri, kriptografik imha |
| Gerçek zamanlı | PostgreSQL transactional outbox + WebSocket (JSON kimlik/durum olayları) + Redis Pub/Sub; dönemsel REST uzlaştırması | Teslim kuyruğu ve mesajlar için düşük gecikme ve güvenilir yeniden deneme |
| Medya | Türkiye'de özel S3 bucket'ları ve yetki denetimli GET geçidi. CDN kullanılacaksa yurt içi olmalı ve özel medya önbelleğe alınmamalı | İmzalı PUT; erişim iptalini uygulayan GET |
| Bildirim | Android FCM HTTP v1, iOS doğrudan APNs HTTP/2, yerel SMS sağlayıcı. **[KVKK]** Çocuk/sağlık/mesaj içeriği yok; push metaverisi için ayrı aktarım değerlendirmesi | Native token'ı doğru sağlayıcıya yönlendirme; OTP ve kritik bildirim yedekleme |
| Kimlik | SMS OTP + JWT (kısa ömürlü erişim, dönen yenileme anahtarı) | Şifresiz, velilere uygun giriş |
| İzleme | **[KVKK]** Yurt içinde barındırılan Sentry, OpenTelemetry; kişisel veri maskeleme | Mobil çökme ve backend iz takibi; Crashlytics ve SaaS çözümler yurt dışı aktarım doğurur |
| CI/CD | GitHub Actions, EAS Build / Fastlane, uzaktan özellik bayrağı | Mağaza sürecinden bağımsız açma-kapama |
| Barındırma | Türkiye'de veri merkezi olan bir bulut | KVKK yurt dışı aktarım yükünü azaltır |
| **[KVKK]** Yerel geliştirme | `infra/docker-compose.yml`: PostgreSQL, Redis, Vault, S3 uyumlu depolama, e-posta yakalayıcı | Üretim verisi olmadan uçtan uca geliştirme |

## 9. MVP kapsamı ve yol haritası

MVP, kullanıcının istediği çekirdek dörtlüyü kapsar: günlük akış ve öğün takibi, duyuru ve fotoğraf, teslim alma ve mesajlaşma. Süreler 2 mobil, 2 backend geliştirici ve 1 tasarımcılık bir ekip varsayımıyla kaba tahmindir. **[KVKK]** KVKK modülünün M öncelikli maddeleri (aydınlatma ve rıza akışı, sunucu tarafı rıza kontrolleri, başvuru kutusu, saklama motoru, push gizliliği) MVP'ye dahildir.

&#91;embedded content: yol haritası · 4 faz, 3 kapı\]

Pilot kapısı geçilmeden ödeme ve konum gibi riskli özelliklere başlanmaz; önce öğretmenlerin uygulamayı gerçekten her gün kullandığı kanıtlanır.

**[KVKK] Pilot öncesi KVKK kapısı** (gerçek çocuk verisi işlenmeden önce tamamlanmalı):

- [ ] Pilot kurumlarla veri işleme sözleşmesi imzalandı; alt veri işleyen listesi paylaşıldı
- [ ] Kurumun aydınlatma ve açık rıza metinleri hukukçu tarafından onaylandı
- [ ] Kurumun VERBİS kaydı veya muafiyet değerlendirmesi tamamlandı
- [ ] Saklama ve imha politikası yayımlandı; imha işi test ortamında doğrulandı
- [ ] Push için yurt dışı aktarım mekanizması netleşti (gerekiyorsa standart sözleşme bildirildi)
- [ ] Bağımsız sızma testi yapıldı, kritik ve yüksek bulgular kapatıldı
- [ ] İhlal müdahale planı yazılı, sorumlular atandı
- [ ] Öğretmenlere KVKK eğitimi verildi, gizlilik taahhütleri alındı
- [ ] Sistemler arası kabul matrisi ([C01–C13](docs/systems/README.md#4-sistemler-arası-kabul-matrisi)), tam yedekten kurtarma tatbikatı ve alarm/nöbetçi kanıtı tamamlandı; SMS ve e-posta sağlayıcıları kabul edildi

**Pilot başarı ölçütleri** (önerilen hedefler, pilot kurumlarla birlikte kesinleştirilmeli):

| Ölçüt | Önerilen hedef |
| --- | --- |
| Günlük kayıt giren öğretmen oranı (okul günlerinde) | ≥ %80 |
| Günlük raporu açan veli oranı | ≥ %70 |
| Uygulama üzerinden yapılan teslim bildirimi oranı | ≥ %50 |
| Duyuru okunma oranı (48 saat içinde) | ≥ %85 |
| Çökmesiz oturum oranı | ≥ %99,5 |

**İş modeli notu.** Bu alanda yaygın model kurumdan çocuk başına aylık abonelik alıp uygulamayı veliye ücretsiz sunmaktır. Gelişim raporları, aidat tahsilatı ve çoklu şube yönetimi üst paket özelliği olabilir.

## 10. Riskler, varsayımlar ve açık sorular

En büyük risk teknik değil, benimseme: öğretmen veri girişini yük olarak görürse veliler boş bir akışla karşılaşır ve uygulama terk edilir.

| Risk | Etki | Önlem |
| --- | --- | --- |
| Öğretmenlerin veri girişini aksatması | Yüksek | Toplu giriş, şablonlar, tablet modu; yöneticiye "kayıt doldurma oranı" göstergesi |
| Velilerin WhatsApp gruplarından vazgeçmemesi | Yüksek | Kurumun resmi kanal olarak duyurması; fotoğrafın yalnızca uygulamada paylaşılması |
| Fotoğraf sızıntısı veya yanlış çocuğa gönderim | Çok yüksek | Etiket zorunluluğu, izin kontrolü, isteğe bağlı yönetici onayı, denetim kaydı |
| Yanlış kişiye teslim | Çok yüksek | PIN/QR doğrulama, fotoğraflı yetkili listesi, yetkisiz kişi alarmı |
| Öğretmene mesai dışı mesaj yükü | Orta | Sessiz saatler, ortak sınıf gelen kutusu, otomatik yanıt |
| Medya depolama maliyetinin hızlı büyümesi | Orta | Cihazda sıkıştırma, video süre sınırı, eski medyanın soğuk depolamaya taşınması |
| KVKK uyumsuzluğu | Çok yüksek | Baştan gizlilik odaklı tasarım, hukuk incelemesi, Türkiye'de barındırma |
| **[KVKK]** Push, analitik veya hata izleme SDK'ları üzerinden fark edilmeden yurt dışı aktarım | Yüksek | Push metaverisi dahil aktarım envanteri, asgari içerik, yurt içinde izleme, SDK beyaz listesi, yayın öncesi ağ trafiği incelemesi |
| **[KVKK]** Öğretmenin kişisel telefonunda çocuk fotoğrafı kalması | Yüksek | Uygulama içi kamera, galeriye kayıt yok, yükleme sonrası yerel silme, uzaktan oturum kapatma |
| **[KVKK]** Ayrı yaşayan ebeveynler arasında rıza ve erişim çatışması | Orta | En kısıtlayıcı rıza kuralı, erişim kısıtlaması, velilerin birbirinin iletişim bilgisini görmemesi |
| **[KVKK]** Saklama süresi tanımlanmadığı için verinin süresiz birikmesi | Yüksek | Saklama motoru, varsayılan süreler, günlük imha işi ve imha tutanakları |

**Varsayımlar**

- Kurumlarda öğretmen başına en az bir akıllı telefon veya sınıf başına bir tablet ve Wi-Fi var.
- Satın alma kararını kurum verir; veli uygulamayı ücretsiz kullanır.
- İlk pazar Türkiye; ilk sürüm Türkçe ve İngilizce.

**Netleştirilmesi gereken sorular**

- [ ] İlk hedef segment hangisi: kreş/anaokulu mu, ilkokul mu? (MVP önceliklerini doğrudan değiştirir)
- [ ] Gelir modeli: kurum başına mı, çocuk başına aylık abonelik mi? Freemium olacak mı?
- [ ] Veli ve öğretmen aynı uygulamada rol bazlı mı olacak, yoksa iki ayrı uygulama mı?
- [ ] Fotoğraflar veli tarafından indirilebilecek mi, yoksa sadece uygulama içinde mi görülecek?
- [ ] Veli–öğretmen mesajlaşması her zaman açık mı, yoksa kurum isterse kapatabilecek mi?
- [ ] Pilot için anlaşılabilecek 2–3 kurum var mı?
- [ ] Teslim talebinin iptali, gün sonunda kendiliğinden kapanması ve engellenen teslimi kimin kapatacağı pilot kurumlarla onaylanmalı (backend önerisi: veli iptal edebilir, açık talep gün sonunda kapanır, engeli yalnız kurum yöneticisi gerekçeyle kapatır)
- [ ] Barındırma: Türkiye'de bir veri merkezi mi, yurt dışı bölge + hukuki aktarım mekanizması mı?
- [ ] **[KVKK]** Kamu okulları hedefleniyor mu? (2019/12 sayılı Genelge ve MEB süreçleri ek yükümlülük getirir)
- [ ] **[KVKK]** Saklama süreleri, özellikle ilaç ve kaza kayıtları için, hukukçuyla kesinleştirilmeli
- [ ] **[KVKK]** Push için FCM/APNs'e aktarım: standart sözleşme mi, yoksa yalnızca uygulama açıkken canlı bağlantı + SMS gibi bir alternatif mi?
- [ ] **[KVKK]** Platformun veri sorumlusu sıfatıyla yaptığı işlemeler için VERBİS kaydı gerekiyor mu?
- [ ] **[KVKK]** Sağlık verisi için açık rıza dışında uygulanabilecek bir m.6 şartı var mı (2024 değişikliği)?

## Ek A: KVKK revizyon özeti

| Alan | Değişiklik | Dayanak |
| --- | --- | --- |
| Rıza modeli | Tek fotoğraf izni yerine 6 türlü, çocuk bazında, varsayılan kapalı ve sürümlü açık rıza; iki velide en kısıtlayıcı rıza kuralı; aydınlatma ile rızanın ayrılması (KVKK-01…04, 6.4) | m.3 (açık rıza tanımı), m.5/1, m.10; Aydınlatma Tebliği |
| Özel nitelikli veri | Dini inancı açığa çıkarabilen diyet bilgisi özel nitelikli sayıldı; sağlık verisi ayrı tablo ve alan şifrelemesiyle; 2024 m.6 değişikliği notu (ORG-03, 6.3) | m.6; Kurul 2018/10 |
| Üçüncü kişiler | Yetkili teslim alan ve acil durum kişilerine aydınlatma; fotoğrafı kişinin kendisinin yüklemesi (KVKK-07, PICK-05) | m.10 |
| İlgili kişi hakları | Uygulama içi başvuru, 30 günlük süre takibi, makine tarafından okunabilir veri dökümü (KVKK-05, KVKK-06) | m.11, m.13; Başvuru Tebliği |
| Saklama ve imha | Süresiz mesaj arşivi kaldırıldı; kategori bazlı süreler, günlük imha işi, imha tutanağı, yedek rotasyonu, kriptografik imha (KVKK-08, 6.7) | m.4/2-d, m.7; Silme/İmha Yönetmeliği |
| Yurt dışı aktarım | Alt veri işleyen tablosu; push'ta asgari içerik ve takma kimlikli metaverinin aktarım değerlendirmesi; yurt içi hata izleme; otomatik çeviri koşulu (KVKK-13, 6.6, MSG-08) | m.9 (2024); Yurt Dışı Aktarım Yönetmeliği |
| Roller ve sözleşmeler | Platformun veri sorumlusu olduğu işlemeler; veri işleme sözleşmesinin asgari içeriği; platform personeline varsayılan erişim yok (KVKK-11, 6.2) | m.12/2 |
| Teknik önlemler | Vault (Transit, dinamik DB kimlikleri, denetim logu), `FORCE` RLS, körleştirilmiş indeks, karantina medya akışı, cihaz verisinin uzaktan silinmesi (6.5, AUTH-07) | m.12; Kişisel Veri Güvenliği Rehberi |
| Veri minimizasyonu | TC kimlik no toplanmaz; gelmeme sebebinde tanı istenmez; konum cihazda işlenir; yetkili teslim alan sağlık ve fotoğraf göremez (ATT-02, PICK-09, yetki matrisi) | m.4/2-ç |
| Personel ve cihaz | Uygulama içi kamera ile galeriye kayıt yok; eğitim ve gizlilik taahhüdü; çalışan aydınlatması (MED-01, 6.2) | m.12; Kurul 2018/10 |
| Erişim kısıtlaması | Velayet kararında dayanak belge, velilerin birbirinin iletişim bilgisini görmemesi (KVKK-09) | m.4/2-a, m.12 |
| İhlal yönetimi | Veri işleyenden kuruma 24 saat, Kurul'a 72 saat, ilgili kişilere bildirim, yıllık tatbikat (6.8) | m.12/5; Kurul 2019/10 |
| Yönetişim | Veri envanteri taslağı, pilot öncesi KVKK kapısı, yeni riskler ve açık sorular (6.3, 9, 10) | — |

## Ek B: Birleşik teknik rafinasyon

28 Ekim 2026'da ürün, backend ve 15 sistem analizi birlikte hizalandı. Güncel yetki/rızanın PostgreSQL'den doğrulanması, dayanıklı outbox ve REST uzlaştırması, medya GET geçidi, erişim iptalinin cihaz sınırları, push metaverisinin aktarımı, 15/30 takvim günü ayrımı, operasyon loglarının saklama süresi ve PG/Vault/S3 birlikte kurtarma kuralları netleştirildi. Sözleşme bitimi imhasında hukuki muhafaza ve eski anahtar yedekleri hesaba katıldı; altı rol sayımı düzeltildi. Ürün öncelikleri korunur; [ortak kabul matrisi](docs/systems/README.md#4-sistemler-arası-kabul-matrisi) uygulama aşamasında kanıtlanmalıdır.

**Sürüm 1.3.** Backend 1.2 ve sistem analizleri 1.1 ile ikinci birlikte okuma yapıldı. 6.7'deki yetkili kişi süresi PICK-05 ile eşitlendi: fotoğraf ve telefon yetki bitimi + 30 günde silinir; teslim alanın adı ve yakınlığı teslim kaydında kalır. Erişim kısıtlaması kapsamına "kapalı" ve KVKK başvurusuna 15 günlük hedef eklendi. Pilot kapısına C01–C13, kurtarma tatbikatı ve SMS/e-posta sağlayıcı kabulü kondu; teslim iptali, gün sonu kapanışı ve engel çözümü açık soru olarak eklendi. Backend tarafında MoSCoW korunarak C öncelikli NOT-03 ve PICK-09 pilot sonrasına alındı; ürün öncelikleri değişmedi.
