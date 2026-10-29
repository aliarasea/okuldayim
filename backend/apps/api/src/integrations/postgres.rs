use std::{sync::Arc, time::Duration};

use secrecy::ExposeSecret;
use sqlx::{
    PgPool, Postgres, Transaction,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use tokio::sync::broadcast;
use tokio::{sync::RwLock, time::Instant};
use tracing::{info, warn};
use uuid::Uuid;

use crate::{config::RuntimeConfig, error::RuntimeError, integrations::vault::VaultClient};

const ROTATION_FRACTION: f64 = 0.70;
const RETRY_DELAY: Duration = Duration::from_secs(5);
const DRAIN_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Clone)]
pub struct PostgresIntegration {
    inner: Arc<PostgresInner>,
}

struct PostgresInner {
    vault: VaultClient,
    settings: DatabaseSettings,
    minimum_lease_remaining: Duration,
    state: RwLock<PoolState>,
}

#[derive(Clone)]
struct DatabaseSettings {
    host: String,
    port: u16,
    database: String,
    ssl_mode: sqlx::postgres::PgSslMode,
    ssl_root_cert: Option<std::path::PathBuf>,
    max_connections: u32,
}

struct PoolState {
    pool: Option<PgPool>,
    lease_expires_at: Option<Instant>,
    rotate_at: Option<Instant>,
}

/// Authenticated tenant identity resolved by the application, never copied from an untrusted
/// request field without authorization checks.
#[allow(dead_code)] // Product request handlers will consume this seam in the next backend slice.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TenantContext {
    pub user_id: Uuid,
    pub organization_id: Uuid,
}

impl PostgresIntegration {
    pub async fn connect(config: &RuntimeConfig, vault: VaultClient) -> Result<Self, RuntimeError> {
        let integration = Self {
            inner: Arc::new(PostgresInner {
                vault,
                settings: DatabaseSettings {
                    host: config.database_host.clone(),
                    port: config.database_port,
                    database: config.database_name.clone(),
                    ssl_mode: config.database_ssl_mode,
                    ssl_root_cert: config.database_ssl_root_cert.clone(),
                    max_connections: config.database_max_connections,
                },
                minimum_lease_remaining: config.minimum_lease_remaining,
                state: RwLock::new(PoolState {
                    pool: None,
                    lease_expires_at: None,
                    rotate_at: None,
                }),
            }),
        };
        integration.rotate_pool().await?;
        Ok(integration)
    }

    /// Starts a fresh lease and verifies its pool before atomically publishing it. A failed
    /// rotation leaves the existing pool in place until its lease safety window is exhausted.
    pub async fn rotate_pool(&self) -> Result<(), RuntimeError> {
        let credentials = self.inner.vault.database_credentials().await?;
        if credentials.lease_duration <= self.inner.minimum_lease_remaining {
            return Err(RuntimeError::DatabaseUnavailable);
        }

        let mut options = PgConnectOptions::new()
            .host(&self.inner.settings.host)
            .port(self.inner.settings.port)
            .database(&self.inner.settings.database)
            .username(&credentials.username)
            .password(credentials.password.expose_secret())
            .ssl_mode(self.inner.settings.ssl_mode)
            .application_name("okuldayim-api");
        if let Some(root_cert) = &self.inner.settings.ssl_root_cert {
            options = options.ssl_root_cert(root_cert);
        }

        let pool = PgPoolOptions::new()
            .max_connections(self.inner.settings.max_connections)
            .acquire_timeout(Duration::from_secs(3))
            .after_connect(|connection, _metadata| {
                Box::pin(async move {
                    sqlx::query("SET search_path TO app, pg_catalog")
                        .execute(&mut *connection)
                        .await?;
                    sqlx::query("SET statement_timeout = '8s'")
                        .execute(&mut *connection)
                        .await?;
                    sqlx::query("SET lock_timeout = '1s'")
                        .execute(&mut *connection)
                        .await?;
                    sqlx::query("SET idle_in_transaction_session_timeout = '10s'")
                        .execute(&mut *connection)
                        .await?;
                    Ok(())
                })
            })
            .connect_with(options)
            .await
            .map_err(|_| RuntimeError::DatabaseUnavailable)?;

        sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(&pool)
            .await
            .map_err(|_| RuntimeError::DatabaseUnavailable)?;

        let now = Instant::now();
        let lease_expires_at = now + credentials.lease_duration;
        let rotate_at = now + credentials.lease_duration.mul_f64(ROTATION_FRACTION);
        let old_pool = {
            let mut state = self.inner.state.write().await;
            let old_pool = state.pool.replace(pool);
            state.lease_expires_at = Some(lease_expires_at);
            state.rotate_at = Some(rotate_at);
            old_pool
        };

        if let Some(old_pool) = old_pool {
            tokio::spawn(async move {
                if tokio::time::timeout(DRAIN_TIMEOUT, old_pool.close())
                    .await
                    .is_err()
                {
                    warn!(
                        component = "postgres",
                        operation = "pool_drain_slow",
                        "Previous PostgreSQL pool is still draining"
                    );
                }
            });
        }
        info!(
            component = "postgres",
            operation = "pool_rotated",
            "PostgreSQL credential pool is ready"
        );
        Ok(())
    }

