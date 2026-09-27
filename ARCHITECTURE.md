# Postwire Architecture & Technical Reference

Postwire is a lightweight, single-binary **SMTP & SMS mail catcher** built in Rust and designed specifically for modern web application development and autonomous AI agent workflows (e2e testing, authentication flow automation, OTP verification).

---

## 1. Executive Summary & Core Design Goals

1. **Single Static Binary**: Compiles into a single self-contained executable with the React single-page frontend embedded directly into the binary using `rust-embed`. Requires no external runtime dependencies (Node, JVM, or web servers).
2. **Dual Email & SMS Catcher**: Captures both SMTP emails (on port `1025`) and SMS messages via JSON REST payloads or Twilio-compatible webhooks (on port `8025`).
3. **Agentic-First Architecture**: Features server-side long-polling (`/api/wait` and `/api/sms/wait`) and signal extraction engines (`/api/messages/:id/extract` and `/api/sms/:id/extract`) for zero-polling, deterministic automated test suites.
4. **Litmus-Style Email Analysis**: Includes a built-in HTML client compatibility checker (`analyze_html`) and heuristic spam scoring engine (`analyze_spam`), exposed via `GET /api/messages/:id/analysis`.
5. **Model Context Protocol (MCP)**: Includes a companion stdio binary (`postwire-mcp`) exposing 14 tools to AI coding agents (Claude, Copilot, Cursor, etc.).
6. **Zero External Network Dependencies**: All data stays local in a SQLite database (`/data/postwire.db` or in-memory `:memory:`).

---

## 2. Monorepo Architecture

Postwire is structured as a Cargo workspace with four main components:

```
postwire/
├── crates/
│   ├── core/     (postwire-core)   — Shared storage engine, SQLite models, MIME parsing, HTML/spam analysis, signal extraction
│   ├── server/   (postwire)        — Async Axum HTTP server, SMTP server on :1025, WebSocket bus, web UI embedding
│   └── mcp/      (postwire-mcp)    — Stdio JSON-RPC 2.0 MCP server wrapping the REST API
├── apps/
│   └── web/                        — React + Vite + TS + Tailwind UI embedded into `postwire` binary
├── Cargo.toml                      — Workspace root definition
└── Dockerfile / docker-compose.yml
```

### Component Breakdown & Dependency Boundaries

```mermaid
graph TD
    UI[apps/web React SPA] -->|Embedded at build time| Server[crates/server postwire]
    MCP[crates/mcp postwire-mcp] -->|Talks JSON-RPC over stdio| AIAgent[AI Agent / IDE]
    MCP -->|Talks HTTP REST API| Server
    Server -->|Uses shared storage & models| Core[crates/core postwire-core]
    Server -->|Binds SMTP :1025| SMTPClient[App SMTP Client]
    Server -->|Binds HTTP :8025| HTTPClient[Web Browser / REST / Webhooks]
    Core -->|Manages| DB[(SQLite postwire.db)]
```

> **Design Constraint**: `crates/server` and `crates/mcp` both depend on `crates/core` for domain models and regex engines. `crates/mcp` never accesses the SQLite database file directly; it communicates exclusively over HTTP to `crates/server`. This allows `postwire-mcp` to run locally while `postwire` runs in Docker or on a remote server.

---

## 3. Storage & Data Model

`postwire-core` manages SQLite database connections using `rusqlite` with WAL mode enabled (`PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;`) and busy timeouts for thread-safe concurrent operations.

### Exact Database Schema DDL

```sql
-- Captured SMTP Emails
CREATE TABLE IF NOT EXISTS messages (
    id TEXT PRIMARY KEY,
    from_addr TEXT NOT NULL,
    to_addrs TEXT NOT NULL,        -- JSON array string of recipient addresses
    subject TEXT NOT NULL,
    size INTEGER NOT NULL,
    received_at TEXT NOT NULL,     -- ISO 8601 / RFC 3339 timestamp with milliseconds
    read INTEGER NOT NULL DEFAULT 0,
    has_html INTEGER NOT NULL DEFAULT 0,
    has_attachments INTEGER NOT NULL DEFAULT 0,
    raw BLOB NOT NULL              -- Full raw RFC 822 MIME message bytes
);

CREATE INDEX IF NOT EXISTS idx_messages_received_at ON messages(received_at);

-- Captured SMS Messages
CREATE TABLE IF NOT EXISTS sms (
    id TEXT PRIMARY KEY,
    from_phone TEXT NOT NULL,      -- Sender phone number or identifier
    to_phone TEXT NOT NULL,        -- Recipient phone number or identifier
    body TEXT NOT NULL,            -- Message text content
    received_at TEXT NOT NULL,     -- ISO 8601 / RFC 3339 timestamp with milliseconds
    read INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_sms_received_at ON sms(received_at);
```

