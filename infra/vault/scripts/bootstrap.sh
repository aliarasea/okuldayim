#!/bin/sh
# Yerel geliştirme Vault kurulumu: ilk başlatma, mühür açma ve yapılandırma. İdempotenttir.
#
# UYARI: Unseal anahtarı ve root token infra/.local/vault altına düz metin yazılır.
# Bu yalnızca geliştirme ortamı içindir; üretimde HSM/KMS ile otomatik unseal kullanılır ve
# root token ilk kurulumdan sonra iptal edilir.
set -eu

export VAULT_ADDR="${VAULT_ADDR:-http://vault:8200}"
OUT=/vault/bootstrap
APPROLE_OUT=/vault/approle
mkdir -p "$OUT"
mkdir -p "$APPROLE_OUT"
# macOS bind mount'larında chmod desteklenmeyebilir; dizin host'ta .gitignore kapsamındadır
chmod 700 "$OUT" 2>/dev/null || true

log() { echo "[vault-bootstrap] $*"; }

# vault status çıkış kodu: 0 = açık, 2 = mühürlü, 1 = hata/ulaşılamıyor
i=0
while :; do
  rc=0
  vault status >/dev/null 2>&1 || rc=$?
  [ "$rc" -ne 1 ] && break
  i=$((i + 1))
  if [ "$i" -ge 60 ]; then
    log "Vault'a ulaşılamadı: $VAULT_ADDR"
    exit 1
  fi
  sleep 1
done

if vault status -format=json 2>/dev/null | grep -Eq '"initialized": *false'; then
  log "Vault ilk kez başlatılıyor (tek anahtar paylaşımı — yalnızca geliştirme)"
  umask 077
  vault operator init -key-shares=1 -key-threshold=1 -format=json > "$OUT/init.json"
  awk '/"unseal_keys_b64"/ { getline; gsub(/[ ",]/, ""); print; exit }' "$OUT/init.json" > "$OUT/unseal.key"
  awk -F'"' '/"root_token"/ { print $4; exit }' "$OUT/init.json" > "$OUT/root.token"
  rm -f "$OUT/init.json"
  if [ ! -s "$OUT/unseal.key" ] || [ ! -s "$OUT/root.token" ]; then
    log "init çıktısı ayrıştırılamadı"
    exit 1
  fi
fi

if [ ! -s "$OUT/unseal.key" ] || [ ! -s "$OUT/root.token" ]; then
  log "Vault başlatılmış ama $OUT altında anahtar yok. Volume'ları sıfırlayın: docker compose down -v"
  exit 1
fi

if vault status -format=json 2>/dev/null | grep -Eq '"sealed": *true'; then
  log "Mühür açılıyor"
  vault operator unseal "$(cat "$OUT/unseal.key")" >/dev/null
fi

# Raft lider seçimini bekle
i=0
until vault read -format=json sys/leader 2>/dev/null | grep -Eq '"is_self": *true'; do
  i=$((i + 1))
  if [ "$i" -ge 30 ]; then
    log "Vault aktif düğüm olmadı"
    exit 1
  fi
  sleep 1
done

VAULT_TOKEN="$(cat "$OUT/root.token")"
export VAULT_TOKEN

# --- Denetim logu (KVKK m.12: anahtar ve sır erişimlerinin kaydı) ---
if ! vault audit list 2>/dev/null | grep -q '^file/'; then
  vault audit enable file file_path=/vault/logs/audit.log >/dev/null \
    || log "UYARI: denetim logu etkinleştirilemedi; sunucu yapılandırmasında audit tanımlayın"
fi

# --- Gizli motorlar ---
enable_engine() {
  path="$1"
  shift
  vault secrets list | grep -q "^$path/" || vault secrets enable -path="$path" "$@" >/dev/null
}
enable_engine transit transit
enable_engine secret -version=2 kv
enable_engine database database

# --- Transit anahtarları ---
# Kurum anahtarları (kurum-<uuid>) backend tarafından kurum açılışında oluşturulur.
ensure_key() {
  name="$1"
  shift
  vault read "transit/keys/$name" >/dev/null 2>&1 || vault write "transit/keys/$name" "$@" >/dev/null
}
ensure_key blind-index type=aes256-gcm96
ensure_key kullanici-pii type=aes256-gcm96
ensure_key jwt-signing type=ecdsa-p256 auto_rotate_period=2160h
ensure_key denetim-checkpoint type=ecdsa-p256

# --- Backend sırları ---
if vault kv get -mount=secret okuldayim/backend >/dev/null 2>&1; then
  vault kv patch -mount=secret okuldayim/backend \
    redis_password="$REDIS_APP_PASSWORD" \
    s3_access_key="$S3_ACCESS_KEY" \
    s3_secret_key="$S3_SECRET_KEY" >/dev/null
