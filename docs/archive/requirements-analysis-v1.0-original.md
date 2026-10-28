# Okul–Veli İletişim Uygulaması: Gereksinim Analizi

Oct 3, 2026 · @Ali Aras

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
| Yoklama, etkinlik takvimi, push bildirim | Yapay zekâ ile fotoğrafta çocuk tanıma |

Brightwheel, Lillio (eski HiMama) ve Famly bu alanın yurtdışındaki referans ürünleri; özellik setini kıyaslamak için incelenebilir.

## 2. Paydaşlar ve kullanıcı rolleri

Sistemde beş rol var; veri erişimi her zaman kurum → sınıf → çocuk hiyerarşisiyle sınırlanır. Bir kullanıcı birden fazla rol taşıyabilir (ör. hem öğretmen hem başka sınıfta velisi olan personel) ve birden fazla kuruma bağlı olabilir (kardeşler farklı okullarda).

| Rol | Kim | Temel ihtiyaç | Kanal |
| --- | --- | --- | --- |
| Kurum yöneticisi | Müdür, müdür yardımcısı, idari personel | Sınıf/öğretmen/çocuk kaydı, duyuru, onay ve raporlar | Web panel + mobil |
| Öğretmen | Sınıf öğretmeni, yardımcı öğretmen, branş öğretmeni | Hızlı toplu kayıt, fotoğraf, mesaj, teslim bildirimi alma | Mobil (çoğunlukla tablet) |
| Veli | Anne, baba, yasal vasi | Çocuğun gününü izleme, mesaj, teslim alma bildirimi | Mobil |
| Yetkili teslim alan | Büyükanne/büyükbaba, bakıcı, aile dostu | Sadece teslim alma; isteğe bağlı salt-okunur akış | Mobil veya uygulamasız (QR/PIN) |
| Platform yöneticisi | Ürün ekibi | Kurum açma, abonelik, destek, denetim | Web admin |

**Yetki matrisi** (✓ tam yetki, S = sadece kendi sınıfı/çocuğu, O = okuma, — yetki yok):

| İşlem | Kurum yön. | Öğretmen | Veli | Yetkili teslim alan |
| --- | --- | --- | --- | --- |
| Çocuk/sınıf kaydı oluşturma | ✓ | — | — | — |
| Günlük aktivite ve öğün girme | ✓ | S | — | — |
| Günlük akışı görme | ✓ | S | S | O (veli izin verirse) |
| Fotoğraf/video yükleme | ✓ | S | — | — |
| Kurum geneli duyuru | ✓ | — | — | — |
| Sınıf duyurusu | ✓ | S | — | — |
| Mesajlaşma | ✓ | S | S | — |
| Teslim alma bildirimi gönderme | — | — | S | S |
| Teslimi onaylama | ✓ | S | — | — |
| Yetkili kişi ekleme/silme | ✓ (onay) | — | S | — |
| Sağlık bilgisi (alerji, ilaç) düzenleme | ✓ | O | S | — |
| Raporlar ve dışa aktarım | ✓ | S | Kendi çocuğu | — |

İki velili ailelerde, mahkeme kararıyla velayeti kısıtlanan ebeveynin erişimi kurum yöneticisi tarafından kapatılabilmeli; bu, gerçek hayatta sık karşılaşılan bir gereksinim.

## 3. Fonksiyonel gereksinimler

Gereksinimler 12 modüle ayrıldı ve MoSCoW ile önceliklendirildi: **M** = olmazsa olmaz (MVP), **S** = olmalı, **C** = olabilir. MVP'nin kalbi günlük akış, öğün, duyuru, fotoğraf, teslim alma ve mesajlaşma.

### 3.1 Kimlik ve hesap (AUTH)

| Kod | Gereksinim | Öncelik |
| --- | --- | --- |
| AUTH-01 | Telefon numarası + SMS OTP ile giriş; e-posta alternatif | M |
| AUTH-02 | Davet tabanlı kayıt: veliyi kurum davet eder (SMS/bağlantı); dışarıdan serbest kayıt yok | M |
| AUTH-03 | Bir veli birden çok çocuğa ve kuruma bağlanabilir; çocuklar arası hızlı geçiş | M |
| AUTH-04 | Hesap silme uygulama içinden yapılabilir (App Store / Google Play zorunluluğu) | M |
| AUTH-05 | Face ID / parmak izi ile hızlı giriş | S |
| AUTH-06 | Sınıfta ortak tablet modu: öğretmenler PIN ile hızlı kullanıcı değiştirir | S |

### 3.2 Kurum, sınıf ve çocuk yönetimi (ORG)

