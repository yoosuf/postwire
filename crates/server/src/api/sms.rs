use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;
use tokio::sync::broadcast;

use postwire_core::mail;
use postwire_core::models::{BulkIdsBody, BulkReadBody, Event, ListQuery, MarkReadBody, SmsList, SmsMessage};
use postwire_core::store::NewSms;

use super::error::ApiError;
use super::AppState;

pub async fn list_sms(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> Result<Json<SmsList>, ApiError> {
    let limit = query.limit.unwrap_or(50).clamp(1, 500);
    let offset = query.offset.unwrap_or(0).max(0);

    let (messages, total) = state.store.list_sms(query.search.as_deref(), limit, offset)?;
    Ok(Json(SmsList { messages, total }))
}

pub async fn clear_sms(State(state): State<AppState>) -> Result<StatusCode, ApiError> {
    state.store.clear_sms()?;
    let _ = state.tx.send(Event::SmsCleared);
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_sms(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<SmsMessage>, ApiError> {
    let sms = state.store.get_sms(&id)?.ok_or(ApiError::NotFound)?;
    Ok(Json(sms))
}

pub async fn delete_sms(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    if state.store.delete_sms(&id)? {
        let _ = state.tx.send(Event::SmsDeleted { id });
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound)
    }
}

pub async fn mark_sms_read(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<MarkReadBody>,
) -> Result<StatusCode, ApiError> {
    if state.store.mark_sms_read(&id, body.read)? {
        let _ = state.tx.send(Event::SmsRead { id, read: body.read });
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound)
    }
}

pub async fn bulk_delete_sms(
    State(state): State<AppState>,
    Json(body): Json<BulkIdsBody>,
) -> Result<StatusCode, ApiError> {
    if body.ids.is_empty() {
        return Err(ApiError::BadRequest("ids list cannot be empty".to_string()));
    }
    state.store.delete_sms_many(&body.ids)?;
    let _ = state.tx.send(Event::BulkSmsDeleted { ids: body.ids });
    Ok(StatusCode::NO_CONTENT)
}

pub async fn bulk_mark_sms_read(
    State(state): State<AppState>,
    Json(body): Json<BulkReadBody>,
) -> Result<StatusCode, ApiError> {
    if body.ids.is_empty() {
        return Err(ApiError::BadRequest("ids list cannot be empty".to_string()));
    }
    state.store.mark_sms_read_many(&body.ids, body.read)?;
    let _ = state.tx.send(Event::BulkSmsRead { ids: body.ids, read: body.read });
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct ExtractSmsQuery {
    pub regex: Option<String>,
}

pub async fn get_sms_extract(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<ExtractSmsQuery>,
) -> Result<Json<postwire_core::mail::ExtractedSignals>, ApiError> {
    let sms = state.store.get_sms(&id)?.ok_or(ApiError::NotFound)?;
    let signals = mail::extract_signals_with_custom_regex(Some(&sms.body), None, query.regex.as_deref());
    Ok(Json(signals))
}

#[derive(Deserialize)]
pub struct IngestSmsPayload {
    pub from: Option<String>,
    pub to: Option<String>,
    pub body: Option<String>,
    // Twilio Webhook fallback fields
    #[serde(rename = "From")]
    pub twilio_from: Option<String>,
    #[serde(rename = "To")]
    pub twilio_to: Option<String>,
    #[serde(rename = "Body")]
    pub twilio_body: Option<String>,
}

pub async fn ingest_sms(
    State(state): State<AppState>,
    Json(payload): Json<IngestSmsPayload>,
) -> Result<impl IntoResponse, ApiError> {
    let from = payload
        .from
        .or(payload.twilio_from)
        .unwrap_or_else(|| "Unknown".to_string());
    let to = payload
        .to
        .or(payload.twilio_to)
        .unwrap_or_else(|| "Pine SMS".to_string());
    let body = payload
        .body
        .or(payload.twilio_body)
        .unwrap_or_default();

    let new_sms = NewSms {
        id: uuid::Uuid::new_v4().to_string(),
        from,
        to,
        body,
    };

    let sms = state.store.insert_sms(new_sms)?;
    let _ = state.tx.send(Event::NewSms(sms.clone()));
    Ok((StatusCode::CREATED, Json(sms)))
}

pub async fn sms_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: String,
) -> Result<impl IntoResponse, ApiError> {
    let content_type = headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let payload: IngestSmsPayload = if content_type.contains("application/x-www-form-urlencoded") {
        serde_urlencoded::from_str(&body).map_err(|_| ApiError::BadRequest("invalid form data".to_string()))?
    } else {
        serde_json::from_str(&body).map_err(|_| ApiError::BadRequest("invalid json payload".to_string()))?
    };

    ingest_sms(State(state), Json(payload)).await
}

#[derive(Deserialize, Default)]
pub struct SendTestSmsBody {
    pub to: Option<String>,
    pub from: Option<String>,
    pub body: Option<String>,
}

pub async fn send_test_sms(
    State(state): State<AppState>,
    Json(body): Json<SendTestSmsBody>,
) -> Result<Json<SmsMessage>, ApiError> {
    let to = body.to.unwrap_or_else(|| "+15550100".to_string());
    let from = body.from.unwrap_or_else(|| "+18005550199".to_string());
    let code = format!("{:06}", rand::random::<u32>() % 1_000_000);
    let text = body.body.unwrap_or_else(|| format!("✅ Your Postwire verification code is {code}"));

    let new_sms = NewSms {
        id: uuid::Uuid::new_v4().to_string(),
        from,
        to,
        body: text,
    };

    let sms = state.store.insert_sms(new_sms)?;
    let _ = state.tx.send(Event::NewSms(sms.clone()));
    Ok(Json(sms))
}

#[derive(Deserialize)]
pub struct WaitSmsQuery {
    pub to: Option<String>,
    pub from: Option<String>,
    pub body: Option<String>,
    pub since: Option<String>,
    pub timeout_ms: Option<u64>,
}

/// Long-polls for the next SMS matching the given filters — the primary hook for
/// agentic/e2e tests that need to wait for an SMS OTP code an action just triggered.
pub async fn wait_for_sms(
    State(state): State<AppState>,
    Query(query): Query<WaitSmsQuery>,
) -> Result<Json<SmsMessage>, ApiError> {
    let since = query
        .since
        .unwrap_or_else(|| chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
    let timeout_ms = query.timeout_ms.unwrap_or(10_000).clamp(100, 60_000);
    let mut rx = state.tx.subscribe();

    // Check pre-existing items first
    if let Some(sms) = state.store.find_matching_sms(
        query.to.as_deref(),
        query.from.as_deref(),
        query.body.as_deref(),
        Some(&since),
    )? {
        return Ok(Json(sms));
    }

    let sleep = tokio::time::sleep(std::time::Duration::from_millis(timeout_ms));
    tokio::pin!(sleep);

    loop {
        tokio::select! {
            _ = &mut sleep => {
                return Err(ApiError::Timeout("timed out waiting for a matching SMS"));
            }
            res = rx.recv() => {
                match res {
                    Ok(Event::NewSms(ref sms)) => {
                        let to_match = query.to.as_deref().is_none_or(|t| sms.to.contains(t));
                        let from_match = query.from.as_deref().is_none_or(|f| sms.from.contains(f));
                        let body_match = query.body.as_deref().is_none_or(|b| sms.body.contains(b));
                        let since_match = sms.received_at > since;

                        if to_match && from_match && body_match && since_match {
                            return Ok(Json(sms.clone()));
                        }
                    }
                    Ok(_) => {}
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        if let Ok(Some(sms)) = state.store.find_matching_sms(
                            query.to.as_deref(),
                            query.from.as_deref(),
                            query.body.as_deref(),
                            Some(&since),
                        ) {
                            return Ok(Json(sms));
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        return Err(ApiError::Internal(anyhow::anyhow!("event bus closed")));
                    }
                }
            }
        }
    }
}

pub async fn replay_sms(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<super::messages::ReplayPayload>,
) -> Result<Json<super::messages::ReplayResponse>, ApiError> {
    let sms = state.store.get_sms(&id)?.ok_or(ApiError::NotFound)?;

    if let Some(target) = body.target_url {
        let client = reqwest::Client::new();
        let res = client.post(&target).json(&sms).send().await;
        match res {
            Ok(resp) => Ok(Json(super::messages::ReplayResponse {
                success: resp.status().is_success(),
                status: resp.status().as_u16(),
                message: format!("Replayed SMS to {}", target),
            })),
            Err(e) => Ok(Json(super::messages::ReplayResponse {
                success: false,
                status: 500,
                message: format!("Failed to replay SMS to {}: {}", target, e),
            })),
        }
    } else {
        let _ = state.tx.send(Event::NewSms(sms));
        Ok(Json(super::messages::ReplayResponse {
            success: true,
            status: 200,
            message: "Re-broadcasted SMS event over WebSocket".to_string(),
        }))
    }
}


