use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use postwire_core::models::Event;
use postwire_core::store::Store;
use postwire_server::api::{router, AppState};
use serde_json::json;
use tokio::sync::broadcast;
use tower::ServiceExt;

fn setup_app() -> axum::Router {
    let store = Arc::new(Store::new(":memory:", 1000).unwrap());
    let (tx, _) = broadcast::channel::<Event>(100);
    let state = AppState { store, tx };
    router(state)
}

#[tokio::test]
async fn synthetic_test_email_uses_the_postwire_sender_identity() {
    let app = setup_app();
    let req = Request::builder()
        .method("POST")
        .uri("/api/test-email")
        .header("content-type", "application/json")
        .body(Body::from(json!({"to": "user@example.com"}).to_string()))
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let message: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(message["from"], "Postwire <no-reply@postwire.local>");
}

#[tokio::test]
async fn test_resend_ingest_and_custom_regex_extract() {
    let app = setup_app();

    // 1. Post to Resend endpoint
    let req = Request::builder()
        .method("POST")
        .uri("/emails")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "from": "resend@app.test",
                "to": ["user@example.com"],
                "subject": "Resend Verification",
                "html": "<p>Your code is 884422 and invite token is INVITE-XYZ789</p>"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let res: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let msg_id = res["id"].as_str().unwrap();

    // 2. Extract with custom regex
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/messages/{msg_id}/extract?regex=INVITE-[A-Z0-9]%2B"))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let signals: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(signals["codes"][0], "884422");
    assert_eq!(signals["matches"][0], "INVITE-XYZ789");
}

#[tokio::test]
async fn test_sendgrid_and_postmark_ingest() {
    let app = setup_app();

    // 1. SendGrid POST /v3/mail/send
    let req = Request::builder()
        .method("POST")
        .uri("/v3/mail/send")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "personalizations": [{"to": [{"email": "sg.to@example.com"}]}],
                "from": {"email": "sg.from@example.com", "name": "SendGrid Sender"},
                "subject": "SendGrid Test",
                "content": [{"type": "text/html", "value": "SendGrid content"}]
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);

    // 2. Postmark POST /email
    let req = Request::builder()
        .method("POST")
        .uri("/email")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "From": "pm.from@example.com",
                "To": "pm.to@example.com",
                "Subject": "Postmark Test",
                "TextBody": "Postmark text body"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_webhooks_slack_discord_generic() {
    let app = setup_app();

    // 1. Slack Webhook
    let req = Request::builder()
        .method("POST")
        .uri("/api/webhooks/slack")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "text": "Slack notification: OTP is 112233",
                "username": "AlertBot"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 2. Discord Webhook
    let req = Request::builder()
        .method("POST")
        .uri("/api/webhooks/discord")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "content": "Discord alert message",
                "username": "DiscordBot"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // 3. Generic Webhook
    let req = Request::builder()
        .method("POST")
        .uri("/api/webhooks/generic")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "from": "custom-service",
                "to": "test-channel",
                "body": "Generic alert code 556677"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
}
