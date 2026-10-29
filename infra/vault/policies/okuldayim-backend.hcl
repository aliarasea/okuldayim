# Okuldayım backend servisinin Vault yetkileri (en az yetki ilkesi).
# Anahtar silme, dışa aktarma ve yapılandırma değiştirme yetkisi YOKTUR.

# Uygulama sırları (salt okunur)
path "secret/data/okuldayim/backend" {
  capabilities = ["read"]
}

# Kısa ömürlü PostgreSQL kimlik bilgisi (RLS'e tabi okuldayim_app grubu)
path "database/creds/okuldayim-app" {
  capabilities = ["read"]
}

# Kurum başına alan şifreleme anahtarı: kurum açılırken oluşturulur
# (ör. vault write transit/keys/kurum-<uuid> auto_rotate_period=8760h).
# Glob, kurum-<uuid>/config, /trim, /import gibi alt yolları da kapsar. İzin verilen parametreler
# yalnızca anahtar tipi ve rotasyon süresi olduğundan deletion_allowed, exportable, eski sürümleri
# silen trim ve dışarıdan anahtar içe aktarma istekleri reddedilir.
path "transit/keys/kurum-*" {
  capabilities = ["create", "update", "read"]
  allowed_parameters = {
    "type"               = ["aes256-gcm96"]
    "auto_rotate_period" = []
  }
}

# Sağlık alanlarının şifrelenmesi / çözülmesi / yeni anahtar sürümüne taşınması
path "transit/encrypt/kurum-*" {
  capabilities = ["update"]
}
path "transit/decrypt/kurum-*" {
  capabilities = ["update"]
}
path "transit/rewrap/kurum-*" {
  capabilities = ["update"]
}

# Kullanıcı iletişim bilgileri, davet hedefleri, push belirteçleri ve TOTP sırları.
path "transit/encrypt/kullanici-pii" {
  capabilities = ["update"]
}
path "transit/decrypt/kullanici-pii" {
  capabilities = ["update"]
}
path "transit/rewrap/kullanici-pii" {
  capabilities = ["update"]
}

# Telefon numarası gibi şifreli alanlarda arama için körleştirilmiş indeks (HMAC)
path "transit/hmac/blind-index" {
  capabilities = ["update"]
}
path "transit/hmac/blind-index/*" {
  capabilities = ["update"]
}

# JWT imzalama: özel anahtar Vault'tan çıkmaz; açık anahtarlar JWKS için okunur
path "transit/sign/jwt-signing" {
  capabilities = ["update"]
}
path "transit/sign/jwt-signing/*" {
  capabilities = ["update"]
}
path "transit/keys/jwt-signing" {
  capabilities = ["read"]
}

# Günlük denetim zinciri checkpoint'i; JWT imza anahtarından ayrıdır.
path "transit/sign/denetim-checkpoint" {
  capabilities = ["update"]
}
path "transit/sign/denetim-checkpoint/*" {
  capabilities = ["update"]
}
path "transit/keys/denetim-checkpoint" {
  capabilities = ["read"]
}

# Token yaşam döngüsü
path "auth/token/renew-self" {
  capabilities = ["update"]
}
path "auth/token/lookup-self" {
  capabilities = ["read"]
}

# Vault Database secrets engine lease'lerini kontrollü biçimde yenileme.
path "sys/leases/renew" {
  capabilities = ["update"]
}
