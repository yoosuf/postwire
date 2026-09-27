use postwire_server::{api, smtp, web};

use std::sync::Arc;

use postwire_core::config::Config;
use postwire_core::store::Store;
use tokio::signal;
use tokio::sync::broadcast;
use tracing::info;
use tracing_subscriber::EnvFilter;

use api::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let config = Config::from_env();
    let store = Arc::new(Store::new_with_ttl(
        &config.db_path,
        config.max_messages,
        config.ttl_seconds,
    )?);
    let (tx, _rx) = broadcast::channel(256);

    smtp::spawn_smtp_server(config.clone(), store.clone(), tx.clone());

    let state = AppState { store, tx };
    let app = api::router(state).fallback(web::static_handler);

    let addr = format!("{}:{}", config.bind_addr, config.http_port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!(%addr, smtp_port = config.smtp_port, "postwire listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
