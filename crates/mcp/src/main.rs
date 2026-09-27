//! Minimal MCP (Model Context Protocol) server, stdio transport, no SDK dependency.
//!
//! Exposes Postwire's inbox as tools so AI coding agents can discover captured
//! emails during agentic/e2e development: list them, wait for a fresh one, pull out
//! OTP codes / magic links, or clean up after a test run.
//!
//! Talks to a running `postwire` server over HTTP via `POSTWIRE_URL` (default
//! `http://127.0.0.1:8025`) rather than the database directly, so it works whether
//! the server is local, in another container, or on another host.

use std::io::Write;

use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, BufReader};

const PROTOCOL_VERSION: &str = "2024-11-05";

fn base_url() -> String {
    server_url(std::env::var("POSTWIRE_URL").ok().as_deref())
}

fn server_url(postwire: Option<&str>) -> String {
    postwire.unwrap_or("http://127.0.0.1:8025").to_string()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let stdin = tokio::io::stdin();
    let mut lines = BufReader::new(stdin).lines();

    while let Some(line) = lines.next_line().await? {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let request: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue, // ignore malformed frames rather than killing the session
        };

        let id = request.get("id").cloned();
        let method = request.get("method").and_then(|m| m.as_str()).unwrap_or("");

        // Notifications (no "id") never get a response, per the JSON-RPC/MCP spec.
        let Some(id) = id else { continue };

        let response = match method {
            "initialize" => ok(id, initialize_result()),
            "ping" => ok(id, json!({})),
            "tools/list" => ok(id, json!({ "tools": tool_definitions() })),
            "tools/call" => match handle_tool_call(&client, &request).await {
                Ok(result) => ok(id, result),
                Err(err) => ok(id, tool_error(&err.to_string())),
            },
            _ => error(id, -32601, "method not found"),
        };

        println!("{response}");
        std::io::stdout().flush()?;
    }

    Ok(())
}

fn initialize_result() -> Value {
    json!({
        "protocolVersion": PROTOCOL_VERSION,
        "capabilities": { "tools": {} },
        "serverInfo": { "name": "postwire-mcp", "version": env!("CARGO_PKG_VERSION") }
    })
}

fn tool_definitions() -> Value {
    json!([
        {
            "name": "list_emails",
            "description": "List captured emails, newest first. Optionally filter with a free-text search over from/to/subject.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "search": { "type": "string", "description": "Free-text filter across from/to/subject" },
                    "limit": { "type": "integer", "description": "Max results (default 50)" },
                    "offset": { "type": "integer", "description": "Pagination offset" }
                }
            }
        },
        {
            "name": "get_email",
            "description": "Fetch a single email's full detail: headers, text/html body, attachment metadata.",
            "inputSchema": {
                "type": "object",
                "properties": { "id": { "type": "string" } },
                "required": ["id"]
            }
        },
        {
            "name": "wait_for_email",
            "description": "Block until a fresh email matching the given filters arrives (or timeout). Use this right after triggering an action (signup, password reset, etc.) in an agentic/e2e test.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "to": { "type": "string", "description": "Substring to match against recipients" },
                    "from": { "type": "string", "description": "Substring to match against the sender" },
                    "subject": { "type": "string", "description": "Substring to match against the subject" },
                    "since_ms": { "type": "integer", "description": "Unix epoch ms; only consider emails received after this. Defaults to now." },
                    "timeout_ms": { "type": "integer", "description": "How long to wait before giving up (default 10000, max 60000)" }
                }
            }
        },
        {
            "name": "extract_signals",
            "description": "Pull likely OTP/verification codes and links out of an email's body. Optionally match custom regular expressions.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string" },
                    "regex": { "type": "string", "description": "Optional custom regular expression pattern to match" }
                },
                "required": ["id"]
            }
        },
        {
            "name": "delete_email",
            "description": "Delete a single captured email by id.",
            "inputSchema": {
                "type": "object",
                "properties": { "id": { "type": "string" } },
                "required": ["id"]
            }
        },
        {
            "name": "clear_inbox",
            "description": "Delete every captured email. Handy for resetting state between test runs.",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "send_test_email",
            "description": "Send a synthetic test email into the inbox without needing a real SMTP client. Useful for smoke-testing the wait_for_email/extract_signals flow itself.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "to": { "type": "string", "description": "Recipient address (default: you@example.com)" }
                }
            }
        },
        {
            "name": "list_sms",
            "description": "List captured SMS messages, newest first. Optionally filter with a free-text search over sender, recipient, or body.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "search": { "type": "string", "description": "Free-text filter across from/to/body" },
                    "limit": { "type": "integer", "description": "Max results (default 50)" },
                    "offset": { "type": "integer", "description": "Pagination offset" }
                }
            }
        },
        {
            "name": "get_sms",
            "description": "Fetch a single SMS message by ID.",
            "inputSchema": {
                "type": "object",
                "properties": { "id": { "type": "string" } },
                "required": ["id"]
            }
        },
        {
            "name": "wait_for_sms",
            "description": "Block until a fresh SMS matching the given filters arrives (or timeout). Use this right after triggering an action that sends an SMS OTP/2FA code in an agentic/e2e test.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "to": { "type": "string", "description": "Substring to match against recipient phone/identifier" },
                    "from": { "type": "string", "description": "Substring to match against sender phone/identifier" },
                    "body": { "type": "string", "description": "Substring to match against message body" },
                    "since_ms": { "type": "integer", "description": "Unix epoch ms; only consider SMS received after this. Defaults to now." },
                    "timeout_ms": { "type": "integer", "description": "How long to wait before giving up (default 10000, max 60000)" }
                }
            }
        },
        {
            "name": "extract_sms_signals",
            "description": "Pull likely OTP/verification codes and links out of an SMS message body. Optionally match custom regular expressions.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string" },
                    "regex": { "type": "string", "description": "Optional custom regular expression pattern to match" }
                },
                "required": ["id"]
            }
        },
        {
            "name": "send_test_sms",
            "description": "Send a synthetic test SMS message into Postwire.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "to": { "type": "string", "description": "Recipient phone number (default: +15550100)" },
                    "from": { "type": "string", "description": "Sender phone number (default: +18005550199)" },
                    "body": { "type": "string", "description": "SMS content body" }
                }
            }
        },
        {
            "name": "delete_sms",
            "description": "Delete a single captured SMS message by ID.",
            "inputSchema": {
                "type": "object",
                "properties": { "id": { "type": "string" } },
                "required": ["id"]
            }
        },
        {
            "name": "clear_sms_inbox",
            "description": "Delete every captured SMS message.",
            "inputSchema": { "type": "object", "properties": {} }
        }
    ])
}

