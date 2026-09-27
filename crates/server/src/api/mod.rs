pub mod error;
pub mod messages;
pub mod sms;
pub mod vendor;
pub mod webhooks;
pub mod ws;


use axum::routing::{get, patch, post};
use axum::Router;
use serde::Serialize;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

pub use messages::AppState;

#[derive(Serialize)]
struct ServerInfo {
    version: &'static str,
    smtp_port: u16,
    http_port: u16,
}

async fn get_config(
    axum::extract::State(_state): axum::extract::State<AppState>,
) -> axum::Json<ServerInfo> {
    axum::Json(ServerInfo {
        version: env!("CARGO_PKG_VERSION"),
        smtp_port: std::env::var("POSTWIRE_SMTP_PORT")
            .or_else(|_| std::env::var("SMTP_PORT"))
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1025),
        http_port: std::env::var("POSTWIRE_HTTP_PORT")
            .or_else(|_| std::env::var("HTTP_PORT"))
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(8025),
    })
}

static START_INSTANT: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

pub fn init_start_instant() {
    START_INSTANT.get_or_init(std::time::Instant::now);
}

#[derive(Serialize)]
struct HealthStatus {
    status: &'static str,
    version: &'static str,
    uptime_seconds: u64,
    messages_count: i64,
    sms_count: i64,
    db_status: &'static str,
}

async fn get_health(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Result<axum::Json<HealthStatus>, (axum::http::StatusCode, String)> {
    init_start_instant();
    let uptime = START_INSTANT.get().map(|i| i.elapsed().as_secs()).unwrap_or(0);
    let (messages_count, sms_count) = state
        .store
        .counts()
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(axum::Json(HealthStatus {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        uptime_seconds: uptime,
        messages_count,
        sms_count,
        db_status: "connected",
    }))
}

pub fn router(state: AppState) -> Router {
    init_start_instant();
    Router::new()
        .route(
            "/api/messages",
            get(messages::list_messages).delete(messages::clear_messages),
        )
        .route("/api/messages/bulk-delete", post(messages::bulk_delete))
        .route("/api/messages/bulk-read", patch(messages::bulk_mark_read))
        .route(
            "/api/messages/:id",
            get(messages::get_message).delete(messages::delete_message),
        )
        .route("/api/messages/:id/read", patch(messages::mark_read))
        .route("/api/messages/:id/raw", get(messages::get_raw))
        .route("/api/messages/:id/html", get(messages::get_html))
        .route(
            "/api/messages/:id/attachments/:index",
            get(messages::get_attachment),
        )
        .route("/api/messages/:id/extract", get(messages::get_extract))
        .route("/api/messages/:id/analysis", get(messages::get_analysis))
        .route("/api/messages/:id/replay", post(messages::replay_message))
        .route("/api/wait", get(messages::wait_for_message))
        .route("/api/test-email", post(messages::send_test_email))
        .route(
            "/api/sms",
            get(sms::list_sms).post(sms::ingest_sms).delete(sms::clear_sms),
        )
        .route("/api/sms/webhook", post(sms::sms_webhook))
        .route("/api/sms/bulk-delete", post(sms::bulk_delete_sms))
        .route("/api/sms/bulk-read", patch(sms::bulk_mark_sms_read))
        .route(
            "/api/sms/:id",
            get(sms::get_sms).delete(sms::delete_sms),
        )
        .route("/api/sms/:id/read", patch(sms::mark_sms_read))
        .route("/api/sms/:id/extract", get(sms::get_sms_extract))
        .route("/api/sms/:id/replay", post(sms::replay_sms))
        .route("/api/sms/wait", get(sms::wait_for_sms))
        .route("/api/test-sms", post(sms::send_test_sms))
        // Vendor API Emulators (Resend, SendGrid, Postmark)
        .route("/emails", post(vendor::resend_ingest))
        .route("/v1/emails", post(vendor::resend_ingest))
        .route("/v3/mail/send", post(vendor::sendgrid_ingest))
        .route("/email", post(vendor::postmark_ingest))
        // Multi-Channel Webhook Catchers
        .route("/api/webhooks/slack", post(webhooks::slack_webhook))
        .route("/api/webhooks/discord", post(webhooks::discord_webhook))
        .route("/api/webhooks/generic", post(webhooks::generic_webhook))
        .route("/api/events", get(ws::ws_handler))
        .route("/api/config", get(get_config))
        .route("/api/health", get(get_health))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}
