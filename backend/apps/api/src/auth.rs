use std::net::SocketAddr;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    Json, Router,
    extract::{ConnectInfo, State},
    http::{Request, StatusCode, header::AUTHORIZATION},
    middleware::Next,
    response::{IntoResponse, Response},
    routing::post,
};
use base64::{
    Engine as _,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use hmac::{Hmac, Mac};
use rand::{Rng, RngCore};
use ring::signature::{ECDSA_P256_SHA256_FIXED, UnparsedPublicKey};
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    config::RuntimeMode,
    http::{ApiState, api_error},
};

const OTP_TTL_SECONDS: u64 = 180;
const OTP_MAX_ATTEMPTS: u64 = 5;
const OTP_PHONE_WINDOW_SECONDS: u64 = 600;
const OTP_MAX_PER_PHONE: u64 = 3;
const ACCESS_TTL_SECONDS: u64 = 15 * 60;
const JWT_ISSUER: &str = "okuldayim-api";
const JWT_AUDIENCE: &str = "okuldayim-mobile";

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct Claims {
    pub(crate) sub: Uuid,
    pub(crate) sid: Uuid,
    iat: u64,
    exp: u64,
    iss: String,
    aud: String,
    jti: Uuid,
}

#[derive(Serialize)]
struct JwtHeader {
    alg: &'static str,
    typ: &'static str,
    kid: String,
}

#[derive(Deserialize)]
struct ParsedJwtHeader {
    alg: String,
    typ: String,
    kid: String,
}

#[derive(Deserialize)]
struct OtpRequest {
    phone: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OtpResponse {
    challenge_id: Uuid,
    expires_in_sec: u64,
    retry_after_sec: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    test_code: Option<String>,
}

#[derive(Deserialize)]
struct OtpVerifyRequest {
    #[serde(rename = "challengeId")]
    challenge_id: Uuid,
    code: String,
    device: DeviceInput,
}

#[derive(Deserialize)]
struct DeviceInput {
    id: String,
    platform: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TokenPair {
    access_token: String,
    refresh_token: String,
    expires_in_sec: u64,
}

#[derive(Deserialize)]
struct RefreshRequest {
    #[serde(rename = "refreshToken")]
    refresh_token: String,
}

#[derive(Deserialize)]
struct LogoutRequest {
    #[serde(rename = "refreshToken")]
    refresh_token: String,
}

#[derive(Serialize)]
struct OtpChallengeState {
    #[serde(rename = "codeHash")]
    code_hash: String,
    #[serde(rename = "phoneIndex")]
    phone_index: String,
    #[serde(rename = "phoneIndexVersion")]
    phone_index_version: i16,
    #[serde(rename = "phoneCiphertext")]
    phone_ciphertext: String,
}

#[derive(Deserialize)]
struct OtpVerification {
    status: String,
    #[serde(rename = "phoneIndex")]
    phone_index: Option<String>,
    #[serde(rename = "phoneIndexVersion")]
    phone_index_version: Option<i16>,
    #[serde(rename = "phoneCiphertext")]
    phone_ciphertext: Option<String>,
    #[serde(rename = "attemptsLeft")]
    attempts_left: Option<u64>,
}

pub(crate) fn router() -> Router<ApiState> {
    Router::new()
        .route("/v1/auth/otp", post(request_otp))
        .route("/v1/auth/otp/verify", post(verify_otp))
        .route("/v1/auth/refresh", post(refresh))
        .route("/v1/auth/logout", post(logout))
}

pub(crate) async fn require_auth(
    State(state): State<ApiState>,
    mut request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let Some(token) = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
    else {
        return api_error(StatusCode::UNAUTHORIZED, "unauthorized", json!({}));
    };
    let Some(claims) = verify_access_token(&state, token).await else {
        return api_error(StatusCode::UNAUTHORIZED, "unauthorized", json!({}));
    };
    let mut transaction = match state.postgres.begin_user(claims.sub).await {
        Ok(transaction) => transaction,
        Err(_) => return api_error(StatusCode::SERVICE_UNAVAILABLE, "server", json!({})),
    };
    let session_is_active = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (
           SELECT 1 FROM app.session AS session
           JOIN app.user_account AS account ON account.id = session.user_id
           WHERE session.id = $1 AND session.user_id = $2
             AND session.revoked_at IS NULL AND account.status = 'AKTIF'
         )",
    )
    .bind(claims.sid)
    .bind(claims.sub)
    .fetch_one(&mut *transaction)
    .await;
    match session_is_active {
        Ok(true) => {}
        Ok(false) => {
            let _ = transaction.rollback().await;
            return api_error(StatusCode::UNAUTHORIZED, "unauthorized", json!({}));
        }
        Err(_) => return api_error(StatusCode::SERVICE_UNAVAILABLE, "server", json!({})),
    }
    if transaction.commit().await.is_err() {
        return api_error(StatusCode::SERVICE_UNAVAILABLE, "server", json!({}));
    }
    request.extensions_mut().insert(claims);
    next.run(request).await
}

async fn request_otp(
    State(state): State<ApiState>,
    ConnectInfo(connect_info): ConnectInfo<SocketAddr>,
    Json(input): Json<OtpRequest>,
) -> Response {
    if !is_e164(&input.phone) {
        return api_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "validation",
            json!({ "field": "phone" }),
        );
    }

