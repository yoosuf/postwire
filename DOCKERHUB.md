# Postwire 🌲📬

**A tiny, single-binary SMTP & SMS catcher for development — built for AI agents.**

Like Mailtrap or Mailpit, but written in Rust (~15MB image, no JVM/Node runtime), capturing both **SMTP emails** and **SMS messages** (via JSON or Twilio webhooks), and designed so that AI coding agents (Claude, Copilot, Cursor, e2e test suites) can **discover, wait for, and extract data from captured emails & SMS** via a REST API or an MCP server — no more regex-scraping raw logs or polling databases to find a signup OTP or magic link.

```bash
docker run -d --name postwire -p 1025:1025 -p 8025:8025 -v postwire-data:/data yoosuf/postwire:latest
```

> Legacy `yoosuf/postwire` tags remain supported for compatibility, but the canonical project and image name is now Postwire.

Point your app's SMTP client at `localhost:1025` or SMS webhook at `http://localhost:8025/api/sms/webhook`, open the inbox at `http://localhost:8025` — done. No external network access, no accounts, no cloud. Everything stays local on your machine.

---

## Why Postwire?

| Feature | Postwire | Mailhog | Mailpit | Mailtrap (SaaS) |
|---|---|---|---|---|
| Single static binary | ✅ Rust | ❌ Go+deps | ✅ Go | ❌ cloud only |
| Dual Email & SMS catcher | ✅ built-in | ❌ | ❌ | partial |
| Long-polling wait API | ✅ built-in | ❌ | partial | ❌ |
| Auto signal extraction (OTP/Links) | ✅ built-in | ❌ | ❌ | ❌ |
| Litmus-style email analysis | ✅ built-in | ❌ | ❌ | ❌ |
| MCP server for AI agents | ✅ (14 tools) | ❌ | ❌ | ❌ |
| Twilio webhook ingestion | ✅ built-in | ❌ | ❌ | ❌ |
| Free & open source | ✅ | ✅ | ✅ | ❌ |

---

## Built for Agentic Development & E2E Testing

Your AI agent (or automated test runner) doesn't need to sleep or write fragile regex scrapers.

### Email Flow
```bash
# 1. Long-poll server-side for matching email (up to 60s)
curl "http://localhost:8025/api/wait?to=user@test.com&subject=Verify&since=<ts>&timeout_ms=15000"

# 2. Extract verification OTP codes / magic links automatically
curl "http://localhost:8025/api/messages/<MESSAGE_ID>/extract"
# -> { "codes": ["482913"], "links": ["https://app.test/verify?token=..."] }

# 3. Litmus-style compatibility & spam analysis
curl "http://localhost:8025/api/messages/<MESSAGE_ID>/analysis"
```

### SMS Flow & Twilio Webhook
```bash
# 1. Ingest SMS via Twilio webhook endpoint
curl -X POST "http://localhost:8025/api/sms/webhook" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "From=%2B15550199&To=%2B15550100&Body=Your+OTP+is+839201"

# 2. Long-poll for SMS arrival
curl "http://localhost:8025/api/sms/wait?to=+15550100&since=<ts>"

# 3. Extract codes/links from SMS
curl "http://localhost:8025/api/sms/<SMS_ID>/extract"
# -> { "codes": ["839201"], "links": [] }
```

### Node.js E2E Test Example
```javascript
// Record timestamp BEFORE triggering your app action
const since = new Date().toISOString();
await triggerSignupOrSmsAction();

// Long-poll server until matching SMS or Email arrives
const res = await fetch(`http://localhost:8025/api/sms/wait?to=%2B15550100&since=${since}`);
const sms = await res.json();