async fn handle_tool_call(client: &reqwest::Client, request: &Value) -> anyhow::Result<Value> {
    let params = request.get("params").cloned().unwrap_or(json!({}));
    let name = params.get("name").and_then(|n| n.as_str()).unwrap_or_default();
    let args = params.get("arguments").cloned().unwrap_or(json!({}));
    let base = base_url();

    let body = match name {
        "list_emails" => {
            let mut url = reqwest::Url::parse(&format!("{base}/api/messages"))?;
            {
                let mut qp = url.query_pairs_mut();
                if let Some(s) = args.get("search").and_then(|v| v.as_str()) {
                    qp.append_pair("search", s);
                }
                if let Some(n) = args.get("limit").and_then(|v| v.as_i64()) {
                    qp.append_pair("limit", &n.to_string());
                }
                if let Some(n) = args.get("offset").and_then(|v| v.as_i64()) {
                    qp.append_pair("offset", &n.to_string());
                }
            }
            client.get(url).send().await?.json::<Value>().await?
        }
        "get_email" => {
            let id = require_str(&args, "id")?;
            client.get(format!("{base}/api/messages/{id}")).send().await?.json::<Value>().await?
        }
        "wait_for_email" => {
            let mut url = reqwest::Url::parse(&format!("{base}/api/wait"))?;
            {
                let mut qp = url.query_pairs_mut();
                for key in ["to", "from", "subject"] {
                    if let Some(v) = args.get(key).and_then(|v| v.as_str()) {
                        qp.append_pair(key, v);
                    }
                }
                if let Some(ms) = args.get("since_ms").and_then(|v| v.as_i64()) {
                    let since = chrono_rfc3339_from_epoch_ms(ms);
                    qp.append_pair("since", &since);
                }
                if let Some(t) = args.get("timeout_ms").and_then(|v| v.as_i64()) {
                    qp.append_pair("timeout_ms", &t.to_string());
                }
            }
            let timeout_ms = args.get("timeout_ms").and_then(|v| v.as_u64()).unwrap_or(10_000);
            client
                .get(url)
                .timeout(std::time::Duration::from_millis(timeout_ms + 5_000))
                .send()
                .await?
                .json::<Value>()
                .await?
        }
        "extract_signals" => {
            let id = require_str(&args, "id")?;
            let mut url = reqwest::Url::parse(&format!("{base}/api/messages/{id}/extract"))?;
            if let Some(re) = args.get("regex").and_then(|v| v.as_str()) {
                url.query_pairs_mut().append_pair("regex", re);
            }
            client
                .get(url)
                .send()
                .await?
                .json::<Value>()
                .await?
        }
        "delete_email" => {
            let id = require_str(&args, "id")?;
            client.delete(format!("{base}/api/messages/{id}")).send().await?;
            json!({ "deleted": id })
        }
        "clear_inbox" => {
            client.delete(format!("{base}/api/messages")).send().await?;
            json!({ "cleared": true })
        }
        "send_test_email" => {
            let to = args.get("to").and_then(|v| v.as_str());
            client
                .post(format!("{base}/api/test-email"))
                .json(&json!({ "to": to }))
                .send()
                .await?
                .json::<Value>()
                .await?
        }
        "list_sms" => {
            let mut url = reqwest::Url::parse(&format!("{base}/api/sms"))?;
            {
                let mut qp = url.query_pairs_mut();
                if let Some(s) = args.get("search").and_then(|v| v.as_str()) {
                    qp.append_pair("search", s);
                }
                if let Some(n) = args.get("limit").and_then(|v| v.as_i64()) {
                    qp.append_pair("limit", &n.to_string());
                }
                if let Some(n) = args.get("offset").and_then(|v| v.as_i64()) {
                    qp.append_pair("offset", &n.to_string());
                }
            }
            client.get(url).send().await?.json::<Value>().await?
        }
        "get_sms" => {
            let id = require_str(&args, "id")?;
            client.get(format!("{base}/api/sms/{id}")).send().await?.json::<Value>().await?
        }
        "wait_for_sms" => {
            let mut url = reqwest::Url::parse(&format!("{base}/api/sms/wait"))?;
            {
                let mut qp = url.query_pairs_mut();
                for key in ["to", "from", "body"] {
                    if let Some(v) = args.get(key).and_then(|v| v.as_str()) {
                        qp.append_pair(key, v);
                    }
                }
                if let Some(ms) = args.get("since_ms").and_then(|v| v.as_i64()) {
                    let since = chrono_rfc3339_from_epoch_ms(ms);
                    qp.append_pair("since", &since);
                }
                if let Some(t) = args.get("timeout_ms").and_then(|v| v.as_i64()) {
                    qp.append_pair("timeout_ms", &t.to_string());
                }
            }
            let timeout_ms = args.get("timeout_ms").and_then(|v| v.as_u64()).unwrap_or(10_000);
            client
                .get(url)
                .timeout(std::time::Duration::from_millis(timeout_ms + 5_000))
                .send()
                .await?
                .json::<Value>()
                .await?
        }
        "extract_sms_signals" => {
            let id = require_str(&args, "id")?;
            let mut url = reqwest::Url::parse(&format!("{base}/api/sms/{id}/extract"))?;
            if let Some(re) = args.get("regex").and_then(|v| v.as_str()) {
                url.query_pairs_mut().append_pair("regex", re);
            }
            client
                .get(url)
                .send()
                .await?
                .json::<Value>()
                .await?
        }
        "send_test_sms" => {
            let to = args.get("to").and_then(|v| v.as_str());
            let from = args.get("from").and_then(|v| v.as_str());
            let body_text = args.get("body").and_then(|v| v.as_str());
            client
                .post(format!("{base}/api/test-sms"))
                .json(&json!({ "to": to, "from": from, "body": body_text }))
                .send()
                .await?
                .json::<Value>()
                .await?
        }
        "delete_sms" => {
            let id = require_str(&args, "id")?;
            client.delete(format!("{base}/api/sms/{id}")).send().await?;
            json!({ "deleted": id })
        }
        "clear_sms_inbox" => {
            client.delete(format!("{base}/api/sms")).send().await?;
            json!({ "cleared": true })
        }
        other => anyhow::bail!("unknown tool: {other}"),
    };

    Ok(json!({
        "content": [{ "type": "text", "text": serde_json::to_string_pretty(&body)? }],
        "isError": false
    }))
}

