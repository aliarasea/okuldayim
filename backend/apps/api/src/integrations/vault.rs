use std::{sync::Arc, time::Duration};

use reqwest::{Client, StatusCode};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use tokio::sync::broadcast;
use tokio::{sync::Mutex, time::Instant};
use tracing::warn;
use url::Url;

use crate::{config::VaultAppRole, error::RuntimeError};
use okuldayim_api::sms::JasminCredentials;

const DATABASE_CREDENTIALS_PATH: &str = "v1/database/creds/okuldayim-app";
const BACKEND_SECRET_PATH: &str = "v1/secret/data/okuldayim/backend";
const MINIMUM_VAULT_LEASE: Duration = Duration::from_secs(60);

#[derive(Clone)]
pub struct VaultClient {
    inner: Arc<VaultInner>,
}

struct VaultInner {
    address: Url,
    http: Client,
    approle: VaultAppRole,
    token: Mutex<Option<VaultToken>>,
}

struct VaultToken {
    value: SecretString,
    refresh_at: Instant,
    expires_at: Instant,
    renewable: bool,
}

pub struct DatabaseCredentials {
    pub username: String,
    pub password: SecretString,
    pub lease_duration: Duration,
}

pub struct BackendSecrets {
    pub redis_password: SecretString,
    pub jasmin_credentials: Option<JasminCredentials>,
}