// Extract OTP verification codes automatically
const ext = await fetch(`http://localhost:8025/api/sms/${sms.id}/extract`);
const { codes } = await ext.json(); // ["839201"]
```

Or plug the bundled `postwire-mcp` stdio server straight into Claude Desktop, Copilot, or Cursor to give your AI agent 14 built-in tools (`list_emails`, `get_email`, `wait_for_email`, `extract_signals`, `delete_email`, `clear_inbox`, `send_test_email`, `list_sms`, `get_sms`, `wait_for_sms`, `extract_sms_signals`, `send_test_sms`, `delete_sms`, `clear_sms_inbox`).

---

## Features

- **SMTP Catcher (`1025`)**: dead-end listener, no relaying, mail never leaves your machine.
- **SMS & Webhook Ingestion (`8025`)**: accepts standard JSON and Twilio form-urlencoded webhooks.
- **Dual Web UI & REST API (`8025`)**: real-time WebSocket updates for both Emails and SMS.
- **HTML / Plain Text Inspection**: view raw headers, HTML, plain text, and download attachments or raw `.eml`.
- **Litmus-Style Analysis**: checks DOCTYPE, layout tables, inline CSS, web fonts, background images, viewport, image alt text, size limits (>102KB Gmail threshold), and heuristic spam score.
- **Search, Pagination & Bulk Operations**: lazily-loaded grid (50 items/page) keeping huge inboxes fast.
- **SQLite Storage**: automatic FIFO pruning when store exceeds `MAX_MESSAGES`.
- **Multi-Arch Docker Image**: `linux/amd64` and `linux/arm64`.

---

## Quick Start

```bash
docker run -d --name postwire \
  -p 1025:1025 -p 8025:8025 \
  -v postwire-data:/data \
  yoosuf/postwire:latest
```

Docker Compose:

```yaml
services:
  postwire:
    image: yoosuf/postwire:latest
    ports:
      - "1025:1025"
      - "8025:8025"
    volumes:
      - postwire-data:/data
volumes:
  postwire-data:
```

Run MCP Server against running container:

```bash
docker run --rm -i -e POSTWIRE_URL=http://host.docker.internal:8025 \
  --entrypoint /usr/local/bin/postwire-mcp yoosuf/postwire:latest
```

### Alternative Installation Methods

Prefer native host binaries without Docker?
- **Homebrew (macOS & Linux)**: `brew tap yoosuf/tap && brew install postwire` (or `brew install yoosuf/tap/postwire`)
- **POSIX Shell Installer**: `curl -fsSL https://raw.githubusercontent.com/yoosuf/postwire/main/install.sh | sh`
- **Windows PowerShell**: `iwr -useb https://raw.githubusercontent.com/yoosuf/postwire/main/install.ps1 | iex`
- **Native Packages**: `.deb` (Debian/Ubuntu), `.rpm` (Fedora/RHEL), Arch Linux (AUR `postwire-bin`), Scoop, Winget, and Chocolatey.

---

## Configuration

| Variable | Default | Description |
|---|---|---|
| `POSTWIRE_SMTP_PORT` | `1025` | SMTP port; `SMTP_PORT` remains a legacy alias |
| `POSTWIRE_HTTP_PORT` | `8025` | Web UI/API port; `HTTP_PORT` remains a legacy alias |
| `POSTWIRE_BIND_ADDR` | `0.0.0.0` | Network bind address; `BIND_ADDR` remains a legacy alias |
| `POSTWIRE_DB_PATH` | `postwire.db` (`/data/postwire.db` in Docker) | SQLite path; `DB_PATH` remains a legacy alias so existing data stays in place |
| `POSTWIRE_MAX_MESSAGES` | `1000` | FIFO limit; `MAX_MESSAGES` remains a legacy alias |
| `POSTWIRE_TTL_SECONDS` | `0` | Ingestion-time retention; `TTL_SECONDS` remains a legacy alias |
| `POSTWIRE_SMTP_HOSTNAME` | `postwire` | SMTP banner hostname; `SMTP_HOSTNAME` remains a legacy alias |
| `POSTWIRE_URL` | `http://127.0.0.1:8025` | MCP server URL; `POSTWIRE_URL` remains a legacy alias |

---

## Links

- **Repository & Docs**: https://github.com/yoosuf/postwire
- **System Architecture**: https://github.com/yoosuf/postwire/blob/main/ARCHITECTURE.md
- **Agent Integration Guide**: https://github.com/yoosuf/postwire/blob/main/AGENTS.md
- **Issue Tracker**: https://github.com/yoosuf/postwire/issues

Built by [Yoosuf](https://yoosuf.me/), who also offers [fractional CTO services](https://yoosuf.me/services/).
