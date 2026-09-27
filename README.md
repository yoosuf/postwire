# Postwire 🌲📬

A tiny, single-binary **SMTP & SMS catcher** for development environments — like Mailtrap or Mailpit, but written in Rust with a minimal footprint (~15MB image), structured as a monorepo, and built for agentic development: AI agents (and e2e tests) can discover, wait for, and extract data from captured emails and SMS messages via a REST API or an MCP server.

**Tags:** `mail` • `SMTP` • `SMS` • `mail-catcher` • `sms-catcher` • `testing` • `e2e` • `agents` • `MCP` • `Docker` • `Rust` • `email` • `development`

---

## Monorepo Layout

```
crates/core/    postwire-core   — shared SQLite store, MIME parsing, HTML/spam analysis, signal extraction
crates/server/  postwire        — SMTP (1025) + HTTP API/UI (8025); what you deploy
crates/mcp/     postwire-mcp    — MCP stdio server exposing emails & SMS to AI agents
apps/web/                       — React + Vite + TS frontend embedded into `postwire`
```

For full system architecture, sequence diagrams, database schemas, and design details, see [ARCHITECTURE.md](ARCHITECTURE.md).  
For runtime integration details for AI agents and test suites, see [AGENTS.md](AGENTS.md).

---

## Feature Comparison

| Feature | Postwire 🌲 | Mailhog | Mailpit | Mailtrap (SaaS) |
|---|---|---|---|---|
| Single Static Binary | ✅ (Rust, ~15MB) | ❌ (Go + deps) | ✅ (Go) | ❌ (Cloud SaaS only) |
| **Dual Email & SMS Catcher** | ✅ **Built-in** | ❌ Email only | ❌ Email only | 🟡 Partial |
| **Twilio Webhook Ingestion** | ✅ **Built-in** | ❌ | ❌ | ❌ |
| **Auto Signal Extraction (OTP / Links)** | ✅ **Built-in** | ❌ | ❌ | ❌ |
| **Long-Polling Wait API (`/api/wait` & `/api/sms/wait`)** | ✅ **Built-in** | ❌ | 🟡 Partial | ❌ |
| **Litmus-Style Email Analysis** | ✅ **Built-in** | ❌ | ❌ | ❌ |
| **14 MCP Tools for AI Agents** | ✅ **Built-in** | ❌ | ❌ | ❌ |
| Free & Open Source | ✅ MIT | ✅ | ✅ | ❌ |

---

## Key Features

- **SMTP Email Catcher (`:1025`)**: dead-end local SMTP listener (no auth, no relaying, no external network calls).
- **SMS Catcher & Twilio Webhook Support (`:8025`)**: ingest SMS via standard JSON (`POST /api/sms`) or Twilio webhooks (`POST /api/sms/webhook` with `x-www-form-urlencoded` payloads).
- **Dual-Tab Web UI & REST API (`:8025`)**: clean React interface for viewing both Emails and SMS messages with live WebSocket updates.
- **Rich Message Inspection**: view HTML/plain-text bodies, raw headers, download attachments or raw `.eml` files.
- **Litmus-Style Email Analysis**: `GET /api/messages/:id/analysis` checks 11 HTML email-client compatibility factors (DOCTYPE, tables, inline CSS, fonts, image alt, size clipping) and provides a SpamAssassin-style heuristic spam score.
- **Search, Pagination & Bulk Actions**: multi-select grid with bulk mark read/unread, bulk delete, and paginated lazy-loading (50 items per page).
- **Agentic Long-Polling**: `GET /api/wait` and `GET /api/sms/wait` long-poll server-side for incoming emails or SMS matching filters (`to`, `from`, `subject`, `body`, `since`).
- **Signal Extraction Engine**: `GET /api/messages/:id/extract` and `GET /api/sms/:id/extract` automatically pull OTP codes (4–8 digits) and magic links out of captured emails and SMS bodies.
- **14 MCP Agent Tools (`postwire-mcp`)**: Model Context Protocol stdio server exposing 7 Email tools and 7 SMS tools directly to Claude Desktop, Copilot, Cursor, and autonomous test runners.
- **SQLite Storage**: persistent SQLite storage (`/data/postwire.db` or `:memory:`) with automatic FIFO pruning past `MAX_MESSAGES`.
- **Zero External Runtime Dependencies**: single binary with embedded frontend built via `rust-embed`.