fn require_str<'a>(args: &'a Value, key: &str) -> anyhow::Result<&'a str> {
    args.get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("missing required argument: {key}"))
}

/// Formats an epoch-millis timestamp as RFC3339 without pulling in the `chrono` dependency.
fn chrono_rfc3339_from_epoch_ms(ms: i64) -> String {
    let secs = ms.div_euclid(1000);
    let millis = ms.rem_euclid(1000) as u32;
    humantime_rfc3339(secs, millis)
}

fn humantime_rfc3339(secs: i64, millis: u32) -> String {
    // Small hand-rolled formatter: avoids adding a whole date/time crate to this tiny binary.
    let (days, time_of_day) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    let (h, rem) = (time_of_day / 3600, time_of_day % 3600);
    let (m, s) = (rem / 60, rem % 60);

    let (y, mo, d) = civil_from_days(days);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}.{millis:03}Z")
}

/// Howard Hinnant's `civil_from_days` algorithm (public domain), used to turn a day
/// count since the Unix epoch into a (year, month, day) triple without extra deps.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn ok(id: Value, result: Value) -> String {
    json!({ "jsonrpc": "2.0", "id": id, "result": result }).to_string()
}

fn tool_error(message: &str) -> Value {
    json!({ "content": [{ "type": "text", "text": message }], "isError": true })
}

fn error(id: Value, code: i64, message: &str) -> String {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } }).to_string()
}

#[cfg(test)]
mod tests {
    use super::server_url;

    #[test]
    fn postwire_url_uses_the_configured_server() {
        assert_eq!(server_url(Some("http://postwire")), "http://postwire");
    }

    #[test]
    fn default_server_url_is_used_when_not_configured() {
        assert_eq!(server_url(None), "http://127.0.0.1:8025");
    }

    #[test]
    fn initialize_reports_the_postwire_product_name() {
        assert_eq!(super::initialize_result()["serverInfo"]["name"], "postwire-mcp");
    }
}
