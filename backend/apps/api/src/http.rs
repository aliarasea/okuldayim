use std::{collections::HashMap, sync::Arc, time::Duration};

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
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{
    auth,
    config::RuntimeMode,
    integrations::{
        postgres::PostgresIntegration,
        redis::RedisIntegration,
        vault::{BackendSecrets, TransitPublicKey, VaultClient},
    },
    product,
};
use okuldayim_api::sms::JasminSmsProvider;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_REQUEST_BODY_BYTES: usize = 1024 * 1024;

#[derive(Clone)]
pub struct ApiState {
    pub(crate) postgres: PostgresIntegration,
    pub(crate) redis: RedisIntegration,
    pub(crate) sms: Option<JasminSmsProvider>,
    pub(crate) vault: VaultClient,
    pub(crate) secrets: Arc<BackendSecrets>,
    pub(crate) mode: RuntimeMode,
    pub(crate) latest_jwt_key_version: Arc<RwLock<u32>>,
    pub(crate) jwt_public_keys: Arc<RwLock<HashMap<u32, Vec<u8>>>>,
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
    vault: VaultClient,
    secrets: BackendSecrets,
    mode: RuntimeMode,
    jwt_keys: (u32, Vec<TransitPublicKey>),
) -> Router {
    let state = ApiState {
        postgres,
        redis,
        sms,
        vault,
        secrets: Arc::new(secrets),
        mode,
        latest_jwt_key_version: Arc::new(RwLock::new(jwt_keys.0)),
        jwt_public_keys: Arc::new(RwLock::new(
            jwt_keys
                .1
                .into_iter()
                .filter_map(|key| auth::p256_public_key(&key.pem).map(|point| (key.version, point)))
                .collect(),
        )),
    };
    let protected = product::router().route_layer(middleware::from_fn_with_state(
        state.clone(),
        auth::require_auth,
    ));
    Router::new()
        .route("/healthz", get(liveness))
        .route("/readyz", get(readiness))
        .route("/openapi.json", get(openapi))
        .merge(auth::router())
        .merge(protected)
        .with_state(state)
        .layer(DefaultBodyLimit::max(MAX_REQUEST_BODY_BYTES))
        .layer(middleware::from_fn(request_envelope))
}

