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
}