| Kod | Gereksinim | Öncelik |
| --- | --- | --- |
| ORG-01 | Kurum profili; zincir kurumlar için şube yapısı | M |
| ORG-02 | Sınıf oluşturma, öğretmen ve çocuk atama, dönem sonunda toplu sınıf geçişi | M |
| ORG-03 | Çocuk profili: ad, doğum tarihi, fotoğraf, alerjiler, kronik durum, düzenli ilaç, diyet (vejetaryen, helal vb.), acil durum kişileri | M |
| ORG-04 | Fotoğraf/medya paylaşım izni çocuk bazında tutulur (veli onayı) | M |
| ORG-05 | Excel/CSV ile toplu çocuk ve veli içe aktarma | S |
| ORG-06 | Eğitim yılı yönetimi ve geçmiş yılların arşivlenmesi | S |

### 3.3 Günlük akış ve aktiviteler (ACT)

| Kod | Gereksinim | Öncelik |
| --- | --- | --- |
| ACT-01 | Aktivite tipleri (oyun, sanat, müzik, açık hava, eğitsel vb.); kurum kendi tiplerini tanımlar | M |
| ACT-02 | Toplu giriş: bir aktivite tek seferde seçili çocuklara veya tüm sınıfa işlenir | M |
| ACT-03 | Çocuğa özel not ve fotoğraf ekleme | M |
| ACT-04 | Gün sonu otomatik günlük rapor ve veliye push bildirimi | M |
| ACT-05 | Yayın modu kurum ayarıdır: anlık veya gün sonu toplu | S |
| ACT-06 | Ruh hali kaydı (mutlu, yorgun, huzursuz…) | S |
| ACT-07 | Gelişim gözlemleri (dil, motor beceri, sosyal) ve dönemsel gelişim raporu | C |

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
| ATT-02 | Veli "bugün gelmeyecek" bildirimi ve sebebi (hastalık, izin) | M |
| ATT-03 | Devamsızlık raporu (çocuk, sınıf, kurum) | S |
| ATT-04 | Kapıda QR veya kiosk tablet ile giriş-çıkış | C |

### 3.7 Teslim alma (PICK)

| Kod | Gereksinim | Öncelik |
| --- | --- | --- |
| PICK-01 | Veli "yola çıktım, \~X dk'ya oradayım" bildirimi gönderir | M |
| PICK-02 | Bildirimde teslim alacak kişi yetkili listeden seçilir | M |
| PICK-03 | Öğretmene push ve uygulamada canlı teslim kuyruğu (geliş sırasına göre) | M |
| PICK-04 | Durumlar: Yolda → Hazırlanıyor → Teslim edildi; her geçişte veliye bildirim | M |
| PICK-05 | Yetkili kişi listesi: fotoğraf, yakınlık, telefon, geçerlilik süresi (tek seferlik yetki dahil) | M |
| PICK-06 | Teslimde doğrulama: tek kullanımlık PIN veya QR; öğretmen ekranında teslim alanın fotoğrafı | M |
| PICK-07 | Listede olmayan kişi gelirse teslim engellenir, veli ve yönetici uyarılır | M |
| PICK-08 | Geç teslim alma kaydı ve raporu | S |
| PICK-09 | Konum tabanlı otomatik "yaklaştı" bildirimi (geofence, veli açık rızasıyla) | C |

### 3.8 Duyurular (ANN)

| Kod | Gereksinim | Öncelik |
| --- | --- | --- |
| ANN-01 | Hedefli duyuru: kurum geneli, sınıf veya seçili veliler | M |
| ANN-02 | Ek desteği: görsel ve PDF | M |
| ANN-03 | Okundu bilgisi ve okunma oranı; okumayanlara hatırlatma | M |
| ANN-04 | Onay gerektiren duyuru (gezi izni vb.): veli "Onaylıyorum / Onaylamıyorum" der, kayıt saklanır | S |
| ANN-05 | Zamanlanmış duyuru | S |
| ANN-06 | Anket | C |
| ANN-07 | Acil duyuru (sessiz modu aşan kritik bildirim; iOS'ta Apple'dan özel izin gerekir) | C |

### 3.9 Fotoğraf ve medya (MED)

| Kod | Gereksinim | Öncelik |
| --- | --- | --- |
| MED-01 | Çoklu fotoğraf ve kısa video yükleme, çocuk etiketleme | M |
| MED-02 | Veli yalnızca kendi çocuğunun etiketli olduğu medyayı görür | M |
| MED-03 | Fotoğraf izni olmayan çocuk etiketlenince öğretmen uyarılır; grup fotoğrafında yayın engellenir veya yüz bulanıklaştırılır | M |
| MED-04 | Cihazda sıkıştırma, küçük önizleme; video süre/boyut sınırı (ör. 60 sn) | M |
| MED-05 | Kurum ayarı: yayın öncesi yönetici onayı | S |
| MED-06 | Çocuk bazlı zaman tüneli ve albüm; indirme izni kurum ayarı | S |
| MED-07 | Yayın sonrası arka planda yükleme; zayıf bağlantıda kuyruk | M |

