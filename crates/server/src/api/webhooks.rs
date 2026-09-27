use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use postwire_core::models::{Event, SmsMessage};
use postwire_core::store::NewSms;
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

use crate::api::messages::AppState;

// --- Slack Webhook ---

#[derive(Debug, Deserialize)]
pub struct SlackWebhookPayload {
    pub text: String,
    pub username: Option<String>,
    pub channel: Option<String>,
}

pub async fn slack_webhook(
    State(state): State<AppState>,
    Json(payload): Json<SlackWebhookPayload>,
) -> Result<(StatusCode, &'static str), (StatusCode, String)> {
    let id = Uuid::new_v4().to_string();
    let from = format!("slack:{}", payload.username.as_deref().unwrap_or("webhook"));
    let to = format!("slack:{}", payload.channel.as_deref().unwrap_or("default"));

    let new_sms = NewSms {
        id,
        from,
        to,
        body: payload.text,
    };

    let sms = state
        .store
        .insert_sms(new_sms)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let _ = state.tx.send(Event::NewSms(sms));

    Ok((StatusCode::OK, "ok"))
}

// --- Discord Webhook ---

#[derive(Debug, Deserialize)]
pub struct DiscordWebhookPayload {
    pub content: String,
    pub username: Option<String>,
}

pub async fn discord_webhook(
    State(state): State<AppState>,
    Json(payload): Json<DiscordWebhookPayload>,
) -> Result<StatusCode, (StatusCode, String)> {
    let id = Uuid::new_v4().to_string();
    let from = format!("discord:{}", payload.username.as_deref().unwrap_or("webhook"));
    let to = "discord:channel".to_string();

    let new_sms = NewSms {
        id,
        from,
        to,
        body: payload.content,
    };

    let sms = state
        .store
        .insert_sms(new_sms)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let _ = state.tx.send(Event::NewSms(sms));

    Ok(StatusCode::NO_CONTENT)
}

// --- Generic Webhook ---

pub async fn generic_webhook(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> Result<(StatusCode, Json<SmsMessage>), (StatusCode, String)> {
    let id = Uuid::new_v4().to_string();

    let from = payload
        .get("from")
        .and_then(|v| v.as_str())
        .unwrap_or("webhook:generic")
        .to_string();

    let to = payload
        .get("to")
        .and_then(|v| v.as_str())
        .unwrap_or("webhook:receiver")
        .to_string();

    let body = if let Some(body_str) = payload.get("body").and_then(|v| v.as_str()) {
        body_str.to_string()
    } else if let Some(text_str) = payload.get("text").and_then(|v| v.as_str()) {
        text_str.to_string()
    } else {
        payload.to_string()
    };

    let new_sms = NewSms { id, from, to, body };

    let sms = state
        .store
        .insert_sms(new_sms)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let _ = state.tx.send(Event::NewSms(sms.clone()));

    Ok((StatusCode::CREATED, Json(sms)))
}