impl VaultClient {
    pub fn new(address: Url, approle: VaultAppRole) -> Result<Self, RuntimeError> {
        let http = Client::builder()
            .timeout(Duration::from_secs(3))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| RuntimeError::Configuration("cannot initialize Vault HTTP client"))?;
        Ok(Self {
            inner: Arc::new(VaultInner {
                address,
                http,
                approle,
                token: Mutex::new(None),
            }),
        })
    }

    pub async fn authenticate(&self) -> Result<(), RuntimeError> {
        let mut token = self.inner.token.lock().await;
        self.login_locked(&mut token).await
    }

    pub async fn database_credentials(&self) -> Result<DatabaseCredentials, RuntimeError> {
        let token = self.current_token().await?;
        let response = self
            .inner
            .http
            .get(self.endpoint(DATABASE_CREDENTIALS_PATH)?)
            .header("X-Vault-Token", token.expose_secret())
            .send()
            .await
            .map_err(|_| RuntimeError::VaultUnavailable)?;
        if response.status() != StatusCode::OK {
            return Err(RuntimeError::VaultUnavailable);
        }
        let payload: DatabaseCredentialsResponse = response
            .json()
            .await
            .map_err(|_| RuntimeError::VaultUnavailable)?;
        let lease_duration = Duration::from_secs(payload.lease_duration);
        if payload.data.username.is_empty()
            || payload.data.password.is_empty()
            || lease_duration < MINIMUM_VAULT_LEASE
        {
            return Err(RuntimeError::VaultUnavailable);
        }
        Ok(DatabaseCredentials {
            username: payload.data.username,
            password: SecretString::new(payload.data.password.into()),
            lease_duration,
        })
    }

    pub async fn backend_secrets(&self) -> Result<BackendSecrets, RuntimeError> {
        let token = self.current_token().await?;
        let response = self
            .inner
            .http
            .get(self.endpoint(BACKEND_SECRET_PATH)?)
            .header("X-Vault-Token", token.expose_secret())
            .send()
            .await
            .map_err(|_| RuntimeError::VaultUnavailable)?;
        if response.status() != StatusCode::OK {
            return Err(RuntimeError::VaultUnavailable);
        }
        let payload: BackendSecretResponse = response
            .json()
            .await
            .map_err(|_| RuntimeError::VaultUnavailable)?;
        if payload.data.data.redis_password.is_empty() {
            return Err(RuntimeError::VaultUnavailable);
        }
        let jasmin_credentials = match (
            payload.data.data.jasmin_username,
            payload.data.data.jasmin_password,
        ) {
            (None, None) => None,
            (Some(username), Some(password)) if !username.is_empty() && !password.is_empty() => {
                Some(JasminCredentials {
                    username: SecretString::new(username.into()),
                    password: SecretString::new(password.into()),
                })
            }
            _ => return Err(RuntimeError::VaultUnavailable),
        };
        Ok(BackendSecrets {
            redis_password: SecretString::new(payload.data.data.redis_password.into()),
            jasmin_credentials,
        })
    }

    pub async fn run_token_renewal(&self, mut shutdown: broadcast::Receiver<()>) {
        loop {
            let delay = self.next_refresh_delay().await;
            tokio::select! {
                _ = tokio::time::sleep(delay) => {}
                _ = shutdown.recv() => return,
            }

            if self.renew_or_reauthenticate().await.is_err() {
                warn!(
                    component = "vault",
                    operation = "token_refresh_failed",
                    "Vault token refresh failed; retrying before expiry"
                );
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_secs(5)) => {}
                    _ = shutdown.recv() => return,
                }
            }
        }
    }

    async fn current_token(&self) -> Result<SecretString, RuntimeError> {
        let mut current = self.inner.token.lock().await;
        if current
            .as_ref()
            .is_none_or(|token| token.expires_at <= Instant::now())
        {
            self.login_locked(&mut current).await?;
        }
        current
            .as_ref()
            .map(|token| SecretString::new(token.value.expose_secret().to_owned().into()))
            .ok_or(RuntimeError::VaultUnavailable)
    }

    async fn next_refresh_delay(&self) -> Duration {
        let current = self.inner.token.lock().await;
        current.as_ref().map_or(Duration::ZERO, |token| {
            token
                .refresh_at
                .saturating_duration_since(Instant::now())
                .max(Duration::from_secs(1))
        })
    }

    async fn renew_or_reauthenticate(&self) -> Result<(), RuntimeError> {
        let mut current = self.inner.token.lock().await;
        let Some(token) = current.as_ref() else {
            return self.login_locked(&mut current).await;
        };
        if !token.renewable {
            return self.login_locked(&mut current).await;
        }
        let current_value = token.value.expose_secret().to_owned();
        let response = self
            .inner
            .http
            .post(self.endpoint("v1/auth/token/renew-self")?)
            .header("X-Vault-Token", current_value)
            .json(&serde_json::json!({}))
            .send()
            .await;
        let Ok(response) = response else {
            return Err(RuntimeError::VaultUnavailable);
        };
        if response.status() != StatusCode::OK {
            return self.login_locked(&mut current).await;
        }
        let payload: TokenResponse = response
            .json()
            .await
            .map_err(|_| RuntimeError::VaultUnavailable)?;
        self.replace_token(&mut current, payload.auth)
    }

    async fn login_locked(&self, current: &mut Option<VaultToken>) -> Result<(), RuntimeError> {
        let response = self
            .inner
            .http
            .post(self.endpoint("v1/auth/approle/login")?)
            .json(&serde_json::json!({
                "role_id": self.inner.approle.role_id,
                "secret_id": self.inner.approle.secret_id.expose_secret(),
            }))
            .send()
            .await
            .map_err(|_| RuntimeError::VaultUnavailable)?;
        if response.status() != StatusCode::OK {
            return Err(RuntimeError::VaultUnavailable);
        }
        let payload: TokenResponse = response
            .json()
            .await
            .map_err(|_| RuntimeError::VaultUnavailable)?;
        self.replace_token(current, payload.auth)
    }

    fn replace_token(
        &self,
        current: &mut Option<VaultToken>,
        auth: VaultAuth,
    ) -> Result<(), RuntimeError> {
        let lease_duration = Duration::from_secs(auth.lease_duration);
        if auth.client_token.is_empty() || lease_duration < MINIMUM_VAULT_LEASE {
            return Err(RuntimeError::VaultUnavailable);
        }
        let now = Instant::now();
        let refresh_after = lease_duration.mul_f64(0.70).max(Duration::from_secs(1));
        *current = Some(VaultToken {
            value: SecretString::new(auth.client_token.into()),
            refresh_at: now + refresh_after,
            expires_at: now + lease_duration,
            renewable: auth.renewable,
        });
        Ok(())
    }

    fn endpoint(&self, path: &str) -> Result<Url, RuntimeError> {
        self.inner
            .address
            .join(path)
            .map_err(|_| RuntimeError::VaultUnavailable)
    }
}

#[derive(Deserialize)]
struct TokenResponse {
    auth: VaultAuth,
}

#[derive(Deserialize)]
struct VaultAuth {
    client_token: String,
    lease_duration: u64,
    renewable: bool,
}

#[derive(Deserialize)]
struct DatabaseCredentialsResponse {
    lease_duration: u64,
    data: DatabaseCredentialData,
}

#[derive(Deserialize)]
struct DatabaseCredentialData {
    username: String,
    password: String,
}

#[derive(Deserialize)]
struct BackendSecretResponse {
    data: VersionedBackendSecret,
}

#[derive(Deserialize)]
struct VersionedBackendSecret {
    data: BackendSecretData,
}

#[derive(Deserialize)]
struct BackendSecretData {
    redis_password: String,
    jasmin_username: Option<String>,
    jasmin_password: Option<String>,
}
