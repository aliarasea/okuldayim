use std::time::Duration;

use redis::Client;
use secrecy::ExposeSecret;
use url::Url;

use crate::{error::RuntimeError, integrations::vault::BackendSecrets};

#[derive(Clone)]
pub struct RedisIntegration {
    client: Client,
}

impl RedisIntegration {
    pub fn new(host: &str, port: u16, secrets: &BackendSecrets) -> Result<Self, RuntimeError> {
        let address = Url::parse(&format!("redis://{host}:{port}/"))
            .map_err(|_| RuntimeError::Configuration("invalid Redis address"))?;
        let mut address = address;
        address
            .set_username("app")
            .map_err(|_| RuntimeError::Configuration("invalid Redis username"))?;
        address
            .set_password(Some(secrets.redis_password.expose_secret()))
            .map_err(|_| RuntimeError::Configuration("invalid Redis credential"))?;
        let client = Client::open(address)
            .map_err(|_| RuntimeError::Configuration("invalid Redis address"))?;
        Ok(Self { client })
    }

    pub async fn ping(&self) -> Result<(), RuntimeError> {
        let mut connection = tokio::time::timeout(
            Duration::from_secs(1),
            self.client.get_multiplexed_async_connection(),
        )
        .await
        .map_err(|_| RuntimeError::RedisUnavailable)?
        .map_err(|_| RuntimeError::RedisUnavailable)?;
        let response: String = tokio::time::timeout(
            Duration::from_secs(1),
            redis::cmd("PING").query_async(&mut connection),
        )
        .await
        .map_err(|_| RuntimeError::RedisUnavailable)?
        .map_err(|_| RuntimeError::RedisUnavailable)?;
        if response == "PONG" {
            Ok(())
        } else {
            Err(RuntimeError::RedisUnavailable)
        }
    }

    pub async fn set_expiring(
        &self,
        key: &str,
        value: &str,
        ttl_seconds: u64,
    ) -> Result<(), RuntimeError> {
        let mut connection = self.connection().await?;
        redis::cmd("SET")
            .arg(key)
            .arg(value)
            .arg("EX")
            .arg(ttl_seconds)
            .query_async(&mut connection)
            .await
            .map_err(|_| RuntimeError::RedisUnavailable)
    }

    /// Atomic fixed-window counter; its expiry is established with the first increment.
    pub async fn increment_window(
        &self,
        key: &str,
        window_seconds: u64,
    ) -> Result<u64, RuntimeError> {
        let script = r#"
            local count = redis.call('INCR', KEYS[1])
            if count == 1 then redis.call('EXPIRE', KEYS[1], ARGV[1]) end
            return count
        "#;
        let mut connection = self.connection().await?;
        let count: i64 = redis::cmd("EVAL")
            .arg(script)
            .arg(1)
            .arg(key)
            .arg(window_seconds)
            .query_async(&mut connection)
            .await
            .map_err(|_| RuntimeError::RedisUnavailable)?;
        u64::try_from(count).map_err(|_| RuntimeError::RedisUnavailable)
    }

    /// Consumes one OTP challenge atomically, increments attempts, and returns only opaque
    /// challenge metadata. The JSON contains a keyed digest, never the OTP itself.
    pub async fn verify_otp(
        &self,
        challenge_key: &str,
        attempts_key: &str,
        submitted_digest: &str,
        ttl_seconds: u64,
        max_attempts: u64,
    ) -> Result<Option<String>, RuntimeError> {
        let script = r#"
            local raw = redis.call('GET', KEYS[1])
            if not raw then return '' end
            local state = cjson.decode(raw)
            local attempts = redis.call('INCR', KEYS[2])
            if attempts == 1 then redis.call('EXPIRE', KEYS[2], ARGV[2]) end
            if state.codeHash == ARGV[1] then
              redis.call('DEL', KEYS[1])
              redis.call('DEL', KEYS[2])
              state.status = 'verified'
              return cjson.encode(state)
            end
            if attempts >= tonumber(ARGV[3]) then
              redis.call('DEL', KEYS[1])
              state.status = 'locked'
              return cjson.encode(state)
            end
            state.status = 'invalid'
            state.attemptsLeft = tonumber(ARGV[3]) - attempts
            return cjson.encode(state)
        "#;
        let mut connection = self.connection().await?;
        let response: String = redis::cmd("EVAL")
            .arg(script)
            .arg(2)
            .arg(challenge_key)
            .arg(attempts_key)
            .arg(submitted_digest)
            .arg(ttl_seconds)
            .arg(max_attempts)
            .query_async(&mut connection)
            .await
            .map_err(|_| RuntimeError::RedisUnavailable)?;
        Ok((!response.is_empty()).then_some(response))
    }

    async fn connection(&self) -> Result<redis::aio::MultiplexedConnection, RuntimeError> {
        tokio::time::timeout(
            Duration::from_secs(1),
            self.client.get_multiplexed_async_connection(),
        )
        .await
        .map_err(|_| RuntimeError::RedisUnavailable)?
        .map_err(|_| RuntimeError::RedisUnavailable)
    }
}
