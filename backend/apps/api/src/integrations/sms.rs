use std::time::Duration;

use reqwest::Client;
use secrecy::{ExposeSecret, SecretString};
use thiserror::Error;
use url::Url;

use crate::error::RuntimeError;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone)]
pub struct JasminCredentials {
    pub username: SecretString,
    pub password: SecretString,
}

#[derive(Clone)]
pub struct JasminSmsProvider {
    http: Client,
    send_url: Url,
    credentials: JasminCredentials,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SmsReceipt {
    /// Jasmin's message ID means the gateway accepted the message into its queue;
    /// it is not proof of delivery to the handset.
    pub message_id: String,
}

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum SmsSendError {
    #[error("SMS destination must be an E.164 phone number")]
    InvalidDestination,
    #[error("SMS content must not be empty")]
    EmptyContent,
    #[error("Jasmin rejected the SMS request")]
    Rejected,
    #[error("SMS outcome is unknown; do not retry automatically")]
    OutcomeUnknown,
}

impl JasminSmsProvider {
    pub fn new(send_url: Url, credentials: JasminCredentials) -> Result<Self, RuntimeError> {
        if !matches!(send_url.scheme(), "http" | "https")
            || send_url.host_str().is_none()
            || !send_url.path().ends_with("/send")
            || send_url.query().is_some()
            || send_url.fragment().is_some()
            || !send_url.username().is_empty()
            || send_url.password().is_some()
            || credentials.username.expose_secret().is_empty()
            || credentials.username.expose_secret().chars().count() > 30
            || credentials.password.expose_secret().is_empty()
            || credentials.password.expose_secret().chars().count() > 30
        {
            return Err(RuntimeError::Configuration(
                "Jasmin endpoint or credentials are invalid",
            ));
        }

        let http = Client::builder()
            .timeout(DEFAULT_TIMEOUT)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| RuntimeError::Configuration("cannot initialize SMS HTTP client"))?;
        Ok(Self {
            http,
            send_url,
            credentials,
        })
    }

    /// Sends one text message through Jasmin's HTTP API.
    ///
    /// Jasmin does not document an idempotency key for `/send`. Transport errors,
    /// 5xx responses, and malformed 200 responses are therefore treated as
    /// ambiguous and must not trigger an automatic retry.
    pub async fn send_text(
        &self,
        destination: &str,
        content: &str,
    ) -> Result<SmsReceipt, SmsSendError> {
        if !is_e164(destination) {
            return Err(SmsSendError::InvalidDestination);
        }
        if content.is_empty() {
            return Err(SmsSendError::EmptyContent);
        }

        // UCS-2 is explicit so Turkish and other non-GSM characters are preserved.
        // Jasmin/SMSC may split long messages into multiple billable segments.
        let response = self
            .http
            .post(self.send_url.clone())
            .form(&[
                ("username", self.credentials.username.expose_secret()),
                ("password", self.credentials.password.expose_secret()),
                ("to", destination),
                ("content", content),
                ("coding", "8"),
            ])
            .send()
            .await
            .map_err(|_| SmsSendError::OutcomeUnknown)?;

        let status = response.status();
        if !status.is_success() {
            return if status.is_server_error() {
                Err(SmsSendError::OutcomeUnknown)
            } else {
                Err(SmsSendError::Rejected)
            };
        }

        let body = response
            .text()
            .await
            .map_err(|_| SmsSendError::OutcomeUnknown)?;
        let message_id = parse_acceptance_response(&body).ok_or(SmsSendError::OutcomeUnknown)?;
        Ok(SmsReceipt { message_id })
    }
}

fn is_e164(value: &str) -> bool {
    let Some(digits) = value.strip_prefix('+') else {
        return false;
    };
    let mut characters = digits.chars();
    (2..=15).contains(&digits.len())
        && characters
            .next()
            .is_some_and(|character| character.is_ascii_digit() && character != '0')
        && digits.chars().all(|character| character.is_ascii_digit())
}

