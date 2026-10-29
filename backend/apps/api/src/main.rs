#![forbid(unsafe_code)]

mod config;
mod http;
mod integrations;

use std::{error::Error, time::Duration};

use config::{RuntimeConfig, VaultAppRole};
use integrations::{postgres::PostgresIntegration, redis::RedisIntegration, vault::VaultClient};
use okuldayim_api::{error, sms::JasminSmsProvider};
use tokio::{net::TcpListener, sync::broadcast};
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    initialize_tracing();
    let config = RuntimeConfig::from_env()?;
    let approle = VaultAppRole::read(&config.approle_file)?;
    let vault = VaultClient::new(config.vault_addr.clone(), approle)?;
    vault.authenticate().await?;

    // The API does not retain Vault's root token or long-lived database credentials.
    let secrets = vault.backend_secrets().await?;
    let postgres = PostgresIntegration::connect(&config, vault.clone()).await?;
    let redis = RedisIntegration::new(&config.redis_host, config.redis_port, &secrets)?;
    let sms =
        config
            .sms_gateway_url
            .map(|send_url| {
                let credentials = secrets.jasmin_credentials.clone().ok_or(
                    error::RuntimeError::Configuration("Jasmin credentials are missing from Vault"),
                )?;
                JasminSmsProvider::new(send_url, credentials)
            })
            .transpose()?;

    let listener = TcpListener::bind(config.http_addr).await?;
    let app = http::router(postgres.clone(), redis.clone(), sms);
    let (shutdown_sender, _) = broadcast::channel(1);
    let vault_task = {
        let vault = vault.clone();
        let shutdown = shutdown_sender.subscribe();
        tokio::spawn(async move { vault.run_token_renewal(shutdown).await })
    };
    let database_task = {
        let postgres = postgres.clone();
        let shutdown = shutdown_sender.subscribe();
        tokio::spawn(async move { postgres.run_pool_rotation(shutdown).await })
    };

    info!(address = %config.http_addr, environment = ?config.mode, "Okuldayim API is listening");
    let result = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await;

    let _ = shutdown_sender.send(());
    await_shutdown(vault_task).await;
    await_shutdown(database_task).await;
    if let Err(error) = result {
        error!(error = %error, "HTTP server stopped with an error");
        return Err(Box::<dyn Error>::from(error));
    }
    Ok(())
}

fn initialize_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(filter)
        .with_target(false)
        .init();
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let mut terminate = signal(SignalKind::terminate()).expect("install SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = terminate.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

async fn await_shutdown(task: tokio::task::JoinHandle<()>) {
    match tokio::time::timeout(Duration::from_secs(5), task).await {
        Ok(Ok(())) => {}
        Ok(Err(_)) => warn!("A backend maintenance task exited unexpectedly"),
        Err(_) => warn!("A backend maintenance task did not stop before the shutdown deadline"),
    }
}
