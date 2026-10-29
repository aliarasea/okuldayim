use std::{
    env,
    net::{IpAddr, Ipv4Addr, SocketAddr},
};

use axum::{Extension, extract::ConnectInfo, serve};
use reqwest::{Client, Response, StatusCode};
use serde_json::{Value, json};
use sha2::Digest;
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use tokio::net::TcpListener;
use uuid::Uuid;

use crate::{
    config::{RuntimeConfig, RuntimeMode, VaultAppRole},
    http,
    integrations::{postgres::PostgresIntegration, redis::RedisIntegration, vault::VaultClient},
};
use okuldayim_api::sms::JasminSmsProvider;

const ORGANIZATION_ID: &str = "019a5000-0000-7000-8000-000000000001";
const SCHOOL_YEAR_ID: &str = "019a5000-0000-7000-8000-000000000002";
const CLASS_ID: &str = "019a5000-0000-7000-8000-000000000003";
const GUARDIAN_ID: &str = "019a5000-0000-7000-8000-000000000004";
const TEACHER_ID: &str = "019a5000-0000-7000-8000-000000000005";
const CHILD_ONE_ID: &str = "019a5000-0000-7000-8000-000000000006";
const CHILD_TWO_ID: &str = "019a5000-0000-7000-8000-000000000007";
const PICKUP_PERSON_ID: &str = "019a5000-0000-7000-8000-000000000008";
const PICKUP_PERSON_TWO_ID: &str = "019a5000-0000-7000-8000-000000000009";
const FIXTURE_NAME: &str = "Okuldayım Phase 1a E2E synthetic tenant";

struct FixturePhones {
    guardian: String,
    teacher: String,
    invited: String,
    unknown: String,
}

impl FixturePhones {
    fn new() -> Self {
        let base = Uuid::now_v7().as_u128() % 9_999_996;
        let phone = |offset| format!("+1555{:07}", (base + offset) % 10_000_000);
        Self {
            guardian: phone(1),
            teacher: phone(2),
            invited: phone(3),
            unknown: phone(4),
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn phase1a_use_cases_over_real_http_postgres_redis_and_vault() {
    if env::var("OKD_RUN_E2E").as_deref() != Ok("1") {
        return;
    }
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::WARN)
        .try_init();
    assert_eq!(env::var("OKD_ENV").as_deref(), Ok("test"));
    let result = run_phase1a_e2e().await;
    if let Err(error) = &result {
        eprintln!("Phase 1a E2E failed: {error}");
    }
    result.expect("Phase 1a HTTP E2E must pass");
}

async fn run_phase1a_e2e() -> Result<(), String> {
    let admin = connect_test_database().await?;
    verify_fixture_owner(&admin).await?;

    let config = RuntimeConfig::from_env().map_err(|_| "invalid runtime test configuration")?;
    if config.mode != RuntimeMode::Test {
        return Err("test OTP provider requires OKD_ENV=test".to_owned());
    }
    let phones = FixturePhones::new();
    let approle = VaultAppRole::read(&config.approle_file)
        .map_err(|_| "cannot read the local backend AppRole fixture")?;
    let vault = VaultClient::new(config.vault_addr.clone(), approle)
        .map_err(|_| "cannot initialize Vault client")?;
    vault
        .authenticate()
        .await
        .map_err(|_| "Vault authentication failed")?;
    let secrets = vault
        .backend_secrets()
        .await
        .map_err(|_| "Vault secrets unavailable")?;
    let jwt_keys = vault
        .jwt_public_keys()
        .await
        .map_err(|_| "Vault JWT key unavailable")?;
    vault
        .ensure_organization_key(uuid(ORGANIZATION_ID))
        .await
        .map_err(|_| "cannot prepare the synthetic tenant Transit key")?;

    cleanup_fixture(&admin).await?;
    seed_fixture(&admin, &vault, &phones).await?;

    let postgres = PostgresIntegration::connect(&config, vault.clone())
        .await
        .map_err(|_| "API PostgreSQL pool failed")?;
    let redis = RedisIntegration::new(&config.redis_host, config.redis_port, &secrets)
        .map_err(|_| "Redis configuration failed")?;
    let test_ip = Ipv4Addr::from(0x7f00_0002 + (Uuid::now_v7().as_u128() % 253) as u32);
    let test_peer = SocketAddr::new(IpAddr::V4(test_ip), 40_000);
    let app = http::router(
        postgres,
        redis,
        None::<JasminSmsProvider>,
        vault,
        secrets,
        RuntimeMode::Test,
        jwt_keys,
    )
    .layer(Extension(ConnectInfo(test_peer)));
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|_| "cannot bind the E2E API listener")?;
    let address = listener
        .local_addr()
        .map_err(|_| "cannot inspect E2E listener")?;
    let base_url = format!("http://{address}");
    let server = tokio::spawn(async move {
        serve(listener, app.into_make_service())
            .await
            .expect("E2E API server");
    });
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|_| "cannot initialize E2E HTTP client")?;
    let mut result = exercise_use_cases(&client, &base_url, &admin, &phones).await;
    server.abort();
    if let Err(cleanup_error) = cleanup_fixture(&admin).await {
        result = Err(cleanup_error);
    }
    admin.close().await;
    result
}

