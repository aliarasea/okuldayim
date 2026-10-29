use std::{env, net::SocketAddr, path::PathBuf, time::Duration};

use secrecy::SecretString;
use sqlx::postgres::PgSslMode;
use url::Url;

use crate::error::RuntimeError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeMode {
    Development,
    Test,
    Production,
}

#[derive(Clone, Debug)]
pub struct RuntimeConfig {
    pub mode: RuntimeMode,
    pub http_addr: SocketAddr,
    pub vault_addr: Url,
    pub sms_gateway_url: Option<Url>,
    pub approle_file: PathBuf,
    pub database_host: String,
    pub database_port: u16,
    pub database_name: String,
    pub database_ssl_mode: PgSslMode,
    pub database_ssl_root_cert: Option<PathBuf>,
    pub database_max_connections: u32,
    pub redis_host: String,
    pub redis_port: u16,
    pub minimum_lease_remaining: Duration,
}

impl RuntimeConfig {
    pub fn from_env() -> Result<Self, RuntimeError> {
        let mode = match env_value("OKD_ENV", "development").as_str() {
            "development" => RuntimeMode::Development,
            "test" => RuntimeMode::Test,
            "production" => RuntimeMode::Production,
            _ => {
                return Err(RuntimeError::Configuration(
                    "OKD_ENV must be development, test, or production",
                ));
            }
        };

        let http_addr = env_value("OKD_HTTP_ADDR", "127.0.0.1:8080")
            .parse()
            .map_err(|_| {
                RuntimeError::Configuration("OKD_HTTP_ADDR is not a valid socket address")
            })?;
        let mut vault_addr = Url::parse(&env_value("OKD_VAULT_ADDR", "http://127.0.0.1:8200"))
            .map_err(|_| RuntimeError::Configuration("OKD_VAULT_ADDR is not a valid URL"))?;
        if !matches!(vault_addr.scheme(), "http" | "https")
            || vault_addr.host_str().is_none()
            || vault_addr.path() != "/"
            || vault_addr.query().is_some()
            || vault_addr.fragment().is_some()
        {
            return Err(RuntimeError::Configuration(
                "OKD_VAULT_ADDR must be an HTTP(S) origin",
            ));
        }
        vault_addr.set_path("/");
        let database_ssl_mode = match env_value("OKD_DB_SSL_MODE", "prefer").as_str() {
            "disable" => PgSslMode::Disable,
            "prefer" => PgSslMode::Prefer,
            "require" => PgSslMode::Require,
            "verify-full" => PgSslMode::VerifyFull,
            _ => return Err(RuntimeError::Configuration("OKD_DB_SSL_MODE is invalid")),
        };
        let database_ssl_root_cert = env::var_os("OKD_DB_SSL_ROOT_CERT").map(PathBuf::from);
        validate_transport_security(
            mode,
            &vault_addr,
            database_ssl_mode,
            database_ssl_root_cert.is_some(),
        )?;
        let sms_gateway_url = match env::var("OKD_SMS_GATEWAY_URL") {
            Ok(value) if !value.trim().is_empty() => Some(parse_sms_gateway_url(&value, mode)?),
            Ok(_) => None,
            Err(env::VarError::NotPresent) => None,
            Err(env::VarError::NotUnicode(_)) => {
                return Err(RuntimeError::Configuration(
                    "OKD_SMS_GATEWAY_URL must be valid text",
                ));
            }
        };

        let minimum_lease_remaining =
            Duration::from_secs(parse_u64("OKD_MINIMUM_LEASE_REMAINING_SECONDS", 300)?);
        if minimum_lease_remaining < Duration::from_secs(30) {
            return Err(RuntimeError::Configuration(
                "minimum lease safety window must be at least 30 seconds",
            ));
        }

        Ok(Self {
            mode,
            http_addr,
            vault_addr,
            sms_gateway_url,
            approle_file: PathBuf::from(env_value(
                "OKD_VAULT_APPROLE_FILE",
                "../infra/.local/vault/backend-approle.env",
            )),
            database_host: env_value("OKD_DB_HOST", "127.0.0.1"),
            database_port: parse_u16("OKD_DB_PORT", 5432)?,
            database_name: env_value("OKD_DB_NAME", "okuldayim"),
            database_ssl_mode,
            database_ssl_root_cert,
            database_max_connections: parse_u32("OKD_DB_MAX_CONNECTIONS", 12)?,
            redis_host: env_value("OKD_REDIS_HOST", "127.0.0.1"),
            redis_port: parse_u16("OKD_REDIS_PORT", 6379)?,
            minimum_lease_remaining,
        })
    }
}

fn parse_sms_gateway_url(value: &str, mode: RuntimeMode) -> Result<Url, RuntimeError> {
    let url = Url::parse(value)
        .map_err(|_| RuntimeError::Configuration("OKD_SMS_GATEWAY_URL is invalid"))?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.path().ends_with("/send")
        || url.query().is_some()
        || url.fragment().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(RuntimeError::Configuration(
            "OKD_SMS_GATEWAY_URL must be an HTTP(S) Jasmin /send URL without credentials or query parameters",
        ));
    }
    if mode == RuntimeMode::Production && url.scheme() != "https" {
        return Err(RuntimeError::Configuration(
            "production requires HTTPS for the SMS gateway",
        ));
    }
    Ok(url)
}

