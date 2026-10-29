use std::{sync::Arc, time::Duration};

use base64::{
    Engine as _,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use reqwest::{Client, StatusCode};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
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
    pub otp_pepper: SecretString,
    pub pickup_pepper: SecretString,
    pub jasmin_credentials: Option<JasminCredentials>,
}

#[derive(Clone, Debug)]
pub struct TransitPublicKey {
    pub version: u32,
    pub pem: String,
}

#[derive(Serialize)]
struct TransitInput<'a> {
    input: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    marshaling_algorithm: Option<&'static str>,
}

#[derive(Deserialize)]
struct TransitResponse {
    data: TransitData,
}

#[derive(Deserialize)]
struct TransitData {
    #[serde(default)]
    ciphertext: Option<String>,
    #[serde(default)]
    plaintext: Option<String>,
    #[serde(default)]
    hmac: Option<String>,
    #[serde(default)]
    signature: Option<String>,
    #[serde(default)]
    key_version: Option<u32>,
    #[serde(default)]
    keys: Option<BTreeMap<String, TransitKey>>,
    #[serde(default)]
    latest_version: Option<u32>,
}

#[derive(Deserialize)]
struct TransitKey {
    public_key: Option<String>,
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
        if payload.data.data.redis_password.is_empty()
            || payload.data.data.otp_pepper.is_empty()
            || payload.data.data.pickup_pepper.is_empty()
        {
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
            otp_pepper: SecretString::new(payload.data.data.otp_pepper.into()),
            pickup_pepper: SecretString::new(payload.data.data.pickup_pepper.into()),
            jasmin_credentials,
        })
    }

    pub async fn transit_hmac(&self, key: &str, value: &[u8]) -> Result<Vec<u8>, RuntimeError> {
        let encoded = STANDARD.encode(value);
        let path = format!("v1/transit/hmac/{key}");
        let response = self
            .transit_post(
                &path,
                &TransitInput {
                    input: &encoded,
                    marshaling_algorithm: None,
                },
            )
            .await?;
        let hmac = response
            .data
            .hmac
            .as_deref()
            .and_then(|value| value.rsplit_once(':').map(|(_, digest)| digest))
            .ok_or(RuntimeError::VaultUnavailable)?;
        STANDARD
            .decode(hmac)
            .map_err(|_| RuntimeError::VaultUnavailable)
    }

    #[allow(dead_code)] // Used by the synthetic tenant fixture and retained as the tenant-key provisioning seam.
    pub async fn ensure_organization_key(
        &self,
        organization_id: uuid::Uuid,
    ) -> Result<(), RuntimeError> {
        let key = format!("kurum-{organization_id}");
        let token = self.current_token().await?;
        let url = self.endpoint(&format!("v1/transit/keys/{key}"))?;
        let response = self
            .inner
            .http
            .get(url.clone())
            .header("X-Vault-Token", token.expose_secret())
            .send()
            .await
            .map_err(|_| RuntimeError::VaultUnavailable)?;
        if response.status() == StatusCode::OK {
            return Ok(());
        }
        if response.status() != StatusCode::NOT_FOUND {
            return Err(RuntimeError::VaultUnavailable);
        }
        let response = self
            .inner
            .http
            .post(url)
            .header("X-Vault-Token", token.expose_secret())
            .json(&serde_json::json!({
                "type": "aes256-gcm96",
                "auto_rotate_period": "8760h"
            }))
            .send()
            .await
            .map_err(|_| RuntimeError::VaultUnavailable)?;
        if response.status().is_success() {
            Ok(())
        } else {
            Err(RuntimeError::VaultUnavailable)
        }
    }

    pub async fn transit_encrypt(
        &self,
        key: &str,
        value: &[u8],
    ) -> Result<(String, u16), RuntimeError> {
        let encoded = STANDARD.encode(value);
        let path = format!("v1/transit/encrypt/{key}");
        let response = self
            .transit_post(&path, &serde_json::json!({ "plaintext": encoded }))
            .await?;
        let ciphertext = response
            .data
            .ciphertext
            .ok_or(RuntimeError::VaultUnavailable)?;
        let version = response
            .data
            .key_version
            .or_else(|| transit_version(&ciphertext))
            .and_then(|version| u16::try_from(version).ok())
            .ok_or(RuntimeError::VaultUnavailable)?;
        Ok((ciphertext, version))
    }

    pub async fn transit_decrypt(
        &self,
        key: &str,
        ciphertext: &str,
    ) -> Result<Vec<u8>, RuntimeError> {
        let path = format!("v1/transit/decrypt/{key}");
        let response = self
            .transit_post(&path, &serde_json::json!({ "ciphertext": ciphertext }))
            .await?;
        let plaintext = response
            .data
            .plaintext
            .ok_or(RuntimeError::VaultUnavailable)?;
        STANDARD
            .decode(plaintext)
            .map_err(|_| RuntimeError::VaultUnavailable)
    }

    pub async fn transit_sign_jws(&self, value: &[u8]) -> Result<(Vec<u8>, u32), RuntimeError> {
        let encoded = STANDARD.encode(value);
        let response = self
            .transit_post(
                "v1/transit/sign/jwt-signing",
                &TransitInput {
                    input: &encoded,
                    marshaling_algorithm: Some("jws"),
                },
            )
            .await?;
        let Some(signature) = response
            .data
            .signature
            .as_deref()
            .and_then(|value| value.rsplit_once(':').map(|(_, signature)| signature))
        else {
            tracing::warn!(component = "vault", operation = "transit_signature_missing");
            return Err(RuntimeError::VaultUnavailable);
        };
        let signature = match URL_SAFE_NO_PAD.decode(signature) {
            Ok(signature) => signature,
            Err(_) => {
                tracing::warn!(
                    component = "vault",
                    operation = "transit_signature_encoding_invalid"
                );
                return Err(RuntimeError::VaultUnavailable);
            }
        };
        let Some(version) = response
            .data
            .key_version
            .or_else(|| response.data.signature.as_deref().and_then(transit_version))
        else {
            tracing::warn!(
                component = "vault",
                operation = "transit_signature_version_missing"
            );
            return Err(RuntimeError::VaultUnavailable);
        };
        Ok((signature, version))
    }

    pub async fn jwt_public_keys(&self) -> Result<(u32, Vec<TransitPublicKey>), RuntimeError> {
        let token = self.current_token().await?;
        let response = self
            .inner
            .http
            .get(self.endpoint("v1/transit/keys/jwt-signing")?)
            .header("X-Vault-Token", token.expose_secret())
            .send()
            .await
            .map_err(|_| RuntimeError::VaultUnavailable)?;
        if response.status() != StatusCode::OK {
            return Err(RuntimeError::VaultUnavailable);
        }
        let payload: TransitResponse = response
            .json()
            .await
            .map_err(|_| RuntimeError::VaultUnavailable)?;
        let keys = payload.data.keys.ok_or(RuntimeError::VaultUnavailable)?;
        let latest_version = payload
            .data
            .latest_version
            .ok_or(RuntimeError::VaultUnavailable)?;
        let mut public_keys = Vec::with_capacity(keys.len());
        for (version, key) in keys {
            let version = version
                .parse::<u32>()
                .map_err(|_| RuntimeError::VaultUnavailable)?;
            if let Some(pem) = key.public_key.filter(|value| !value.is_empty()) {
                public_keys.push(TransitPublicKey { version, pem });
            }
        }
        if public_keys.is_empty() {
            return Err(RuntimeError::VaultUnavailable);
        }
        Ok((latest_version, public_keys))
    }

    async fn transit_post<T: Serialize>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<TransitResponse, RuntimeError> {
        let token = self.current_token().await?;
        let response = self
            .inner
            .http
            .post(self.endpoint(path)?)
            .header("X-Vault-Token", token.expose_secret())
            .json(body)
            .send()
            .await
            .map_err(|_| RuntimeError::VaultUnavailable)?;
        if response.status() != StatusCode::OK {
            let status = response.status().as_u16();
            #[cfg(test)]
            {
                let errors = response
                    .json::<serde_json::Value>()
                    .await
                    .ok()
                    .and_then(|body| body.get("errors").cloned())
                    .unwrap_or(serde_json::Value::Null);
                tracing::warn!(
                    component = "vault",
                    operation = "transit_request_rejected",
                    path,
                    status,
                    errors = ?errors,
                    "Vault rejected a Transit operation"
                );
            }
            #[cfg(not(test))]
            tracing::warn!(
                component = "vault",
                operation = "transit_request_rejected",
                path,
                status,
                "Vault rejected a Transit operation"
            );
            return Err(RuntimeError::VaultUnavailable);
        }
        response
            .json()
            .await
            .map_err(|_| RuntimeError::VaultUnavailable)
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
    otp_pepper: String,
    pickup_pepper: String,
    jasmin_username: Option<String>,
    jasmin_password: Option<String>,
}

fn transit_version(value: &str) -> Option<u32> {
    value
        .split(':')
        .nth(1)
        .and_then(|version| version.strip_prefix('v'))
        .and_then(|version| version.parse().ok())
}