async fn exercise_use_cases(
    client: &Client,
    base_url: &str,
    admin: &PgPool,
    phones: &FixturePhones,
) -> Result<(), String> {
    let readiness = client
        .get(format!("{base_url}/readyz"))
        .send()
        .await
        .map_err(|_| "API readiness request failed")?;
    require_status(readiness, StatusCode::OK).await?;

    let openapi_response = client
        .get(format!("{base_url}/openapi.json"))
        .send()
        .await
        .map_err(|_| "OpenAPI request failed")?;
    let openapi = require_status(openapi_response, StatusCode::OK).await?;
    for path in [
        "/v1/auth/otp",
        "/v1/me",
        "/v1/classes/{class_id}/meal-records",
        "/v1/pickup-requests",
        "/v1/privacy/data-exports",
        "/v1/audit/events",
    ] {
        if openapi["paths"].get(path).is_none() {
            return Err(format!("OpenAPI document omits implemented path: {path}"));
        }
    }

    let unauthenticated = client
        .get(format!("{base_url}/v1/me"))
        .send()
        .await
        .map_err(|_| "unauthenticated /me request failed")?;
    require_status(unauthenticated, StatusCode::UNAUTHORIZED).await?;

    let (unknown_challenge, unknown_code, unknown_status, unknown_expiry, unknown_retry) =
        request_otp(client, base_url, &phones.unknown).await?;
    let unknown_verify = verify_otp(
        client,
        base_url,
        &unknown_challenge,
        &unknown_code,
        "unknown-device",
    )
    .await?;
    require_status(unknown_verify, StatusCode::UNAUTHORIZED).await?;

    let (guardian_challenge, guardian_code, guardian_status, guardian_expiry, guardian_retry) =
        request_otp(client, base_url, &phones.guardian).await?;
    if unknown_status != guardian_status
        || unknown_expiry != guardian_expiry
        || unknown_retry != guardian_retry
    {
        return Err("OTP response differs between registered and unregistered numbers".to_owned());
    }
    let wrong_code = if guardian_code == "000000" {
        "000001"
    } else {
        "000000"
    };
    for _ in 0..4 {
        let invalid = verify_otp(
            client,
            base_url,
            &guardian_challenge,
            wrong_code,
            "guardian-device",
        )
        .await?;
        require_status(invalid, StatusCode::BAD_REQUEST).await?;
    }
    let guardian_tokens = verify_otp(
        client,
        base_url,
        &guardian_challenge,
        &guardian_code,
        "guardian-device",
    )
    .await?;
    let guardian_tokens = require_status(guardian_tokens, StatusCode::OK).await?;
    let guardian_access = string_field(&guardian_tokens, "accessToken")?;
    let guardian_refresh = string_field(&guardian_tokens, "refreshToken")?;

    let me = client
        .get(format!("{base_url}/v1/me"))
        .bearer_auth(&guardian_access)
        .send()
        .await
        .map_err(|_| "/me request failed")?;
    let me = require_status(me, StatusCode::OK).await?;
    let has_expected_child = me["memberships"].as_array().is_some_and(|memberships| {
        memberships.iter().any(|membership| {
            membership["children"].as_array().is_some_and(|children| {
                children
                    .iter()
                    .any(|child| child["id"] == CHILD_ONE_ID && child["firstName"] == "Ada")
            })
        })
    });
    if me["user"]["id"] != GUARDIAN_ID
        || !has_expected_child
        || me["pendingNoticeKurumIds"]
            .as_array()
            .is_none_or(|values| values.is_empty())
    {
        return Err(
            "/me response did not include tenant-scoped identity/onboarding state".to_owned(),
        );
    }

    let notice_url = format!("{base_url}/v1/organizations/{ORGANIZATION_ID}/privacy-notice");
    let notice_response = client
        .get(&notice_url)
        .bearer_auth(&guardian_access)
        .send()
        .await
        .map_err(|_| "privacy notice request failed")?;
    let notice = require_status(notice_response, StatusCode::OK).await?;
    if notice["acknowledged"] != false || notice["version"] != "e2e-v1" {
        return Err("current privacy notice DTO is incorrect".to_owned());
    }
    let ack = client
        .post(format!("{notice_url}/e2e-v1/acknowledgement"))
        .bearer_auth(&guardian_access)
        .send()
        .await
        .map_err(|_| "privacy acknowledgement request failed")?;
    require_status(ack, StatusCode::NO_CONTENT).await?;

    let consent_url = format!("{base_url}/v1/children/{CHILD_ONE_ID}/consents");
    let consent_response = client
        .get(&consent_url)
        .bearer_auth(&guardian_access)
        .send()
        .await
        .map_err(|_| "consent GET failed")?;
    let consent = require_status(consent_response, StatusCode::OK).await?;
    if consent["items"]
        .as_array()
        .and_then(|items| items.iter().find(|item| item["type"] == "SAGLIK"))
        .is_none_or(|item| item["effective"] != true)
    {
        return Err("effective health consent was not computed from the DB".to_owned());
    }

    let (teacher_challenge, teacher_code, _, _, _) =
        request_otp(client, base_url, &phones.teacher).await?;
    let teacher_tokens = verify_otp(
        client,
        base_url,
        &teacher_challenge,
        &teacher_code,
        "teacher-device",
    )
    .await?;
    let teacher_tokens = require_status(teacher_tokens, StatusCode::OK).await?;
    let teacher_access = string_field(&teacher_tokens, "accessToken")?;

    let today_response = client
        .get(format!("{base_url}/v1/classes/{CLASS_ID}/today"))
        .bearer_auth(&teacher_access)
        .send()
        .await
        .map_err(|_| "class today request failed")?;
    let today = require_status(today_response, StatusCode::OK).await?;
    let date = string_field(&today, "date")?;
    if today["children"]
        .as_array()
        .and_then(|children| children.iter().find(|child| child["id"] == CHILD_ONE_ID))
        .is_none_or(|child| child["allergens"] != json!(["GLUTEN"]))
        || today["menu"].as_array().is_none_or(Vec::is_empty)
    {
        return Err("teacher class DTO did not honor active health consent/menu data".to_owned());
    }

    let feed = client
        .get(format!("{base_url}/v1/children/{CHILD_ONE_ID}/feed"))
        .bearer_auth(&guardian_access)
        .query(&[("date", date.as_str())])
        .send()
        .await
        .map_err(|_| "parent feed request failed")?;
    let feed = require_status(feed, StatusCode::OK).await?;
    if feed["entries"].as_array().is_none_or(Vec::is_empty) {
        return Err("daily feed did not return the synthetic daily record".to_owned());
    }
    let forbidden_child = client
        .get(format!("{base_url}/v1/children/{CLASS_ID}/feed"))
        .bearer_auth(&guardian_access)
        .send()
        .await
        .map_err(|_| "out-of-scope child request failed")?;
    require_status(forbidden_child, StatusCode::FORBIDDEN).await?;

    let deny_health = client
        .put(&consent_url)
        .bearer_auth(&guardian_access)
        .json(&json!({
            "textVersion": "e2e-v1",
            "decisions": [{ "type": "SAGLIK", "decision": "DENIED" }]
        }))
        .send()
        .await
        .map_err(|_| "consent update request failed")?;
    let deny_health = require_status(deny_health, StatusCode::OK).await?;
    if deny_health["items"]
        .as_array()
        .and_then(|items| items.iter().find(|item| item["type"] == "SAGLIK"))
        .is_none_or(|item| item["effective"] != false)
    {
        return Err("consent withdrawal did not become effective".to_owned());
    }
    let after_denial = client
        .get(format!("{base_url}/v1/classes/{CLASS_ID}/today"))
        .bearer_auth(&teacher_access)
        .send()
        .await
        .map_err(|_| "class today after consent withdrawal failed")?;
    let after_denial = require_status(after_denial, StatusCode::OK).await?;
    if after_denial["children"]
        .as_array()
        .and_then(|children| children.iter().find(|child| child["id"] == CHILD_ONE_ID))
        .is_none_or(|child| !child["allergens"].is_null())
    {
        return Err("health fields were not suppressed after consent withdrawal".to_owned());
    }

    let meal_body = json!({
        "date": date,
        "slot": "OGLE",
        "entries": [{ "childId": CHILD_ONE_ID, "amount": "COGU" }]
    });
    let meal_key = Uuid::now_v7().to_string();
    let meal_url = format!("{base_url}/v1/classes/{CLASS_ID}/meal-records");
    let meal = client
        .post(&meal_url)
        .bearer_auth(&teacher_access)
        .header("Idempotency-Key", &meal_key)
        .json(&meal_body)
        .send()
        .await
        .map_err(|_| "meal-record POST failed")?;
    let meal = require_status(meal, StatusCode::CREATED).await?;
    if meal["saved"] != 1 {
        return Err("meal endpoint response count is incorrect".to_owned());
    }
    let meal_replay = client
        .post(&meal_url)
        .bearer_auth(&teacher_access)
        .header("Idempotency-Key", &meal_key)
        .json(&meal_body)
        .send()
        .await
        .map_err(|_| "meal idempotency replay failed")?;
    let meal_replay = require_status(meal_replay, StatusCode::CREATED).await?;
    if meal_replay != meal {
        return Err("meal replay did not return its stored response".to_owned());
    }
    let mismatched_replay = client
        .post(&meal_url)
        .bearer_auth(&teacher_access)
        .header("Idempotency-Key", &meal_key)
        .json(&json!({
            "date": date,
            "slot": "OGLE",
            "entries": [{ "childId": CHILD_ONE_ID, "amount": "AZ" }]
        }))
        .send()
        .await
        .map_err(|_| "mismatched meal replay failed")?;
    require_status(mismatched_replay, StatusCode::CONFLICT).await?;
    let meal_count = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM app.daily_record
         WHERE organization_id = $1 AND child_id = $2 AND record_type = 'OGUN'
           AND date = $3::date AND details_json->>'slot' = 'OGLE'",
    )
    .bind(uuid(ORGANIZATION_ID))
    .bind(uuid(CHILD_ONE_ID))
    .bind(&date)
    .fetch_one(admin)
    .await
    .map_err(|_| "cannot verify unique persisted meal write")?;
    if meal_count != 1 {
        return Err("idempotent meal request created duplicate DB rows".to_owned());
    }

    let people_response = client
        .get(format!(
            "{base_url}/v1/children/{CHILD_ONE_ID}/authorized-pickup-people"
        ))
        .bearer_auth(&guardian_access)
        .send()
        .await
        .map_err(|_| "authorized pickup people request failed")?;
    let people = require_status(people_response, StatusCode::OK).await?;
    if people.as_array().is_none_or(|items| {
        !items
            .iter()
            .any(|item| item["id"] == PICKUP_PERSON_ID && item["verified"] == true)
    }) {
        return Err("verified authorized pickup person missing from API response".to_owned());
    }

    let active_before = client
        .get(format!(
            "{base_url}/v1/children/{CHILD_ONE_ID}/pickup-requests/active"
        ))
        .bearer_auth(&guardian_access)
        .send()
        .await
        .map_err(|_| "active pickup request failed")?;
    let active_before = require_status(active_before, StatusCode::OK).await?;
    if !active_before.is_null() {
        return Err("unexpected active pickup request before test creation".to_owned());
    }
    let pickup_body = json!({
        "childId": CHILD_ONE_ID,
        "pickupPersonId": PICKUP_PERSON_ID,
        "etaMinutes": 12
    });
    let pickup_key = Uuid::now_v7().to_string();
    let pickup_url = format!("{base_url}/v1/pickup-requests");
    let pickup_response = client
        .post(&pickup_url)
        .bearer_auth(&guardian_access)
        .header("Idempotency-Key", &pickup_key)
        .json(&pickup_body)
        .send()
        .await
        .map_err(|_| "pickup request creation failed")?;
    let pickup = require_status(pickup_response, StatusCode::CREATED).await?;
    let pickup_id = string_field(&pickup, "id")?;
    let pin = string_field(&pickup, "pin")?;
    let pickup_replay = client
        .post(&pickup_url)
        .bearer_auth(&guardian_access)
        .header("Idempotency-Key", &pickup_key)
        .json(&pickup_body)
        .send()
        .await
        .map_err(|_| "pickup idempotency replay failed")?;
    let pickup_replay = require_status(pickup_replay, StatusCode::CREATED).await?;
    if pickup_replay != pickup || pickup_replay["pin"] != pin {
        return Err("pickup replay lost the encrypted, requester-only PIN response".to_owned());
    }
    let active = client
        .get(format!(
            "{base_url}/v1/children/{CHILD_ONE_ID}/pickup-requests/active"
        ))
        .bearer_auth(&guardian_access)
        .send()
        .await
        .map_err(|_| "active pickup request read failed")?;
    let active = require_status(active, StatusCode::OK).await?;
    if active["pin"] != pin {
        return Err("requester could not retrieve the still-active PIN".to_owned());
    }
    let queue = client
        .get(format!("{base_url}/v1/classes/{CLASS_ID}/pickup-queue"))
        .bearer_auth(&teacher_access)
        .send()
        .await
        .map_err(|_| "teacher pickup queue request failed")?;
    let queue = require_status(queue, StatusCode::OK).await?;
    if queue
        .as_array()
        .and_then(|items| items.iter().find(|item| item["id"] == pickup_id))
        .is_none_or(|item| !item["pin"].is_null())
    {
        return Err("teacher pickup queue leaked the PIN".to_owned());
    }
    let preparing = client
        .post(format!(
            "{base_url}/v1/pickup-requests/{pickup_id}/preparing"
        ))
        .bearer_auth(&teacher_access)
        .send()
        .await
        .map_err(|_| "pickup preparing transition failed")?;
    let preparing = require_status(preparing, StatusCode::OK).await?;
    if preparing["status"] != "HAZIRLANIYOR" || !preparing["pin"].is_null() {
        return Err("preparing transition has the wrong DTO or disclosed PIN".to_owned());
    }
    let wrong_pin = if pin == "0000" { "0001" } else { "0000" };
    let invalid_pin = client
        .post(format!(
            "{base_url}/v1/pickup-requests/{pickup_id}/handover"
        ))
        .bearer_auth(&teacher_access)
        .json(&json!({ "pin": wrong_pin }))
        .send()
        .await
        .map_err(|_| "invalid PIN attempt failed")?;
    require_status(invalid_pin, StatusCode::UNPROCESSABLE_ENTITY).await?;
    let handover = client
        .post(format!(
            "{base_url}/v1/pickup-requests/{pickup_id}/handover"
        ))
        .bearer_auth(&teacher_access)
        .json(&json!({ "pin": pin }))
        .send()
        .await
        .map_err(|_| "valid PIN handover failed")?;
    let handover = require_status(handover, StatusCode::OK).await?;
    if handover["status"] != "TESLIM_EDILDI" || !handover["pin"].is_null() {
        return Err("handover did not consume the PIN and finish the state transition".to_owned());
    }

    let blocked_request = client
        .post(&pickup_url)
        .bearer_auth(&guardian_access)
        .header("Idempotency-Key", Uuid::now_v7().to_string())
        .json(&json!({
            "childId": CHILD_TWO_ID,
            "pickupPersonId": PICKUP_PERSON_TWO_ID,
            "etaMinutes": 20
        }))
        .send()
        .await
        .map_err(|_| "second pickup request creation failed")?;
    let blocked_request = require_status(blocked_request, StatusCode::CREATED).await?;
    let blocked_id = string_field(&blocked_request, "id")?;
    let blocked_pin = string_field(&blocked_request, "pin")?;
    let wrong_block_pin = if blocked_pin == "0000" {
        "0001"
    } else {
        "0000"
    };
    for attempt in 1..=3 {
        let response = client
            .post(format!(
                "{base_url}/v1/pickup-requests/{blocked_id}/handover"
            ))
            .bearer_auth(&teacher_access)
            .json(&json!({ "pin": wrong_block_pin }))
            .send()
            .await
            .map_err(|_| "pickup PIN lockout request failed")?;
        let expected = if attempt == 3 {
            StatusCode::LOCKED
        } else {
            StatusCode::UNPROCESSABLE_ENTITY
        };
        require_status(response, expected).await?;
    }
    let blocked_active = client
        .get(format!(
            "{base_url}/v1/children/{CHILD_TWO_ID}/pickup-requests/active"
        ))
        .bearer_auth(&guardian_access)
        .send()
        .await
        .map_err(|_| "blocked pickup state read failed")?;
    let blocked_active = require_status(blocked_active, StatusCode::OK).await?;
    if blocked_active["status"] != "ENGELLENDI" || !blocked_active["pin"].is_null() {
        return Err(
            "third wrong PIN did not lock the request and erase its display PIN".to_owned(),
        );
    }

    let audit_key = Uuid::now_v7().to_string();
    let audit_url = format!("{base_url}/v1/audit/events");
    for _ in 0..2 {
        let response = client
            .post(&audit_url)
            .bearer_auth(&guardian_access)
            .header("Idempotency-Key", &audit_key)
            .json(&json!({
                "type": "EKRAN_GORUNTUSU",
                "screen": "teslim-kodu",
                "at": "2026-10-30T08:00:00.000Z"
            }))
            .send()
            .await
            .map_err(|_| "audit event outbox request failed")?;
        require_status(response, StatusCode::NO_CONTENT).await?;
    }
    let audit_count = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM app.audit_record
         WHERE organization_id = $1 AND actor_user_id = $2 AND action = 'EKRAN_GORUNTUSU'",
    )
    .bind(uuid(ORGANIZATION_ID))
    .bind(uuid(GUARDIAN_ID))
    .fetch_one(admin)
    .await
    .map_err(|_| "cannot verify persisted audit event")?;
    if audit_count != 1 {
        return Err("audit event idempotency created duplicate records".to_owned());
    }

    let privacy_create = client
        .post(format!("{base_url}/v1/privacy/requests"))
        .bearer_auth(&guardian_access)
        .json(&json!({
            "kurumId": ORGANIZATION_ID,
            "type": "BILGI",
            "description": "synthetic E2E request"
        }))
        .send()
        .await
        .map_err(|_| "privacy request creation failed")?;
    require_status(privacy_create, StatusCode::CREATED).await?;
    let privacy_list = client
        .get(format!("{base_url}/v1/privacy/requests"))
        .bearer_auth(&guardian_access)
        .send()
        .await
        .map_err(|_| "privacy request list failed")?;
    let privacy_list = require_status(privacy_list, StatusCode::OK).await?;
    if privacy_list.as_array().is_none_or(|items| items.is_empty()) {
        return Err("privacy request was not visible to its owner".to_owned());
    }
    let stored_description = sqlx::query_scalar::<_, String>(
        "SELECT description_ciphertext FROM app.privacy_request
         WHERE organization_id = $1 AND user_id = $2 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(uuid(ORGANIZATION_ID))
    .bind(uuid(GUARDIAN_ID))
    .fetch_one(admin)
    .await
    .map_err(|_| "cannot verify encrypted privacy request")?;
    if stored_description.contains("synthetic E2E request") {
        return Err("privacy description was stored as plaintext".to_owned());
    }

    let export = client
        .post(format!("{base_url}/v1/privacy/data-exports"))
        .bearer_auth(&guardian_access)
        .json(&json!({ "childId": CHILD_ONE_ID }))
        .send()
        .await
        .map_err(|_| "data export request failed")?;
    let export = require_status(export, StatusCode::ACCEPTED).await?;
    if export["linkValidHours"] != 24 {
        return Err("data export request did not return its 24-hour link contract".to_owned());
    }

    let (invite_challenge, invite_code, _, _, _) =
        request_otp(client, base_url, &phones.invited).await?;
    let invited_tokens = verify_otp(
        client,
        base_url,
        &invite_challenge,
        &invite_code,
        "invited-user-device",
    )
    .await?;
    let invited_tokens = require_status(invited_tokens, StatusCode::OK).await?;
    let invited_me_response = client
        .get(format!("{base_url}/v1/me"))
        .bearer_auth(string_field(&invited_tokens, "accessToken")?)
        .send()
        .await
        .map_err(|_| "new invited user /me request failed")?;
    let invited_me = require_status(invited_me_response, StatusCode::OK).await?;
    let invited_user_id = string_field(&invited_me["user"], "id")?;
    let has_invited_child_scope = invited_me["memberships"]
        .as_array()
        .is_some_and(|memberships| {
            memberships.iter().any(|membership| {
                membership["children"].as_array().is_some_and(|children| {
                    children.iter().any(|child| child["id"] == CHILD_TWO_ID)
                })
            })
        });
    if invited_user_id == GUARDIAN_ID || invited_user_id == TEACHER_ID || !has_invited_child_scope {
        return Err(
            "verified invitation did not atomically create the expected guardian scope".to_owned(),
        );
    }
    let invitations_left = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM app.invitation
         WHERE organization_id = $1 AND status = 'BEKLIYOR'
           AND target_type = 'TELEFON'",
    )
    .bind(uuid(ORGANIZATION_ID))
    .fetch_one(admin)
    .await
    .map_err(|_| "cannot verify consumed invitation state")?;
    if invitations_left != 0 {
        return Err("phone invitation was not consumed".to_owned());
    }

    let rotated = client
        .post(format!("{base_url}/v1/auth/refresh"))
        .json(&json!({ "refreshToken": guardian_refresh }))
        .send()
        .await
        .map_err(|_| "refresh request failed")?;
    let rotated = require_status(rotated, StatusCode::OK).await?;
    let rotated_access = string_field(&rotated, "accessToken")?;
    let rotated_refresh = string_field(&rotated, "refreshToken")?;
    if rotated_refresh == guardian_refresh {
        return Err("refresh token did not rotate".to_owned());
    }
    let replay = client
        .post(format!("{base_url}/v1/auth/refresh"))
        .json(&json!({ "refreshToken": guardian_refresh }))
        .send()
        .await
        .map_err(|_| "refresh replay request failed")?;
    require_status(replay, StatusCode::UNAUTHORIZED).await?;
    let revoked = client
        .get(format!("{base_url}/v1/me"))
        .bearer_auth(rotated_access)
        .send()
        .await
        .map_err(|_| "post-replay access check failed")?;
    require_status(revoked, StatusCode::UNAUTHORIZED).await?;

    Ok(())
}