pub(crate) fn api_error(status: StatusCode, code: &'static str, details: Value) -> Response {
    let mut response = (
        status,
        Json(ErrorEnvelope {
            error: ApiErrorBody { code, details },
        }),
    )
        .into_response();
    if status == StatusCode::SERVICE_UNAVAILABLE {
        response
            .headers_mut()
            .insert(header::RETRY_AFTER, HeaderValue::from_static("1"));
    }
    response
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
            "description": "Phase 1a mobile API. OTP is sent asynchronously through Jasmin when configured; a testCode is returned only when OKD_ENV=test. Data-export requests are accepted and persisted, but export processing is not yet implemented."
        },
        "security": [{ "bearerAuth": [] }],
        "components": {
            "securitySchemes": {
                "bearerAuth": { "type": "http", "scheme": "bearer", "bearerFormat": "JWT" }
            }
        },
        "paths": {
            "/healthz": { "get": { "security": [], "summary": "Process liveness", "responses": { "200": { "description": "The process is alive." } } } },
            "/readyz": { "get": { "security": [], "summary": "Dependency readiness", "responses": { "200": { "description": "Ready or degraded." }, "503": { "description": "Database is unavailable or its credential lease is too close to expiry." } } } },
            "/openapi.json": { "get": { "security": [], "summary": "This OpenAPI document", "responses": { "200": { "description": "OpenAPI JSON document." } } } },
            "/v1/auth/otp": { "post": { "security": [], "summary": "Request a one-time password", "responses": { "200": { "description": "Generic challenge response." }, "422": { "description": "Invalid phone number." }, "429": { "description": "Rate limit exceeded." }, "503": { "description": "A required dependency is unavailable." } } } },
            "/v1/auth/otp/verify": { "post": { "security": [], "summary": "Verify OTP and create a session", "responses": { "200": { "description": "Access and refresh tokens." }, "400": { "description": "OTP is invalid or expired." }, "401": { "description": "Phone is not registered or invited." }, "429": { "description": "Challenge is locked or rate limited." }, "503": { "description": "A required dependency is unavailable." } } } },
            "/v1/auth/refresh": { "post": { "security": [], "summary": "Rotate a refresh token", "responses": { "200": { "description": "Rotated token pair." }, "401": { "description": "Refresh token is invalid or replayed." }, "503": { "description": "Database is unavailable." } } } },
            "/v1/auth/logout": { "post": { "security": [], "summary": "Revoke a session", "responses": { "204": { "description": "Session revoked or already absent." }, "503": { "description": "Database is unavailable." } } } },
            "/v1/me": { "get": { "summary": "Get the authenticated user and memberships", "responses": { "200": { "description": "User, role, children, and onboarding state." }, "401": { "description": "Authentication is required." }, "503": { "description": "A required dependency is unavailable." } } } },
            "/v1/organizations/{organization_id}/privacy-notice": { "parameters": [{ "name": "organization_id", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }], "get": { "summary": "Get the current privacy notice", "responses": { "200": { "description": "Current notice and read state." }, "403": { "description": "Organization access is denied." } } } },
            "/v1/organizations/{organization_id}/privacy-notice/{version}/acknowledgement": { "parameters": [{ "name": "organization_id", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }, { "name": "version", "in": "path", "required": true, "schema": { "type": "string" } }], "post": { "summary": "Record privacy-notice acknowledgement", "responses": { "204": { "description": "Acknowledgement recorded." }, "403": { "description": "Organization access is denied." }, "409": { "description": "Notice version is no longer current." } } } },
            "/v1/children/{child_id}/consents": { "parameters": [{ "name": "child_id", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }], "get": { "summary": "Get child consent state", "responses": { "200": { "description": "Consent decisions and effective state." }, "403": { "description": "Child access is denied." } } }, "put": { "summary": "Record consent decisions", "responses": { "200": { "description": "Updated consent state." }, "403": { "description": "Child access is denied." }, "409": { "description": "Notice version conflict." } } } },
            "/v1/children/{child_id}/feed": { "parameters": [{ "name": "child_id", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }, { "name": "date", "in": "query", "required": false, "schema": { "type": "string", "format": "date" } }], "get": { "summary": "Get a child's daily feed", "responses": { "200": { "description": "Daily feed entries." }, "403": { "description": "Child access is denied." } } } },
            "/v1/classes/{class_id}/today": { "parameters": [{ "name": "class_id", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }], "get": { "summary": "Get class roster, health visibility, and today's menu", "responses": { "200": { "description": "Class snapshot; health data is consent-gated." }, "403": { "description": "Class access is denied." } } } },
            "/v1/classes/{class_id}/meal-records": { "parameters": [{ "name": "class_id", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }], "post": { "summary": "Record meal consumption (requires Idempotency-Key)", "responses": { "201": { "description": "Meal records persisted." }, "403": { "description": "Class access is denied." }, "409": { "description": "Idempotency key conflicts with an earlier request." }, "422": { "description": "Invalid meal data." } } } },
            "/v1/children/{child_id}/authorized-pickup-people": { "parameters": [{ "name": "child_id", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }], "get": { "summary": "List authorized pickup people", "responses": { "200": { "description": "Authorized people visible to the guardian." }, "403": { "description": "Child access is denied." } } } },
            "/v1/children/{child_id}/pickup-requests/active": { "parameters": [{ "name": "child_id", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }], "get": { "summary": "Get the active pickup request", "responses": { "200": { "description": "Active request or null." }, "403": { "description": "Child access is denied." } } } },
            "/v1/pickup-requests": { "post": { "summary": "Create a pickup request (requires Idempotency-Key)", "responses": { "201": { "description": "Pickup request and requester-only PIN." }, "403": { "description": "Child access is denied." }, "409": { "description": "An active request already exists." }, "422": { "description": "Pickup person is not eligible." } } } },
            "/v1/classes/{class_id}/pickup-queue": { "parameters": [{ "name": "class_id", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }], "get": { "summary": "List the class pickup queue", "responses": { "200": { "description": "Queue items; PIN values are always omitted." }, "403": { "description": "Class access is denied." } } } },
            "/v1/pickup-requests/{request_id}/preparing": { "parameters": [{ "name": "request_id", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }], "post": { "summary": "Mark a pickup request as preparing", "responses": { "200": { "description": "Updated pickup request." }, "403": { "description": "Class access is denied." }, "409": { "description": "Invalid state transition." } } } },
            "/v1/pickup-requests/{request_id}/handover": { "parameters": [{ "name": "request_id", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }], "post": { "summary": "Verify PIN and complete handover", "responses": { "200": { "description": "Handover completed." }, "403": { "description": "Class access is denied." }, "409": { "description": "Request is already completed." }, "422": { "description": "PIN is invalid." }, "423": { "description": "Request is locked after failed PIN attempts." } } } },
            "/v1/privacy/requests": { "get": { "summary": "List the authenticated user's privacy requests", "responses": { "200": { "description": "Privacy requests." } } }, "post": { "summary": "Create a privacy request", "responses": { "201": { "description": "Request recorded." }, "403": { "description": "Organization or child access is denied." } } } },
            "/v1/privacy/data-exports": { "post": { "summary": "Queue a personal-data export request", "responses": { "202": { "description": "Request accepted; export processing is not yet implemented." }, "403": { "description": "Child access is denied." } } } },
            "/v1/audit/events": { "post": { "summary": "Record an idempotent mobile audit event", "responses": { "204": { "description": "Event recorded or replayed." }, "400": { "description": "Invalid event payload or missing Idempotency-Key." } } } }
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
