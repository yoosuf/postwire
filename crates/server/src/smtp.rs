use std::io;
use std::net::IpAddr;
use std::sync::Arc;

use mailin_embedded::{Handler, Response, SslConfig};
use tokio::sync::broadcast;
use tracing::{error, info};
use uuid::Uuid;

use postwire_core::config::Config;
use postwire_core::mail;
use postwire_core::models::Event;
use postwire_core::store::{NewMessage, Store};

#[derive(Clone)]
pub struct MailHandler {
    store: Arc<Store>,
    tx: broadcast::Sender<Event>,
    from: String,
    to: Vec<String>,
    buffer: Vec<u8>,
}

impl MailHandler {
    pub fn new(store: Arc<Store>, tx: broadcast::Sender<Event>) -> Self {
        Self {
            store,
            tx,
            from: String::new(),
            to: Vec::new(),
            buffer: Vec::new(),
        }
    }
}

impl Handler for MailHandler {
    fn helo(&mut self, _ip: IpAddr, _domain: &str) -> Response {
        mailin_embedded::response::OK
    }

    fn mail(&mut self, _ip: IpAddr, _domain: &str, from: &str) -> Response {
        self.from = from.to_string();
        self.to.clear();
        self.buffer.clear();
        mailin_embedded::response::OK
    }

    fn rcpt(&mut self, to: &str) -> Response {
        self.to.push(to.to_string());
        mailin_embedded::response::OK
    }

    fn data_start(&mut self, _domain: &str, _from: &str, _is8bit: bool, _to: &[String]) -> Response {
        self.buffer.clear();
        mailin_embedded::response::OK
    }

    fn data(&mut self, buf: &[u8]) -> Result<(), io::Error> {
        self.buffer.extend_from_slice(buf);
        Ok(())
    }

    fn data_end(&mut self) -> Response {
        let raw = std::mem::take(&mut self.buffer);
        let size = raw.len() as i64;
        let (subject, from, to) = mail::envelope_display(&raw, &self.from, &self.to);

        let msg = NewMessage {
            id: Uuid::new_v4().to_string(),
            from,
            to,
            subject,
            size,
            raw,
        };

        match self.store.insert(msg) {
            Ok(summary) => {
                info!(id = %summary.id, from = %summary.from, "received message");
                let _ = self.tx.send(Event::New(summary));
            }
            Err(err) => {
                error!(%err, "failed to store message");
            }
        }

        mailin_embedded::response::OK
    }
}

/// Runs the blocking SMTP server on its own OS thread.
pub fn spawn_smtp_server(config: Config, store: Arc<Store>, tx: broadcast::Sender<Event>) {
    std::thread::spawn(move || {
        let handler = MailHandler::new(store, tx);
        let mut server = mailin_embedded::Server::new(handler);
        let addr = format!("{}:{}", config.bind_addr, config.smtp_port);

        let configured = server
            .with_name(config.smtp_hostname.clone())
            .with_ssl(SslConfig::None)
            .and_then(|s| s.with_addr(&addr));

        if let Err(err) = configured {
            error!(%err, %addr, "failed to start SMTP server");
            return;
        }

        info!(%addr, "SMTP server listening");
        if let Err(err) = server.serve() {
            error!(%err, "SMTP server stopped unexpectedly");
        }
    });
}
