# Postwire v0.1.0

Postwire is now available as an open-source SMTP and SMS catcher for local development and end-to-end testing.

Postwire gives development teams a self-hosted notification inbox with:

- SMTP capture on port 1025 and a web UI/API on port 8025
- SMS ingestion through JSON and Twilio webhooks
- OTP and magic-link extraction from email and SMS messages
- Long-polling wait endpoints for reliable automated tests
- Email HTML compatibility and heuristic spam analysis
- 14 MCP tools for AI agents and test runners
- A lightweight Rust binary with SQLite storage and an embedded frontend

Install it with Docker:

```bash
docker run -d --name postwire \
  -p 1025:1025 -p 8025:8025 \
  -v postwire-data:/data \
  yoosuf/postwire:latest
```

Read the documentation and release details at [github.com/yoosuf/postwire](https://github.com/yoosuf/postwire), or install the `v0.1.0` binaries from [GitHub Releases](https://github.com/yoosuf/postwire/releases/tag/v0.1.0).