### 3.10 Mesajlaşma (MSG)

| Kod | Gereksinim | Öncelik |
| --- | --- | --- |
| MSG-01 | Veli ↔ sınıf öğretmenleri: sınıfın tüm öğretmenlerinin gördüğü ortak konuşma | M |
| MSG-02 | Veli ↔ yönetim konuşması | M |
| MSG-03 | Veliler arası mesajlaşma bilinçli olarak yok | M |
| MSG-04 | Metin, fotoğraf, dosya; okundu bilgisi | M |
| MSG-05 | Mesajlar kurum arşivinde saklanır; silme sadece "herkesten gizle" şeklinde, iz kalır | M |
| MSG-06 | Öğretmen mesai saatleri ve sessiz saat; saat dışında otomatik yanıt | S |
| MSG-07 | Sesli mesaj | S |
| MSG-08 | Hazır cevap şablonları; otomatik çeviri (yabancı veliler) | C |

### 3.11 Takvim (CAL)

| Kod | Gereksinim | Öncelik |
| --- | --- | --- |
| CAL-01 | Kurum takvimi: tatiller, etkinlikler, veli toplantıları | M |
| CAL-02 | Etkinliği telefon takvimine ekleme | S |
| CAL-03 | Veli toplantısı için randevu slotu seçimi | C |

### 3.12 Bildirimler ve raporlama (NOT, REP)

| Kod | Gereksinim | Öncelik |
| --- | --- | --- |
| NOT-01 | Push bildirim ve uygulama içi bildirim merkezi | M |
| NOT-02 | Kategori bazlı bildirim tercihi; teslim, kaza ve ilaç bildirimleri kapatılamaz | M |
| NOT-03 | Kritik bildirim okunmazsa belirli süre sonra SMS ile yedekleme | C |
| REP-01 | Yönetici paneli: doluluk, devamsızlık, duyuru okunma oranı, öğretmenlerin kayıt doldurma oranı | S |
| REP-02 | Çocuk bazlı haftalık/aylık rapor (PDF) | S |
| REP-03 | Denetim kayıtlarının yönetici tarafından görüntülenmesi | M |

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

1. Öğretmen fotoğrafları çeker veya galeriden seçer, her fotoğrafta çocukları etiketler.
2. Fotoğraf izni olmayan bir çocuk etiketlendiyse uygulama uyarır; öğretmen fotoğrafı çıkarır veya yüzü bulanıklaştırır.
3. Kurum ayarı açıksa yayından önce yönetici onayı beklenir.
4. Fotoğraf yalnızca etiketli çocukların velilerine görünür ve onlara bildirim gider.

## 5. Fonksiyonel olmayan gereksinimler

En kritik iki kalite özelliği hız (öğretmen bir sınıfa öğün girişini 30 saniyede bitirmeli) ve güvenilirlik (teslim bildirimi asla kaybolmamalı). Hedef değerler aşağıda; ilk pilot sonrası gerçek ölçümlerle güncellenmeli.

| Kategori | Gereksinim | Hedef |
| --- | --- | --- |
| Kullanılabilirlik | Öğretmenin 20 kişilik sınıfa bir öğün girişini tamamlama süresi | ≤ 30 sn |
| Kullanılabilirlik | Sık işlemler ana ekrandan en fazla 2 dokunuş uzakta; tek elle kullanım | 2 dokunuş |
| Performans | API yanıt süresi (p95) | < 300 ms |
| Performans | Uygulama soğuk açılış süresi (orta segment Android) | < 2,5 sn |
| Performans | Teslim bildiriminin öğretmen cihazına ulaşma süresi | < 5 sn |
| Güvenilirlik | Servis erişilebilirliği, okul saatlerinde (07:00–19:00) | ≥ %99,9 |
| Güvenilirlik | Medya yükleme başarısızlığında otomatik yeniden deneme | Kayıp 0 |
| Çevrimdışı | Öğretmen girişleri bağlantı yokken cihazda kuyruklanır, bağlantı gelince senkronize olur | Tüm kayıt tipleri |
| Ölçeklenebilirlik | İlk yıl hedef ölçek; yatay ölçeklenebilir mimari | 500 kurum, 50 bin çocuk, 100 bin veli |
| Ölçeklenebilirlik | Gün içi yoğunluk: sabah girişi ve 16:00–18:00 teslim saatleri | 10× ortalama yük |
| Depolama | Çocuk başı yıllık medya tahmini (sıkıştırılmış) | \~1–2 GB |
| Uyumluluk | Desteklenen işletim sistemleri | iOS 16+, Android 8 (API 26)+ |
| Uyumluluk | Telefon ve tablet yerleşimi (öğretmenler sık tablet kullanır) | Her ikisi |
| Yerelleştirme | Türkçe ve İngilizce; altyapı çoklu dil ve sağdan sola yazıya hazır | TR, EN |
| Erişilebilirlik | Dinamik yazı boyutu, ekran okuyucu etiketleri, yeterli kontrast | WCAG 2.1 AA |
| Bakım | Merkezi hata ve çökme izleme; çökmesiz oturum oranı | ≥ %99,5 |
| Dağıtım | Uygulama mağazası beklemeden yapılandırma ve özellik açma/kapatma | Uzaktan özellik bayrağı |

