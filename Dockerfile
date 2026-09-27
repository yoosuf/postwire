# ---- Stage 1: build the React/Vite frontend ----
FROM node:20-alpine AS frontend
WORKDIR /app/web
COPY apps/web/package.json apps/web/package-lock.json* ./
RUN npm install
COPY apps/web/ ./
RUN npm run build

# ---- Stage 2: build the Rust workspace (server + mcp binaries) ----
FROM rust:1-alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
COPY crates ./crates
COPY --from=frontend /app/web/dist ./apps/web/dist
RUN cargo build --release --bin postwire --bin postwire-mcp && \
    strip target/release/postwire target/release/postwire-mcp

# ---- Stage 3: minimal runtime image ----
FROM alpine:3.20
RUN apk add --no-cache tini && adduser -D -u 1000 postwire
COPY --from=builder /app/target/release/postwire /usr/local/bin/postwire
COPY --from=builder /app/target/release/postwire-mcp /usr/local/bin/postwire-mcp
RUN mkdir -p /data && chown postwire:postwire /data
USER postwire
ENV POSTWIRE_DB_PATH=/data/postwire.db \
    POSTWIRE_HTTP_PORT=8025 \
    POSTWIRE_SMTP_PORT=1025 \
    POSTWIRE_BIND_ADDR=0.0.0.0
EXPOSE 1025 8025
VOLUME ["/data"]
ENTRYPOINT ["tini", "--", "/usr/local/bin/postwire"]