fn parse_acceptance_response(body: &str) -> Option<String> {
    let message_id = body.trim().strip_prefix("Success \"")?.strip_suffix('"')?;
    if message_id.is_empty() || message_id.len() > 128 || message_id.chars().any(char::is_control) {
        return None;
    }
    Some(message_id.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Form, Router, extract::State, http::StatusCode, routing::post};
    use secrecy::SecretString;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::{
        net::TcpListener,
        sync::{Mutex, oneshot},
    };

    #[derive(Clone)]
    struct MockState {
        status: StatusCode,
        body: String,
        received: Arc<Mutex<Option<HashMap<String, String>>>>,
    }

    async fn mock_send(
        State(state): State<MockState>,
        Form(form): Form<HashMap<String, String>>,
    ) -> (StatusCode, String) {
        *state.received.lock().await = Some(form);
        (state.status, state.body)
    }

    async fn mock_provider(
        status: StatusCode,
        body: &str,
    ) -> (
        JasminSmsProvider,
        Arc<Mutex<Option<HashMap<String, String>>>>,
        oneshot::Sender<()>,
        tokio::task::JoinHandle<()>,
    ) {
        let received = Arc::new(Mutex::new(None));
        let state = MockState {
            status,
            body: body.to_owned(),
            received: received.clone(),
        };
        let app = Router::new()
            .route("/send", post(mock_send))
            .with_state(state);
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind mock");
        let address = listener.local_addr().expect("mock address");
        let (shutdown_sender, shutdown_receiver) = oneshot::channel();
        let task = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = shutdown_receiver.await;
                })
                .await
                .expect("mock server");
        });
        let provider = JasminSmsProvider::new(
            Url::parse(&format!("http://{address}/send")).expect("mock URL"),
            JasminCredentials {
                username: SecretString::new("local-user".into()),
                password: SecretString::new("local-password".into()),
            },
        )
        .expect("provider config");
        (provider, received, shutdown_sender, task)
    }

    #[tokio::test]
    async fn sends_form_post_and_returns_gateway_queue_id() {
        let (provider, received, shutdown, task) =
            mock_provider(StatusCode::OK, "Success \"msg-123\"").await;

        let receipt = provider
            .send_text("+905551234567", "Giriş kodunuz: 123456")
            .await
            .expect("accepted message");
        assert_eq!(receipt.message_id, "msg-123");

        let form = received.lock().await.clone().expect("form received");
        assert_eq!(form.get("username").map(String::as_str), Some("local-user"));
        assert_eq!(
            form.get("password").map(String::as_str),
            Some("local-password")
        );
        assert_eq!(form.get("to").map(String::as_str), Some("+905551234567"));
        assert_eq!(
            form.get("content").map(String::as_str),
            Some("Giriş kodunuz: 123456")
        );
        assert_eq!(form.get("coding").map(String::as_str), Some("8"));
        let _ = shutdown.send(());
        task.await.expect("mock task");
    }

    #[tokio::test]
    async fn rejection_is_sanitized_and_invalid_destination_is_not_sent() {
        let (provider, received, shutdown, task) = mock_provider(
            StatusCode::FORBIDDEN,
            "Error \"private gateway diagnostic password=hidden\"",
        )
        .await;

        let error = provider
            .send_text("+905551234567", "code")
            .await
            .expect_err("gateway rejection");
        assert_eq!(error, SmsSendError::Rejected);
        assert!(!error.to_string().contains("hidden"));
        assert_eq!(
            provider.send_text("05551234567", "code").await,
            Err(SmsSendError::InvalidDestination)
        );
        assert!(received.lock().await.is_some());
        let _ = shutdown.send(());
        task.await.expect("mock task");
    }

    #[test]
    fn malformed_success_body_is_ambiguous() {
        assert_eq!(parse_acceptance_response("Error \"No route found\""), None);
        assert_eq!(parse_acceptance_response("Success \"\""), None);
        assert_eq!(
            parse_acceptance_response("Success \"message-1\"\n"),
            Some("message-1".into())
        );
    }
}