#[derive(Clone)]
pub struct VaultAppRole {
    pub role_id: String,
    pub secret_id: SecretString,
}

impl VaultAppRole {
    pub fn read(path: &std::path::Path) -> Result<Self, RuntimeError> {
        let contents = std::fs::read_to_string(path)
            .map_err(|_| RuntimeError::Configuration("cannot read Vault AppRole file"))?;
        parse_approle_contents(&contents)
    }
}

fn env_value(name: &str, default: &str) -> String {
    env::var(name).unwrap_or_else(|_| default.to_owned())
}

fn parse_u16(name: &'static str, default: u16) -> Result<u16, RuntimeError> {
    env_value(name, &default.to_string())
        .parse()
        .map_err(|_| RuntimeError::Configuration("a numeric port setting is invalid"))
}

fn parse_u32(name: &'static str, default: u32) -> Result<u32, RuntimeError> {
    let value: u32 = env_value(name, &default.to_string())
        .parse()
        .map_err(|_| RuntimeError::Configuration("a numeric pool setting is invalid"))?;
    if !(1..=100).contains(&value) {
        return Err(RuntimeError::Configuration(
            "OKD_DB_MAX_CONNECTIONS must be between 1 and 100",
        ));
    }
    Ok(value)
}

fn parse_u64(name: &'static str, default: u64) -> Result<u64, RuntimeError> {
    env_value(name, &default.to_string())
        .parse()
        .map_err(|_| RuntimeError::Configuration("a numeric duration setting is invalid"))
}

fn validate_transport_security(
    mode: RuntimeMode,
    vault_addr: &Url,
    database_ssl_mode: PgSslMode,
    has_database_root_cert: bool,
) -> Result<(), RuntimeError> {
    if mode == RuntimeMode::Production && vault_addr.scheme() != "https" {
        return Err(RuntimeError::Configuration(
            "production requires TLS for Vault",
        ));
    }
    if mode == RuntimeMode::Production
        && (!matches!(database_ssl_mode, PgSslMode::VerifyFull) || !has_database_root_cert)
    {
        return Err(RuntimeError::Configuration(
            "production requires PostgreSQL verify-full TLS and OKD_DB_SSL_ROOT_CERT",
        ));
    }
    Ok(())
}

fn parse_approle_contents(contents: &str) -> Result<VaultAppRole, RuntimeError> {
    let mut role_id = None;
    let mut secret_id = None;
    for line in contents.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(RuntimeError::Configuration(
                "Vault AppRole file has invalid format",
            ));
        };
        match key.trim() {
            "VAULT_ROLE_ID" => role_id = Some(value.trim().to_owned()),
            "VAULT_SECRET_ID" => secret_id = Some(value.trim().to_owned()),
            _ => {}
        }
    }
    let role_id = role_id
        .filter(|value| !value.is_empty())
        .ok_or(RuntimeError::Configuration(
            "Vault AppRole file is missing VAULT_ROLE_ID",
        ))?;
    let secret_id =
        secret_id
            .filter(|value| !value.is_empty())
            .ok_or(RuntimeError::Configuration(
                "Vault AppRole file is missing VAULT_SECRET_ID",
            ))?;
    Ok(VaultAppRole {
        role_id,
        secret_id: SecretString::new(secret_id.into()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::ExposeSecret;

    #[test]
    fn production_rejects_plaintext_vault_address() {
        let address = Url::parse("http://localhost:8200").expect("valid URL");
        assert!(
            validate_transport_security(
                RuntimeMode::Production,
                &address,
                PgSslMode::Require,
                false
            )
            .is_err()
        );
        assert!(
            validate_transport_security(
                RuntimeMode::Development,
                &address,
                PgSslMode::Disable,
                false
            )
            .is_ok()
        );
    }

    #[test]
    fn approle_parser_ignores_vault_addr_but_requires_both_credentials() {
        let role = parse_approle_contents(
            "VAULT_ADDR=http://127.0.0.1:8200\nVAULT_ROLE_ID=role\nVAULT_SECRET_ID=secret\n",
        )
        .expect("complete AppRole settings");
        assert_eq!(role.role_id, "role");
        assert_eq!(role.secret_id.expose_secret(), "secret");
        assert!(parse_approle_contents("VAULT_ROLE_ID=role\n").is_err());
    }

    #[test]
    fn sms_gateway_url_requires_jasmin_send_endpoint_and_tls_in_production() {
        let local = parse_sms_gateway_url("http://jasmin:1401/send", RuntimeMode::Development)
            .expect("local Jasmin endpoint");
        assert_eq!(local.path(), "/send");
        assert!(parse_sms_gateway_url("http://jasmin:1401/send", RuntimeMode::Production).is_err());
        assert!(parse_sms_gateway_url("https://jasmin:1401/", RuntimeMode::Development).is_err());
        assert!(
            parse_sms_gateway_url(
                "https://user:pass@jasmin:1401/send",
                RuntimeMode::Production
            )
            .is_err()
        );
    }
}