async fn request_otp(
    client: &Client,
    base_url: &str,
    phone: &str,
) -> Result<(String, String, StatusCode, u64, u64), String> {
    let response = client
        .post(format!("{base_url}/v1/auth/otp"))
        .json(&json!({ "phone": phone }))
        .send()
        .await
        .map_err(|_| "OTP request failed")?;
    let status = response.status();
    let body: Value = response
        .json()
        .await
        .map_err(|_| "OTP response was not JSON")?;
    if status != StatusCode::OK {
        return Err("OTP request did not return the generic success response".to_owned());
    }
    Ok((
        string_field(&body, "challengeId")?,
        string_field(&body, "testCode")?,
        status,
        body["expiresInSec"].as_u64().ok_or("missing OTP TTL")?,
        body["retryAfterSec"]
            .as_u64()
            .ok_or("missing OTP retry interval")?,
    ))
}

async fn verify_otp(
    client: &Client,
    base_url: &str,
    challenge_id: &str,
    code: &str,
    device_id: &str,
) -> Result<Response, String> {
    client
        .post(format!("{base_url}/v1/auth/otp/verify"))
        .json(&json!({
            "challengeId": challenge_id,
            "code": code,
            "device": { "id": device_id, "platform": "ios" }
        }))
        .send()
        .await
        .map_err(|_| "OTP verification request failed".to_owned())
}