    let mut scoped_phone = b"TELEFON\0".to_vec();
    scoped_phone.extend_from_slice(input.phone.as_bytes());
    let phone_index = match state.vault.transit_hmac("blind-index", &scoped_phone).await {
        Ok(index) if index.len() == 32 => index,
        _ => return unavailable(),
    };
    let phone_rate_key = format!("otp:rate:phone:{}", URL_SAFE_NO_PAD.encode(&phone_index));
    let phone_count = match state
        .redis
        .increment_window(&phone_rate_key, OTP_PHONE_WINDOW_SECONDS)
        .await
    {
        Ok(count) => count,
        Err(_) => return unavailable(),
    };
    let ip = connect_info.ip().to_string();
    let mut ip_digest = Sha256::new();
    ip_digest.update(ip.as_bytes());
    let ip_rate_key = format!(
        "otp:rate:ip:{}",
        URL_SAFE_NO_PAD.encode(ip_digest.finalize())
    );
    let ip_count = match state
        .redis
        .increment_window(&ip_rate_key, OTP_PHONE_WINDOW_SECONDS)
        .await
    {
        Ok(count) => count,
        Err(_) => return unavailable(),
    };
    if phone_count > OTP_MAX_PER_PHONE || ip_count > 10 {
        return rate_limited(OTP_PHONE_WINDOW_SECONDS);
    }

    let user_id = match state
        .postgres
        .find_user_by_phone_index(1, &phone_index)
        .await
    {
        Ok(user_id) => user_id,
        Err(_) => return unavailable(),
    };
    let has_invitation = match state
        .postgres
        .has_pending_phone_invitation(1, &phone_index)
        .await
    {
        Ok(value) => value,
        Err(_) => return unavailable(),
    };
    let (phone_ciphertext, _) = match state
        .vault
        .transit_encrypt("kullanici-pii", input.phone.as_bytes())
        .await
    {
        Ok(value) => value,
        Err(_) => return unavailable(),
    };