    pub async fn run_pool_rotation(&self, mut shutdown: broadcast::Receiver<()>) {
        loop {
            let delay = self.next_rotation_delay().await;
            tokio::select! {
                _ = tokio::time::sleep(delay) => {}
                _ = shutdown.recv() => return,
            }

            if self.rotate_pool().await.is_err() {
                warn!(
                    component = "postgres",
                    operation = "pool_rotation_failed",
                    "PostgreSQL pool rotation failed; the current lease remains active"
                );
                tokio::select! {
                    _ = tokio::time::sleep(RETRY_DELAY) => {}
                    _ = shutdown.recv() => return,
                }
            }
        }
    }

    async fn next_rotation_delay(&self) -> Duration {
        self.inner
            .state
            .read()
            .await
            .rotate_at
            .map_or(Duration::ZERO, |at| {
                at.saturating_duration_since(Instant::now())
                    .max(Duration::from_secs(1))
            })
    }

    pub async fn is_ready(&self) -> bool {
        let (pool, lease_expires_at) = {
            let state = self.inner.state.read().await;
            (state.pool.clone(), state.lease_expires_at)
        };
        let (Some(pool), Some(expires_at)) = (pool, lease_expires_at) else {
            return false;
        };
        if expires_at.saturating_duration_since(Instant::now())
            <= self.inner.minimum_lease_remaining
        {
            return false;
        }
        tokio::time::timeout(
            Duration::from_secs(1),
            sqlx::query_scalar::<_, i32>("SELECT 1").fetch_one(&pool),
        )
        .await
        .is_ok_and(|result| result.is_ok())
    }

    /// Opens a short-lived transaction and sets RLS context transaction-locally so pooled
    /// connections cannot carry one request's tenant identity into another request.
    #[allow(dead_code)] // Kept as the single DB access seam until product handlers are added.
    pub async fn begin_tenant(
        &self,
        context: TenantContext,
    ) -> Result<Transaction<'static, Postgres>, RuntimeError> {
        let (pool, lease_expires_at) = {
            let state = self.inner.state.read().await;
            (state.pool.clone(), state.lease_expires_at)
        };
        let (Some(pool), Some(expires_at)) = (pool, lease_expires_at) else {
            return Err(RuntimeError::DatabaseUnavailable);
        };
        if expires_at.saturating_duration_since(Instant::now())
            <= self.inner.minimum_lease_remaining
        {
            return Err(RuntimeError::DatabaseUnavailable);
        }

        let mut transaction = pool
            .begin()
            .await
            .map_err(|_| RuntimeError::DatabaseUnavailable)?;
        if sqlx::query(
            "SELECT set_config('app.user_id', $1, true), set_config('app.organization_id', $2, true)",
        )
        .bind(context.user_id.to_string())
        .bind(context.organization_id.to_string())
        .execute(&mut *transaction)
        .await
        .is_err()
        {
            let _ = transaction.rollback().await;
            return Err(RuntimeError::DatabaseUnavailable);
        }
        Ok(transaction)
    }
}