async fn require_status(response: Response, expected: StatusCode) -> Result<Value, String> {
    let actual = response.status();
    if actual != expected {
        let path = response.url().path().to_owned();
        let error_code = response
            .json::<Value>()
            .await
            .ok()
            .and_then(|body| body["error"]["code"].as_str().map(str::to_owned));
        return Err(format!(
            "unexpected API status at {path}: expected {expected}, got {actual}{}",
            error_code
                .map(|code| format!(" (error: {code})"))
                .unwrap_or_default()
        ));
    }
    if expected == StatusCode::NO_CONTENT {
        return Ok(Value::Null);
    }
    response
        .json()
        .await
        .map_err(|_| format!("expected JSON body for status {expected}"))
}

fn string_field(value: &Value, field: &str) -> Result<String, String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("missing response field: {field}"))
}

fn uuid(value: &str) -> Uuid {
    Uuid::parse_str(value).expect("static synthetic UUID")
}

async fn connect_test_database() -> Result<PgPool, String> {
    let host = env::var("OKD_TEST_DB_HOST").unwrap_or_else(|_| "127.0.0.1".to_owned());
    if !matches!(host.as_str(), "127.0.0.1" | "localhost" | "::1") {
        return Err("E2E seed database must be loopback-only".to_owned());
    }
    let database = env::var("OKD_TEST_DB_NAME").unwrap_or_else(|_| "okuldayim".to_owned());
    if database != "okuldayim" {
        return Err("E2E seed database must be the local synthetic okuldayim database".to_owned());
    }
    let password = env::var("POSTGRES_SUPERUSER_PASSWORD")
        .map_err(|_| "POSTGRES_SUPERUSER_PASSWORD must be loaded from the local infra .env")?;
    let port = env::var("POSTGRES_PORT")
        .unwrap_or_else(|_| "5432".to_owned())
        .parse::<u16>()
        .map_err(|_| "invalid local PostgreSQL test port")?;
    let options = PgConnectOptions::new()
        .host(&host)
        .port(port)
        .database(&database)
        .username("postgres")
        .password(&password)
        .application_name("okuldayim-phase1a-e2e");
    PgPoolOptions::new()
        .max_connections(2)
        .connect_with(options)
        .await
        .map_err(|_| "cannot connect to the local synthetic PostgreSQL database".to_owned())
}