    let challenge_id = Uuid::now_v7();
    let code = format!("{:06}", rand::thread_rng().gen_range(0..1_000_000));
    let code_hash = otp_digest(
        state.secrets.otp_pepper.expose_secret().as_bytes(),
        challenge_id,
        &code,
    );
    let challenge_state = OtpChallengeState {
        code_hash: URL_SAFE_NO_PAD.encode(code_hash),
        phone_index: URL_SAFE_NO_PAD.encode(phone_index),
        phone_index_version: 1,
        phone_ciphertext,
    };
    let serialized = match serde_json::to_string(&challenge_state) {
        Ok(value) => value,
        Err(_) => return unavailable(),
    };
    let challenge_key = format!("otp:challenge:{challenge_id}");
    if state
        .redis
        .set_expiring(&challenge_key, &serialized, OTP_TTL_SECONDS)
        .await
        .is_err()
    {
        return unavailable();
    }

    if state.mode != RuntimeMode::Test
        && (user_id.is_some() || has_invitation)
        && let Some(sms) = state.sms.clone()
    {
        // The API response does not wait for the SMS gateway, keeping registered and
        // unregistered phone responses indistinguishable. No phone or OTP enters logs.
        let phone = input.phone;
        let sms_code = code.clone();
        tokio::spawn(async move {
            let message = format!("Okuldayim dogrulama kodunuz: {sms_code}");
            if sms.send_text(&phone, &message).await.is_err() {
                tracing::warn!("OTP SMS delivery failed");
            }
        });
    }

    let test_code = (state.mode == RuntimeMode::Test).then_some(code);
    Json(OtpResponse {
        challenge_id,
        expires_in_sec: OTP_TTL_SECONDS,
        retry_after_sec: 60,
        test_code,
    })
    .into_response()
}

async fn verify_otp(
    State(state): State<ApiState>,
    Json(input): Json<OtpVerifyRequest>,
) -> Response {
    if input.code.len() != 6 || !input.code.bytes().all(|byte| byte.is_ascii_digit()) {
        return api_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "validation",
            json!({ "field": "code" }),
        );
    }
    if input.device.id.is_empty() || input.device.id.len() > 256 {
        return api_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "validation",
            json!({ "field": "device.id" }),
        );
    }
    let platform = match input.device.platform.as_str() {
        "ios" | "IOS" => "IOS",
        "android" | "ANDROID" => "ANDROID",
        "web" | "WEB" => "WEB",
        _ => {
            return api_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "validation",
                json!({ "field": "device.platform" }),
            );
        }
    };
    let digest = otp_digest(
        state.secrets.otp_pepper.expose_secret().as_bytes(),
        input.challenge_id,
        &input.code,
    );
    let challenge_key = format!("otp:challenge:{}", input.challenge_id);
    let attempts_key = format!("otp:attempts:{}", input.challenge_id);
    let result = match state
        .redis
        .verify_otp(
            &challenge_key,
            &attempts_key,
            &URL_SAFE_NO_PAD.encode(digest),
            OTP_TTL_SECONDS,
            OTP_MAX_ATTEMPTS,
        )
        .await
    {
        Ok(Some(result)) => result,
        Ok(None) => return api_error(StatusCode::BAD_REQUEST, "otp_expired", json!({})),
        Err(_) => return unavailable(),
    };
    let result: OtpVerification = match serde_json::from_str(&result) {
        Ok(result) => result,
        Err(_) => return unavailable(),
    };
    match result.status.as_str() {
        "invalid" => {
            return api_error(
                StatusCode::BAD_REQUEST,
                "otp_invalid",
                json!({ "attemptsLeft": result.attempts_left.unwrap_or(0) }),
            );
        }
        "locked" => return rate_limited(OTP_TTL_SECONDS),
        "verified" => {}
        _ => return api_error(StatusCode::BAD_REQUEST, "otp_expired", json!({})),
    }
    let Some(phone_index) = result
        .phone_index
        .as_deref()
        .and_then(|value| URL_SAFE_NO_PAD.decode(value).ok())
    else {
        return unavailable();
    };
    let phone_index_version = result.phone_index_version.unwrap_or_default();
    let mut user_id = match state
        .postgres
        .find_user_by_phone_index(phone_index_version, &phone_index)
        .await
    {
        Ok(user_id) => user_id,
        Err(_) => {
            tracing::warn!(
                component = "auth",
                operation = "verified_phone_lookup_failed"
            );
            return unavailable();
        }
    };
    if user_id.is_none() {
        let Some(phone_ciphertext) = result.phone_ciphertext.as_deref() else {
            return unavailable();
        };
        user_id = match state
            .postgres
            .register_user_from_phone_invitation(
                Uuid::now_v7(),
                phone_ciphertext,
                phone_index_version,
                &phone_index,
            )
            .await
        {
            Ok(user_id) => user_id,
            Err(_) => {
                tracing::warn!(
                    component = "auth",
                    operation = "invited_user_registration_failed"
                );
                return unavailable();
            }
        };
    }
    let Some(user_id) = user_id else {
        return unauthorized();
    };
    match create_session(&state, user_id, &input.device.id, platform).await {
        Ok(tokens) => Json(tokens).into_response(),
        Err(response) => response,
    }
}

