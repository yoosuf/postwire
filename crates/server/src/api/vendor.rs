use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use postwire_core::mail::build_vendor_mime;
use postwire_core::models::Event;
use postwire_core::store::NewMessage;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::messages::AppState;

// --- Resend API Models & Handler ---

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum StringOrVec {
    Single(String),
    Multiple(Vec<String>),
}

impl StringOrVec {
    fn into_vec(self) -> Vec<String> {
        match self {
            StringOrVec::Single(s) => vec![s],
            StringOrVec::Multiple(v) => v,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ResendPayload {
    pub from: String,
    pub to: StringOrVec,
    pub subject: String,
    pub text: Option<String>,
    pub html: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ResendResponse {
    pub id: String,
}

pub async fn resend_ingest(
    State(state): State<AppState>,
    Json(payload): Json<ResendPayload>,
) -> Result<(StatusCode, Json<ResendResponse>), (StatusCode, String)> {
    let id = Uuid::new_v4().to_string();
    let to_vec = payload.to.into_vec();
    let raw = build_vendor_mime(
        &payload.from,
        &to_vec,
        &payload.subject,
        payload.text.as_deref(),
        payload.html.as_deref(),
    );

    let new_msg = NewMessage {
        id: id.clone(),
        from: payload.from,
        to: to_vec,
        subject: payload.subject,
        size: raw.len() as i64,
        raw,
    };

    let summary = state
        .store
        .insert(new_msg)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let _ = state.tx.send(Event::New(summary));

    Ok((StatusCode::OK, Json(ResendResponse { id })))
}

// --- SendGrid API Models & Handler ---

#[derive(Debug, Deserialize)]
pub struct SendGridEmail {
    pub email: String,
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SendGridPersonalization {
    pub to: Vec<SendGridEmail>,
}

#[derive(Debug, Deserialize)]
pub struct SendGridContent {
    pub r#type: String,
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct SendGridPayload {
    pub personalizations: Vec<SendGridPersonalization>,
    pub from: SendGridEmail,
    pub subject: String,
    pub content: Option<Vec<SendGridContent>>,
}

pub async fn sendgrid_ingest(
    State(state): State<AppState>,
    Json(payload): Json<SendGridPayload>,
) -> Result<StatusCode, (StatusCode, String)> {
    let id = Uuid::new_v4().to_string();

    let mut to_vec = Vec::new();
    for p in &payload.personalizations {
        for t in &p.to {
            let addr = match &t.name {
                Some(name) if !name.is_empty() => format!("{} <{}>", name, t.email),
                _ => t.email.clone(),
            };
            to_vec.push(addr);
        }
    }

    let from_addr = match &payload.from.name {
        Some(name) if !name.is_empty() => format!("{} <{}>", name, payload.from.email),
        _ => payload.from.email.clone(),
    };

    let mut text_body = None;
    let mut html_body = None;
    if let Some(contents) = payload.content {
        for c in contents {
            if c.r#type.contains("text/plain") {
                text_body = Some(c.value);
            } else if c.r#type.contains("text/html") {
                html_body = Some(c.value);
            }
        }
    }

    let raw = build_vendor_mime(
        &from_addr,
        &to_vec,
        &payload.subject,
        text_body.as_deref(),
        html_body.as_deref(),
    );

    let new_msg = NewMessage {
        id,
        from: from_addr,
        to: to_vec,
        subject: payload.subject,
        size: raw.len() as i64,
        raw,
    };

    let summary = state
        .store
        .insert(new_msg)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let _ = state.tx.send(Event::New(summary));

    Ok(StatusCode::ACCEPTED)
}

// --- Postmark API Models & Handler ---

#[derive(Debug, Deserialize)]
pub struct PostmarkPayload {
    #[serde(rename = "From")]
    pub from: String,
    #[serde(rename = "To")]
    pub to: String,
    #[serde(rename = "Subject")]
    pub subject: String,
    #[serde(rename = "TextBody")]
    pub text_body: Option<String>,
    #[serde(rename = "HtmlBody")]
    pub html_body: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PostmarkResponse {
    #[serde(rename = "To")]
    pub to: String,
    #[serde(rename = "SubmittedAt")]
    pub submitted_at: String,
    #[serde(rename = "MessageID")]
    pub message_id: String,
    #[serde(rename = "ErrorCode")]
    pub error_code: i32,
    #[serde(rename = "Message")]
    pub message: String,
}

pub async fn postmark_ingest(
    State(state): State<AppState>,
    Json(payload): Json<PostmarkPayload>,
) -> Result<(StatusCode, Json<PostmarkResponse>), (StatusCode, String)> {
    let id = Uuid::new_v4().to_string();
    let to_vec = vec![payload.to.clone()];

    let raw = build_vendor_mime(
        &payload.from,
        &to_vec,
        &payload.subject,
        payload.text_body.as_deref(),
        payload.html_body.as_deref(),
    );

    let new_msg = NewMessage {
        id: id.clone(),
        from: payload.from,
        to: to_vec,
        subject: payload.subject,
        size: raw.len() as i64,
        raw,
    };

    let summary = state
        .store
        .insert(new_msg)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let _ = state.tx.send(Event::New(summary));

    Ok((
        StatusCode::OK,
        Json(PostmarkResponse {
            to: payload.to,
            submitted_at: chrono::Utc::now().to_rfc3339(),
            message_id: id,
            error_code: 0,
            message: "OK".to_string(),
        }),
    ))
}
