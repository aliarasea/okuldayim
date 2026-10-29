-- Narrow, SECURITY DEFINER lookups needed before a user-scoped RLS transaction exists.
BEGIN;
SET LOCAL ROLE okuldayim_owner;
SET LOCAL lock_timeout = '2s';
SET LOCAL statement_timeout = '15s';
SET LOCAL search_path = app, pg_catalog;

CREATE FUNCTION app.find_user_id_by_refresh_hash(p_token_hash bytea)
RETURNS uuid
LANGUAGE sql
STABLE
SECURITY DEFINER
SET search_path = pg_catalog, app
AS $function$
  SELECT token.user_id
  FROM app.refresh_token AS token
  WHERE token.token_hash = p_token_hash
  LIMIT 1;
$function$;

REVOKE ALL ON FUNCTION app.find_user_id_by_refresh_hash(bytea) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION app.find_user_id_by_refresh_hash(bytea) TO okuldayim_app;

INSERT INTO app.schema_migrations (version, description)
VALUES ('0002_auth_lookup', 'Scoped refresh-token owner lookup for user-scoped RLS transactions');
COMMIT;