## 6. Güvenlik, gizlilik ve KVKK

Uygulama çocukların fotoğrafını, sağlık bilgisini (alerji, ilaç) ve kimin teslim alabileceğini tutuyor; bu yüzden güvenlik bir özellik değil, ürünün kendisi. Aşağıdaki maddeler yayın öncesi bir KVKK uzmanı ve hukuk danışmanıyla doğrulanmalı.

### 6.1 Yasal çerçeve ve roller

- **Roller:** Kurum veri sorumlusu, uygulama sağlayıcısı veri işleyendir. Her kurumla bir veri işleme sözleşmesi imzalanır; VERBİS kaydı kurumun sorumluluğundadır.
- **Özel nitelikli veri:** Alerji, kronik hastalık ve ilaç bilgisi KVKK'da özel nitelikli kişisel veridir; ayrı açık rıza ve ek güvenlik önlemi gerektirir.
- **Ayrıştırılmış rızalar:** Aydınlatma metni + ayrı ayrı açık rıza: sağlık verisi, fotoğraf/video paylaşımı, konum (geofence). Rıza uygulama içinden geri alınabilir ve her değişiklik loglanır.
- **Yurt dışına aktarım:** Yurt dışındaki bulut bölgesi KVKK m.9 kapsamında ek yükümlülük getirir. Türkiye'de barındırma tercih edilmeli; edilmezse aktarım mekanizması hukukçuyla netleştirilmeli.
- **İhlal bildirimi:** Veri ihlali Kurul'a 72 saat içinde bildirilir; olay müdahale planı yayından önce yazılı olmalı.
- **Mağaza:** Uygulama veli ve öğretmenler içindir; App Store "Kids" kategorisine konmamalı, gizlilik etiketleri eksiksiz doldurulmalı.

### 6.2 Teknik önlemler

| Alan | Önlem |
| --- | --- |
| Kiracı izolasyonu | Her sorgu kurum kimliğiyle filtrelenir (veritabanında satır düzeyi güvenlik); kurumlar arası veri sızıntısı otomatik testlerle denetlenir |
| Yetkilendirme | Rol + kapsam (kurum, sınıf, çocuk) bazlı erişim kontrolü, sunucu tarafında zorunlu |
| Şifreleme | İletimde TLS 1.2+, depoda AES-256; sağlık alanları ayrıca alan düzeyinde şifrelenir |
| Medya erişimi | Medya hiçbir zaman herkese açık URL ile sunulmaz; kısa ömürlü (ör. 5 dk) imzalı URL. Yüklemede EXIF ve konum bilgisi silinir |
| Kimlik doğrulama | OTP deneme sınırı ve kilitleme; cihaz listesi ve uzaktan oturum kapatma; yönetici paneli için 2FA zorunlu |
| Denetim kaydı | Kim, hangi çocuğun verisini ne zaman gördü/değiştirdi; teslim ve ilaç kayıtları değiştirilemez (yalnızca düzeltme kaydı eklenir) |
| Ekran görüntüsü | Android'de hassas ekranlarda engelleme mümkün; iOS'ta yalnızca tespit edilip loglanabilir. Bu sınır kurumlara açıkça söylenmeli |
| Güvenlik testi | OWASP MASVS'ye göre mobil güvenlik kontrolü ve yayın öncesi bağımsız sızma testi |

### 6.3 Saklama ve silme