### Automatic Capacity Maintenance & FIFO Pruning

Postwire implements automated capacity management via the `MAX_MESSAGES` configuration parameter (default: 1000):

- Whenever a new email is inserted, if `MAX_MESSAGES > 0`, the store deletes the oldest messages where `COUNT(*) > MAX_MESSAGES`.
- SMS messages are independently pruned using the same `MAX_MESSAGES` ceiling.
- Memory storage (`DB_PATH=:memory:`) follows identical pruning logic in SQLite memory databases.

---

## 4. Ingestion Pipelines & Real-Time Event Architecture

### 4.1 SMTP Email Ingestion Flow

```mermaid
sequenceDiagram
    autonumber
    actor App as App under test
    participant SMTP as SMTP Listener (:1025)
    participant Parser as MIME Parser (postwire-core)
    participant Store as SQLite Store
    participant Bus as Tokio Broadcast Channel
    participant WS as WebSocket Clients

    App->>SMTP: SMTP Connect & DATA (RFC 822)
    SMTP->>Parser: Raw EML bytes
    Parser->>Parser: Parse headers, text/html parts & attachments
    Parser->>Store: insert(NewMessage)
    Store->>Store: Apply FIFO MAX_MESSAGES pruning
    Store->>Bus: Send Event::New(MessageSummary)
    Bus->>WS: Broadcast Event::New JSON payload to connected WebSockets
    SMTP-->>App: 250 2.0.0 Ok queued
```

### 4.2 SMS Ingestion Flow (REST & Webhook)

Postwire supports two ingestion formats for SMS:
1. Standard JSON payload (`{"from": "+1555...", "to": "+1800...", "body": "Your code is 123456"}`)
2. Form-urlencoded Twilio Webhook (`From=+1555...&To=+1800...&Body=Your+code+is+123456`)

```mermaid
sequenceDiagram
    autonumber
    actor Service as App / Twilio Webhook
    participant API as Axum HTTP Server (:8025)
    participant Ingest as SMS Ingest Handler
    participant Store as SQLite Store
    participant Bus as Tokio Broadcast Channel

    Service->>API: POST /api/sms or POST /api/sms/webhook
    API->>Ingest: Parse JSON or x-www-form-urlencoded
    Ingest->>Store: insert_sms(NewSms)
    Store->>Bus: Send Event::NewSms(SmsMessage)
    Bus-->>Service: 201 Created (SmsMessage JSON)
```

### 4.3 WebSocket Event Bus (`/api/events`)

The backend maintains a `tokio::sync::broadcast` channel. When messages or SMS arrive or are updated, events are broadcast to connected WebSocket clients (`ws://localhost:8025/api/events`):

- `Event::New(MessageSummary)`
- `Event::Deleted { id }`
- `Event::BulkDeleted { ids }`
- `Event::Cleared`
- `Event::Read { id, read }`
- `Event::BulkRead { ids, read }`
- `Event::NewSms(SmsMessage)`
- `Event::SmsDeleted { id }`
- `Event::BulkSmsDeleted { ids }`
- `Event::SmsCleared`
- `Event::SmsRead { id, read }`
- `Event::BulkSmsRead { ids, read }`

---

## 5. Agentic & E2E Integration Engine

Postwire provides specialized primitives for AI agents and automated testing suites: **Long-Polling Wait**, **Signal Extraction**, and **Email Analysis**.

### 5.1 Long-Polling Wait Mechanism (`/api/wait` & `/api/sms/wait`)