async fn create_session(
    state: &ApiState,
    user_id: Uuid,
    device_id: &str,
    platform: &str,
) -> Result<TokenPair, Response> {
    let session_id = Uuid::now_v7();
    let family_id = Uuid::now_v7();
    let refresh_token = new_refresh_token();
    let refresh_hash = Sha256::digest(refresh_token.as_bytes());
    let access_token = sign_access_token(state, user_id, session_id)
        .await
        .map_err(|response| {
            tracing::warn!(
                component = "auth",
                operation = "access_token_signing_failed"
            );
            response
        })?;
    let mut transaction = state.postgres.begin_user(user_id).await.map_err(|_| {
        tracing::warn!(
            component = "auth",
            operation = "session_transaction_begin_failed"
        );
        unavailable()
    })?;
    let device_hash = Sha256::digest(device_id.as_bytes());
    let device_uuid = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO app.device (user_id, platform, device_hash)
         VALUES ($1, $2, $3)
         ON CONFLICT (user_id, platform, device_hash) DO UPDATE
           SET last_seen_at = clock_timestamp(), revoked_at = NULL
         RETURNING id",
    )
    .bind(user_id)
    .bind(platform)
    .bind(device_hash.as_slice())
    .fetch_one(&mut *transaction)
    .await
    .map_err(|_| {
        tracing::warn!(component = "auth", operation = "device_registration_failed");
        unavailable()
    })?;
    if sqlx::query(
        "INSERT INTO app.session (id, user_id, family_id, device_id)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(session_id)
    .bind(user_id)
    .bind(family_id)
    .bind(device_uuid)
    .execute(&mut *transaction)
    .await
    .is_err()
    {
        tracing::warn!(component = "auth", operation = "session_insert_failed");
        return Err(unavailable());
    }
    if sqlx::query(
        "INSERT INTO app.refresh_token (session_id, user_id, token_hash, expires_at)
         VALUES ($1, $2, $3, clock_timestamp() + interval '30 days')",
    )
    .bind(session_id)
    .bind(user_id)
    .bind(refresh_hash.as_slice())
    .execute(&mut *transaction)
    .await
    .is_err()
    {
        tracing::warn!(
            component = "auth",
            operation = "refresh_token_insert_failed"
        );
        return Err(unavailable());
    }
    transaction.commit().await.map_err(|_| {
        tracing::warn!(
            component = "auth",
            operation = "session_transaction_commit_failed"
        );
        unavailable()
    })?;
    Ok(TokenPair {
        access_token,
        refresh_token,
        expires_in_sec: ACCESS_TTL_SECONDS,
    })
}

