#!/bin/sh
# Veritabanı ilk kez oluşturulurken bir kez çalışır (volume boşken).
#
# Rol modeli (KVKK m.12 — en az yetki, kiracı izolasyonu):
#   okuldayim_owner     Şema ve tabloların sahibi; giriş yapamaz.
#   okuldayim_migrator  Şema migration'larını çalıştırır (okuldayim-migrate, sqlx); bağlantıda
#                       SET ROLE okuldayim_owner ile nesneleri okuldayim_owner adına oluşturur.
#   okuldayim_app       Uygulama grubu; tablo sahibi değil, BYPASSRLS yok — RLS her sorguda uygulanır.
#                       Giriş yapan kullanıcılar Vault tarafından kısa ömürlü üretilir ve bu gruba eklenir.
#   vault_admin         Vault database secrets engine'in dinamik kullanıcı üretip sildiği rol.
set -eu

psql -v ON_ERROR_STOP=1 \
  --username "$POSTGRES_USER" \
  --dbname "$POSTGRES_DB" \
  -v db="$POSTGRES_DB" \
  -v migrator_pw="$OKULDAYIM_MIGRATOR_PASSWORD" \
  -v vault_pw="$VAULT_DB_ADMIN_PASSWORD" <<'EOSQL'
-- Sunucu log_statement=ddl ile çalışır; parola içeren ifadeler loga düşmesin.
SET log_statement = 'none';

CREATE ROLE okuldayim_owner NOLOGIN;
CREATE ROLE okuldayim_migrator LOGIN PASSWORD :'migrator_pw' IN ROLE okuldayim_owner;
CREATE ROLE okuldayim_app NOLOGIN NOSUPERUSER NOBYPASSRLS NOCREATEDB NOCREATEROLE;

-- PostgreSQL 16+: CREATEROLE sahibi yalnızca ADMIN yetkisi olduğu rollere üye ekleyebilir.
CREATE ROLE vault_admin LOGIN PASSWORD :'vault_pw' CREATEROLE;
GRANT okuldayim_app TO vault_admin WITH ADMIN OPTION;
-- Vault'un dinamik kullanıcı oluşturan CREATE ROLE ... PASSWORD ifadeleri de loglanmaz.
ALTER ROLE vault_admin SET log_statement = 'none';

REVOKE ALL ON DATABASE :"db" FROM PUBLIC;
GRANT CONNECT ON DATABASE :"db" TO okuldayim_migrator, okuldayim_app, vault_admin;

REVOKE ALL ON SCHEMA public FROM PUBLIC;
CREATE SCHEMA app AUTHORIZATION okuldayim_owner;
GRANT USAGE ON SCHEMA app TO okuldayim_app;

-- Migration'ların oluşturacağı nesneler için varsayılan yetkiler.
-- Denetim ve rıza tabloları için INSERT dışındaki yetkiler migration içinde geri alınır (append-only).
ALTER DEFAULT PRIVILEGES FOR ROLE okuldayim_owner IN SCHEMA app
  GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO okuldayim_app;
ALTER DEFAULT PRIVILEGES FOR ROLE okuldayim_owner IN SCHEMA app
  GRANT USAGE, SELECT ON SEQUENCES TO okuldayim_app;
ALTER DEFAULT PRIVILEGES FOR ROLE okuldayim_owner IN SCHEMA app
  GRANT EXECUTE ON FUNCTIONS TO okuldayim_app;

ALTER DATABASE :"db" SET search_path = app;
EOSQL

echo "okuldayim: roller ve 'app' şeması oluşturuldu"
