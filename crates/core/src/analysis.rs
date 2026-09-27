//! HTML email-client-compatibility checks and a lightweight, heuristic spam score —
//! the same broad categories tools like Litmus / mail-tester report on, reimplemented
//! small enough to ship inside a slim mail catcher.

use regex::Regex;
use serde::Serialize;

use crate::models::{AttachmentMeta, HeaderKv};

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    Pass,
    Warn,
    Fail,
}

#[derive(Debug, Clone, Serialize)]
pub struct Check {
    pub id: &'static str,
    pub label: &'static str,
    pub status: CheckStatus,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct HtmlAnalysis {
    pub checks: Vec<Check>,
    pub size_bytes: usize,
    pub image_count: usize,
    pub external_image_count: usize,
    pub link_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct SpamRule {
    pub id: &'static str,
    pub description: String,
    pub score: f32,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SpamVerdict {
    Clean,
    Suspicious,
    LikelySpam,
}

#[derive(Debug, Clone, Serialize)]
pub struct SpamAnalysis {
    pub score: f32,
    pub verdict: SpamVerdict,
    pub rules: Vec<SpamRule>,
}

fn re(pattern: &str) -> Regex {
    Regex::new(pattern).expect("static regex is valid")
}

fn tag_count(html: &str, tag: &str) -> usize {
    re(&format!(r"(?i)<{tag}[\s>]")).find_iter(html).count()
}

/// Runs a set of email-client-compatibility checks against the HTML body, the same
/// broad categories tools like Litmus report on (layout technique, CSS support,
/// scripting, image hygiene, size limits).
pub fn analyze_html(html: Option<&str>) -> HtmlAnalysis {
    let Some(html) = html else {
        return HtmlAnalysis {
            checks: vec![Check {
                id: "no_html",
                label: "No HTML body",
                status: CheckStatus::Warn,
                detail: "This message has no HTML part to analyze.".to_string(),
            }],
            size_bytes: 0,
            image_count: 0,
            external_image_count: 0,
            link_count: 0,
        };
    };

    let size_bytes = html.len();
    let image_count = tag_count(html, "img");
    let link_count = tag_count(html, "a");
    let external_image_count = re(r#"(?i)<img[^>]+src\s*=\s*["']https?://"#).find_iter(html).count();
    let inline_style_count = re(r#"(?i)\sstyle\s*="#).find_iter(html).count();
    let has_style_block = re(r"(?is)<style[\s>]").is_match(html);
    let has_table = tag_count(html, "table") > 0;
    let has_doctype = html.trim_start().to_lowercase().starts_with("<!doctype");
    let has_viewport = re(r#"(?i)<meta[^>]+name\s*=\s*["']viewport["']"#).is_match(html);
    let has_charset = re(r#"(?i)<meta[^>]+charset"#).is_match(html);
    let has_script = re(r"(?is)<script[\s>]").is_match(html);
    let has_external_css = re(r#"(?i)<link[^>]+rel\s*=\s*["']stylesheet["']"#).is_match(html);
    let has_web_fonts = html.to_lowercase().contains("@font-face") || html.to_lowercase().contains("fonts.googleapis.com");
    let has_bg_image_css = re(r"(?i)background(-image)?\s*:\s*url\(").is_match(html);
    let missing_alt = re(r"(?i)<img[^>]*>")
        .find_iter(html)
        .filter(|m| !m.as_str().to_lowercase().contains("alt="))
        .count();

    let checks = vec![
        Check {
            id: "doctype",
            label: "DOCTYPE present",
            status: if has_doctype { CheckStatus::Pass } else { CheckStatus::Warn },
            detail: if has_doctype {
                "A DOCTYPE helps clients render consistently.".to_string()
            } else {
                "No DOCTYPE found; some clients fall back to quirks mode.".to_string()
            },
        },
        Check {
            id: "layout",
            label: "Layout technique",
            status: if has_table { CheckStatus::Pass } else { CheckStatus::Warn },
            detail: if has_table {
                "Uses <table> layout, the most broadly supported technique in email clients.".to_string()
            } else {
                "No <table> elements found; div/flex/grid layouts break in Outlook (Word rendering engine).".to_string()
            },
        },
        Check {
            id: "css",
            label: "CSS technique",
            status: if inline_style_count > 0 {
                CheckStatus::Pass
            } else if has_style_block {
                CheckStatus::Warn
            } else {
                CheckStatus::Fail
            },
            detail: format!(
                "{inline_style_count} inline style attribute(s), {} a <style> block. Inline styles are the most reliable across clients.",
                if has_style_block { "has" } else { "no" }
            ),
        },
        Check {
            id: "external_css",
            label: "External stylesheet",
            status: if has_external_css { CheckStatus::Fail } else { CheckStatus::Pass },
            detail: if has_external_css {
                "<link rel=\"stylesheet\"> found; most email clients strip external stylesheets entirely.".to_string()
            } else {
                "No external stylesheet links.".to_string()
            },
        },
        Check {
            id: "script",
            label: "JavaScript",
            status: if has_script { CheckStatus::Fail } else { CheckStatus::Pass },
            detail: if has_script {
                "<script> tag found; every major email client strips JavaScript.".to_string()
            } else {
                "No <script> tags.".to_string()
            },
        },
        Check {
            id: "web_fonts",
            label: "Web fonts",
            status: if has_web_fonts { CheckStatus::Warn } else { CheckStatus::Pass },
            detail: if has_web_fonts {
                "Custom/web fonts detected; support is inconsistent, always set a fallback font stack.".to_string()
            } else {
                "No custom web fonts detected.".to_string()
            },
        },
        Check {
            id: "background_images",
            label: "CSS background images",
            status: if has_bg_image_css { CheckStatus::Warn } else { CheckStatus::Pass },
            detail: if has_bg_image_css {
                "CSS background-image found; Outlook desktop does not support it — use a VML fallback.".to_string()
            } else {
                "No CSS background-image usage.".to_string()
            },
        },
        Check {
            id: "viewport",
            label: "Mobile viewport meta",
            status: if has_viewport { CheckStatus::Pass } else { CheckStatus::Warn },
            detail: if has_viewport {
                "Viewport meta tag present.".to_string()
            } else {
                "No viewport meta tag; mobile clients may not scale the layout correctly.".to_string()
            },
        },
        Check {
            id: "charset",
            label: "Character encoding",
            status: if has_charset { CheckStatus::Pass } else { CheckStatus::Warn },
            detail: if has_charset {
                "Charset meta tag present.".to_string()
            } else {
                "No charset meta tag; non-ASCII content may render incorrectly in some clients.".to_string()
            },
        },
        Check {
            id: "image_alt",
            label: "Image alt text",
            status: if image_count == 0 || missing_alt == 0 {
                CheckStatus::Pass
            } else {
                CheckStatus::Warn
            },
            detail: if image_count == 0 {
                "No images.".to_string()
            } else {
                format!("{missing_alt} of {image_count} image(s) missing an alt attribute (images are often blocked by default).")
            },
        },
        Check {
            id: "size",
            label: "HTML size",
            status: if size_bytes > 102_000 { CheckStatus::Fail } else { CheckStatus::Pass },
            detail: format!(
                "{:.1} KB{}",
                size_bytes as f64 / 1024.0,
                if size_bytes > 102_000 { " — Gmail clips messages over ~102KB." } else { "" }
            ),
        },
    ];

    HtmlAnalysis { checks, size_bytes, image_count, external_image_count, link_count }
}

const SPAM_WORDS: &[&str] = &[
    "free", "winner", "viagra", "click here", "act now", "limited time", "guarantee",
    "100% free", "no cost", "buy now", "cash bonus", "urgent", "congratulations",
    "risk-free", "act immediately", "double your", "earn money", "cheap", "unsubscribe now",
    "not spam", "work from home", "casino", "lottery", "credit card", "viagra",
];

const URL_SHORTENERS: &[&str] = &["bit.ly", "tinyurl.com", "goo.gl", "t.co", "ow.ly", "is.gd"];

fn caps_ratio(s: &str) -> f32 {
    let letters: Vec<char> = s.chars().filter(|c| c.is_alphabetic()).collect();
    if letters.is_empty() {
        return 0.0;
    }
    let upper = letters.iter().filter(|c| c.is_uppercase()).count();
    upper as f32 / letters.len() as f32
}

/// Runs a small set of SpamAssassin-style heuristics over subject/body/headers and
/// returns a 0-10ish score plus the individual rule hits, purely for local dev
/// triage — this is not a substitute for a real spam-filter test.
pub fn analyze_spam(
    subject: &str,
    text: Option<&str>,
    html: Option<&str>,
    headers: &[HeaderKv],
    attachments: &[AttachmentMeta],
) -> SpamAnalysis {
    let mut rules: Vec<SpamRule> = Vec::new();
    let subject_lower = subject.to_lowercase();
    let body_lower = format!("{} {}", text.unwrap_or(""), html.unwrap_or("")).to_lowercase();

    let hit_words: Vec<&str> = SPAM_WORDS.iter().copied().filter(|w| subject_lower.contains(w) || body_lower.contains(w)).collect();
    if !hit_words.is_empty() {
        rules.push(SpamRule {
            id: "trigger_words",
            description: format!("Contains common spam trigger word(s): {}", hit_words.join(", ")),
            score: 0.6 * hit_words.len().min(5) as f32,
        });
    }

    let subj_caps = caps_ratio(subject);
    if subject.chars().filter(|c| c.is_alphabetic()).count() > 6 && subj_caps > 0.7 {
        rules.push(SpamRule {
            id: "subject_caps",
            description: "Subject is mostly uppercase.".to_string(),
            score: 1.5,
        });
    }

    let bangs = subject.matches('!').count();
    if bangs >= 2 {
        rules.push(SpamRule {
            id: "subject_exclamation",
            description: format!("Subject has {bangs} exclamation marks."),
            score: 1.0,
        });
    }

    if subject.trim().is_empty() {
        rules.push(SpamRule {
            id: "empty_subject",
            description: "Subject is empty.".to_string(),
            score: 1.0,
        });
    }

    if text.map(|t| t.trim().is_empty()).unwrap_or(true) && html.is_some() {
        rules.push(SpamRule {
            id: "no_text_part",
            description: "No plain-text alternative — HTML-only emails score worse with most filters.".to_string(),
            score: 1.2,
        });
    }

    if let Some(html) = html {
        let text_len = text.map(|t| t.trim().len()).unwrap_or(0);
        let tag_free_len: usize = re(r"(?is)<[^>]+>").replace_all(html, "").trim().len();
        let effective_text = text_len.max(tag_free_len);
        let image_count = tag_count(html, "img");
        if effective_text < 25 && image_count > 0 {
            rules.push(SpamRule {
                id: "image_heavy",
                description: "Very little text relative to images — a common spam-filter red flag.".to_string(),
                score: 1.5,
            });
        }
    }

    let shortener_hits: Vec<&str> = URL_SHORTENERS.iter().copied().filter(|s| body_lower.contains(s)).collect();
    if !shortener_hits.is_empty() {
        rules.push(SpamRule {
            id: "url_shortener",
            description: format!("Uses a URL shortener ({}), which hides the real destination.", shortener_hits.join(", ")),
            score: 1.0,
        });
    }

    let has_list_unsubscribe = headers.iter().any(|h| h.name.eq_ignore_ascii_case("list-unsubscribe"));
    let looks_bulk = html.map(|h| tag_count(h, "img") > 3).unwrap_or(false) || body_lower.contains("unsubscribe");
    if looks_bulk && !has_list_unsubscribe {
        rules.push(SpamRule {
            id: "missing_list_unsubscribe",
            description: "Looks like bulk/marketing mail but has no List-Unsubscribe header.".to_string(),
            score: 0.8,
        });
    }

    let risky_ext = re(r"(?i)\.(exe|scr|js|jar|bat|cmd|vbs)$");
    let risky_attachments: Vec<&str> = attachments
        .iter()
        .filter(|a| risky_ext.is_match(&a.filename))
        .map(|a| a.filename.as_str())
        .collect();
    if !risky_attachments.is_empty() {
        rules.push(SpamRule {
            id: "risky_attachment",
            description: format!("Executable-looking attachment(s): {}", risky_attachments.join(", ")),
            score: 3.0,
        });
    }

    let score: f32 = rules.iter().map(|r| r.score).sum();
    let verdict = if score >= 5.0 {
        SpamVerdict::LikelySpam
    } else if score >= 2.0 {
        SpamVerdict::Suspicious
    } else {
        SpamVerdict::Clean
    };

    SpamAnalysis { score, verdict, rules }
}