async fn refresh(State(state): State<ApiState>, Json(input): Json<RefreshRequest>) -> Response {
    if input.refresh_token.len() > 128 {
        return unauthorized();
    }
    let token_hash = Sha256::digest(input.refresh_token.as_bytes());
    let user_id = match state
        .postgres
        .find_refresh_owner(token_hash.as_slice())
        .await
    {
        Ok(Some(user_id)) => user_id,
        Ok(None) => return unauthorized(),
        Err(_) => return unavailable(),
    };
    let mut transaction = match state.postgres.begin_user(user_id).await {
        Ok(transaction) => transaction,
        Err(_) => return unavailable(),
    };
    let row = sqlx::query_as::<_, (Uuid, Uuid, Uuid, bool, bool, bool)>(
        "SELECT token.id, token.session_id, session.family_id,
                token.consumed_at IS NOT NULL,
                token.expires_at > clock_timestamp(),
                session.revoked_at IS NOT NULL
         FROM app.refresh_token AS token
         JOIN app.session AS session ON session.id = token.session_id
         WHERE token.user_id = $1 AND token.token_hash = $2
         FOR UPDATE OF token",
    )
    .bind(user_id)
    .bind(token_hash.as_slice())
    .fetch_optional(&mut *transaction)
    .await;
    let Some((token_id, session_id, family_id, consumed, unexpired, revoked)) = (match row {
        Ok(row) => row,
        Err(_) => return unavailable(),
    }) else {
        return unauthorized();
    };
    if consumed {
        if sqlx::query(
            "UPDATE app.session SET revoked_at = COALESCE(revoked_at, clock_timestamp()),
                    revocation_reason = 'refresh_replay'
             WHERE user_id = $1 AND family_id = $2",
        )
        .bind(user_id)
        .bind(family_id)
        .execute(&mut *transaction)
        .await
        .is_err()
        {
            return unavailable();
        }
        if transaction.commit().await.is_err() {
            return unavailable();
        }
        return unauthorized();
    }
    if !unexpired || revoked {
        let _ = transaction.rollback().await;
        return unauthorized();
    }

    let new_token = new_refresh_token();
    let new_hash = Sha256::digest(new_token.as_bytes());
    let new_access_token = match sign_access_token(&state, user_id, session_id).await {
        Ok(token) => token,
        Err(response) => return response,
    };
    if sqlx::query("UPDATE app.refresh_token SET consumed_at = clock_timestamp() WHERE id = $1")
        .bind(token_id)
        .execute(&mut *transaction)
        .await
        .is_err()
    {
        return unavailable();
    }
    if sqlx::query(
        "INSERT INTO app.refresh_token (session_id, user_id, token_hash, previous_token_id, expires_at)
         VALUES ($1, $2, $3, $4, clock_timestamp() + interval '30 days')",
    )
    .bind(session_id)
    .bind(user_id)
    .bind(new_hash.as_slice())
    .bind(token_id)
    .execute(&mut *transaction)
    .await
    .is_err()
    {
        return unavailable();
    }
    if sqlx::query("UPDATE app.session SET last_activity_at = clock_timestamp() WHERE id = $1")
        .bind(session_id)
        .execute(&mut *transaction)
        .await
        .is_err()
    {
        return unavailable();
    }
    if transaction.commit().await.is_err() {
        return unavailable();
    }
    Json(TokenPair {
        access_token: new_access_token,
        refresh_token: new_token,
        expires_in_sec: ACCESS_TTL_SECONDS,
    })
    .into_response()
}

