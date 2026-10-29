#!/bin/sh
# Şema migration'larını okuldayim_migrator kimliğiyle ve transaction içinde uygular.
set -eu

MIGRATIONS_DIR=/scripts/migrations

psql_db() {
  PGPASSWORD="$MIGRATION_DATABASE_PASSWORD" psql \
    -X -w -v ON_ERROR_STOP=1 \
    -h "$MIGRATION_DATABASE_HOST" \
    -p "$MIGRATION_DATABASE_PORT" \
    -U "$MIGRATION_DATABASE_USER" \
    -d "$MIGRATION_DATABASE_NAME" "$@"
}

echo "[postgres-migrate] PostgreSQL bağlantısı doğrulanıyor"
psql_db -Atqc 'SELECT 1' >/dev/null
psql_db -f "$MIGRATIONS_DIR/0000_migration_ledger.sql" >/dev/null

tracked_migrations=$(psql_db -Atqc "SELECT count(*) FROM app.schema_migrations WHERE version <> '0000_migration_ledger'")
legacy_tables=$(psql_db -Atqc "SELECT count(*) FROM pg_catalog.pg_tables WHERE schemaname = 'app' AND tablename <> 'schema_migrations'")
if [ "$tracked_migrations" -eq 0 ] && [ "$legacy_tables" -gt 0 ]; then
  echo "[postgres-migrate] app şemasında migration geçmişi olmayan tablolar bulundu; mevcut şemayı korumak için durduruldu." >&2
  echo "[postgres-migrate] Önce bu tabloların migration geçmişiyle eşleştiğini doğrulayın; otomatik DROP/yeniden oluşturma yapılmadı." >&2
  exit 1
fi

found_migration=false
for migration in "$MIGRATIONS_DIR"/[0-9][0-9][0-9][0-9]_*.sql; do
  [ -f "$migration" ] || continue
  version=${migration##*/}
  version=${version%.sql}
  [ "$version" = "0000_migration_ledger" ] && continue
  case "$version" in
    *[!A-Za-z0-9_]*|'')
      echo "[postgres-migrate] Geçersiz migration adı: $version" >&2
      exit 1
      ;;
  esac
  found_migration=true

  applied=$(psql_db -Atqc "SELECT EXISTS (SELECT 1 FROM app.schema_migrations WHERE version = '$version')")
  if [ "$applied" = "t" ]; then
    echo "[postgres-migrate] $version zaten uygulanmış"
    continue
  fi

  echo "[postgres-migrate] $version uygulanıyor"
  psql_db -f "$migration"
done

[ "$found_migration" = true ] || {
  echo "[postgres-migrate] Uygulanabilir migration dosyası bulunamadı" >&2
  exit 1
}

bad_rls=$(psql_db -Atqc "
  SELECT count(*)
  FROM pg_catalog.pg_class c
  JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
  WHERE n.nspname = 'app'
    AND c.relkind = 'r'
    AND c.relname <> 'schema_migrations'
    AND (NOT c.relrowsecurity OR NOT c.relforcerowsecurity)
")
[ "$bad_rls" = "0" ] || {
  echo "[postgres-migrate] HATA: RLS zorunlu tabloların tamamında ENABLE/FORCE değil" >&2
  exit 1
}

bad_role=$(psql_db -Atqc "
  SELECT count(*)
  FROM pg_catalog.pg_roles
  WHERE rolname = 'okuldayim_app'
    AND (rolsuper OR rolbypassrls OR rolcreatedb OR rolcreaterole)
")
[ "$bad_role" = "0" ] || {
  echo "[postgres-migrate] HATA: okuldayim_app rol yetkileri beklenen sınırda değil" >&2
  exit 1
}

table_count=$(psql_db -Atqc "SELECT count(*) FROM pg_catalog.pg_tables WHERE schemaname = 'app' AND tablename <> 'schema_migrations'")
echo "[postgres-migrate] Doğrulama başarılı: $table_count uygulama tablosu, RLS ENABLE/FORCE açık, migration tamamlandı"
