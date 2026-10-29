use std::time::Duration;

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State},
    http::{HeaderValue, Request, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Serialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::integrations::{postgres::PostgresIntegration, redis::RedisIntegration};
use okuldayim_api::sms::JasminSmsProvider;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_REQUEST_BODY_BYTES: usize = 1024 * 1024;

#[derive(Clone)]
pub struct ApiState {
    postgres: PostgresIntegration,
    redis: RedisIntegration,
    sms: Option<JasminSmsProvider>,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[derive(Serialize)]
struct ReadinessResponse {
    status: &'static str,
    components: ReadinessComponents,
}

#[derive(Serialize)]
struct ReadinessComponents {
    database: &'static str,
    redis: &'static str,
    sms_provider: &'static str,
}

#[derive(Serialize)]
struct ErrorEnvelope {
    error: ApiErrorBody,
}

#[derive(Serialize)]
struct ApiErrorBody {
    code: &'static str,
    details: Value,
}

pub fn router(
    postgres: PostgresIntegration,
    redis: RedisIntegration,
    sms: Option<JasminSmsProvider>,
) -> Router {
    let state = ApiState {
        postgres,
        redis,
        sms,
    };
    Router::new()
        .route("/healthz", get(liveness))
        .route("/readyz", get(readiness))
        .route("/openapi.json", get(openapi))
        .with_state(state)
        .layer(DefaultBodyLimit::max(MAX_REQUEST_BODY_BYTES))
        .layer(middleware::from_fn(request_envelope))
}

async fn liveness() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

async fn readiness(State(state): State<ApiState>) -> Response {
    let (database_ready, redis_result) =
        tokio::join!(state.postgres.is_ready(), state.redis.ping());
    let redis_ready = redis_result.is_ok();
    let status = if !database_ready {
        "unavailable"
    } else if !redis_ready {
        "degraded"
    } else {
        "ready"
    };
    let response = ReadinessResponse {
        status,
        components: ReadinessComponents {
            database: if database_ready {
                "ready"
            } else {
                "unavailable"
            },
            redis: if redis_ready { "ready" } else { "unavailable" },
            sms_provider: if state.sms.is_some() {
                "configured"
            } else {
                "disabled"
            },
        },
    };
    let code = if database_ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (code, Json(response)).into_response()
}

async fn openapi() -> Json<Value> {
    Json(json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Okuldayim API",
            "version": "0.1.0",
            "description": "Operational endpoints available in the initial backend runtime. Jasmin transport configuration is reported by /readyz; product SMS flows are not active."
        },
        "paths": {
            "/healthz": { "get": { "summary": "Process liveness", "responses": { "200": { "description": "The process is alive." } } } },
            "/readyz": { "get": { "summary": "Dependency readiness", "responses": { "200": { "description": "Ready or degraded." }, "503": { "description": "Database is unavailable or its credential lease is too close to expiry." } } } },
            "/openapi.json": { "get": { "summary": "This OpenAPI document", "responses": { "200": { "description": "OpenAPI JSON document." } } } }
        }
    }))
}

async fn request_envelope(request: Request<axum::body::Body>, next: Next) -> Response {
    let request_id = Uuid::now_v7().to_string();
    let method = request.method().to_string();
    // Deliberately omit the query string; it may contain sensitive values.
    let path = request.uri().path().to_owned();
    let response = match tokio::time::timeout(REQUEST_TIMEOUT, next.run(request)).await {
        Ok(response) => response,
        Err(_) => {
            let body = ErrorEnvelope {
                error: ApiErrorBody {
                    code: "server",
                    details: json!({}),
                },
            };
            let mut response = (StatusCode::SERVICE_UNAVAILABLE, Json(body)).into_response();
            response
                .headers_mut()
                .insert(header::RETRY_AFTER, HeaderValue::from_static("1"));
            response
        }
    };
    let status = response.status().as_u16();
    tracing::info!(
        request_id = %request_id,
        method = %method,
        path = %path,
        status,
        "HTTP request completed"
    );

    let mut response = response;
    let headers = response.headers_mut();
    let request_id_header = request_id
        .parse::<HeaderValue>()
        .expect("a UUID request ID is a valid HTTP header value");
    headers.insert("x-request-id", request_id_header);
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response
}
