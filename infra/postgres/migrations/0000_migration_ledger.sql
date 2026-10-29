-- Migration geçmişini idempotent biçimde hazırlar. Veri tablolarını değiştirmez.
BEGIN;
SET LOCAL ROLE okuldayim_owner;

CREATE TABLE IF NOT EXISTS app.schema_migrations (
  version text PRIMARY KEY,
  description text NOT NULL,
  applied_at timestamptz NOT NULL DEFAULT clock_timestamp()
);

REVOKE ALL ON app.schema_migrations FROM PUBLIC, okuldayim_app;
COMMIT;
