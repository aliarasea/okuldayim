# Rust backend runtime

This backend provides a runnable API process, Vault/PostgreSQL/Redis integrations, OTP/session authentication, and a tested Phase 1a subset of the `/v1` product API. It is not the complete MVP or pilot-ready system. Implemented mobile flows, E2E coverage, and remaining boundaries are documented in [mobile-backend-use-cases](../docs/mobile-backend-use-cases.md).

## Run locally

Start local services and apply the database migrations from `infra/`. The API container is built by Compose and receives a read-only volume containing only its AppRole file.

```sh
cd infra
./scripts/generate-dev-env.sh
docker compose up -d --build
docker compose --profile tools run --rm postgres-migrate
curl -i http://127.0.0.1:8080/readyz
```

`GET /healthz` is process liveness. `GET /readyz` requires a reachable PostgreSQL pool and a DB lease with its configured safety window; Redis failure is reported as `degraded` without taking unrelated routes offline. `GET /openapi.json` describes operational and implemented Phase 1a product routes.

The API authenticates to Vault with AppRole, reads the Redis credential from KV v2, and obtains short-lived PostgreSQL credentials from the database secrets engine. A new DB pool is tested before replacing the current pool; rotation starts at 70% of the lease, and failures retain the old pool until the safety threshold is reached. Pooled PostgreSQL sessions have bounded statement, lock, and idle-transaction timeouts. Tenant transactions use transaction-local `app.user_id` and `app.organization_id` settings to satisfy the RLS policies without leaking context across pool reuse.

An optional Jasmin SMS HTTP transport is available when `OKD_SMS_GATEWAY_URL` points to the gateway's `/send` endpoint. The gateway account username/password are read from Vault KV (`jasmin_username`, `jasmin_password`); local bootstrap can populate these from `infra/.env`. `/readyz` reports only whether the adapter is configured, not gateway health. Jasmin's success response is queue acceptance, not delivery; ambiguous transport/5xx results are not automatically retried. OTP/authentication routes exist, but the durable SMS outbox worker/retries, DLR callback, and carrier/SMSC provisioning are not included. See [SMS gateway requirements](../docs/systems/sms-provider-requirements.md).

The API container runs as UID/GID 10001. It receives only the AppRole credential volume, not the bind-mounted directory containing the local Vault root token and unseal key. Local host execution may use `OKD_VAULT_APPROLE_FILE=../infra/.local/vault/backend-approle.env`; that file and all local credentials must remain outside source control and production images.

Production configuration requires Vault HTTPS plus PostgreSQL `verify-full` and `OKD_DB_SSL_ROOT_CERT`. Local Compose intentionally disables PostgreSQL TLS only on its private development network.

## Deliberate non-goals in this slice

The implemented product API is a Phase 1a subset: OTP/session auth, `/v1/me`, privacy notice acknowledgement and consent, feed/class views, meal records, pickup/PIN handover, privacy-request/data-export acceptance, and audit events. Remaining work includes durable SMS/SMTP workers, delivery receipts, WebSocket/push, S3/media processing, export generation, seed tooling, and production workload identity and operations. Jasmin is the selected open-source gateway and its transport adapter exists, but the carrier/SMSC and account are not provisioned. Media is a later phase. Tenant transactions provide RLS context, while implemented handlers enforce the checks relevant to their flows; remaining routes and acceptance criteria must be completed before pilot use.