Instead of requiring client-side retry loops or sleep timers, the `/api/wait` and `/api/sms/wait` endpoints hold the HTTP request open server-side (up to `timeout_ms`, default 10s, max 60s) using Tokio channels:

```mermaid
sequenceDiagram
    autonumber
    actor Agent as AI Agent / E2E Test
    participant API as GET /api/wait?since=<timestamp>
    participant Store as SQLite Store
    participant Bus as Tokio Broadcast Channel

    Agent->>API: Long-poll request with filters & `since` timestamp
    API->>Store: Check pre-existing matching items received after `since`
    alt Matching item already in DB
        Store-->>Agent: 200 OK (MessageDetail)
    else No matching item yet
        API->>Bus: Subscribe to broadcast channel
        Note over API,Bus: Server waits asynchronously (zero CPU load)
        Bus->>API: Event::New(MessageSummary) matches filters & since
        API-->>Agent: 200 OK (MessageDetail)
    end
```

> [!IMPORTANT]
> **Timestamp Synchronization Requirement**: Always record `since` *before* initiating the user action that triggers an email/SMS. If `since` is captured after calling the backend API, the message might arrive in the millisecond window before `/api/wait` is invoked and be filtered out.

### 5.2 Signal Extraction Engine (`postwire_core::mail`)

Postwire scans captured text and HTML bodies with regular expressions to extract verification payloads:

- **Verification / OTP Codes**: Matches 4-8 digit numerical sequences or alphanumeric codes (e.g., `482913`, `A-940218`).
- **Action Links**: Matches `https?://` URLs, stripping trailing punctuation, tracking pixels, and unsubscribes.

API Endpoints:
- `GET /api/messages/:id/extract` -> `{"codes": ["482913"], "links": ["https://app.test/verify?token=abc"]}`
- `GET /api/sms/:id/extract` -> `{"codes": ["193049"], "links": ["https://app.test/m/xyz"]}`

### 5.3 Litmus-Style Email Analysis Engine (`postwire_core::analysis`)

`GET /api/messages/:id/analysis` performs 11 automated HTML email-client compatibility checks and SpamAssassin-style heuristic spam scoring:

- **HTML Client Checks**: DOCTYPE, table layout structure, inline CSS usage, external stylesheets, JavaScript presence, web fonts, background images, viewport meta tag, character encoding, image alt text, and HTML size clipping (>102KB Gmail threshold).
- **Spam Scoring**: Trigger words, uppercase subjects, exclamation counts, empty subjects, missing text alternatives, image-to-text ratios, URL shorteners, missing List-Unsubscribe headers, and risky attachments (`.exe`, `.js`, etc.).

---

## 6. Complete REST API Endpoint Directory

### Email Endpoints

| Method | Endpoint | Description |
|---|---|---|
| `GET` | `/api/messages` | List emails (`search`, `limit`, `offset`) |
| `GET` | `/api/messages/:id` | Get email details (text/html body, headers, attachments) |
| `GET` | `/api/messages/:id/raw` | Download raw `.eml` source file |
| `GET` | `/api/messages/:id/html` | Render raw HTML body |
| `GET` | `/api/messages/:id/attachments/:index` | Download attachment file by index |
| `GET` | `/api/messages/:id/extract` | Extract OTP codes and URLs from email body |
| `GET` | `/api/messages/:id/analysis` | HTML compatibility checks & heuristic spam score |
| `GET` | `/api/wait` | Long-poll server-side for matching email |
| `PATCH` | `/api/messages/:id/read` | Toggle read/unread state |
| `DELETE` | `/api/messages/:id` | Delete single email by ID |
| `POST` | `/api/messages/bulk-delete` | Bulk delete emails (`{"ids": [...]}`) |
| `PATCH` | `/api/messages/bulk-read` | Bulk mark read (`{"ids": [...], "read": true}`) |
| `DELETE` | `/api/messages` | Clear all emails |
| `POST` | `/api/test-email` | Deliver synthetic test email |

### SMS Endpoints