---

## Installation & Distribution Methods

Postwire provides official distribution packages and single static binaries for **macOS**, **Linux**, and **Windows**.

### 🍺 Homebrew (macOS & Linux)

Install Postwire from the canonical project and release assets in [`yoosuf/postwire`](https://github.com/yoosuf/postwire). The existing Homebrew formula remains available under its legacy `postwire` package/service name, while the project itself is now branded as Postwire and the release archives provide the new `postwire` commands.

```bash
# Add the Homebrew tap and install
brew tap yoosuf/tap
brew install postwire

# Or install directly in a single command:
brew install yoosuf/tap/postwire
```

**Manage as a background service (macOS & Linux):**

```bash
# Start Postwire as a background service (SMTP on :1025, Web UI & API on :8025)
brew services start postwire

# Check service status, stop, or restart
brew services info postwire
brew services stop postwire
brew services restart postwire
```

---

### ⚡ Automated One-Liner Installers

Auto-detects OS and CPU architecture, downloads the latest binary release, and places executables in PATH.

**macOS & Linux (POSIX Shell):**
```bash
curl -fsSL https://raw.githubusercontent.com/yoosuf/postwire/main/install.sh | sh
```

**Windows (PowerShell):**
```powershell
iwr -useb https://raw.githubusercontent.com/yoosuf/postwire/main/install.ps1 | iex
```

---

### 🐧 Linux Native Packages

- **Debian / Ubuntu (`.deb`)**:
  ```bash
  curl -LO https://github.com/yoosuf/postwire/releases/latest/download/postwire_amd64.deb
  sudo dpkg -i postwire_amd64.deb
  ```
- **Fedora / RHEL / CentOS (`.rpm`)**:
  ```bash
  sudo rpm -i https://github.com/yoosuf/postwire/releases/latest/download/postwire.x86_64.rpm
  ```
- **Arch Linux (AUR)**:
  ```bash
  yay -S postwire-bin
  ```

---

### 🪟 Windows Package Managers

- **Scoop**:
  ```powershell
  scoop bucket add postwire https://github.com/yoosuf/scoop-bucket
  scoop install postwire
  ```
- **Winget (Windows Package Manager)**:
  ```cmd
  winget install Postwire.Postwire
  ```
- **Chocolatey**:
  ```cmd
  choco install postwire
  ```

---

### 📦 Pre-Compiled GitHub Release Binaries

Download standalone pre-built binaries from [GitHub Releases](https://github.com/yoosuf/postwire/releases/latest):

| Operating System | Architecture | Package File |
|---|---|---|
| **macOS** | Apple Silicon (`aarch64`) | `postwire-v*-aarch64-apple-darwin.tar.gz` |
| **macOS** | Intel (`x86_64`) | `postwire-v*-x86_64-apple-darwin.tar.gz` |
| **Linux** | x86_64 | `postwire-v*-x86_64-unknown-linux-gnu.tar.gz` |
| **Linux** | ARM64 (`aarch64`) | `postwire-v*-aarch64-unknown-linux-gnu.tar.gz` |
| **Windows** | x86_64 | `postwire-v*-x86_64-pc-windows-msvc.zip` |

Each archive also includes the legacy `postwire` and `postwire-mcp` command names.

---

### 🦀 Cargo (Rust Workspace)

Install directly via `cargo`:

```bash
cargo install --git https://github.com/yoosuf/postwire postwire-server --bin postwire
cargo install --git https://github.com/yoosuf/postwire postwire-mcp --bin postwire-mcp
```

---

### 🐳 Docker & Docker Compose

Pull the published multi-arch image (`linux/amd64` + `linux/arm64`) from Docker Hub:

```bash
docker run -d --name postwire -p 1025:1025 -p 8025:8025 -v postwire-data:/data yoosuf/postwire:latest
```

Or run via Docker Compose:

```bash
docker compose up --build
```

To run the MCP server against it:

```bash
docker compose --profile mcp run --rm mcp
```

**Docker Hub:** [`yoosuf/postwire`](https://hub.docker.com/r/yoosuf/postwire) — canonical Postwire image; legacy `yoosuf/postwire` tags remain available for compatibility.

---

## Agent Integration Examples

### REST API (Long-Polling + Signal Extraction)

```bash
# 1. Long-poll for a fresh email matching criteria (up to 10s default, max 60s)
curl "http://localhost:8025/api/wait?to=user@example.com&subject=Verify&since=2026-09-04T09:00:00Z"

# 2. Extract OTP codes and links from the received email
curl "http://localhost:8025/api/messages/<MESSAGE_ID>/extract"
# Response: { "codes": ["482913"], "links": ["https://app.test/verify?token=..."] }

# 3. Analyze HTML compatibility & spam score
curl "http://localhost:8025/api/messages/<MESSAGE_ID>/analysis"

# 4. Long-poll for a fresh SMS message
curl "http://localhost:8025/api/sms/wait?to=+15550100&since=2026-09-04T09:00:00Z"

# 5. Extract OTP codes from the received SMS
curl "http://localhost:8025/api/sms/<SMS_ID>/extract"
# Response: { "codes": ["940182"], "links": [] }
```

### Twilio SMS Ingestion Webhook

Point your application or local webhook relay to:
```http
POST http://localhost:8025/api/sms/webhook
Content-Type: application/x-www-form-urlencoded

From=%2B15550199&To=%2B15550100&Body=Your+verification+code+is+839201
```

### Node.js (E2E Test Automation)

Using Node 18+ native `fetch` to wait for messages and extract OTP verification codes:

```javascript
// 1. Capture ISO timestamp BEFORE triggering signup / SMS 2FA action
const since = new Date().toISOString();
const userPhone = "+15550199";

// 2. Trigger your application action (e.g. request 2FA SMS code)
await triggerSmsCodeAction({ to: userPhone });

// 3. Long-poll Postwire server-side until SMS arrives (blocks up to timeout_ms)
const waitRes = await fetch(
  `http://localhost:8025/api/sms/wait?to=${encodeURIComponent(userPhone)}&since=${since}&timeout_ms=10000`
);
const sms = await waitRes.json();

// 4. Extract 4-8 digit OTP verification code
const extractRes = await fetch(`http://localhost:8025/api/sms/${sms.id}/extract`);
const { codes } = await extractRes.json();
const otpCode = codes[0]; // e.g. "839201"

// 5. Submit extracted OTP code into your test runner / app
await submitOtpCode(otpCode);
```

---

## REST API Directory

### Email Endpoints (`/api/messages`)

| Method | Endpoint | Description |
|---|---|---|
| `GET` | `/api/messages` | List captured emails (`search`, `limit`, `offset`) |
| `GET` | `/api/messages/:id` | Fetch full email details (headers, text/html, attachments) |
| `GET` | `/api/messages/:id/raw` | Download raw MIME file (`.eml`) |
| `GET` | `/api/messages/:id/html` | Render raw HTML body |
| `GET` | `/api/messages/:id/attachments/:index` | Download attachment by index |
| `GET` | `/api/messages/:id/extract` | Extract OTP codes (4-8 digits) and HTTP/HTTPS links |
| `GET` | `/api/messages/:id/analysis` | HTML email compatibility checks & heuristic spam score |
| `GET` | `/api/wait` | Server-side long-polling wait (`to`, `from`, `subject`, `since`, `timeout_ms`) |
| `PATCH` | `/api/messages/:id/read` | Toggle read/unread state |
| `DELETE` | `/api/messages/:id` | Delete email by ID |
| `POST` | `/api/messages/bulk-delete` | Bulk delete emails by ID list |
| `PATCH` | `/api/messages/bulk-read` | Bulk update read state |
| `DELETE` | `/api/messages` | Clear all emails |
| `POST` | `/api/test-email` | Inject synthetic test email |

### SMS Endpoints (`/api/sms`)

| Method | Endpoint | Description |
|---|---|---|
| `GET` | `/api/sms` | List captured SMS messages (`search`, `limit`, `offset`) |
| `GET` | `/api/sms/:id` | Fetch single SMS details |
| `POST` | `/api/sms` | Ingest SMS via JSON payload |
| `POST` | `/api/sms/webhook` | Ingest SMS via Twilio form-urlencoded or JSON webhook |
| `GET` | `/api/sms/wait` | Server-side long-polling wait for SMS (`to`, `from`, `body`, `since`, `timeout_ms`) |
| `GET` | `/api/sms/:id/extract` | Extract OTP codes (4-8 digits) and HTTP/HTTPS links from SMS |
| `PATCH` | `/api/sms/:id/read` | Toggle read/unread state |
| `DELETE` | `/api/sms/:id` | Delete SMS by ID |
| `POST` | `/api/sms/bulk-delete` | Bulk delete SMS by ID list |
| `PATCH` | `/api/sms/bulk-read` | Bulk update SMS read state |
| `DELETE` | `/api/sms` | Clear all SMS messages |
| `POST` | `/api/test-sms` | Inject synthetic test SMS message |

---

## Model Context Protocol (MCP) Server (`postwire-mcp`)

Postwire includes 14 built-in MCP tools for AI agents (Claude Desktop, Copilot, Cursor, agentic E2E tests):

| Category | Tool Name | Parameters | Description |
|---|---|---|---|
| **Email Tools** | `list_emails` | `search?`, `limit?`, `offset?` | Query emails with search filter and pagination |
| | `get_email` | `id` | Get email headers, text/html content, and attachment metadata |
| | `wait_for_email` | `to?`, `from?`, `subject?`, `since_ms?`, `timeout_ms?` | Long-poll server-side until matching email arrives |
| | `extract_signals` | `id` | Automatically extract OTP codes and links from email |
| | `send_test_email` | `to?` | Inject synthetic test email |
| | `delete_email` | `id` | Delete email by ID |
| | `clear_inbox` | *none* | Clear all emails |
| **SMS Tools** | `list_sms` | `search?`, `limit?`, `offset?` | Query captured SMS with search filter and pagination |
| | `get_sms` | `id` | Get single SMS details |
| | `wait_for_sms` | `to?`, `from?`, `body?`, `since_ms?`, `timeout_ms?` | Long-poll server-side until matching SMS arrives |
| | `extract_sms_signals` | `id` | Automatically extract OTP codes and links from SMS body |
| | `send_test_sms` | `to?`, `from?`, `body?` | Inject synthetic test SMS |
| | `delete_sms` | `id` | Delete SMS by ID |
| | `clear_sms_inbox` | *none* | Clear all SMS messages |

---

## Configuration (Environment Variables)

| Variable | Default | Description |
|---|---|---|
| `POSTWIRE_SMTP_PORT` | `1025` | SMTP listener port; `SMTP_PORT` remains a legacy alias |
| `POSTWIRE_HTTP_PORT` | `8025` | Web UI / REST API port; `HTTP_PORT` remains a legacy alias |
| `POSTWIRE_BIND_ADDR` | `0.0.0.0` | Bind address; `BIND_ADDR` remains a legacy alias |
| `POSTWIRE_DB_PATH` | `postwire.db` (`/data/postwire.db` in Docker) | SQLite path; `DB_PATH` remains a legacy alias and the default preserves existing data |
| `POSTWIRE_MAX_MESSAGES` | `1000` | FIFO limit (`0` = unlimited); `MAX_MESSAGES` remains a legacy alias |
| `POSTWIRE_TTL_SECONDS` | `0` | Remove messages/SMS older than this many seconds on ingestion; `TTL_SECONDS` remains a legacy alias |
| `POSTWIRE_SMTP_HOSTNAME` | `postwire` | SMTP banner hostname; `SMTP_HOSTNAME` remains a legacy alias |
| `POSTWIRE_URL` | `http://127.0.0.1:8025` | (`postwire-mcp` only) Server URL; `POSTWIRE_URL` remains a legacy alias |

---

## Local Development

```bash
# Terminal 1 — Frontend hot reload (Vite dev server proxied to Rust API)
cd apps/web && npm install && npm run dev

# Terminal 2 — Backend server (SMTP on :1025, API on :8025)
cargo run -p postwire-server --bin postwire
```

## Building Release Binaries

```bash
cd apps/web && npm install && npm run build && cd ..
cargo build --release --workspace
```

The frontend static assets must be built to `apps/web/dist` before building `postwire-server`, as they are embedded directly into the `postwire` binary using `rust-embed`.

---

## Author

Built by [Yoosuf](https://yoosuf.me/), who also offers [fractional CTO services](https://yoosuf.me/services/).
