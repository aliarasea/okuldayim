# Rust backend runtime

This backend slice provides a runnable API process, operational HTTP endpoints, and the Vault, PostgreSQL, and Redis runtime integrations. Product workflows are not implemented yet; the routes below must not be mistaken for the MVP product API.

## Run locally

Start local services and apply the database migrations from `infra/`. The API container is built by Compose and receives a read-only volume containing only its AppRole file.

```sh
cd infra
./scripts/generate-dev-env.sh
docker compose up -d --build
docker compose --profile tools run --rm postgres-migrate
curl -i http://127.0.0.1:8080/readyz
```

`GET /healthz` is process liveness. `GET /readyz` requires a reachable PostgreSQL pool and a DB lease with its configured safety window; Redis failure is reported as `degraded` without taking unrelated routes offline. `GET /openapi.json` describes the operational routes.

The API authenticates to Vault with AppRole, reads the Redis credential from KV v2, and obtains short-lived PostgreSQL credentials from the database secrets engine. A new DB pool is tested before replacing the current pool; rotation starts at 70% of the lease, and failures retain the old pool until the safety threshold is reached. Pooled PostgreSQL sessions have bounded statement, lock, and idle-transaction timeouts. Tenant transactions use transaction-local `app.user_id` and `app.organization_id` settings to satisfy the RLS policies without leaking context across pool reuse.

An optional Jasmin SMS HTTP transport is available when `OKD_SMS_GATEWAY_URL` points to the gateway's `/send` endpoint. The gateway account username/password are read from Vault KV (`jasmin_username`, `jasmin_password`); local bootstrap can populate these from `infra/.env`. `/readyz` reports only whether the adapter is configured, not gateway health. Jasmin's success response is queue acceptance, not delivery; ambiguous transport/5xx results are not automatically retried. No OTP route, SMS outbox worker, DLR callback, or carrier/SMSC provisioning is included yet. See [SMS gateway requirements](../docs/systems/sms-provider-requirements.md).

The API container runs as UID/GID 10001. It receives only the AppRole credential volume, not the bind-mounted directory containing the local Vault root token and unseal key. Local host execution may use `OKD_VAULT_APPROLE_FILE=../infra/.local/vault/backend-approle.env`; that file and all local credentials must remain outside source control and production images.

Production configuration requires Vault HTTPS plus PostgreSQL `verify-full` and `OKD_DB_SSL_ROOT_CERT`. Local Compose intentionally disables PostgreSQL TLS only on its private development network.

## Deliberate non-goals in this slice

No OTP/authentication, `/v1/me`, consent, feed, meals, pickup, WebSocket, worker, SMTP, S3/media, or production workload-identity path is implemented yet. Jasmin is selected as the open-source gateway and its transport adapter exists, but the SMS carrier/SMSC and account are not provisioned. Media is a later phase. `TenantTx` establishes the DB context seam but does not itself implement authentication, authorization, consent, auditing, idempotency, or a product handler. Those acceptance requirements remain before pilot use.