else
  vault kv put -mount=secret okuldayim/backend \
    otp_pepper="$(vault write -f -field=random_bytes sys/tools/random/32)" \
    redis_password="$REDIS_APP_PASSWORD" \
    s3_access_key="$S3_ACCESS_KEY" \
    s3_secret_key="$S3_SECRET_KEY" >/dev/null
fi

# Jasmin is optional and requires a separately provisioned gateway account.
# Credentials are stored only in Vault; the API receives them through the KV read.
if [ -n "${JASMIN_USERNAME:-}" ] || [ -n "${JASMIN_PASSWORD:-}" ]; then
  if [ -z "${JASMIN_USERNAME:-}" ] || [ -z "${JASMIN_PASSWORD:-}" ]; then
    log "Jasmin username and password must be configured together"
    exit 1
  fi
  vault kv patch -mount=secret okuldayim/backend \
    jasmin_username="$JASMIN_USERNAME" \
    jasmin_password="$JASMIN_PASSWORD" >/dev/null
fi
unset JASMIN_USERNAME JASMIN_PASSWORD

# Süreç boyunca sabit kalması gereken sırlar yalnızca ilk eksiklikte üretilir.
# KV v2 sürümlemesi eski medya bağlantısı imzalarının en çok 5 dk doğrulanmasını sağlar.
ensure_backend_secret() {
  field="$1"
  if ! vault kv get -field="$field" -mount=secret okuldayim/backend >/dev/null 2>&1; then
    value="$(vault write -f -field=random_bytes sys/tools/random/32)"
    vault kv patch -mount=secret okuldayim/backend "$field=$value" >/dev/null
    unset value
  fi
}
ensure_backend_secret otp_pepper
ensure_backend_secret pickup_pepper
ensure_backend_secret media_link_key

# --- Dinamik PostgreSQL kimlik bilgileri ---
vault write database/config/okuldayim \
  plugin_name=postgresql-database-plugin \
  connection_url="postgresql://{{username}}:{{password}}@${POSTGRES_HOST}:5432/${POSTGRES_DB}?sslmode=disable" \
  allowed_roles=okuldayim-app \
  username=vault_admin \
  password="$VAULT_DB_ADMIN_PASSWORD" \
  password_authentication=scram-sha-256 >/dev/null

vault write database/roles/okuldayim-app \
  db_name=okuldayim \
  creation_statements="CREATE ROLE \"{{name}}\" WITH LOGIN PASSWORD '{{password}}' VALID UNTIL '{{expiration}}' IN ROLE okuldayim_app;" \
  revocation_statements="REVOKE okuldayim_app FROM \"{{name}}\"; DROP ROLE IF EXISTS \"{{name}}\";" \
  default_ttl=1h \
  max_ttl=24h >/dev/null

# Duman testi: kimlik üret ve hemen iptal et
lease_id="$(vault read -format=json database/creds/okuldayim-app | awk -F'"' '/"lease_id"/ { print $4; exit }')"
if [ -z "$lease_id" ]; then
  log "Dinamik veritabanı kimliği üretilemedi"
  exit 1
fi
vault lease revoke "$lease_id" >/dev/null

# --- Politikalar ve backend AppRole ---
vault policy write okuldayim-backend /policies/okuldayim-backend.hcl >/dev/null
vault policy write kvkk-ops /policies/kvkk-ops.hcl >/dev/null

vault auth list | grep -q '^approle/' || vault auth enable approle >/dev/null
vault write auth/approle/role/okuldayim-backend \
  token_policies=okuldayim-backend \
  token_ttl=1h \
  token_max_ttl=4h \
  secret_id_ttl=720h >/dev/null

role_id="$(vault read -field=role_id auth/approle/role/okuldayim-backend/role-id)"
secret_id="$(vault write -f -field=secret_id auth/approle/role/okuldayim-backend/secret-id)"
umask 077
printf 'VAULT_ADDR=http://127.0.0.1:8200\nVAULT_ROLE_ID=%s\nVAULT_SECRET_ID=%s\n' "$role_id" "$secret_id" \
  > "$OUT/backend-approle.env"
printf 'VAULT_ADDR=http://127.0.0.1:8200\nVAULT_ROLE_ID=%s\nVAULT_SECRET_ID=%s\n' "$role_id" "$secret_id" \
  > "$APPROLE_OUT/backend-approle.env.tmp"
chmod 440 "$APPROLE_OUT/backend-approle.env.tmp"
mv -f "$APPROLE_OUT/backend-approle.env.tmp" "$APPROLE_OUT/backend-approle.env"

log "Hazır. Backend AppRole bilgileri: infra/.local/vault/backend-approle.env"
