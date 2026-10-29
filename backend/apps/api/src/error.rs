use thiserror::Error;

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("invalid configuration: {0}")]
    Configuration(&'static str),
    #[error("Vault is unavailable or returned an invalid response")]
    VaultUnavailable,
    #[error("PostgreSQL is unavailable or its credential lease is expiring")]
    DatabaseUnavailable,
    #[error("Redis is unavailable")]
    RedisUnavailable,
}
