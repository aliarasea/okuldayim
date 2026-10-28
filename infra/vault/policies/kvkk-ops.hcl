# KVKK operasyon politikası: kriptografik imha (sözleşmesi biten kurumun anahtarının silinmesi).
#
# Bu politikaya sahip token yalnızca iki kişi onayıyla, kısa süreli (ör. 15 dk) üretilir ve her kullanım
# Vault denetim loguna düşer. Akış:
#   vault write transit/keys/kurum-<uuid>/config deletion_allowed=true
#   vault delete transit/keys/kurum-<uuid>
# Silinen anahtarla şifrelenmiş veriler (yedekler dahil) bir daha çözülemez.

path "transit/keys/*" {
  capabilities = ["read", "update", "delete"]
}

path "transit/keys" {
  capabilities = ["list"]
}
