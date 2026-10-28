# Okuldayım — yerel geliştirme Vault yapılandırması
#
# UYARI: Tek düğüm ve TLS kapalı; port yalnızca 127.0.0.1'e açılır.
# Üretimde: TLS, 3+ düğümlü raft, HSM/KMS ile otomatik unseal ve denetim logunun merkezi
# log sistemine aktarımı zorunludur.

ui            = true
disable_mlock = true

storage "raft" {
  path    = "/vault/file"
  node_id = "vault-dev-1"
}

listener "tcp" {
  address     = "0.0.0.0:8200"
  tls_disable = true
}

api_addr     = "http://vault:8200"
cluster_addr = "https://vault:8201"