async fn logout(State(state): State<ApiState>, Json(input): Json<LogoutRequest>) -> Response {
    if input.refresh_token.len() > 128 {
        return StatusCode::NO_CONTENT.into_response();
    }
    let token_hash = Sha256::digest(input.refresh_token.as_bytes());
    let user_id = match state
        .postgres
        .find_refresh_owner(token_hash.as_slice())
        .await
    {
        Ok(user_id) => user_id,
        // Logout is deliberately idempotent; a dependency outage is not.
        Err(_) => return unavailable(),
    };
    let Some(user_id) = user_id else {
        return StatusCode::NO_CONTENT.into_response();
    };
    let mut transaction = match state.postgres.begin_user(user_id).await {
        Ok(transaction) => transaction,
        Err(_) => return unavailable(),
    };
    if sqlx::query(
        "UPDATE app.session AS session
         SET revoked_at = COALESCE(session.revoked_at, clock_timestamp()),
             revocation_reason = 'logout'
         WHERE session.user_id = $1
           AND session.id IN (
             SELECT token.session_id FROM app.refresh_token AS token
             WHERE token.user_id = $1 AND token.token_hash = $2
           )",
    )
    .bind(user_id)
    .bind(token_hash.as_slice())
    .execute(&mut *transaction)
    .await
    .is_err()
    {
        return unavailable();
    }
    if transaction.commit().await.is_err() {
        return unavailable();
    }
    StatusCode::NO_CONTENT.into_response()
}

async fn sign_access_token(
    state: &ApiState,
    user_id: Uuid,
    session_id: Uuid,
) -> Result<String, Response> {
    let now = unix_time().ok_or_else(unavailable)?;
    let claims = Claims {
        sub: user_id,
        sid: session_id,
        iat: now,
        exp: now + ACCESS_TTL_SECONDS,
        iss: JWT_ISSUER.to_owned(),
        aud: JWT_AUDIENCE.to_owned(),
        jti: Uuid::now_v7(),
    };
    let expected_version = *state.latest_jwt_key_version.read().await;
    let (mut signing_input, signature, version) =
        sign_jwt_parts(state, expected_version, &claims).await?;
    let (signature, version) = if version == expected_version {
        (signature, version)
    } else {
        let (input, signature, actual_version) = sign_jwt_parts(state, version, &claims).await?;
        if actual_version != version {
            return Err(unavailable());
        }
        signing_input = input;
        *state.latest_jwt_key_version.write().await = version;
        (signature, version)
    };
    if signature.len() != 64 {
        tracing::warn!(
            component = "auth",
            operation = "jwt_signature_length_invalid"
        );
        return Err(unavailable());
    }
    if !state.jwt_public_keys.read().await.contains_key(&version) {
        let (latest_version, public_keys) = state
            .vault
            .jwt_public_keys()
            .await
            .map_err(|_| unavailable())?;
        let mut current = state.jwt_public_keys.write().await;
        for key in public_keys {
            if let Some(point) = p256_public_key(&key.pem) {
                current.insert(key.version, point);
            }
        }
        *state.latest_jwt_key_version.write().await = latest_version;
        if !current.contains_key(&version) {
            tracing::warn!(
                component = "auth",
                operation = "jwt_signing_key_version_unavailable"
            );
            return Err(unavailable());
        }
    }
    Ok(format!(
        "{signing_input}.{}",
        URL_SAFE_NO_PAD.encode(signature)
    ))
}

async fn sign_jwt_parts(
    state: &ApiState,
    key_version: u32,
    claims: &Claims,
) -> Result<(String, Vec<u8>, u32), Response> {
    let header = JwtHeader {
        alg: "ES256",
        typ: "JWT",
        kid: format!("jwt-signing:v{key_version}"),
    };
    let encoded_header =
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(&header).map_err(|_| unavailable())?);
    let encoded_claims =
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(claims).map_err(|_| unavailable())?);
    let signing_input = format!("{encoded_header}.{encoded_claims}");
    let (signature, version) = state
        .vault
        .transit_sign_jws(signing_input.as_bytes())
        .await
        .map_err(|_| unavailable())?;
    Ok((signing_input, signature, version))
}

