use mail_parser::{Address, Message, MessageParser, MimeHeaders};

use crate::models::{AttachmentMeta, HeaderKv};

/// Flattens a parsed address field into "Name <email>" strings.
pub fn format_address(addr: Option<&Address>) -> Vec<String> {
    let Some(addr) = addr else {
        return Vec::new();
    };
    let mut out = Vec::new();
    match addr {
        Address::List(list) => {
            for a in list {
                out.push(format_addr(a.name.as_deref(), a.address.as_deref()));
            }
        }
        Address::Group(groups) => {
            for g in groups {
                for a in &g.addresses {
                    out.push(format_addr(a.name.as_deref(), a.address.as_deref()));
                }
            }
        }
    }
    out
}

fn format_addr(name: Option<&str>, email: Option<&str>) -> String {
    let email = email.unwrap_or("");
    match name {
        Some(n) if !n.is_empty() => format!("{n} <{email}>"),
        _ => email.to_string(),
    }
}

/// Quick check used for list rendering, without allocating full body strings.
pub fn body_flags(raw: &[u8]) -> (bool, bool) {
    match MessageParser::default().parse(raw) {
        Some(msg) => (msg.html_body_count() > 0, msg.attachment_count() > 0),
        None => (false, false),
    }
}

pub struct ParsedDetail {
    pub text_body: Option<String>,
    pub html_body: Option<String>,
    pub headers: Vec<HeaderKv>,
    pub attachments: Vec<AttachmentMeta>,
}

pub fn parse_detail(raw: &[u8]) -> Option<ParsedDetail> {
    let msg = MessageParser::default().parse(raw)?;
    Some(build_detail(&msg))
}

fn build_detail(msg: &Message) -> ParsedDetail {
    let headers = msg
        .headers_raw()
        .map(|(name, value)| HeaderKv {
            name: name.to_string(),
            value: value.trim().to_string(),
        })
        .collect();

    let text_body = msg.body_text(0).map(|c| c.to_string());
    let html_body = msg.body_html(0).map(|c| c.to_string());

    let attachments = msg
        .attachments()
        .enumerate()
        .map(|(index, part)| {
            let content_type = part
                .content_type()
                .map(|ct| match ct.subtype() {
                    Some(sub) => format!("{}/{}", ct.ctype(), sub),
                    None => ct.ctype().to_string(),
                })
                .unwrap_or_else(|| "application/octet-stream".to_string());
            AttachmentMeta {
                index,
                filename: part
                    .attachment_name()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("attachment-{index}")),
                content_type,
                size: part.contents().len(),
                content_id: part.content_id().map(|s| s.to_string()),
            }
        })
        .collect();

    ParsedDetail {
        text_body,
        html_body,
        headers,
        attachments,
    }
}

/// Returns the raw bytes and content-type of a single attachment by index.
pub fn attachment_bytes(raw: &[u8], index: usize) -> Option<(Vec<u8>, String, String)> {
    let msg = MessageParser::default().parse(raw)?;
    let part = msg.attachment(index as u32)?;
    let content_type = part
        .content_type()
        .map(|ct| match ct.subtype() {
            Some(sub) => format!("{}/{}", ct.ctype(), sub),
            None => ct.ctype().to_string(),
        })
        .unwrap_or_else(|| "application/octet-stream".to_string());
    let filename = part
        .attachment_name()
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("attachment-{index}"));
    Some((part.contents().to_vec(), content_type, filename))
}

/// Derives display subject/from/to for a freshly received message, preferring the
/// parsed header values but falling back to the SMTP envelope when headers are absent.
pub fn envelope_display(raw: &[u8], envelope_from: &str, envelope_to: &[String]) -> (String, String, Vec<String>) {
    let Some(msg) = MessageParser::default().parse(raw) else {
        return (
            "(no subject)".to_string(),
            envelope_from.to_string(),
            envelope_to.to_vec(),
        );
    };

    let subject = msg.subject().map(|s| s.to_string()).unwrap_or_else(|| "(no subject)".to_string());

    let from = format_address(msg.from())
        .into_iter()
        .next()
        .unwrap_or_else(|| envelope_from.to_string());

    let to = {
        let headers_to = format_address(msg.to());
        if headers_to.is_empty() {
            envelope_to.to_vec()
        } else {
            headers_to
        }
    };

    (subject, from, to)
}