- Çocuk kurumdan ayrıldığında veriler kurumun belirlediği süre (ör. 1 yıl) sonra silinir veya anonimleştirilir.
- Veli ayrılmadan önce çocuğunun fotoğraflarını ve günlük raporlarını toplu indirebilir.
- Silinen hesapların yedeklerden de temizlenme süresi tanımlı olmalı.

## 7. Kavramsal veri modeli

Modelin merkezinde çocuk var; günlük kayıt, medya ve teslim talebi çocuğa, çocuk sınıfa, sınıf kuruma bağlanır. Her tablo kurum kimliğini taşır; kiracı izolasyonu buna dayanır.

&#91;embedded content: kavramsal veri modeli · 10 ana varlık\]

Günlük kayıt tek bir tablo + `tip` alanı (aktivite, öğün, uyku, tuvalet, ilaç, olay) ve tipe özel JSON detay olarak tasarlanırsa yeni kayıt türleri şema değişikliği olmadan eklenir.

| Diğer varlık | Bağlı olduğu | Not |
| --- | --- | --- |
| Menü | Kurum | Haftalık öğünler ve içerik/alerjen listesi |
| Yoklama | Çocuk + gün | Giriş-çıkış saati, gelmeme sebebi |
| Rıza | Çocuk + veli | Türü (sağlık, fotoğraf, konum), tarih, geri alma |
| Bildirim | Kullanıcı | Kanal, okunma durumu |
| Denetim kaydı | Her varlık | Kim, ne zaman, hangi işlem; değiştirilemez |

## 8. Teknik mimari önerisi

Başlangıç için önerim: React Native ile tek mobil uygulama ve Spring Boot ile modüler monolit backend. Mikroservise ancak yük veya ekip büyüklüğü gerçekten gerektirdiğinde, modül sınırlarından bölünerek geçilir.

&#91;embedded content: önerilen sistem mimarisi · istemci, backend, veri, asenkron katman\]

Mobil uygulama medyayı imzalı URL ile doğrudan nesne depolamaya yükler; medya işleyici küçük önizlemeyi üretip EXIF'i siler ve sonucu yine depolamaya yazar. Teslim kuyruğu ve mesajlaşma WebSocket üzerinden canlı akar; birden fazla backend örneğinde Redis pub/sub ile dağıtılır.

| Katman | Öneri | Gerekçe |
| --- | --- | --- |
| Mobil | React Native (Expo), TypeScript | Tek kod tabanı; web paneliyle React bilgisi ve bileşen paylaşımı. Alternatif: Flutter |
| Çevrimdışı | Cihazda yerel veritabanı + giden işlem kuyruğu | Öğretmen girişleri bağlantı kopsa da kaybolmaz |
| Backend | Java 21, Spring Boot 3, Spring Modulith | Modül sınırları kodla denetlenir; ileride servis olarak ayrılabilir |
| Veritabanı | PostgreSQL, satır düzeyi güvenlik | İlişkisel model + kiracı izolasyonu; günlük kayıt detayı için JSONB |
| Gerçek zamanlı | WebSocket (STOMP) + Redis pub/sub | Teslim kuyruğu ve mesajlar için düşük gecikme |
| Medya | S3 uyumlu nesne depolama + CDN | Doğrudan yükleme, kısa ömürlü imzalı URL ile erişim |
| Bildirim | FCM (Android + iOS/APNs), yerel SMS sağlayıcı | OTP ve kritik bildirim yedekleme |
| Kimlik | SMS OTP + JWT (kısa ömürlü erişim, dönen yenileme anahtarı) | Şifresiz, velilere uygun giriş |
| İzleme | Crashlytics veya Sentry, OpenTelemetry | Mobil çökme ve backend iz takibi |
| CI/CD | GitHub Actions, EAS Build / Fastlane, uzaktan özellik bayrağı | Mağaza sürecinden bağımsız açma-kapama |
| Barındırma | Türkiye'de veri merkezi olan bir bulut | KVKK yurt dışı aktarım yükünü azaltır |

## 9. MVP kapsamı ve yol haritası

MVP, kullanıcının istediği çekirdek dörtlüyü kapsar: günlük akış ve öğün takibi, duyuru ve fotoğraf, teslim alma ve mesajlaşma. Süreler 2 mobil, 2 backend geliştirici ve 1 tasarımcılık bir ekip varsayımıyla kaba tahmindir.

&#91;embedded content: yol haritası · 4 faz, 3 kapı\]

Pilot kapısı geçilmeden ödeme ve konum gibi riskli özelliklere başlanmaz; önce öğretmenlerin uygulamayı gerçekten her gün kullandığı kanıtlanır.

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
- [ ] Barındırma: Türkiye'de bir veri merkezi mi, yurt dışı bölge + hukuki aktarım mekanizması mı?
