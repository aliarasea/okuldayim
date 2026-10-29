use axum::{
    Extension, Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use hmac::{Hmac, Mac};
use rand::Rng;
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    auth::Claims,
    http::{ApiState, api_error},
    integrations::postgres::TenantContext,
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MeResponse {
    user: MeUser,
    memberships: Vec<MembershipDto>,
    pending_notice_kurum_ids: Vec<Uuid>,
    consent_review_child_ids: Vec<Uuid>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MeUser {
    id: Uuid,
    display_name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MembershipDto {
    id: Uuid,
    kurum_id: Uuid,
    kurum_name: String,
    role: String,
    class_ids: Vec<Uuid>,
    children: Vec<ChildSummary>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ChildSummary {
    id: Uuid,
    first_name: String,
    last_initial: String,
    class_id: Uuid,
    class_name: String,
}

type MembershipRow = (Uuid, Uuid, String, String, String);
type HmacSha256 = Hmac<Sha256>;

#[derive(Clone, Copy)]
struct TenantMembership {
    organization_id: Uuid,
    role: TenantRole,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum TenantRole {
    Guardian,
    Teacher,
    Other,
}

#[derive(Clone, Copy)]
struct ChildContext {
    tenant: TenantContext,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConsentUpdate {
    text_version: String,
    decisions: Vec<ConsentDecisionInput>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConsentDecisionInput {
    #[serde(rename = "type")]
    consent_type: String,
    decision: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConsentItemDto {
    #[serde(rename = "type")]
    consent_type: String,
    my_decision: Option<String>,
    other_guardian_denied: bool,
    effective: bool,
    decided_at: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConsentsDto {
    child_id: Uuid,
    text_version: String,
    items: Vec<ConsentItemDto>,
}

#[derive(Deserialize)]
struct FeedQuery {
    date: Option<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct MealRecordsInput {
    date: String,
    slot: String,
    #[serde(default)]
    recorded_at: Option<String>,
    entries: Vec<MealEntryInput>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct MealEntryInput {
    child_id: Uuid,
    amount: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AuthorizedPersonDto {
    id: Uuid,
    display_name: String,
    relation: String,
    verified: bool,
    valid_until: Option<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct PickupCreateInput {
    child_id: Uuid,
    pickup_person_id: Uuid,
    eta_minutes: i32,
}

#[derive(Deserialize)]
struct HandoverInput {
    pin: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct PickupPersonDto {
    id: Uuid,
    display_name: String,
    relation: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct PickupDto {
    id: Uuid,
    child_id: Uuid,
    child_name: String,
    pickup_person: PickupPersonDto,
    eta_minutes: i32,
    created_at: String,
    status: String,
    pin: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PrivacyRequestInput {
    kurum_id: Uuid,
    #[serde(rename = "type")]
    request_type: String,
    description: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PrivacyRequestDto {
    id: Uuid,
    kurum_id: Uuid,
    #[serde(rename = "type")]
    request_type: String,
    created_at: String,
    due_at: String,
    status: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DataExportInput {
    child_id: Uuid,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DataExportResponse {
    request_id: Uuid,
    link_valid_hours: u8,
}

enum IdempotencyDecision {
    New(Uuid),
    Replay(StatusCode, Value),
}

async fn user_tenant_memberships(
    state: &ApiState,
    user_id: Uuid,
) -> Result<Vec<TenantMembership>, Response> {
    let mut transaction = state
        .postgres
        .begin_user(user_id)
        .await
        .map_err(|_| server_error())?;
    let rows = sqlx::query_as::<_, (Uuid, String, String)>(
        "SELECT organization_id, role, membership_status
         FROM app.list_user_memberships()",
    )
    .fetch_all(&mut *transaction)
    .await
    .map_err(|_| server_error())?;
    transaction.commit().await.map_err(|_| server_error())?;
    Ok(rows
        .into_iter()
        .filter(|(_, _, status)| status == "AKTIF")
        .map(|(organization_id, role, _)| TenantMembership {
            organization_id,
            role: match role.as_str() {
                "VELI" => TenantRole::Guardian,
                "OGRETMEN" => TenantRole::Teacher,
                _ => TenantRole::Other,
            },
        })
        .collect())
}

async fn resolve_organization_member(
    state: &ApiState,
    user_id: Uuid,
    organization_id: Uuid,
) -> Result<Option<TenantContext>, Response> {
    let mut transaction = state
        .postgres
        .begin_tenant(TenantContext {
            user_id,
            organization_id,
        })
        .await
        .map_err(|_| server_error())?;
    let active = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (
           SELECT 1 FROM app.membership
           WHERE organization_id = $1 AND user_id = $2 AND status = 'AKTIF'
         )",
    )
    .bind(organization_id)
    .bind(user_id)
    .fetch_one(&mut *transaction)
    .await
    .map_err(|_| server_error())?;
    transaction.commit().await.map_err(|_| server_error())?;
    Ok(active.then_some(TenantContext {
        user_id,
        organization_id,
    }))
}

async fn resolve_child_context(
    state: &ApiState,
    user_id: Uuid,
    child_id: Uuid,
    require_guardian_write: bool,
) -> Result<Option<ChildContext>, Response> {
    for membership in user_tenant_memberships(state, user_id).await? {
        let context = TenantContext {
            user_id,
            organization_id: membership.organization_id,
        };
        let mut transaction = state
            .postgres
            .begin_tenant(context)
            .await
            .map_err(|_| server_error())?;
        let row = sqlx::query_as::<_, (Uuid, bool, bool)>(
            "SELECT child.class_id,
                    EXISTS (
                      SELECT 1 FROM app.guardian_child AS guardian_child
                      WHERE guardian_child.organization_id = child.organization_id
                        AND guardian_child.child_id = child.id
                        AND guardian_child.guardian_user_id = $2
                        AND guardian_child.is_active
                        AND guardian_child.access_level <> 'KAPALI'
                    ),
                    EXISTS (
                      SELECT 1 FROM app.membership AS membership
                      JOIN app.membership_class AS membership_class
                        ON membership_class.organization_id = membership.organization_id
                       AND membership_class.membership_id = membership.id
                      WHERE membership.organization_id = child.organization_id
                        AND membership.user_id = $2 AND membership.status = 'AKTIF'
                        AND membership.role = 'OGRETMEN'
                        AND membership_class.class_id = child.class_id
                    )
             FROM app.child AS child
             WHERE child.organization_id = $1 AND child.id = $3 AND child.status = 'AKTIF'",
        )
        .bind(membership.organization_id)
        .bind(user_id)
        .bind(child_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|_| server_error())?;
        transaction.commit().await.map_err(|_| server_error())?;
        let Some((_class_id, guardian, teacher)) = row else {
            continue;
        };
        let authorized = if require_guardian_write {
            guardian && membership.role == TenantRole::Guardian
        } else {
            (guardian && membership.role == TenantRole::Guardian)
                || (teacher && membership.role == TenantRole::Teacher)
        };
        return Ok(authorized.then_some(ChildContext { tenant: context }));
    }
    Ok(None)
}

async fn resolve_class_context(
    state: &ApiState,
    user_id: Uuid,
    class_id: Uuid,
) -> Result<Option<(TenantContext, String)>, Response> {
    for membership in user_tenant_memberships(state, user_id).await? {
        if membership.role != TenantRole::Teacher {
            continue;
        }
        let context = TenantContext {
            user_id,
            organization_id: membership.organization_id,
        };
        let mut transaction = state
            .postgres
            .begin_tenant(context)
            .await
            .map_err(|_| server_error())?;
        let class_name = sqlx::query_scalar::<_, String>(
            "SELECT classroom.name
             FROM app.classroom AS classroom
             JOIN app.membership_class AS membership_class
               ON membership_class.organization_id = classroom.organization_id
              AND membership_class.class_id = classroom.id
             JOIN app.membership AS membership
               ON membership.organization_id = membership_class.organization_id
              AND membership.id = membership_class.membership_id
             WHERE classroom.organization_id = $1 AND classroom.id = $2
               AND classroom.is_active AND membership.user_id = $3
               AND membership.role = 'OGRETMEN' AND membership.status = 'AKTIF'
             LIMIT 1",
        )
        .bind(membership.organization_id)
        .bind(class_id)
        .bind(user_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|_| server_error())?;
        transaction.commit().await.map_err(|_| server_error())?;
        if let Some(class_name) = class_name {
            return Ok(Some((context, class_name)));
        }
    }
    Ok(None)
}

fn forbidden() -> Response {
    api_error(StatusCode::FORBIDDEN, "forbidden", json!({}))
}

fn server_error() -> Response {
    api_error(StatusCode::SERVICE_UNAVAILABLE, "server", json!({}))
}

fn validation_error(field: &'static str) -> Response {
    api_error(
        StatusCode::UNPROCESSABLE_ENTITY,
        "validation",
        json!({ "field": field }),
    )
}

async fn get_privacy_notice(
    State(state): State<ApiState>,
    Extension(claims): Extension<Claims>,
    Path(organization_id): Path<Uuid>,
) -> Response {
    let Some(context) =
        (match resolve_organization_member(&state, claims.sub, organization_id).await {
            Ok(context) => context,
            Err(error) => return error,
        })
    else {
        return forbidden();
    };
    let mut transaction = match state.postgres.begin_tenant(context).await {
        Ok(transaction) => transaction,
        Err(_) => return server_error(),
    };
    let row = sqlx::query_as::<_, (Uuid, String, String, String, String, String, bool)>(
        "SELECT notice.id, organization.name, notice.version,
                to_char(notice.effective_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"'),
                notice.summary, notice.full_text,
                EXISTS (
                  SELECT 1 FROM app.privacy_notice_read AS notice_read
                  WHERE notice_read.organization_id = notice.organization_id
                    AND notice_read.notice_id = notice.id AND notice_read.user_id = $2
                )
         FROM app.privacy_notice AS notice
         JOIN app.organization AS organization ON organization.id = notice.organization_id
         WHERE notice.organization_id = $1 AND notice.audience = 'VELI'
           AND notice.effective_at IS NOT NULL AND notice.effective_at <= clock_timestamp()
         ORDER BY notice.effective_at DESC, notice.id DESC
         LIMIT 1",
    )
    .bind(organization_id)
    .bind(claims.sub)
    .fetch_optional(&mut *transaction)
    .await;
    let row = match row {
        Ok(Some(row)) => row,
        Ok(None) => {
            let _ = transaction.rollback().await;
            return api_error(StatusCode::NOT_FOUND, "not_found", json!({}));
        }
        Err(_) => return server_error(),
    };
    if transaction.commit().await.is_err() {
        return server_error();
    }
    let summary: Vec<String> = row
        .4
        .lines()
        .map(str::to_owned)
        .filter(|line| !line.is_empty())
        .collect();
    Json(json!({
        "kurumId": organization_id,
        "kurumName": row.1,
        "version": row.2,
        "publishedAt": row.3,
        "summary": summary,
        "fullText": row.5,
        "acknowledged": row.6
    }))
    .into_response()
}

async fn acknowledge_privacy_notice(
    State(state): State<ApiState>,
    Extension(claims): Extension<Claims>,
    Path((organization_id, version)): Path<(Uuid, String)>,
) -> Response {
    let Some(context) =
        (match resolve_organization_member(&state, claims.sub, organization_id).await {
            Ok(context) => context,
            Err(error) => return error,
        })
    else {
        return forbidden();
    };
    let mut transaction = match state.postgres.begin_tenant(context).await {
        Ok(transaction) => transaction,
        Err(_) => return server_error(),
    };
    let notice_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM app.privacy_notice
         WHERE organization_id = $1 AND audience = 'VELI' AND version = $2
           AND effective_at IS NOT NULL AND effective_at <= clock_timestamp()
         LIMIT 1",
    )
    .bind(organization_id)
    .bind(version)
    .fetch_optional(&mut *transaction)
    .await;
    let Some(notice_id) = (match notice_id {
        Ok(value) => value,
        Err(_) => return server_error(),
    }) else {
        let _ = transaction.rollback().await;
        return api_error(StatusCode::CONFLICT, "conflict", json!({}));
    };
    let lock_key = format!(
        "privacy-notice:{organization_id}:{notice_id}:{}",
        claims.sub
    );
    if sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(lock_key)
        .execute(&mut *transaction)
        .await
        .is_err()
    {
        return server_error();
    }
    let already_read = match sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (
           SELECT 1 FROM app.privacy_notice_read
           WHERE organization_id = $1 AND notice_id = $2 AND user_id = $3
         )",
    )
    .bind(organization_id)
    .bind(notice_id)
    .bind(claims.sub)
    .fetch_one(&mut *transaction)
    .await
    {
        Ok(value) => value,
        Err(_) => return server_error(),
    };
    if !already_read
        && sqlx::query(
            "INSERT INTO app.privacy_notice_read (organization_id, notice_id, user_id, channel)
             VALUES ($1, $2, $3, 'MOBIL')",
        )
        .bind(organization_id)
        .bind(notice_id)
        .bind(claims.sub)
        .execute(&mut *transaction)
        .await
        .is_err()
    {
        return server_error();
    }
    if transaction.commit().await.is_err() {
        return server_error();
    }
    StatusCode::NO_CONTENT.into_response()
}

async fn current_consent_notice(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    organization_id: Uuid,
) -> Result<Option<(Uuid, String)>, Response> {
    sqlx::query_as::<_, (Uuid, String)>(
        "SELECT id, version FROM app.privacy_notice
         WHERE organization_id = $1 AND audience = 'VELI'
           AND effective_at IS NOT NULL AND effective_at <= clock_timestamp()
         ORDER BY effective_at DESC, id DESC LIMIT 1",
    )
    .bind(organization_id)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(|_| server_error())
}

async fn consent_dto(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    context: TenantContext,
    child_id: Uuid,
    notice_version: String,
) -> Result<ConsentsDto, Response> {
    const CONSENT_TYPES: [&str; 6] = [
        "SAGLIK",
        "FOTO_VELI",
        "FOTO_GRUP",
        "FOTO_TANITIM",
        "KONUM",
        "AKIS_YETKILI",
    ];
    let mut items = Vec::with_capacity(CONSENT_TYPES.len());
    for consent_type in CONSENT_TYPES {
        let mine = sqlx::query_as::<_, (String, String)>(
            "SELECT decision,
                    to_char(decision_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"')
             FROM app.consent_record
             WHERE organization_id = $1 AND child_id = $2 AND guardian_user_id = $3
               AND consent_type = $4
             ORDER BY decision_at DESC, id DESC LIMIT 1",
        )
        .bind(context.organization_id)
        .bind(child_id)
        .bind(context.user_id)
        .bind(consent_type)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(|_| server_error())?;
        let other_denied = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (
               SELECT 1 FROM (
                 SELECT DISTINCT ON (guardian_user_id) guardian_user_id, decision
                 FROM app.consent_record
                 WHERE organization_id = $1 AND child_id = $2
                   AND guardian_user_id <> $3 AND consent_type = $4
                 ORDER BY guardian_user_id, decision_at DESC, id DESC
               ) AS latest
               WHERE latest.decision = 'RED'
             )",
        )
        .bind(context.organization_id)
        .bind(child_id)
        .bind(context.user_id)
        .bind(consent_type)
        .fetch_one(&mut **transaction)
        .await
        .map_err(|_| server_error())?;
        let effective = sqlx::query_scalar::<_, Option<bool>>(
            "SELECT is_enabled FROM app.effective_consent
             WHERE organization_id = $1 AND child_id = $2 AND consent_type = $3",
        )
        .bind(context.organization_id)
        .bind(child_id)
        .bind(consent_type)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(|_| server_error())?
        .flatten()
        .unwrap_or(false);
        items.push(ConsentItemDto {
            consent_type: consent_type.to_owned(),
            my_decision: mine.as_ref().map(|row| match row.0.as_str() {
                "ONAY" => "GRANTED".to_owned(),
                _ => "DENIED".to_owned(),
            }),
            other_guardian_denied: other_denied,
            effective,
            decided_at: mine.map(|row| row.1),
        });
    }
    Ok(ConsentsDto {
        child_id,
        text_version: notice_version,
        items,
    })
}

async fn get_consents(
    State(state): State<ApiState>,
    Extension(claims): Extension<Claims>,
    Path(child_id): Path<Uuid>,
) -> Response {
    let Some(context) = (match resolve_child_context(&state, claims.sub, child_id, false).await {
        Ok(context) => context,
        Err(error) => return error,
    }) else {
        return forbidden();
    };
    let mut transaction = match state.postgres.begin_tenant(context.tenant).await {
        Ok(transaction) => transaction,
        Err(_) => return server_error(),
    };
    let is_guardian = match sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (
           SELECT 1 FROM app.guardian_child
           WHERE organization_id = $1 AND child_id = $2
             AND guardian_user_id = $3 AND is_active AND access_level <> 'KAPALI'
         )",
    )
    .bind(context.tenant.organization_id)
    .bind(child_id)
    .bind(claims.sub)
    .fetch_one(&mut *transaction)
    .await
    {
        Ok(value) => value,
        Err(_) => return server_error(),
    };
    if !is_guardian {
        let _ = transaction.rollback().await;
        return forbidden();
    }
    let notice =
        match current_consent_notice(&mut transaction, context.tenant.organization_id).await {
            Ok(notice) => notice,
            Err(error) => return error,
        };
    let dto = match consent_dto(
        &mut transaction,
        context.tenant,
        child_id,
        notice.map(|notice| notice.1).unwrap_or_default(),
    )
    .await
    {
        Ok(dto) => dto,
        Err(error) => return error,
    };
    if transaction.commit().await.is_err() {
        return server_error();
    }
    Json(dto).into_response()
}

async fn update_consents(
    State(state): State<ApiState>,
    Extension(claims): Extension<Claims>,
    Path(child_id): Path<Uuid>,
    Json(input): Json<ConsentUpdate>,
) -> Response {
    const CONSENT_TYPES: [&str; 6] = [
        "SAGLIK",
        "FOTO_VELI",
        "FOTO_GRUP",
        "FOTO_TANITIM",
        "KONUM",
        "AKIS_YETKILI",
    ];
    if input.decisions.is_empty()
        || input.decisions.len() > CONSENT_TYPES.len()
        || input.decisions.iter().any(|decision| {
            !CONSENT_TYPES.contains(&decision.consent_type.as_str())
                || !matches!(decision.decision.as_str(), "GRANTED" | "DENIED")
        })
    {
        return validation_error("decisions");
    }
    let Some(context) = (match resolve_child_context(&state, claims.sub, child_id, true).await {
        Ok(context) => context,
        Err(error) => return error,
    }) else {
        return forbidden();
    };
    let mut transaction = match state.postgres.begin_tenant(context.tenant).await {
        Ok(transaction) => transaction,
        Err(_) => return server_error(),
    };
    let notice =
        match current_consent_notice(&mut transaction, context.tenant.organization_id).await {
            Ok(Some(notice)) => notice,
            Ok(None) => return api_error(StatusCode::CONFLICT, "conflict", json!({})),
            Err(error) => return error,
        };
    if notice.1 != input.text_version {
        let _ = transaction.rollback().await;
        return api_error(StatusCode::CONFLICT, "conflict", json!({}));
    }
    let lock_key = format!("consent:{}:{child_id}", context.tenant.organization_id);
    if sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(lock_key)
        .execute(&mut *transaction)
        .await
        .is_err()
    {
        return server_error();
    }
    let reviewed = match sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (
           SELECT 1 FROM app.consent_review
           WHERE organization_id = $1 AND notice_id = $2 AND child_id = $3
             AND guardian_user_id = $4
         )",
    )
    .bind(context.tenant.organization_id)
    .bind(notice.0)
    .bind(child_id)
    .bind(claims.sub)
    .fetch_one(&mut *transaction)
    .await
    {
        Ok(value) => value,
        Err(_) => return server_error(),
    };
    if !reviewed
        && sqlx::query(
            "INSERT INTO app.consent_review (organization_id, child_id, guardian_user_id, notice_id)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(context.tenant.organization_id)
        .bind(child_id)
        .bind(claims.sub)
        .bind(notice.0)
        .execute(&mut *transaction)
        .await
        .is_err()
    {
        return server_error();
    }
    for decision in &input.decisions {
        let db_decision = if decision.decision == "GRANTED" {
            "ONAY"
        } else {
            "RED"
        };
        if sqlx::query(
            "INSERT INTO app.consent_record
               (organization_id, child_id, guardian_user_id, consent_type, decision, notice_id, channel)
             VALUES ($1, $2, $3, $4, $5, $6, 'MOBIL')",
        )
        .bind(context.tenant.organization_id)
        .bind(child_id)
        .bind(claims.sub)
        .bind(&decision.consent_type)
        .bind(db_decision)
        .bind(notice.0)
        .execute(&mut *transaction)
        .await
        .is_err()
        {
            return server_error();
        }
    }
    let dto = match consent_dto(&mut transaction, context.tenant, child_id, notice.1).await {
        Ok(dto) => dto,
        Err(error) => return error,
    };
    if transaction.commit().await.is_err() {
        return server_error();
    }
    Json(dto).into_response()
}

async fn local_date(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    organization_id: Uuid,
) -> Result<String, Response> {
    sqlx::query_scalar::<_, String>(
        "SELECT to_char((clock_timestamp() AT TIME ZONE organization.timezone)::date, 'YYYY-MM-DD')
         FROM app.organization AS organization WHERE organization.id = $1",
    )
    .bind(organization_id)
    .fetch_one(&mut **transaction)
    .await
    .map_err(|_| server_error())
}

fn valid_iso_date(date: &str) -> bool {
    let bytes = date.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
}

async fn get_feed(
    State(state): State<ApiState>,
    Extension(claims): Extension<Claims>,
    Path(child_id): Path<Uuid>,
    Query(query): Query<FeedQuery>,
) -> Response {
    let Some(context) = (match resolve_child_context(&state, claims.sub, child_id, false).await {
        Ok(context) => context,
        Err(error) => return error,
    }) else {
        return forbidden();
    };
    let mut transaction = match state.postgres.begin_tenant(context.tenant).await {
        Ok(transaction) => transaction,
        Err(_) => return server_error(),
    };
    let date = match query.date {
        Some(date) if valid_iso_date(&date) => date,
        Some(_) => return validation_error("date"),
        None => match local_date(&mut transaction, context.tenant.organization_id).await {
            Ok(date) => date,
            Err(error) => return error,
        },
    };
    let rows = sqlx::query_as::<_, (Uuid, String, Option<Value>, String)>(
        "SELECT id, record_type, details_json,
                to_char(occurred_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"')
         FROM app.daily_record
         WHERE organization_id = $1 AND child_id = $2 AND date = $3::date
           AND record_type IN ('OGUN', 'UYKU', 'AKTIVITE', 'TESLIM')
         ORDER BY occurred_at, id",
    )
    .bind(context.tenant.organization_id)
    .bind(child_id)
    .bind(&date)
    .fetch_all(&mut *transaction)
    .await;
    let rows = match rows {
        Ok(rows) => rows,
        Err(_) => return validation_error("date"),
    };
    let mut entries = Vec::with_capacity(rows.len());
    let mut pickup_refs = Vec::new();
    for (id, record_type, details, at) in rows {
        let details = details.unwrap_or_else(|| json!({}));
        let entry = match record_type.as_str() {
            "OGUN" => json!({
                "id": id,
                "kind": "MEAL",
                "at": at,
                "slot": details.get("slot").and_then(Value::as_str).unwrap_or("OGLE"),
                "amount": details.get("amount").and_then(Value::as_str).unwrap_or("HIC")
            }),
            "UYKU" => json!({
                "id": id,
                "kind": "NAP",
                "at": at,
                "endAt": details.get("endAt").cloned().unwrap_or(Value::Null)
            }),
            "AKTIVITE" => json!({
                "id": id,
                "kind": "ACTIVITY",
                "at": at,
                "activityType": details.get("activityType").and_then(Value::as_str).unwrap_or("Etkinlik"),
                "note": details.get("note").cloned().unwrap_or(Value::Null)
            }),
            "TESLIM" => {
                if let Some(request_id) = details.get("pickupRequestId").and_then(Value::as_str) {
                    if let Ok(request_id) = Uuid::parse_str(request_id) {
                        pickup_refs.push((id, request_id, at));
                    }
                }
                continue;
            }
            _ => continue,
        };
        entries.push(entry);
    }
    let mut pickup_rows = Vec::new();
    for (feed_id, request_id, at) in pickup_refs {
        let row = sqlx::query_as::<_, (String, String)>(
            "SELECT recipient_name_ciphertext, recipient_relationship
             FROM app.pickup_request WHERE organization_id = $1 AND id = $2",
        )
        .bind(context.tenant.organization_id)
        .bind(request_id)
        .fetch_optional(&mut *transaction)
        .await;
        match row {
            Ok(Some(row)) => pickup_rows.push((feed_id, row.0, row.1, at)),
            Ok(None) => {}
            Err(_) => return server_error(),
        }
    }
    if transaction.commit().await.is_err() {
        return server_error();
    }
    for (id, name_ciphertext, relation, at) in pickup_rows {
        let name = match decrypt_text(
            &state,
            &format!("kurum-{}", context.tenant.organization_id),
            &name_ciphertext,
        )
        .await
        {
            Ok(name) => name,
            Err(_) => return server_error(),
        };
        entries.push(json!({
            "id": id,
            "kind": "PICKUP",
            "at": at,
            "pickupPersonName": name,
            "relation": relation
        }));
    }
    entries.sort_by(|left, right| {
        left.get("at")
            .and_then(Value::as_str)
            .cmp(&right.get("at").and_then(Value::as_str))
    });
    Json(json!({ "childId": child_id, "date": date, "entries": entries })).into_response()
}

async fn get_class_today(
    State(state): State<ApiState>,
    Extension(claims): Extension<Claims>,
    Path(class_id): Path<Uuid>,
) -> Response {
    let Some((context, class_name)) =
        (match resolve_class_context(&state, claims.sub, class_id).await {
            Ok(context) => context,
            Err(error) => return error,
        })
    else {
        return forbidden();
    };
    let mut transaction = match state.postgres.begin_tenant(context).await {
        Ok(transaction) => transaction,
        Err(_) => return server_error(),
    };
    let date = match local_date(&mut transaction, context.organization_id).await {
        Ok(date) => date,
        Err(error) => return error,
    };
    let child_rows = sqlx::query_as::<_, (Uuid, String, String, Option<String>, bool)>(
        "SELECT child.id, child.first_name_ciphertext, child.last_name_ciphertext,
                health_profile.content_ciphertext,
                COALESCE(effective_consent.is_enabled, false)
         FROM app.child AS child
         LEFT JOIN app.health_profile AS health_profile
           ON health_profile.organization_id = child.organization_id
          AND health_profile.child_id = child.id
         LEFT JOIN app.effective_consent AS effective_consent
           ON effective_consent.organization_id = child.organization_id
          AND effective_consent.child_id = child.id
          AND effective_consent.consent_type = 'SAGLIK'
         WHERE child.organization_id = $1 AND child.class_id = $2
           AND child.status = 'AKTIF'
         ORDER BY child.id",
    )
    .bind(context.organization_id)
    .bind(class_id)
    .fetch_all(&mut *transaction)
    .await;
    let child_rows = match child_rows {
        Ok(rows) => rows,
        Err(_) => return server_error(),
    };
    let menu_rows = sqlx::query_as::<_, (Uuid, String, String, Option<String>)>(
        "SELECT definition.id, definition.type, definition.name, definition.description
         FROM app.meal_menu AS menu
         JOIN app.meal_definition AS definition
           ON definition.organization_id = menu.organization_id AND definition.menu_id = menu.id
         WHERE menu.organization_id = $1 AND menu.status = 'YAYINDA'
           AND menu.start_date <= $2::date AND menu.end_date >= $2::date
           AND definition.date = $2::date
         ORDER BY definition.sort_order, definition.type",
    )
    .bind(context.organization_id)
    .bind(&date)
    .fetch_all(&mut *transaction)
    .await;
    let menu_rows = match menu_rows {
        Ok(rows) => rows,
        Err(_) => return server_error(),
    };
    let mut menu = Vec::with_capacity(menu_rows.len());
    for (meal_id, meal_type, name, description) in menu_rows {
        let allergens = match sqlx::query_scalar::<_, String>(
            "SELECT lower(allergen_code) FROM app.meal_allergen
             WHERE organization_id = $1 AND meal_definition_id = $2
             ORDER BY allergen_code",
        )
        .bind(context.organization_id)
        .bind(meal_id)
        .fetch_all(&mut *transaction)
        .await
        {
            Ok(allergens) => allergens,
            Err(_) => return server_error(),
        };
        let slot = match meal_type.as_str() {
            "KAHVALTI" => "KAHVALTI",
            "OGLE" => "OGLE",
            "IKINDI" => "IKINDI",
            _ => "ARA_OGUN",
        };
        let mut items = vec![name];
        if let Some(description) = description.filter(|description| !description.trim().is_empty())
        {
            items.push(description);
        }
        menu.push(json!({ "slot": slot, "items": items, "allergens": allergens }));
    }
    if transaction.commit().await.is_err() {
        return server_error();
    }

    let mut children = Vec::with_capacity(child_rows.len());
    for (child_id, first_ciphertext, last_ciphertext, health_ciphertext, health_consent) in
        child_rows
    {
        let key = format!("kurum-{}", context.organization_id);
        let first_name = match decrypt_text(&state, &key, &first_ciphertext).await {
            Ok(name) => name,
            Err(_) => return server_error(),
        };
        let last_name = match decrypt_text(&state, &key, &last_ciphertext).await {
            Ok(name) => name,
            Err(_) => return server_error(),
        };
        let allergens = if !health_consent {
            Value::Null
        } else if let Some(ciphertext) = health_ciphertext {
            let plaintext = match decrypt_text(&state, &key, &ciphertext).await {
                Ok(value) => value,
                Err(_) => return server_error(),
            };
            let profile: Value = match serde_json::from_str(&plaintext) {
                Ok(value) => value,
                Err(_) => return server_error(),
            };
            profile
                .get("allergens")
                .cloned()
                .unwrap_or_else(|| json!([]))
        } else {
            json!([])
        };
        children.push(json!({
            "id": child_id,
            "firstName": first_name,
            "lastInitial": last_name.chars().next().map(|value| value.to_string()).unwrap_or_default(),
            "allergens": allergens
        }));
    }
    Json(json!({
        "classId": class_id,
        "className": class_name,
        "date": date,
        "children": children,
        "menu": menu
    }))
    .into_response()
}

async fn reserve_idempotency<T: Serialize>(
    state: &ApiState,
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    context: TenantContext,
    headers: &HeaderMap,
    route_template: &str,
    body: &T,
) -> Result<IdempotencyDecision, Response> {
    let key = headers
        .get("Idempotency-Key")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| Uuid::parse_str(value).ok())
        .ok_or_else(|| validation_error("Idempotency-Key"))?;
    let request_bytes = serde_json::to_vec(body).map_err(|_| server_error())?;
    let request_hash = Sha256::digest(request_bytes);
    let inserted = sqlx::query(
        "INSERT INTO app.idempotency_key
           (organization_id, user_id, http_method, route_template, idempotency_key,
            request_sha256, expires_at)
         VALUES ($1, $2, 'POST', $3, $4, $5, clock_timestamp() + interval '7 days')
         ON CONFLICT DO NOTHING",
    )
    .bind(context.organization_id)
    .bind(context.user_id)
    .bind(route_template)
    .bind(key)
    .bind(request_hash.as_slice())
    .execute(&mut **transaction)
    .await
    .map_err(|_| server_error())?;
    if inserted.rows_affected() == 1 {
        return Ok(IdempotencyDecision::New(key));
    }

    let row = sqlx::query_as::<_, (Uuid, Vec<u8>, String, Option<i16>, Option<String>, bool)>(
        "SELECT id, request_sha256, status, response_status, response_ciphertext,
                expires_at <= clock_timestamp()
         FROM app.idempotency_key
         WHERE organization_id = $1 AND user_id = $2 AND http_method = 'POST'
           AND route_template = $3 AND idempotency_key = $4
         FOR UPDATE",
    )
    .bind(context.organization_id)
    .bind(context.user_id)
    .bind(route_template)
    .bind(key)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(|_| server_error())?
    .ok_or_else(server_error)?;
    let (row_id, stored_hash, status, response_status, response_ciphertext, expired) = row;
    if expired {
        if sqlx::query(
            "UPDATE app.idempotency_key
             SET request_sha256 = $2, status = 'BEKLIYOR', response_status = NULL,
                 response_ciphertext = NULL, expires_at = clock_timestamp() + interval '7 days'
             WHERE id = $1",
        )
        .bind(row_id)
        .bind(request_hash.as_slice())
        .execute(&mut **transaction)
        .await
        .is_err()
        {
            return Err(server_error());
        }
        return Ok(IdempotencyDecision::New(key));
    }
    if stored_hash.as_slice() != request_hash.as_slice() {
        return Err(api_error(
            StatusCode::CONFLICT,
            "conflict",
            json!({ "reason": "idempotency_mismatch" }),
        ));
    }
    if status != "TAMAMLANDI" {
        return Err(server_error());
    }
    let status = response_status
        .and_then(|value| StatusCode::from_u16(value as u16).ok())
        .ok_or_else(server_error)?;
    let response = match response_ciphertext {
        Some(ciphertext) => {
            let plaintext = state
                .vault
                .transit_decrypt(&format!("kurum-{}", context.organization_id), &ciphertext)
                .await
                .map_err(|_| server_error())?;
            serde_json::from_slice(&plaintext).map_err(|_| server_error())?
        }
        None => Value::Null,
    };
    Ok(IdempotencyDecision::Replay(status, response))
}

async fn complete_idempotency(
    state: &ApiState,
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    context: TenantContext,
    key: Uuid,
    status: StatusCode,
    response: &Value,
) -> Result<(), Response> {
    let response_json = serde_json::to_vec(response).map_err(|_| server_error())?;
    let (ciphertext, _) = state
        .vault
        .transit_encrypt(
            &format!("kurum-{}", context.organization_id),
            &response_json,
        )
        .await
        .map_err(|_| server_error())?;
    sqlx::query(
        "UPDATE app.idempotency_key
         SET status = 'TAMAMLANDI', response_status = $2, response_ciphertext = $3
         WHERE organization_id = $4 AND user_id = $5 AND idempotency_key = $1",
    )
    .bind(key)
    .bind(status.as_u16() as i16)
    .bind(ciphertext)
    .bind(context.organization_id)
    .bind(context.user_id)
    .execute(&mut **transaction)
    .await
    .map_err(|_| server_error())?;
    Ok(())
}

fn replay_response(status: StatusCode, body: Value) -> Response {
    if status == StatusCode::NO_CONTENT {
        status.into_response()
    } else {
        (status, Json(body)).into_response()
    }
}

async fn record_meals(
    State(state): State<ApiState>,
    Extension(claims): Extension<Claims>,
    Path(class_id): Path<Uuid>,
    headers: HeaderMap,
    Json(input): Json<MealRecordsInput>,
) -> Response {
    const VALID_SLOTS: [&str; 4] = ["KAHVALTI", "ARA_OGUN", "OGLE", "IKINDI"];
    const VALID_AMOUNTS: [&str; 5] = ["HIC", "AZ", "YARISI", "COGU", "TAMAMI"];
    if !valid_iso_date(&input.date)
        || !VALID_SLOTS.contains(&input.slot.as_str())
        || input.entries.is_empty()
        || input.entries.len() > 40
        || input
            .entries
            .iter()
            .any(|entry| !VALID_AMOUNTS.contains(&entry.amount.as_str()))
    {
        return validation_error("mealRecords");
    }
    if input
        .recorded_at
        .as_ref()
        .is_some_and(|value| value.len() > 40)
    {
        return validation_error("recordedAt");
    }
    let Some((context, _class_name)) =
        (match resolve_class_context(&state, claims.sub, class_id).await {
            Ok(context) => context,
            Err(error) => return error,
        })
    else {
        return forbidden();
    };
    let mut transaction = match state.postgres.begin_tenant(context).await {
        Ok(transaction) => transaction,
        Err(_) => return server_error(),
    };
    let valid_children = sqlx::query_scalar::<_, i64>(
        "SELECT count(DISTINCT child.id)
         FROM app.child AS child
         WHERE child.organization_id = $1 AND child.class_id = $2
           AND child.status = 'AKTIF' AND child.id = ANY($3)",
    )
    .bind(context.organization_id)
    .bind(class_id)
    .bind(
        input
            .entries
            .iter()
            .map(|entry| entry.child_id)
            .collect::<Vec<_>>(),
    )
    .fetch_one(&mut *transaction)
    .await;
    if !matches!(valid_children, Ok(count) if count == input.entries.len() as i64) {
        let _ = transaction.rollback().await;
        return validation_error("entries");
    }
    let date_is_valid = sqlx::query_scalar::<_, bool>(
        "SELECT $1::date <= (clock_timestamp() AT TIME ZONE organization.timezone)::date
         FROM app.organization AS organization WHERE organization.id = $2",
    )
    .bind(&input.date)
    .bind(context.organization_id)
    .fetch_one(&mut *transaction)
    .await;
    if !matches!(date_is_valid, Ok(true)) {
        let _ = transaction.rollback().await;
        return validation_error("date");
    }
    let idempotency_key = match reserve_idempotency(
        &state,
        &mut transaction,
        context,
        &headers,
        "/v1/classes/{classId}/meal-records",
        &input,
    )
    .await
    {
        Ok(IdempotencyDecision::New(key)) => key,
        Ok(IdempotencyDecision::Replay(status, body)) => {
            if transaction.commit().await.is_err() {
                return server_error();
            }
            return replay_response(status, body);
        }
        Err(error) => return error,
    };

    for entry in &input.entries {
        let advisory_key = format!(
            "meal:{}:{}:{}:{}",
            context.organization_id, entry.child_id, input.date, input.slot
        );
        if sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
            .bind(advisory_key)
            .execute(&mut *transaction)
            .await
            .is_err()
        {
            return server_error();
        }
        let timing = if let Some(recorded_at) = input.recorded_at.as_deref() {
            sqlx::query_as::<_, (String, bool)>(
                "SELECT to_char(
                          LEAST($1::timestamptz, clock_timestamp()) AT TIME ZONE 'UTC',
                          'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"'
                        ),
                        ($1::timestamptz > clock_timestamp())
                         OR (($1::timestamptz AT TIME ZONE 'Europe/Istanbul')::date <> $2::date)
                         OR (clock_timestamp() - $1::timestamptz > interval '12 hours')",
            )
            .bind(recorded_at)
            .bind(&input.date)
            .fetch_one(&mut *transaction)
            .await
        } else {
            sqlx::query_as::<_, (String, bool)>(
                "SELECT to_char(clock_timestamp() AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"'), false",
            )
            .fetch_one(&mut *transaction)
            .await
        };
        let (occurred_at, is_late) = match timing {
            Ok(timing) => timing,
            Err(_) => return validation_error("recordedAt"),
        };
        let existing = sqlx::query_as::<_, (Uuid, String)>(
            "SELECT id,
                    to_char(occurred_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"')
             FROM app.daily_record
             WHERE organization_id = $1 AND child_id = $2 AND date = $3::date
               AND record_type = 'OGUN' AND details_json->>'slot' = $4
             ORDER BY occurred_at DESC, id DESC LIMIT 1 FOR UPDATE",
        )
        .bind(context.organization_id)
        .bind(entry.child_id)
        .bind(&input.date)
        .bind(&input.slot)
        .fetch_optional(&mut *transaction)
        .await;
        let existing = match existing {
            Ok(value) => value,
            Err(_) => return server_error(),
        };
        let newer_than_saved = existing
            .as_ref()
            .is_none_or(|(_, saved_at)| occurred_at >= *saved_at);
        let details = json!({ "slot": input.slot, "amount": entry.amount });
        if let Some((record_id, _)) = existing.filter(|_| newer_than_saved) {
            if sqlx::query(
                "UPDATE app.daily_record
                 SET occurred_at = $2::timestamptz, received_at = clock_timestamp(),
                     time_source = $3, is_late = $4, details_json = $5
                 WHERE organization_id = $1 AND id = $6",
            )
            .bind(context.organization_id)
            .bind(&occurred_at)
            .bind(if input.recorded_at.is_some() {
                "CIHAZ"
            } else {
                "SUNUCU"
            })
            .bind(is_late)
            .bind(details)
            .bind(record_id)
            .execute(&mut *transaction)
            .await
            .is_err()
            {
                return server_error();
            }
        } else if sqlx::query(
            "INSERT INTO app.daily_record
               (organization_id, child_id, record_type, data_category, date, occurred_at,
                time_source, is_late, details_json, created_by_user_id)
             VALUES ($1, $2, 'OGUN', 'GUNLUK', $3::date, $4::timestamptz,
                     $5, $6, $7, $8)",
        )
        .bind(context.organization_id)
        .bind(entry.child_id)
        .bind(&input.date)
        .bind(&occurred_at)
        .bind(if input.recorded_at.is_some() {
            "CIHAZ"
        } else {
            "SUNUCU"
        })
        .bind(is_late)
        .bind(details)
        .bind(claims.sub)
        .execute(&mut *transaction)
        .await
        .is_err()
        {
            return server_error();
        }
    }
    let response = json!({ "saved": input.entries.len() });
    if complete_idempotency(
        &state,
        &mut transaction,
        context,
        idempotency_key,
        StatusCode::CREATED,
        &response,
    )
    .await
    .is_err()
    {
        return server_error();
    }
    if transaction.commit().await.is_err() {
        return server_error();
    }
    (StatusCode::CREATED, Json(response)).into_response()
}

async fn get_authorized_people(
    State(state): State<ApiState>,
    Extension(claims): Extension<Claims>,
    Path(child_id): Path<Uuid>,
) -> Response {
    let Some(child_context) =
        (match resolve_child_context(&state, claims.sub, child_id, false).await {
            Ok(context) => context,
            Err(error) => return error,
        })
    else {
        return forbidden();
    };
    let context = child_context.tenant;
    let mut transaction = match state.postgres.begin_tenant(context).await {
        Ok(transaction) => transaction,
        Err(_) => return server_error(),
    };
    let guardian = match sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (
           SELECT 1 FROM app.guardian_child
           WHERE organization_id = $1 AND child_id = $2
             AND guardian_user_id = $3 AND is_active AND access_level <> 'KAPALI'
         )",
    )
    .bind(context.organization_id)
    .bind(child_id)
    .bind(claims.sub)
    .fetch_one(&mut *transaction)
    .await
    {
        Ok(value) => value,
        Err(_) => return server_error(),
    };
    if !guardian {
        let _ = transaction.rollback().await;
        return forbidden();
    }
    let rows = sqlx::query_as::<_, (Uuid, String, String, bool, String)>(
        "SELECT person.id, person.first_name_ciphertext, person.relationship,
                person.is_verified AND person.valid_from <= clock_timestamp()
                  AND person.valid_until > clock_timestamp()
                  AND (person.remaining_uses IS NULL OR person.remaining_uses > 0),
                to_char(person.valid_until AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"')
         FROM app.authorized_user_child AS link
         JOIN app.authorized_person AS person
           ON person.organization_id = link.organization_id
          AND person.id = link.authorized_person_id
         WHERE link.organization_id = $1 AND link.child_id = $2
           AND link.added_by_guardian_user_id = $3
         ORDER BY person.created_at, person.id",
    )
    .bind(context.organization_id)
    .bind(child_id)
    .bind(claims.sub)
    .fetch_all(&mut *transaction)
    .await;
    let rows = match rows {
        Ok(rows) => rows,
        Err(_) => return server_error(),
    };
    if transaction.commit().await.is_err() {
        return server_error();
    }
    let mut people = Vec::with_capacity(rows.len());
    for (id, ciphertext, relation, verified, valid_until) in rows {
        let display_name = match decrypt_text(
            &state,
            &format!("kurum-{}", context.organization_id),
            &ciphertext,
        )
        .await
        {
            Ok(name) => name,
            Err(_) => return server_error(),
        };
        people.push(AuthorizedPersonDto {
            id,
            display_name,
            relation,
            verified,
            valid_until: Some(valid_until),
        });
    }
    Json(people).into_response()
}

type PickupSqlRow = (
    Uuid,
    Uuid,
    String,
    String,
    Uuid,
    String,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    Uuid,
);

async fn pickup_dto(
    state: &ApiState,
    organization_id: Uuid,
    row: PickupSqlRow,
    viewer_id: Uuid,
    allow_pin: bool,
) -> Result<PickupDto, Response> {
    let key = format!("kurum-{organization_id}");
    let child_name = decrypt_text(state, &key, &row.2)
        .await
        .map_err(|_| server_error())?;
    let pickup_person_name = decrypt_text(state, &key, &row.5)
        .await
        .map_err(|_| server_error())?;
    let pin = if allow_pin && row.11 == viewer_id && row.7 != "ENGELLENDI" {
        match row.10.as_deref() {
            Some(ciphertext) => Some(
                decrypt_text(state, &key, ciphertext)
                    .await
                    .map_err(|_| server_error())?,
            ),
            None => None,
        }
    } else {
        None
    };
    let eta_minutes = row
        .9
        .as_deref()
        .and_then(|value| value.parse::<i32>().ok())
        .unwrap_or_default()
        .max(0);
    Ok(PickupDto {
        id: row.0,
        child_id: row.1,
        child_name,
        pickup_person: PickupPersonDto {
            id: row.4,
            display_name: pickup_person_name,
            relation: row.6,
        },
        eta_minutes,
        created_at: row.8,
        status: row.7,
        pin,
    })
}

async fn get_active_pickup(
    State(state): State<ApiState>,
    Extension(claims): Extension<Claims>,
    Path(child_id): Path<Uuid>,
) -> Response {
    let Some(child_context) =
        (match resolve_child_context(&state, claims.sub, child_id, false).await {
            Ok(context) => context,
            Err(error) => return error,
        })
    else {
        return forbidden();
    };
    let context = child_context.tenant;
    let mut transaction = match state.postgres.begin_tenant(context).await {
        Ok(transaction) => transaction,
        Err(_) => return server_error(),
    };
    let guardian = match sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (
           SELECT 1 FROM app.guardian_child
           WHERE organization_id = $1 AND child_id = $2
             AND guardian_user_id = $3 AND is_active AND access_level <> 'KAPALI'
         )",
    )
    .bind(context.organization_id)
    .bind(child_id)
    .bind(claims.sub)
    .fetch_one(&mut *transaction)
    .await
    {
        Ok(value) => value,
        Err(_) => return server_error(),
    };
    if !guardian {
        let _ = transaction.rollback().await;
        return forbidden();
    }
    let row = sqlx::query_as::<_, PickupSqlRow>(
        "SELECT pickup.id, child.id, child.first_name_ciphertext, child.last_name_ciphertext,
                COALESCE(person.id, pickup.id), pickup.recipient_name_ciphertext,
                pickup.recipient_relationship, pickup.status,
                to_char(pickup.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"'),
                CASE WHEN pickup.estimated_arrival_at IS NULL THEN NULL
                     ELSE GREATEST(0, ceil(extract(epoch FROM (pickup.estimated_arrival_at - clock_timestamp())) / 60)::int)::text
                END,
                pickup.pin_display_ciphertext, pickup.requested_by_user_id
         FROM app.pickup_request AS pickup
         JOIN app.child AS child
           ON child.organization_id = pickup.organization_id AND child.id = pickup.child_id
         LEFT JOIN app.authorized_person AS person
           ON person.organization_id = pickup.organization_id AND person.id = pickup.authorized_person_id
         WHERE pickup.organization_id = $1 AND pickup.child_id = $2
           AND pickup.status IN ('YOLDA', 'HAZIRLANIYOR', 'ENGELLENDI')
         ORDER BY pickup.created_at DESC LIMIT 1",
    )
    .bind(context.organization_id)
    .bind(child_id)
    .fetch_optional(&mut *transaction)
    .await;
    let row = match row {
        Ok(row) => row,
        Err(_) => return server_error(),
    };
    if transaction.commit().await.is_err() {
        return server_error();
    }
    let Some(row) = row else {
        return Json(Value::Null).into_response();
    };
    match pickup_dto(&state, context.organization_id, row, claims.sub, true).await {
        Ok(dto) => Json(dto).into_response(),
        Err(error) => error,
    }
}

async fn create_pickup(
    State(state): State<ApiState>,
    Extension(claims): Extension<Claims>,
    headers: HeaderMap,
    Json(input): Json<PickupCreateInput>,
) -> Response {
    if !(1..=120).contains(&input.eta_minutes) {
        return validation_error("etaMinutes");
    }
    let Some(child_context) =
        (match resolve_child_context(&state, claims.sub, input.child_id, true).await {
            Ok(context) => context,
            Err(error) => return error,
        })
    else {
        return forbidden();
    };
    let context = child_context.tenant;
    let mut transaction = match state.postgres.begin_tenant(context).await {
        Ok(transaction) => transaction,
        Err(_) => return server_error(),
    };
    let repeated = match reserve_idempotency(
        &state,
        &mut transaction,
        context,
        &headers,
        "/v1/pickup-requests",
        &input,
    )
    .await
    {
        Ok(IdempotencyDecision::New(key)) => key,
        Ok(IdempotencyDecision::Replay(status, body)) => {
            if transaction.commit().await.is_err() {
                return server_error();
            }
            return replay_response(status, body);
        }
        Err(error) => return error,
    };
    let person = sqlx::query_as::<_, (String, String)>(
        "SELECT person.first_name_ciphertext, person.relationship
         FROM app.authorized_user_child AS link
         JOIN app.authorized_person AS person
           ON person.organization_id = link.organization_id
          AND person.id = link.authorized_person_id
         WHERE link.organization_id = $1 AND link.child_id = $2
           AND link.authorized_person_id = $3
           AND link.added_by_guardian_user_id = $4
           AND person.is_verified AND person.valid_from <= clock_timestamp()
           AND person.valid_until > clock_timestamp()
           AND (person.remaining_uses IS NULL OR person.remaining_uses > 0)
           AND (
             NOT COALESCE((SELECT authorized_person_approval_required
                           FROM app.organization_settings WHERE organization_id = $1), false)
             OR person.approved_at IS NOT NULL
           )",
    )
    .bind(context.organization_id)
    .bind(input.child_id)
    .bind(input.pickup_person_id)
    .bind(claims.sub)
    .fetch_optional(&mut *transaction)
    .await;
    let (person_name_ciphertext, relationship) = match person {
        Ok(Some(person)) => person,
        Ok(None) => {
            let _ = transaction.rollback().await;
            return validation_error("pickupPersonId");
        }
        Err(_) => return server_error(),
    };
    let open_request = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (
           SELECT 1 FROM app.pickup_request
           WHERE organization_id = $1 AND child_id = $2
             AND status IN ('YOLDA', 'HAZIRLANIYOR', 'ENGELLENDI')
         )",
    )
    .bind(context.organization_id)
    .bind(input.child_id)
    .fetch_one(&mut *transaction)
    .await;
    if !matches!(open_request, Ok(false)) {
        let _ = transaction.rollback().await;
        return api_error(StatusCode::CONFLICT, "conflict", json!({}));
    }
    let request_id = Uuid::now_v7();
    let pin = format!("{:04}", rand::thread_rng().gen_range(0..10_000));
    let mut mac =
        match HmacSha256::new_from_slice(state.secrets.pickup_pepper.expose_secret().as_bytes()) {
            Ok(mac) => mac,
            Err(_) => return server_error(),
        };
    mac.update(request_id.as_bytes());
    mac.update(pin.as_bytes());
    let pin_hash = mac.finalize().into_bytes();
    let (pin_display_ciphertext, _) = match state
        .vault
        .transit_encrypt(
            &format!("kurum-{}", context.organization_id),
            pin.as_bytes(),
        )
        .await
    {
        Ok(value) => value,
        Err(_) => return server_error(),
    };
    let inserted = sqlx::query(
        "INSERT INTO app.pickup_request
           (id, organization_id, child_id, requested_by_user_id, authorized_person_id,
            recipient_name_ciphertext, recipient_relationship, estimated_arrival_at,
            pin_hash, pin_display_ciphertext)
         VALUES ($1, $2, $3, $4, $5, $6, $7,
                 clock_timestamp() + ($8 * interval '1 minute'), $9, $10)",
    )
    .bind(request_id)
    .bind(context.organization_id)
    .bind(input.child_id)
    .bind(claims.sub)
    .bind(input.pickup_person_id)
    .bind(person_name_ciphertext)
    .bind(&relationship)
    .bind(input.eta_minutes)
    .bind(pin_hash.as_slice())
    .bind(pin_display_ciphertext)
    .execute(&mut *transaction)
    .await;
    if inserted.is_err() {
        let _ = transaction.rollback().await;
        return api_error(StatusCode::CONFLICT, "conflict", json!({}));
    }
    let row = sqlx::query_as::<_, PickupSqlRow>(
        "SELECT pickup.id, child.id, child.first_name_ciphertext, child.last_name_ciphertext,
                person.id, pickup.recipient_name_ciphertext, pickup.recipient_relationship,
                pickup.status,
                to_char(pickup.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"'),
                GREATEST(0, ceil(extract(epoch FROM (pickup.estimated_arrival_at - clock_timestamp())) / 60)::int)::text,
                pickup.pin_display_ciphertext, pickup.requested_by_user_id
         FROM app.pickup_request AS pickup
         JOIN app.child AS child
           ON child.organization_id = pickup.organization_id AND child.id = pickup.child_id
         JOIN app.authorized_person AS person
           ON person.organization_id = pickup.organization_id AND person.id = pickup.authorized_person_id
         WHERE pickup.organization_id = $1 AND pickup.id = $2",
    )
    .bind(context.organization_id)
    .bind(request_id)
    .fetch_one(&mut *transaction)
    .await;
    let row = match row {
        Ok(row) => row,
        Err(_) => return server_error(),
    };
    let dto = match pickup_dto(&state, context.organization_id, row, claims.sub, true).await {
        Ok(dto) => dto,
        Err(error) => return error,
    };
    let response = match serde_json::to_value(dto) {
        Ok(value) => value,
        Err(_) => return server_error(),
    };
    if complete_idempotency(
        &state,
        &mut transaction,
        context,
        repeated,
        StatusCode::CREATED,
        &response,
    )
    .await
    .is_err()
    {
        return server_error();
    }
    if transaction.commit().await.is_err() {
        return server_error();
    }
    (StatusCode::CREATED, Json(response)).into_response()
}

async fn resolve_pickup_teacher_context(
    state: &ApiState,
    user_id: Uuid,
    request_id: Uuid,
) -> Result<Option<(TenantContext, Uuid)>, Response> {
    for membership in user_tenant_memberships(state, user_id).await? {
        if membership.role != TenantRole::Teacher {
            continue;
        }
        let context = TenantContext {
            user_id,
            organization_id: membership.organization_id,
        };
        let mut transaction = state
            .postgres
            .begin_tenant(context)
            .await
            .map_err(|_| server_error())?;
        let class_id = sqlx::query_scalar::<_, Uuid>(
            "SELECT child.class_id
             FROM app.pickup_request AS pickup
             JOIN app.child AS child
               ON child.organization_id = pickup.organization_id AND child.id = pickup.child_id
             JOIN app.membership_class AS membership_class
               ON membership_class.organization_id = child.organization_id
              AND membership_class.class_id = child.class_id
             JOIN app.membership AS membership
               ON membership.organization_id = membership_class.organization_id
              AND membership.id = membership_class.membership_id
             WHERE pickup.organization_id = $1 AND pickup.id = $2
               AND membership.user_id = $3 AND membership.status = 'AKTIF'
               AND membership.role = 'OGRETMEN'
             LIMIT 1",
        )
        .bind(context.organization_id)
        .bind(request_id)
        .bind(user_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|_| server_error())?;
        transaction.commit().await.map_err(|_| server_error())?;
        if let Some(class_id) = class_id {
            return Ok(Some((context, class_id)));
        }
    }
    Ok(None)
}

async fn get_pickup_queue(
    State(state): State<ApiState>,
    Extension(claims): Extension<Claims>,
    Path(class_id): Path<Uuid>,
) -> Response {
    let Some((context, _)) = (match resolve_class_context(&state, claims.sub, class_id).await {
        Ok(context) => context,
        Err(error) => return error,
    }) else {
        return forbidden();
    };
    let mut transaction = match state.postgres.begin_tenant(context).await {
        Ok(transaction) => transaction,
        Err(_) => return server_error(),
    };
    let rows = sqlx::query_as::<_, PickupSqlRow>(
        "SELECT pickup.id, child.id, child.first_name_ciphertext, child.last_name_ciphertext,
                COALESCE(person.id, pickup.id), pickup.recipient_name_ciphertext,
                pickup.recipient_relationship, pickup.status,
                to_char(pickup.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"'),
                CASE WHEN pickup.estimated_arrival_at IS NULL THEN NULL
                     ELSE GREATEST(0, ceil(extract(epoch FROM (pickup.estimated_arrival_at - clock_timestamp())) / 60)::int)::text
                END,
                NULL::text, pickup.requested_by_user_id
         FROM app.pickup_request AS pickup
         JOIN app.child AS child
           ON child.organization_id = pickup.organization_id AND child.id = pickup.child_id
         LEFT JOIN app.authorized_person AS person
           ON person.organization_id = pickup.organization_id AND person.id = pickup.authorized_person_id
         WHERE pickup.organization_id = $1 AND child.class_id = $2
           AND pickup.status IN ('YOLDA', 'HAZIRLANIYOR')
         ORDER BY pickup.estimated_arrival_at NULLS LAST, pickup.created_at, pickup.id",
    )
    .bind(context.organization_id)
    .bind(class_id)
    .fetch_all(&mut *transaction)
    .await;
    let rows = match rows {
        Ok(rows) => rows,
        Err(_) => return server_error(),
    };
    if transaction.commit().await.is_err() {
        return server_error();
    }
    let mut queue = Vec::with_capacity(rows.len());
    for row in rows {
        match pickup_dto(&state, context.organization_id, row, claims.sub, false).await {
            Ok(dto) => queue.push(dto),
            Err(error) => return error,
        }
    }
    Json(queue).into_response()
}

async fn pickup_row(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    context: TenantContext,
    request_id: Uuid,
) -> Result<Option<PickupSqlRow>, Response> {
    sqlx::query_as::<_, PickupSqlRow>(
        "SELECT pickup.id, child.id, child.first_name_ciphertext, child.last_name_ciphertext,
                COALESCE(person.id, pickup.id), pickup.recipient_name_ciphertext,
                pickup.recipient_relationship, pickup.status,
                to_char(pickup.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"'),
                CASE WHEN pickup.estimated_arrival_at IS NULL THEN NULL
                     ELSE GREATEST(0, ceil(extract(epoch FROM (pickup.estimated_arrival_at - clock_timestamp())) / 60)::int)::text
                END,
                pickup.pin_display_ciphertext, pickup.requested_by_user_id
         FROM app.pickup_request AS pickup
         JOIN app.child AS child
           ON child.organization_id = pickup.organization_id AND child.id = pickup.child_id
         LEFT JOIN app.authorized_person AS person
           ON person.organization_id = pickup.organization_id AND person.id = pickup.authorized_person_id
         WHERE pickup.organization_id = $1 AND pickup.id = $2",
    )
    .bind(context.organization_id)
    .bind(request_id)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(|_| server_error())
}

async fn mark_pickup_preparing(
    State(state): State<ApiState>,
    Extension(claims): Extension<Claims>,
    Path(request_id): Path<Uuid>,
) -> Response {
    let Some((context, _)) =
        (match resolve_pickup_teacher_context(&state, claims.sub, request_id).await {
            Ok(context) => context,
            Err(error) => return error,
        })
    else {
        return forbidden();
    };
    let mut transaction = match state.postgres.begin_tenant(context).await {
        Ok(transaction) => transaction,
        Err(_) => return server_error(),
    };
    let updated = sqlx::query(
        "UPDATE app.pickup_request
         SET status = 'HAZIRLANIYOR', preparing_at = clock_timestamp(),
             updated_at = clock_timestamp()
         WHERE organization_id = $1 AND id = $2 AND status = 'YOLDA'",
    )
    .bind(context.organization_id)
    .bind(request_id)
    .execute(&mut *transaction)
    .await;
    if !matches!(updated, Ok(result) if result.rows_affected() == 1) {
        let _ = transaction.rollback().await;
        return api_error(StatusCode::CONFLICT, "conflict", json!({}));
    }
    let row = match pickup_row(&mut transaction, context, request_id).await {
        Ok(Some(row)) => row,
        Ok(None) => return forbidden(),
        Err(error) => return error,
    };
    if transaction.commit().await.is_err() {
        return server_error();
    }
    match pickup_dto(&state, context.organization_id, row, claims.sub, false).await {
        Ok(dto) => Json(dto).into_response(),
        Err(error) => error,
    }
}

async fn hand_over_pickup(
    State(state): State<ApiState>,
    Extension(claims): Extension<Claims>,
    Path(request_id): Path<Uuid>,
    Json(input): Json<HandoverInput>,
) -> Response {
    if input.pin.len() != 4 || !input.pin.bytes().all(|byte| byte.is_ascii_digit()) {
        return validation_error("pin");
    }
    let Some((context, _)) =
        (match resolve_pickup_teacher_context(&state, claims.sub, request_id).await {
            Ok(context) => context,
            Err(error) => return error,
        })
    else {
        return forbidden();
    };
    let mut transaction = match state.postgres.begin_tenant(context).await {
        Ok(transaction) => transaction,
        Err(_) => return server_error(),
    };
    let request = sqlx::query_as::<_, (String, Option<Vec<u8>>, i16, Option<Uuid>)>(
        "SELECT status, pin_hash, failed_pin_attempts, authorized_person_id
         FROM app.pickup_request
         WHERE organization_id = $1 AND id = $2
         FOR UPDATE",
    )
    .bind(context.organization_id)
    .bind(request_id)
    .fetch_optional(&mut *transaction)
    .await;
    let Some((status, stored_hash, failed_attempts, person_id)) = (match request {
        Ok(request) => request,
        Err(_) => return server_error(),
    }) else {
        return forbidden();
    };
    if status == "ENGELLENDI" {
        let _ = transaction.rollback().await;
        return api_error(StatusCode::LOCKED, "pickup_blocked", json!({}));
    }
    if !matches!(status.as_str(), "YOLDA" | "HAZIRLANIYOR") {
        let _ = transaction.rollback().await;
        return api_error(StatusCode::CONFLICT, "conflict", json!({}));
    }
    let Some(stored_hash) = stored_hash else {
        let _ = transaction.rollback().await;
        return api_error(StatusCode::LOCKED, "pickup_blocked", json!({}));
    };
    let mut mac =
        match HmacSha256::new_from_slice(state.secrets.pickup_pepper.expose_secret().as_bytes()) {
            Ok(mac) => mac,
            Err(_) => return server_error(),
        };
    mac.update(request_id.as_bytes());
    mac.update(input.pin.as_bytes());
    if mac.verify_slice(&stored_hash).is_err() {
        let failed_attempts = failed_attempts.saturating_add(1).min(3);
        let blocked = failed_attempts >= 3;
        let update = if blocked {
            sqlx::query(
                "UPDATE app.pickup_request
                 SET failed_pin_attempts = $3, status = 'ENGELLENDI',
                     pin_hash = NULL, pin_display_ciphertext = NULL,
                     block_code = 'WRONG_PIN', updated_at = clock_timestamp()
                 WHERE organization_id = $1 AND id = $2",
            )
        } else {
            sqlx::query(
                "UPDATE app.pickup_request
                 SET failed_pin_attempts = $3, updated_at = clock_timestamp()
                 WHERE organization_id = $1 AND id = $2",
            )
        };
        if update
            .bind(context.organization_id)
            .bind(request_id)
            .bind(failed_attempts)
            .execute(&mut *transaction)
            .await
            .is_err()
        {
            return server_error();
        }
        if transaction.commit().await.is_err() {
            return server_error();
        }
        return if blocked {
            api_error(StatusCode::LOCKED, "pickup_blocked", json!({}))
        } else {
            api_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "pin_invalid",
                json!({ "attemptsLeft": 3 - failed_attempts }),
            )
        };
    }

    if let Some(person_id) = person_id {
        let person_valid = sqlx::query_scalar::<_, bool>(
            "SELECT person.is_verified AND person.valid_from <= clock_timestamp()
                    AND person.valid_until > clock_timestamp()
                    AND (person.remaining_uses IS NULL OR person.remaining_uses > 0)
                    AND (
                      NOT COALESCE((SELECT authorized_person_approval_required
                                    FROM app.organization_settings WHERE organization_id = $1), false)
                      OR person.approved_at IS NOT NULL
                    )
             FROM app.authorized_person AS person
             WHERE person.organization_id = $1 AND person.id = $2
             FOR UPDATE",
        )
        .bind(context.organization_id)
        .bind(person_id)
        .fetch_optional(&mut *transaction)
        .await;
        if !matches!(person_valid, Ok(Some(true))) {
            let _ = transaction.rollback().await;
            return api_error(StatusCode::CONFLICT, "conflict", json!({}));
        }
    }

    let updated = sqlx::query(
        "UPDATE app.pickup_request
         SET status = 'TESLIM_EDILDI', handed_over_by_user_id = $3,
             completed_at = clock_timestamp(), pin_hash = NULL,
             pin_display_ciphertext = NULL, updated_at = clock_timestamp()
         WHERE organization_id = $1 AND id = $2
           AND status IN ('YOLDA', 'HAZIRLANIYOR')",
    )
    .bind(context.organization_id)
    .bind(request_id)
    .bind(claims.sub)
    .execute(&mut *transaction)
    .await;
    if !matches!(updated, Ok(result) if result.rows_affected() == 1) {
        return api_error(StatusCode::CONFLICT, "conflict", json!({}));
    }
    if let Some(person_id) = person_id
        && sqlx::query(
            "UPDATE app.authorized_person
             SET remaining_uses = remaining_uses - 1
             WHERE organization_id = $1 AND id = $2 AND remaining_uses IS NOT NULL",
        )
        .bind(context.organization_id)
        .bind(person_id)
        .execute(&mut *transaction)
        .await
        .is_err()
    {
        return server_error();
    }
    let inserted = sqlx::query(
        "INSERT INTO app.daily_record
           (organization_id, child_id, record_type, data_category, date, occurred_at,
            details_json, created_by_user_id)
         SELECT $1, pickup.child_id, 'TESLIM', 'TESLIM',
                (clock_timestamp() AT TIME ZONE organization.timezone)::date,
                clock_timestamp(), jsonb_build_object('pickupRequestId', pickup.id), $3
         FROM app.pickup_request AS pickup
         JOIN app.organization AS organization ON organization.id = pickup.organization_id
         WHERE pickup.organization_id = $1 AND pickup.id = $2",
    )
    .bind(context.organization_id)
    .bind(request_id)
    .bind(claims.sub)
    .execute(&mut *transaction)
    .await;
    if inserted.is_err() {
        return server_error();
    }
    let row = match pickup_row(&mut transaction, context, request_id).await {
        Ok(Some(row)) => row,
        Ok(None) => return forbidden(),
        Err(error) => return error,
    };
    if transaction.commit().await.is_err() {
        return server_error();
    }
    match pickup_dto(&state, context.organization_id, row, claims.sub, false).await {
        Ok(dto) => Json(dto).into_response(),
        Err(error) => error,
    }
}

async fn list_privacy_requests(
    State(state): State<ApiState>,
    Extension(claims): Extension<Claims>,
) -> Response {
    let memberships = match user_tenant_memberships(&state, claims.sub).await {
        Ok(memberships) => memberships,
        Err(error) => return error,
    };
    let mut requests = Vec::new();
    for membership in memberships {
        let context = TenantContext {
            user_id: claims.sub,
            organization_id: membership.organization_id,
        };
        let mut transaction = match state.postgres.begin_tenant(context).await {
            Ok(transaction) => transaction,
            Err(_) => return server_error(),
        };
        let rows = sqlx::query_as::<_, (Uuid, String, String, String, String)>(
            "SELECT id, type,
                    to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"'),
                    to_char(legal_deadline_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"'),
                    status
             FROM app.privacy_request
             WHERE organization_id = $1 AND user_id = $2
             ORDER BY created_at DESC, id DESC",
        )
        .bind(context.organization_id)
        .bind(claims.sub)
        .fetch_all(&mut *transaction)
        .await;
        let rows = match rows {
            Ok(rows) => rows,
            Err(_) => return server_error(),
        };
        if transaction.commit().await.is_err() {
            return server_error();
        }
        requests.extend(
            rows.into_iter()
                .map(|(id, kind, created_at, due_at, status)| PrivacyRequestDto {
                    id,
                    kurum_id: context.organization_id,
                    request_type: if kind == "DIGER" {
                        "HESAP_SILME".to_owned()
                    } else {
                        kind
                    },
                    created_at,
                    due_at,
                    status: if matches!(status.as_str(), "YANITLANDI" | "KAPATILDI") {
                        "YANITLANDI".to_owned()
                    } else {
                        "ACIK".to_owned()
                    },
                }),
        );
    }
    Json(requests).into_response()
}

async fn create_privacy_request(
    State(state): State<ApiState>,
    Extension(claims): Extension<Claims>,
    Json(input): Json<PrivacyRequestInput>,
) -> Response {
    const VALID_TYPES: [&str; 5] = ["BILGI", "DUZELTME", "SILME", "ITIRAZ", "HESAP_SILME"];
    if !VALID_TYPES.contains(&input.request_type.as_str())
        || input.description.trim().is_empty()
        || input.description.chars().count() > 2000
    {
        return validation_error("request");
    }
    let Some(context) = (match resolve_organization_member(&state, claims.sub, input.kurum_id).await
    {
        Ok(context) => context,
        Err(error) => return error,
    }) else {
        return forbidden();
    };
    let (description_ciphertext, _) = match state
        .vault
        .transit_encrypt(
            &format!("kurum-{}", context.organization_id),
            input.description.as_bytes(),
        )
        .await
    {
        Ok(ciphertext) => ciphertext,
        Err(_) => return server_error(),
    };
    let database_type = if input.request_type == "HESAP_SILME" {
        "DIGER"
    } else {
        input.request_type.as_str()
    };
    let mut transaction = match state.postgres.begin_tenant(context).await {
        Ok(transaction) => transaction,
        Err(_) => return server_error(),
    };
    let row = sqlx::query_as::<_, (Uuid, String, String, String)>(
        "INSERT INTO app.privacy_request
           (organization_id, user_id, type, description_ciphertext,
            operational_target_at, legal_deadline_at)
         VALUES ($1, $2, $3, $4, clock_timestamp() + interval '15 days',
                 clock_timestamp() + interval '30 days')
         RETURNING id,
                   to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"'),
                   to_char(legal_deadline_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"'),
                   status",
    )
    .bind(context.organization_id)
    .bind(claims.sub)
    .bind(database_type)
    .bind(description_ciphertext)
    .fetch_one(&mut *transaction)
    .await;
    let (id, created_at, due_at, status) = match row {
        Ok(row) => row,
        Err(_) => return server_error(),
    };
    if transaction.commit().await.is_err() {
        return server_error();
    }
    (
        StatusCode::CREATED,
        Json(PrivacyRequestDto {
            id,
            kurum_id: context.organization_id,
            request_type: input.request_type,
            created_at,
            due_at,
            status: if status == "ACIK" {
                "ACIK"
            } else {
                "YANITLANDI"
            }
            .to_owned(),
        }),
    )
        .into_response()
}

async fn create_data_export(
    State(state): State<ApiState>,
    Extension(claims): Extension<Claims>,
    Json(input): Json<DataExportInput>,
) -> Response {
    let Some(child_context) =
        (match resolve_child_context(&state, claims.sub, input.child_id, true).await {
            Ok(context) => context,
            Err(error) => return error,
        })
    else {
        return forbidden();
    };
    let context = child_context.tenant;
    let mut transaction = match state.postgres.begin_tenant(context).await {
        Ok(transaction) => transaction,
        Err(_) => return server_error(),
    };
    let request_id = match sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO app.data_export_request (organization_id, user_id)
         VALUES ($1, $2) RETURNING id",
    )
    .bind(context.organization_id)
    .bind(claims.sub)
    .fetch_one(&mut *transaction)
    .await
    {
        Ok(id) => id,
        Err(_) => return server_error(),
    };
    if transaction.commit().await.is_err() {
        return server_error();
    }
    (
        StatusCode::ACCEPTED,
        Json(DataExportResponse {
            request_id,
            link_valid_hours: 24,
        }),
    )
        .into_response()
}

async fn record_audit_event(
    State(state): State<ApiState>,
    Extension(claims): Extension<Claims>,
    headers: HeaderMap,
    Json(input): Json<Value>,
) -> Response {
    let Some(event_type) = input.get("type").and_then(Value::as_str) else {
        return validation_error("type");
    };
    let Some(screen) = input.get("screen").and_then(Value::as_str) else {
        return validation_error("screen");
    };
    if event_type != "EKRAN_GORUNTUSU" || !matches!(screen, "teslim-kodu" | "teslim-dogrulama") {
        return validation_error("event");
    }
    let memberships = match user_tenant_memberships(&state, claims.sub).await {
        Ok(memberships) => memberships,
        Err(error) => return error,
    };
    if memberships.is_empty() {
        return forbidden();
    }
    for membership in memberships {
        let context = TenantContext {
            user_id: claims.sub,
            organization_id: membership.organization_id,
        };
        let mut transaction = match state.postgres.begin_tenant(context).await {
            Ok(transaction) => transaction,
            Err(_) => return server_error(),
        };
        let idempotency_key = match reserve_idempotency(
            &state,
            &mut transaction,
            context,
            &headers,
            "/v1/audit/events",
            &input,
        )
        .await
        {
            Ok(IdempotencyDecision::New(key)) => key,
            Ok(IdempotencyDecision::Replay(status, _)) => {
                if transaction.commit().await.is_err() {
                    return server_error();
                }
                if status != StatusCode::NO_CONTENT {
                    return server_error();
                }
                continue;
            }
            Err(error) => return error,
        };
        let chain_id = match sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO app.audit_chain_head (organization_id, chain_type)
             VALUES ($1, 'KURUM')
             ON CONFLICT (organization_id) WHERE organization_id IS NOT NULL DO NOTHING
             RETURNING id",
        )
        .bind(context.organization_id)
        .fetch_optional(&mut *transaction)
        .await
        {
            Ok(Some(id)) => id,
            Ok(None) => match sqlx::query_scalar::<_, Uuid>(
                "SELECT id FROM app.audit_chain_head
                 WHERE organization_id = $1 AND chain_type = 'KURUM'
                 FOR UPDATE",
            )
            .bind(context.organization_id)
            .fetch_one(&mut *transaction)
            .await
            {
                Ok(id) => id,
                Err(_) => return server_error(),
            },
            Err(_) => return server_error(),
        };
        let head = sqlx::query_as::<_, (Option<Vec<u8>>, Option<Uuid>)>(
            "SELECT last_hash, last_record_id FROM app.audit_chain_head
             WHERE id = $1 AND organization_id = $2 FOR UPDATE",
        )
        .bind(chain_id)
        .bind(context.organization_id)
        .fetch_one(&mut *transaction)
        .await;
        let (previous_hash, _) = match head {
            Ok(head) => head,
            Err(_) => return server_error(),
        };
        let record_id = Uuid::now_v7();
        let details = json!({ "screen": screen });
        let details_json = match serde_json::to_vec(&details) {
            Ok(value) => value,
            Err(_) => return server_error(),
        };
        let (details_ciphertext, _) = match state
            .vault
            .transit_encrypt(&format!("kurum-{}", context.organization_id), &details_json)
            .await
        {
            Ok(value) => value,
            Err(_) => return server_error(),
        };
        let occurred_at = match sqlx::query_scalar::<_, String>(
            "SELECT to_char(clock_timestamp() AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"')",
        )
        .fetch_one(&mut *transaction)
        .await
        {
            Ok(value) => value,
            Err(_) => return server_error(),
        };
        let hash_material = json!({
            "recordId": record_id,
            "organizationId": context.organization_id,
            "actorUserId": claims.sub,
            "action": event_type,
            "targetType": "MOBIL_EKRAN",
            "occurredAt": occurred_at
        });
        let mut digest = Sha256::new();
        if let Some(previous_hash) = &previous_hash {
            digest.update(previous_hash);
        }
        digest.update(match serde_json::to_vec(&hash_material) {
            Ok(bytes) => bytes,
            Err(_) => return server_error(),
        });
        let record_hash = digest.finalize();
        if sqlx::query(
            "INSERT INTO app.audit_record
               (id, organization_id, chain_id, actor_user_id, action, target_type,
                occurred_at, previous_hash, record_hash, details_ciphertext)
             VALUES ($1, $2, $3, $4, $5, 'MOBIL_EKRAN', $6::timestamptz, $7, $8, $9)",
        )
        .bind(record_id)
        .bind(context.organization_id)
        .bind(chain_id)
        .bind(claims.sub)
        .bind(event_type)
        .bind(&occurred_at)
        .bind(previous_hash.as_deref())
        .bind(record_hash.as_slice())
        .bind(details_ciphertext)
        .execute(&mut *transaction)
        .await
        .is_err()
        {
            return server_error();
        }
        if sqlx::query(
            "UPDATE app.audit_chain_head
             SET last_record_id = $2, last_hash = $3, last_event_at = $4::timestamptz
             WHERE id = $1 AND organization_id = $5",
        )
        .bind(chain_id)
        .bind(record_id)
        .bind(record_hash.as_slice())
        .bind(&occurred_at)
        .bind(context.organization_id)
        .execute(&mut *transaction)
        .await
        .is_err()
        {
            return server_error();
        }
        if complete_idempotency(
            &state,
            &mut transaction,
            context,
            idempotency_key,
            StatusCode::NO_CONTENT,
            &Value::Null,
        )
        .await
        .is_err()
        {
            return server_error();
        }
        if transaction.commit().await.is_err() {
            return server_error();
        }
    }
    StatusCode::NO_CONTENT.into_response()
}

pub(crate) fn router() -> Router<ApiState> {
    Router::new()
        .route("/v1/me", get(get_me))
        .route(
            "/v1/organizations/{organization_id}/privacy-notice",
            get(get_privacy_notice),
        )
        .route(
            "/v1/organizations/{organization_id}/privacy-notice/{version}/acknowledgement",
            post(acknowledge_privacy_notice),
        )
        .route(
            "/v1/children/{child_id}/consents",
            get(get_consents).put(update_consents),
        )
        .route("/v1/children/{child_id}/feed", get(get_feed))
        .route("/v1/classes/{class_id}/today", get(get_class_today))
        .route("/v1/classes/{class_id}/meal-records", post(record_meals))
        .route(
            "/v1/children/{child_id}/authorized-pickup-people",
            get(get_authorized_people),
        )
        .route(
            "/v1/children/{child_id}/pickup-requests/active",
            get(get_active_pickup),
        )
        .route("/v1/pickup-requests", post(create_pickup))
        .route("/v1/classes/{class_id}/pickup-queue", get(get_pickup_queue))
        .route(
            "/v1/pickup-requests/{request_id}/preparing",
            post(mark_pickup_preparing),
        )
        .route(
            "/v1/pickup-requests/{request_id}/handover",
            post(hand_over_pickup),
        )
        .route(
            "/v1/privacy/requests",
            get(list_privacy_requests).post(create_privacy_request),
        )
        .route("/v1/privacy/data-exports", post(create_data_export))
        .route("/v1/audit/events", post(record_audit_event))
}

async fn get_me(
    State(state): State<ApiState>,
    Extension(claims): Extension<Claims>,
) -> impl IntoResponse {
    let mut user_tx = match state.postgres.begin_user(claims.sub).await {
        Ok(transaction) => transaction,
        Err(_) => return api_error(StatusCode::SERVICE_UNAVAILABLE, "server", json!({})),
    };
    let user_row = sqlx::query_as::<_, (Option<String>, Option<String>)>(
        "SELECT first_name_ciphertext, last_name_ciphertext
         FROM app.user_account WHERE id = $1 AND status = 'AKTIF'",
    )
    .bind(claims.sub)
    .fetch_optional(&mut *user_tx)
    .await;
    let user_row = match user_row {
        Ok(Some(row)) => row,
        Ok(None) => {
            let _ = user_tx.rollback().await;
            return api_error(StatusCode::UNAUTHORIZED, "unauthorized", json!({}));
        }
        Err(_) => return api_error(StatusCode::SERVICE_UNAVAILABLE, "server", json!({})),
    };
    let memberships = sqlx::query_as::<_, MembershipRow>(
        "SELECT organization_id, membership_id, organization_name, role, membership_status
         FROM app.list_user_memberships()",
    )
    .fetch_all(&mut *user_tx)
    .await;
    let memberships = match memberships {
        Ok(memberships) => memberships,
        Err(_) => return api_error(StatusCode::SERVICE_UNAVAILABLE, "server", json!({})),
    };
    if user_tx.commit().await.is_err() {
        return api_error(StatusCode::SERVICE_UNAVAILABLE, "server", json!({}));
    }

    let mut display_name_parts = Vec::new();
    for ciphertext in [user_row.0.as_deref(), user_row.1.as_deref()]
        .into_iter()
        .flatten()
    {
        match decrypt_text(&state, "kullanici-pii", ciphertext).await {
            Ok(value) if !value.trim().is_empty() => display_name_parts.push(value),
            Ok(_) => {}
            Err(_) => return api_error(StatusCode::SERVICE_UNAVAILABLE, "server", json!({})),
        }
    }
    let display_name = if display_name_parts.is_empty() {
        "Okuldayım kullanıcısı".to_owned()
    } else {
        display_name_parts.join(" ")
    };

    let mut membership_dtos = Vec::new();
    let mut pending_notices = Vec::new();
    let mut consent_review_children = Vec::new();
    for (organization_id, membership_id, organization_name, role, status) in memberships {
        if status != "AKTIF" {
            continue;
        }
        let mut transaction = match state
            .postgres
            .begin_tenant(TenantContext {
                user_id: claims.sub,
                organization_id,
            })
            .await
        {
            Ok(transaction) => transaction,
            Err(_) => return api_error(StatusCode::SERVICE_UNAVAILABLE, "server", json!({})),
        };

        let class_ids = if matches!(role.as_str(), "OGRETMEN" | "KURUM_YONETICISI") {
            match sqlx::query_scalar::<_, Uuid>(
                "SELECT class_id FROM app.membership_class
                 WHERE organization_id = $1 AND membership_id = $2
                 ORDER BY class_id",
            )
            .bind(organization_id)
            .bind(membership_id)
            .fetch_all(&mut *transaction)
            .await
            {
                Ok(values) => values,
                Err(_) => return api_error(StatusCode::SERVICE_UNAVAILABLE, "server", json!({})),
            }
        } else {
            Vec::new()
        };

        let child_rows = if role == "VELI" {
            match sqlx::query_as::<_, (Uuid, String, String, Uuid, String)>(
                "SELECT child.id, child.first_name_ciphertext, child.last_name_ciphertext,
                        classroom.id, classroom.name
                 FROM app.guardian_child AS guardian_child
                 JOIN app.child AS child
                   ON child.organization_id = guardian_child.organization_id
                  AND child.id = guardian_child.child_id
                 JOIN app.classroom AS classroom
                   ON classroom.organization_id = child.organization_id
                  AND classroom.id = child.class_id
                 WHERE guardian_child.organization_id = $1
                   AND guardian_child.guardian_user_id = $2
                   AND guardian_child.is_active
                   AND guardian_child.access_level <> 'KAPALI'
                   AND child.status = 'AKTIF'
                 ORDER BY child.id",
            )
            .bind(organization_id)
            .bind(claims.sub)
            .fetch_all(&mut *transaction)
            .await
            {
                Ok(values) => values,
                Err(_) => return api_error(StatusCode::SERVICE_UNAVAILABLE, "server", json!({})),
            }
        } else {
            Vec::new()
        };

        let has_pending_notice = match sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (
               SELECT 1 FROM app.privacy_notice AS notice
               WHERE notice.organization_id = $1 AND notice.audience = 'VELI'
                 AND notice.effective_at IS NOT NULL AND notice.effective_at <= clock_timestamp()
                 AND NOT EXISTS (
                   SELECT 1 FROM app.privacy_notice_read AS notice_read
                   WHERE notice_read.organization_id = notice.organization_id
                     AND notice_read.notice_id = notice.id AND notice_read.user_id = $2
                 )
             )",
        )
        .bind(organization_id)
        .bind(claims.sub)
        .fetch_one(&mut *transaction)
        .await
        {
            Ok(value) => value,
            Err(_) => return api_error(StatusCode::SERVICE_UNAVAILABLE, "server", json!({})),
        };
        if has_pending_notice {
            pending_notices.push(organization_id);
        }

        for (child_id, _, _, _, _) in &child_rows {
            let needs_review = match sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS (
                   SELECT 1 FROM app.privacy_notice AS notice
                   WHERE notice.organization_id = $1 AND notice.audience = 'VELI'
                     AND notice.effective_at IS NOT NULL AND notice.effective_at <= clock_timestamp()
                     AND NOT EXISTS (
                       SELECT 1 FROM app.consent_review AS review
                       WHERE review.organization_id = notice.organization_id
                         AND review.notice_id = notice.id AND review.child_id = $2
                         AND review.guardian_user_id = $3
                     )
                 )",
            )
            .bind(organization_id)
            .bind(child_id)
            .bind(claims.sub)
            .fetch_one(&mut *transaction)
            .await
            {
                Ok(value) => value,
                Err(_) => return api_error(StatusCode::SERVICE_UNAVAILABLE, "server", json!({})),
            };
            if needs_review {
                consent_review_children.push(*child_id);
            }
        }

        if transaction.commit().await.is_err() {
            return api_error(StatusCode::SERVICE_UNAVAILABLE, "server", json!({}));
        }

        let mut children = Vec::with_capacity(child_rows.len());
        for (child_id, first_ciphertext, last_ciphertext, class_id, class_name) in child_rows {
            let first_name = match decrypt_text(
                &state,
                &format!("kurum-{organization_id}"),
                &first_ciphertext,
            )
            .await
            {
                Ok(value) => value,
                Err(_) => return api_error(StatusCode::SERVICE_UNAVAILABLE, "server", json!({})),
            };
            let last_name = match decrypt_text(
                &state,
                &format!("kurum-{organization_id}"),
                &last_ciphertext,
            )
            .await
            {
                Ok(value) => value,
                Err(_) => return api_error(StatusCode::SERVICE_UNAVAILABLE, "server", json!({})),
            };
            children.push(ChildSummary {
                id: child_id,
                first_name,
                last_initial: last_name
                    .chars()
                    .next()
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
                class_id,
                class_name,
            });
        }
        membership_dtos.push(MembershipDto {
            id: membership_id,
            kurum_id: organization_id,
            kurum_name: organization_name,
            role,
            class_ids,
            children,
        });
    }

    Json(MeResponse {
        user: MeUser {
            id: claims.sub,
            display_name,
        },
        memberships: membership_dtos,
        pending_notice_kurum_ids: pending_notices,
        consent_review_child_ids: consent_review_children,
    })
    .into_response()
}

pub(crate) async fn decrypt_text(
    state: &ApiState,
    key: &str,
    ciphertext: &str,
) -> Result<String, ()> {
    let bytes = state
        .vault
        .transit_decrypt(key, ciphertext)
        .await
        .map_err(|_| ())?;
    String::from_utf8(bytes).map_err(|_| ())
}