/// Codes and links pulled out of a message body — built for agentic e2e tests that
/// need to grab an OTP or a magic link without hand-rolling regexes of their own.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExtractedSignals {
    pub codes: Vec<String>,
    pub links: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub matches: Vec<String>,
}

fn code_regex() -> &'static regex::Regex {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"\b\d{4,8}\b").unwrap())
}

fn link_regex() -> &'static regex::Regex {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r#"https?://[^\s"'<>]+"#).unwrap())
}

fn strip_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

pub fn extract_signals(text: Option<&str>, html: Option<&str>) -> ExtractedSignals {
    extract_signals_with_custom_regex(text, html, None)
}

pub fn extract_signals_with_custom_regex(
    text: Option<&str>,
    html: Option<&str>,
    custom_regex: Option<&str>,
) -> ExtractedSignals {
    let plain_html = html.map(strip_tags).unwrap_or_default();
    let haystack = format!("{} {}", text.unwrap_or(""), plain_html);

    let mut codes = Vec::new();
    for m in code_regex().find_iter(&haystack) {
        let s = m.as_str().to_string();
        if !codes.contains(&s) {
            codes.push(s);
        }
    }

    let mut links = Vec::new();
    for m in link_regex().find_iter(&haystack) {
        let s = m.as_str().trim_end_matches(['.', ',', ')']).to_string();
        if !links.contains(&s) {
            links.push(s);
        }
    }

    let mut matches = Vec::new();
    if let Some(pattern) = custom_regex {
        if let Ok(re) = regex::Regex::new(pattern) {
            for m in re.find_iter(&haystack) {
                let s = m.as_str().to_string();
                if !matches.contains(&s) {
                    matches.push(s);
                }
            }
        }
    }

    ExtractedSignals {
        codes,
        links,
        matches,
    }
}

/// Builds raw RFC 822 MIME bytes from simple text/html components for vendor API ingestion.
pub fn build_vendor_mime(
    from: &str,
    to: &[String],
    subject: &str,
    text: Option<&str>,
    html: Option<&str>,
) -> Vec<u8> {
    let to_header = to.join(", ");
    let text_content = text.unwrap_or("");
    let html_content = html.unwrap_or("");

    match (text.is_some(), html.is_some()) {
        (true, true) => {
            let boundary = "----postwire_vendor_boundary_12345";
            let mut mime = format!(
                "From: {from}\r\nTo: {to_header}\r\nSubject: {subject}\r\nMIME-Version: 1.0\r\nContent-Type: multipart/alternative; boundary=\"{boundary}\"\r\n\r\n"
            );
            mime.push_str(&format!(
                "--{boundary}\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n{text_content}\r\n\r\n"
            ));
            mime.push_str(&format!(
                "--{boundary}\r\nContent-Type: text/html; charset=utf-8\r\n\r\n{html_content}\r\n\r\n"
            ));
            mime.push_str(&format!("--{boundary}--\r\n"));
            mime.into_bytes()
        }
        (false, true) => {
            format!(
                "From: {from}\r\nTo: {to_header}\r\nSubject: {subject}\r\nMIME-Version: 1.0\r\nContent-Type: text/html; charset=utf-8\r\n\r\n{html_content}\r\n"
            )
            .into_bytes()
        }
        _ => {
            format!(
                "From: {from}\r\nTo: {to_header}\r\nSubject: {subject}\r\nMIME-Version: 1.0\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n{text_content}\r\n"
            )
            .into_bytes()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_custom_regex_extraction() {
        let text = "Welcome! Your invite token is INVITE-998822 and session is SESS-12345";
        let signals = extract_signals_with_custom_regex(Some(text), None, Some(r"INVITE-[A-Z0-9]+"));
        assert_eq!(signals.matches, vec!["INVITE-998822"]);
    }

    #[test]
    fn test_build_vendor_mime_and_parse() {
        let raw = build_vendor_mime(
            "sender@example.com",
            &["receiver@example.com".to_string()],
            "Vendor Test",
            Some("Text body 123456"),
            Some("<p>HTML body 123456</p>"),
        );
        let detail = parse_detail(&raw).unwrap();
        assert!(detail.text_body.as_deref().unwrap().contains("Text body 123456"));
        assert!(detail.html_body.as_deref().unwrap().contains("<p>HTML body 123456</p>"));
    }
}