async fn verify_fixture_owner(pool: &PgPool) -> Result<(), String> {
    let name =
        sqlx::query_scalar::<_, Option<String>>("SELECT name FROM app.organization WHERE id = $1")
            .bind(uuid(ORGANIZATION_ID))
            .fetch_optional(pool)
            .await
            .map_err(|_| "cannot inspect E2E fixture marker")?;
    if let Some(name) = name.flatten()
        && name != FIXTURE_NAME
    {
        return Err("fixed E2E fixture ID belongs to a non-test organization".to_owned());
    }
    Ok(())
}

async fn seed_fixture(
    pool: &PgPool,
    vault: &VaultClient,
    phones: &FixturePhones,
) -> Result<(), String> {
    let organization_id = uuid(ORGANIZATION_ID);
    let guardian_id = uuid(GUARDIAN_ID);
    let teacher_id = uuid(TEACHER_ID);
    let child_one = uuid(CHILD_ONE_ID);
    let child_two = uuid(CHILD_TWO_ID);
    let invite_phone_index = phone_index(vault, &phones.invited).await?;
    let guardian_phone_index = phone_index(vault, &phones.guardian).await?;
    let teacher_phone_index = phone_index(vault, &phones.teacher).await?;

    let guardian_phone_cipher = transit_encrypt(vault, "kullanici-pii", &phones.guardian).await?;
    let teacher_phone_cipher = transit_encrypt(vault, "kullanici-pii", &phones.teacher).await?;
    let guardian_first = transit_encrypt(vault, "kullanici-pii", "Test").await?;
    let guardian_last = transit_encrypt(vault, "kullanici-pii", "Veli").await?;
    let teacher_first = transit_encrypt(vault, "kullanici-pii", "Test").await?;
    let teacher_last = transit_encrypt(vault, "kullanici-pii", "Ogretmen").await?;
    let tenant_key = format!("kurum-{organization_id}");
    let child_one_first = transit_encrypt(vault, &tenant_key, "Ada").await?;
    let child_one_last = transit_encrypt(vault, &tenant_key, "Yilmaz").await?;
    let child_two_first = transit_encrypt(vault, &tenant_key, "Ece").await?;
    let child_two_last = transit_encrypt(vault, &tenant_key, "Kaya").await?;
    let health_profile = transit_encrypt(vault, &tenant_key, r#"{"allergens":["GLUTEN"]}"#).await?;
    let person_name = transit_encrypt(vault, &tenant_key, "Ayse Nine").await?;
    let person_two_name = transit_encrypt(vault, &tenant_key, "Murat Amca").await?;
    let person_phone = transit_encrypt(vault, &tenant_key, "+15550005555").await?;
    let person_two_phone = transit_encrypt(vault, &tenant_key, "+15550006666").await?;
    let person_phone_index = vault
        .transit_hmac("blind-index", b"TELEFON\0+15550005555")
        .await
        .map_err(|_| "Vault test person blind index failed")?;
    let person_two_phone_index = vault
        .transit_hmac("blind-index", b"TELEFON\0+15550006666")
        .await
        .map_err(|_| "Vault second test person blind index failed")?;
    let invitation_cipher = transit_encrypt(vault, "kullanici-pii", &phones.invited).await?;
    let mut invitation_hash = [0_u8; 32];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut invitation_hash);
    let notice_text = "Synthetic privacy notice for Phase 1a E2E.";
    let notice_hash = sha2::Sha256::digest(notice_text.as_bytes());

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| "cannot begin seed transaction")?;
    sqlx::query(
        "INSERT INTO app.organization (id, name, transit_key_name, timezone)
         VALUES ($1, $2, $3, 'Europe/Istanbul')",
    )
    .bind(organization_id)
    .bind(FIXTURE_NAME)
    .bind(&tenant_key)
    .execute(&mut *tx)
    .await
    .map_err(|_| "cannot seed synthetic organization")?;
    sqlx::query(
        "INSERT INTO app.school_year (id, organization_id, name, start_date, end_date)
         VALUES ($1, $2, 'E2E synthetic year', '2025-09-01', '2027-08-31')",
    )
    .bind(uuid(SCHOOL_YEAR_ID))
    .bind(organization_id)
    .execute(&mut *tx)
    .await
    .map_err(|_| "cannot seed synthetic school year")?;
    sqlx::query(
        "INSERT INTO app.classroom (id, organization_id, school_year_id, name)
         VALUES ($1, $2, $3, 'E2E synthetic class')",
    )
    .bind(uuid(CLASS_ID))
    .bind(organization_id)
    .bind(uuid(SCHOOL_YEAR_ID))
    .execute(&mut *tx)
    .await
    .map_err(|_| "cannot seed synthetic classroom")?;
    sqlx::query(
        "INSERT INTO app.user_account (id, first_name_ciphertext, last_name_ciphertext)
         VALUES ($1, $2, $3), ($4, $5, $6)",
    )
    .bind(guardian_id)
    .bind(guardian_first)
    .bind(guardian_last)
    .bind(teacher_id)
    .bind(teacher_first)
    .bind(teacher_last)
    .execute(&mut *tx)
    .await
    .map_err(|_| "cannot seed synthetic user accounts")?;
    sqlx::query(
        "INSERT INTO app.user_contact
           (user_id, type, value_ciphertext, blind_index, blind_index_version, verified_at, is_primary)
         VALUES ($1, 'TELEFON', $2, $3, 1, clock_timestamp(), true),
                ($4, 'TELEFON', $5, $6, 1, clock_timestamp(), true)",
    )
    .bind(guardian_id)
    .bind(guardian_phone_cipher)
    .bind(guardian_phone_index)
    .bind(teacher_id)
    .bind(teacher_phone_cipher)
    .bind(teacher_phone_index)
    .execute(&mut *tx)
    .await
    .map_err(|_| "cannot seed encrypted phone contacts")?;
    sqlx::query(
        "INSERT INTO app.membership (organization_id, user_id, role, status)
         VALUES ($1, $2, 'VELI', 'AKTIF'), ($1, $3, 'OGRETMEN', 'AKTIF')",
    )
    .bind(organization_id)
    .bind(guardian_id)
    .bind(teacher_id)
    .execute(&mut *tx)
    .await
    .map_err(|_| "cannot seed synthetic memberships")?;
    let teacher_membership = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM app.membership WHERE organization_id = $1 AND user_id = $2",
    )
    .bind(organization_id)
    .bind(teacher_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| "cannot resolve synthetic teacher membership")?;
    sqlx::query(
        "INSERT INTO app.membership_class (organization_id, membership_id, class_id)
         VALUES ($1, $2, $3)",
    )
    .bind(organization_id)
    .bind(teacher_membership)
    .bind(uuid(CLASS_ID))
    .execute(&mut *tx)
    .await
    .map_err(|_| "cannot seed teacher class assignment")?;
    sqlx::query(
        "INSERT INTO app.child
           (id, organization_id, class_id, first_name_ciphertext, last_name_ciphertext, enrollment_date)
         VALUES ($1, $2, $3, $4, $5, (clock_timestamp() AT TIME ZONE 'Europe/Istanbul')::date),
                ($6, $2, $3, $7, $8, (clock_timestamp() AT TIME ZONE 'Europe/Istanbul')::date)",
    )
    .bind(child_one)
    .bind(organization_id)
    .bind(uuid(CLASS_ID))
    .bind(child_one_first)
    .bind(child_one_last)
    .bind(child_two)
    .bind(child_two_first)
    .bind(child_two_last)
    .execute(&mut *tx)
    .await
    .map_err(|_| "cannot seed synthetic children")?;
    sqlx::query(
        "INSERT INTO app.guardian_child (organization_id, guardian_user_id, child_id, relationship)
         VALUES ($1, $2, $3, 'ANNE'), ($1, $2, $4, 'ANNE')",
    )
    .bind(organization_id)
    .bind(guardian_id)
    .bind(child_one)
    .bind(child_two)
    .execute(&mut *tx)
    .await
    .map_err(|_| "cannot seed guardian-child authorization")?;
    let notice_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO app.privacy_notice
           (organization_id, audience, version, title, summary, full_text, content_sha256, effective_at)
         VALUES ($1, 'VELI', 'e2e-v1', 'Synthetic notice', 'Synthetic summary', $2, $3, clock_timestamp())
         RETURNING id",
    )
    .bind(organization_id)
    .bind(notice_text)
    .bind(notice_hash.as_slice())
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| "cannot seed synthetic privacy notice")?;
    sqlx::query(
        "INSERT INTO app.consent_record
           (organization_id, child_id, guardian_user_id, consent_type, decision, notice_id, channel)
         VALUES ($1, $2, $3, 'SAGLIK', 'ONAY', $4, 'MOBIL')",
    )
    .bind(organization_id)
    .bind(child_one)
    .bind(guardian_id)
    .bind(notice_id)
    .execute(&mut *tx)
    .await
    .map_err(|_| "cannot seed synthetic health consent")?;
    sqlx::query(
        "INSERT INTO app.health_profile
           (organization_id, child_id, content_ciphertext, key_version, updated_by_user_id)
         VALUES ($1, $2, $3, 1, $4)",
    )
    .bind(organization_id)
    .bind(child_one)
    .bind(health_profile)
    .bind(guardian_id)
    .execute(&mut *tx)
    .await
    .map_err(|_| "cannot seed encrypted synthetic health profile")?;
    let meal_menu_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO app.meal_menu
           (organization_id, start_date, end_date, status, created_by_user_id, published_at)
         VALUES ($1, '2025-01-01', '2030-12-31', 'YAYINDA', $2, clock_timestamp())
         RETURNING id",
    )
    .bind(organization_id)
    .bind(teacher_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| "cannot seed synthetic meal menu")?;
    let meal_definition_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO app.meal_definition
           (organization_id, menu_id, date, type, name, description, sort_order)
         VALUES ($1, $2, (clock_timestamp() AT TIME ZONE 'Europe/Istanbul')::date,
                 'OGLE', 'Synthetic lunch', 'Synthetic ingredients', 1)
         RETURNING id",
    )
    .bind(organization_id)
    .bind(meal_menu_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| "cannot seed synthetic meal definition")?;
    sqlx::query(
        "INSERT INTO app.meal_allergen (organization_id, meal_definition_id, allergen_code)
         VALUES ($1, $2, 'GLUTEN')",
    )
    .bind(organization_id)
    .bind(meal_definition_id)
    .execute(&mut *tx)
    .await
    .map_err(|_| "cannot seed synthetic menu allergen")?;
    sqlx::query(
        "INSERT INTO app.authorized_person
           (id, organization_id, added_by_user_id, first_name_ciphertext, phone_ciphertext,
            phone_blind_index, relationship, is_verified, approved_at, valid_from, valid_until)
         VALUES ($1, $2, $3, $4, $5, $6, 'Babaanne', true, clock_timestamp(),
                 clock_timestamp() - interval '1 day', clock_timestamp() + interval '1 day'),
                ($7, $2, $3, $8, $9, $10, 'Amca', true, clock_timestamp(),
                 clock_timestamp() - interval '1 day', clock_timestamp() + interval '1 day')",
    )
    .bind(uuid(PICKUP_PERSON_ID))
    .bind(organization_id)
    .bind(guardian_id)
    .bind(person_name)
    .bind(person_phone)
    .bind(person_phone_index)
    .bind(uuid(PICKUP_PERSON_TWO_ID))
    .bind(person_two_name)
    .bind(person_two_phone)
    .bind(person_two_phone_index)
    .execute(&mut *tx)
    .await
    .map_err(|_| "cannot seed synthetic authorized pickup people")?;
    sqlx::query(
        "INSERT INTO app.authorized_user_child
           (organization_id, authorized_person_id, child_id, added_by_guardian_user_id)
         VALUES ($1, $2, $3, $4), ($1, $5, $6, $4)",
    )
    .bind(organization_id)
    .bind(uuid(PICKUP_PERSON_ID))
    .bind(child_one)
    .bind(guardian_id)
    .bind(uuid(PICKUP_PERSON_TWO_ID))
    .bind(child_two)
    .execute(&mut *tx)
    .await
    .map_err(|_| "cannot seed authorized-person child links")?;
    sqlx::query(
        "INSERT INTO app.invitation
           (organization_id, created_by_user_id, target_type, target_ciphertext,
            target_blind_index, blind_index_version, role, token_hash, expires_at)
         VALUES ($1, $2, 'TELEFON', $3, $4, 1, 'VELI', $5,
                 clock_timestamp() + interval '1 day')",
    )
    .bind(organization_id)
    .bind(teacher_id)
    .bind(invitation_cipher)
    .bind(invite_phone_index)
    .bind(invitation_hash.as_slice())
    .execute(&mut *tx)
    .await
    .map_err(|_| "cannot seed phone invitation")?;
    let invitation_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM app.invitation WHERE organization_id = $1 AND target_blind_index = $2",
    )
    .bind(organization_id)
    .bind(phone_index(vault, &phones.invited).await?)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| "cannot resolve synthetic invitation")?;
    sqlx::query(
        "INSERT INTO app.invitation_scope (organization_id, invitation_id, child_id)
         VALUES ($1, $2, $3)",
    )
    .bind(organization_id)
    .bind(invitation_id)
    .bind(child_two)
    .execute(&mut *tx)
    .await
    .map_err(|_| "cannot seed invitation child scope")?;
    sqlx::query(
        "INSERT INTO app.daily_record
           (organization_id, child_id, record_type, data_category, date, occurred_at,
            details_json, created_by_user_id)
         SELECT $1, $2, 'AKTIVITE', 'GUNLUK',
                (clock_timestamp() AT TIME ZONE 'Europe/Istanbul')::date,
                clock_timestamp(), jsonb_build_object('activityType', 'Synthetic activity', 'note', 'E2E'), $3",
    )
    .bind(organization_id)
    .bind(child_one)
    .bind(teacher_id)
    .execute(&mut *tx)
    .await
    .map_err(|_| "cannot seed daily feed record")?;
    tx.commit()
        .await
        .map_err(|_| "cannot commit E2E fixture".to_owned())
}