| Method | Endpoint | Description |
|---|---|---|
| `GET` | `/api/sms` | List SMS messages (`search`, `limit`, `offset`) |
| `GET` | `/api/sms/:id` | Get single SMS by ID |
| `POST` | `/api/sms` | Ingest SMS via JSON payload |
| `POST` | `/api/sms/webhook` | Ingest SMS via Twilio form-urlencoded or JSON |
| `GET` | `/api/sms/wait` | Long-poll server-side for matching SMS |
| `GET` | `/api/sms/:id/extract` | Extract OTP codes and URLs from SMS body |
| `PATCH` | `/api/sms/:id/read` | Toggle read/unread state |
| `DELETE` | `/api/sms/:id` | Delete single SMS by ID |
| `POST` | `/api/sms/bulk-delete` | Bulk delete SMS (`{"ids": [...]}`) |
| `PATCH` | `/api/sms/bulk-read` | Bulk mark read (`{"ids": [...], "read": true}`) |
| `DELETE` | `/api/sms` | Clear all SMS messages |
| `POST` | `/api/test-sms` | Deliver synthetic test SMS |

### System Endpoints

| Method | Endpoint | Description |
|---|---|---|
| `GET` | `/api/config` | Returns server metadata (`version`, `smtp_port`, `http_port`) |
| `GET` | `/api/events` | WebSocket live event stream |

---

## 7. Model Context Protocol (MCP) Integration

`crates/mcp/src/main.rs` implements a zero-dependency JSON-RPC 2.0 stdio MCP server (`postwire-mcp`).

### Available MCP Tools (14 Tools)

| Category | Tool Name | Parameters | Description |
|---|---|---|---|
| **Email** | `list_emails` | `search?`, `limit?`, `offset?` | Query emails with search filter, limit, and offset |
| | `get_email` | `id` | Fetch full message headers, text/html content, and attachments |
| | `wait_for_email` | `to?`, `from?`, `subject?`, `since_ms?`, `timeout_ms?` | Server-side long-poll for fresh matching email |
| | `extract_signals` | `id` | Extract OTP codes and verification URLs from email body |
| | `send_test_email` | `to?` | Generate synthetic test email into inbox |
| | `delete_email` | `id` | Remove single email by ID |
| | `clear_inbox` | *none* | Purge all emails from store |
| **SMS** | `list_sms` | `search?`, `limit?`, `offset?` | Query SMS messages with search filter, limit, and offset |
| | `get_sms` | `id` | Fetch single SMS message by ID |
| | `wait_for_sms` | `to?`, `from?`, `body?`, `since_ms?`, `timeout_ms?` | Server-side long-poll for fresh matching SMS |
| | `extract_sms_signals`| `id` | Extract OTP codes and URLs from SMS body |
| | `send_test_sms` | `to?`, `from?`, `body?` | Generate synthetic test SMS message |
| | `delete_sms` | `id` | Remove single SMS message by ID |
| | `clear_sms_inbox` | *none* | Purge all SMS messages from store |

---

## 8. Configuration Parameters

Postwire relies on environment variables for execution settings:

| Variable | Type | Default | Description |
|---|---|---|---|
| `POSTWIRE_SMTP_PORT` | u16 | `1025` | SMTP port; `SMTP_PORT` remains a legacy alias |
| `POSTWIRE_HTTP_PORT` | u16 | `8025` | HTTP/UI port; `HTTP_PORT` remains a legacy alias |
| `POSTWIRE_BIND_ADDR` | String | `0.0.0.0` | Bind address; `BIND_ADDR` remains a legacy alias |
| `POSTWIRE_DB_PATH` | String | `postwire.db` (`/data/postwire.db` in Docker) | SQLite path; `DB_PATH` remains a legacy alias to preserve existing data |
| `POSTWIRE_MAX_MESSAGES` | u64 | `1000` | FIFO count ceiling; `MAX_MESSAGES` remains a legacy alias |
| `POSTWIRE_TTL_SECONDS` | u64 | `0` | Ingestion-time retention; `TTL_SECONDS` remains a legacy alias |
| `POSTWIRE_SMTP_HOSTNAME` | String | `postwire` | SMTP EHLO hostname; `SMTP_HOSTNAME` remains a legacy alias |
| `POSTWIRE_URL` | String | `http://127.0.0.1:8025` | MCP server URL; `POSTWIRE_URL` remains a legacy alias |