async fn verify_access_token(state: &ApiState, token: &str) -> Option<Claims> {
    if token.len() > 4096 {
        return None;
    }
    let mut parts = token.split('.');
    let (Some(header_part), Some(claims_part), Some(signature_part), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return None;
    };
    let header: ParsedJwtHeader =
        serde_json::from_slice(&URL_SAFE_NO_PAD.decode(header_part).ok()?).ok()?;
    if header.alg != "ES256" || header.typ != "JWT" {
        return None;
    }
    let version = header
        .kid
        .strip_prefix("jwt-signing:v")?
        .parse::<u32>()
        .ok()?;
    let mut public_key = state.jwt_public_keys.read().await.get(&version).cloned();
    if public_key.is_none() {
        let (latest_version, public_keys) = state.vault.jwt_public_keys().await.ok()?;
        let mut current = state.jwt_public_keys.write().await;
        for key in public_keys {
            if let Some(point) = p256_public_key(&key.pem) {
                current.insert(key.version, point);
            }
        }
        *state.latest_jwt_key_version.write().await = latest_version;
        public_key = current.get(&version).cloned();
    }
    let public_key = public_key?;
    let signature = URL_SAFE_NO_PAD.decode(signature_part).ok()?;
    if signature.len() != 64 {
        return None;
    }
    let signing_input = format!("{header_part}.{claims_part}");
    UnparsedPublicKey::new(&ECDSA_P256_SHA256_FIXED, &public_key)
        .verify(signing_input.as_bytes(), &signature)
        .ok()?;
    let claims: Claims = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(claims_part).ok()?).ok()?;
    let now = unix_time()?;
    if claims.iss != JWT_ISSUER
        || claims.aud != JWT_AUDIENCE
        || claims.exp <= now
        || claims.iat > now.saturating_add(30)
        || claims.exp <= claims.iat
        || claims.exp - claims.iat > ACCESS_TTL_SECONDS
    {
        return None;
    }
    Some(claims)
}

pub(crate) fn p256_public_key(pem: &str) -> Option<Vec<u8>> {
    let encoded = pem
        .lines()
        .filter(|line| !line.starts_with("-----"))
        .collect::<String>();
    let der = STANDARD.decode(encoded).ok()?;
    const P256_SPKI_PREFIX: [u8; 26] = [
        0x30, 0x59, 0x30, 0x13, 0x06, 0x07, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01, 0x06, 0x08,
        0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07, 0x03, 0x42, 0x00,
    ];
    let point = der.strip_prefix(&P256_SPKI_PREFIX)?;
    (point.len() == 65 && point.first() == Some(&0x04)).then(|| point.to_vec())
}

fn otp_digest(pepper: &[u8], challenge_id: Uuid, code: &str) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(pepper).expect("HMAC accepts any key length");
    mac.update(challenge_id.as_bytes());
    mac.update(b"\0");
    mac.update(code.as_bytes());
    mac.finalize().into_bytes().to_vec()
}

fn new_refresh_token() -> String {
    let mut token = [0_u8; 32];
    rand::thread_rng().fill_bytes(&mut token);
    URL_SAFE_NO_PAD.encode(token)
}

fn unix_time() -> Option<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|time| time.as_secs())
}

fn is_e164(phone: &str) -> bool {
    let Some(digits) = phone.strip_prefix('+') else {
        return false;
    };
    (2..=15).contains(&digits.len())
        && digits
            .as_bytes()
            .first()
            .is_some_and(|byte| byte.is_ascii_digit() && *byte != b'0')
        && digits.bytes().all(|byte| byte.is_ascii_digit())
}

fn unauthorized() -> Response {
    api_error(StatusCode::UNAUTHORIZED, "unauthorized", json!({}))
}

fn unavailable() -> Response {
    api_error(StatusCode::SERVICE_UNAVAILABLE, "server", json!({}))
}

fn rate_limited(retry_after: u64) -> Response {
    let mut response = api_error(StatusCode::TOO_MANY_REQUESTS, "otp_locked", json!({}));
    response.headers_mut().insert(
        axum::http::header::RETRY_AFTER,
        retry_after
            .to_string()
            .parse()
            .unwrap_or_else(|_| "600".parse().expect("valid header")),
    );
    response
}