async fn cleanup_fixture(pool: &PgPool) -> Result<(), String> {
    let organization_id = uuid(ORGANIZATION_ID);
    let marker =
        sqlx::query_scalar::<_, Option<String>>("SELECT name FROM app.organization WHERE id = $1")
            .bind(organization_id)
            .fetch_optional(pool)
            .await
            .map_err(|_| "cannot inspect E2E tenant during cleanup")?
            .flatten();
    if marker.is_none() {
        return Ok(());
    }
    if marker.as_deref() != Some(FIXTURE_NAME) {
        return Err("refusing to clean a non-E2E organization".to_owned());
    }
    let mut user_ids = vec![uuid(GUARDIAN_ID), uuid(TEACHER_ID)];
    user_ids.extend(
        sqlx::query_scalar::<_, Uuid>(
            "SELECT user_id FROM app.membership WHERE organization_id = $1",
        )
        .bind(organization_id)
        .fetch_all(pool)
        .await
        .map_err(|_| "cannot enumerate synthetic tenant users for cleanup")?,
    );
    user_ids.sort_unstable();
    user_ids.dedup();
    for statement in [
        "DELETE FROM app.idempotency_key WHERE organization_id = $1",
        "DELETE FROM app.audit_record WHERE organization_id = $1",
        "DELETE FROM app.audit_chain_head WHERE organization_id = $1",
        "DELETE FROM app.data_export_request WHERE organization_id = $1",
        "DELETE FROM app.privacy_request WHERE organization_id = $1",
        "DELETE FROM app.daily_record WHERE organization_id = $1",
        "DELETE FROM app.pickup_request WHERE organization_id = $1",
        "DELETE FROM app.consent_record WHERE organization_id = $1",
        "DELETE FROM app.consent_review WHERE organization_id = $1",
        "DELETE FROM app.privacy_notice_read WHERE organization_id = $1",
        "DELETE FROM app.health_profile WHERE organization_id = $1",
        "DELETE FROM app.authorized_user_child WHERE organization_id = $1",
        "DELETE FROM app.authorized_person WHERE organization_id = $1",
        "DELETE FROM app.invitation_scope WHERE organization_id = $1",
        "DELETE FROM app.invitation WHERE organization_id = $1",
        "DELETE FROM app.meal_allergen WHERE organization_id = $1",
        "DELETE FROM app.meal_definition WHERE organization_id = $1",
        "DELETE FROM app.meal_menu WHERE organization_id = $1",
        "DELETE FROM app.guardian_child WHERE organization_id = $1",
        "DELETE FROM app.membership_class WHERE organization_id = $1",
    ] {
        sqlx::query(statement)
            .bind(organization_id)
            .execute(pool)
            .await
            .map_err(|_| "failed to clean synthetic tenant records")?;
    }
    loop {
        let deleted = sqlx::query(
            "DELETE FROM app.refresh_token AS token
             WHERE token.user_id = ANY($1)
               AND NOT EXISTS (
                 SELECT 1 FROM app.refresh_token AS child_token
                 WHERE child_token.previous_token_id = token.id
               )",
        )
        .bind(&user_ids)
        .execute(pool)
        .await
        .map_err(|_| "failed to clean synthetic refresh-token chain")?;
        if deleted.rows_affected() == 0 {
            break;
        }
    }
    for statement in [
        "DELETE FROM app.session WHERE user_id = ANY($1)",
        "DELETE FROM app.device WHERE user_id = ANY($1)",
        "DELETE FROM app.user_contact WHERE user_id = ANY($1)",
    ] {
        sqlx::query(statement)
            .bind(&user_ids)
            .execute(pool)
            .await
            .map_err(|_| "failed to clean synthetic user records")?;
    }
    sqlx::query("DELETE FROM app.membership WHERE organization_id = $2 OR user_id = ANY($1)")
        .bind(&user_ids)
        .bind(organization_id)
        .execute(pool)
        .await
        .map_err(|_| "failed to clean synthetic memberships")?;
    for statement in [
        "DELETE FROM app.child WHERE organization_id = $1",
        "DELETE FROM app.classroom WHERE organization_id = $1",
        "DELETE FROM app.school_year WHERE organization_id = $1",
        "DELETE FROM app.privacy_notice WHERE organization_id = $1",
        "DELETE FROM app.organization WHERE id = $1",
    ] {
        sqlx::query(statement)
            .bind(organization_id)
            .execute(pool)
            .await
            .map_err(|_| "failed to clean synthetic tenant records")?;
    }
    sqlx::query("DELETE FROM app.user_account WHERE id = ANY($1)")
        .bind(&user_ids)
        .execute(pool)
        .await
        .map_err(|_| "failed to clean synthetic user accounts")?;
    Ok(())
}

async fn phone_index(vault: &VaultClient, phone: &str) -> Result<Vec<u8>, String> {
    let mut input = b"TELEFON\0".to_vec();
    input.extend_from_slice(phone.as_bytes());
    vault
        .transit_hmac("blind-index", &input)
        .await
        .map_err(|_| "Vault test contact blind-index failed".to_owned())
}

async fn transit_encrypt(vault: &VaultClient, key: &str, value: &str) -> Result<String, String> {
    vault
        .transit_encrypt(key, value.as_bytes())
        .await
        .map(|value| value.0)
        .map_err(|_| "Vault synthetic fixture encryption failed".to_owned())
}
