# Postwire Rename Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rename Pine Mail to Postwire across the workspace while preserving old commands, settings, and stored data.

**Architecture:** Rename Cargo package and primary binary identities, provide legacy command aliases, and centralize new-over-old environment variable resolution. Keep the existing SQLite default and external repository/image coordinates, then update front-end, installer, release, Docker, and documentation surfaces to the new product name.

**Tech Stack:** Rust Cargo workspace, Axum/Tokio, React/Vite/TypeScript, Docker, GitHub Actions, POSIX shell, PowerShell.

**Spec:** `docs/superpowers/specs/2026-09-23-postwire-rename-design.md`

## Global Constraints

- Postwire names take precedence when both new and legacy environment variables are supplied.
- Legacy variables remain accepted.
- Old executable aliases should invoke the same behavior as their Postwire counterparts.
- Existing database files and data must remain usable; do not migrate or rename user data as part of branding.
- Use `yoosuf/postwire` as the canonical GitHub URL after the repository rename; retain old-URL redirects.
- Docker Hub cannot rename repositories: preserve the legacy image and migrate to a new public `yoosuf/postwire` image repository when account access is available.
- Preserve all existing in-progress source changes in the worktree.
- Do not commit, push, publish an image, or change the remote GitHub repository.

## Review Focus

- Conflicting POSTWIRE/POSTWIRE values resolve to POSTWIRE, while legacy-only configuration still works; test config resolution.
- Old executable names remain available beside Postwire names; verify Cargo metadata/build outputs.
- Existing database path remains unchanged and opens existing data; verify unchanged default and Store tests.
- Release archive/package names match files actually built on each OS; inspect workflow artifact commands.
- The frontend, MCP metadata, docs, and examples consistently display and invoke Postwire while retaining intentional compatibility references.

## File Map

- `Cargo.toml`, `Cargo.lock`, and crate manifests define workspace/package identities and binaries.
- `crates/core/src/config.rs` resolves Postwire and legacy settings; `crates/server/src/main.rs` applies the configured store options.
- `crates/mcp/src/main.rs` owns MCP runtime identity and defaults.
- `apps/web/**` owns UI title, brand copy, package metadata, and icon references.
- `Dockerfile`, `docker-compose.yml`, `.github/workflows/**`, `install.sh`, and `install.ps1` define shipped names and launch behavior.
- `README.md`, `DOCKERHUB.md`, `ARCHITECTURE.md`, `AGENTS.md`, and `examples/**` define user and agent instructions.
- `crates/core` and `crates/server/tests` hold regression coverage for compatibility and configuration.

### Task 1: Rename Cargo packages and add command aliases

**Files:** Modify `Cargo.toml`, `Cargo.lock`, `crates/core/Cargo.toml`, `crates/server/Cargo.toml`, `crates/mcp/Cargo.toml`, and Rust imports. Add legacy executable targets that share the Postwire entrypoints.

- [x] Rename package IDs to `postwire-core`, `postwire-server`, and `postwire-mcp`; rename the primary server and MCP commands to `postwire` and `postwire-mcp`.
- [x] Preserve `postwire` and `postwire-mcp` as compatibility targets that call the same entrypoint code.
- [x] Update internal crate imports and workspace references.
- [x] Run `cargo metadata --no-deps --format-version 1` and confirm all four executable target names and renamed package IDs.

### Task 2: Preserve and rename configuration/runtime identity

**Files:** Modify `crates/core/src/config.rs`, `crates/server/src/main.rs`, `crates/mcp/src/main.rs`; add/update config tests.

- [x] Add Postwire-prefixed settings with precedence over existing `POSTWIRE_*` variables and preserve legacy-only behavior.
- [x] Keep the default database path `postwire.db`; `POSTWIRE_DB_PATH` is supported without relocating existing data.
- [x] Set Postwire as the default SMTP greeting and MCP server identity while preserving existing protocol contracts and tool names.
- [x] Add configuration and MCP URL precedence/legacy/default tests.
- [x] Run focused and workspace tests.

### Task 3: Rename application and deployment surfaces

**Files:** Modify `apps/web/index.html`, `apps/web/package.json`, `apps/web/package-lock.json`, `apps/web/src/App.tsx`, `apps/web/src/components/SetupPanel.tsx`, `apps/web/src/components/Icons.tsx`, `Dockerfile`, `docker-compose.yml`, and `.github/workflows/docker-publish.yml`.

- [x] Update visible product copy, package metadata, Compose service/local image names, and Docker runtime account/binary paths to Postwire; retain the data volume name for compatibility.
- [ ] Migrate Docker publishing to a new public `yoosuf/postwire` repository once it exists; preserve `yoosuf/postwire` images.
- [x] Build the frontend and check Compose/Docker references for matching executable and image names.

### Task 4: Rename installers and release artifacts

**Files:** Modify `install.sh`, `install.ps1`, and `.github/workflows/release.yml`.

- [x] Make new release archives and packages use Postwire names and install Postwire executables, while producing legacy-named archive aliases.
- [x] Install old command aliases alongside new commands; keep legacy installer environment variables working and support Postwire-prefixed equivalents.
- [x] Point release download URLs at `yoosuf/postwire` after the GitHub repository rename.
- [x] Run POSIX shell syntax and GitHub workflow validation; inspect Windows targets against Cargo outputs (PowerShell parser unavailable in this environment).

### Task 5: Update documentation, examples, and final verification

**Files:** Modify `README.md`, `DOCKERHUB.md`, `ARCHITECTURE.md`, `AGENTS.md`, `examples/mcp_e2e_demo.py`, `examples/node_e2e_demo.js`, and the design/plan notes if implementation details require clarification.

- [x] Change user-facing product and invocation copy to Postwire; retain documented legacy aliases and hosting URLs only where required.
- [x] Search project files for stale names and classify remaining matches as compatibility, historical data paths, or external coordinates.
- [x] Run `cargo check --workspace`, `cargo test --workspace`, release build, and `npm run build`. `cargo fmt --all -- --check` was attempted but reports formatting drift in pre-existing dirty feature files; workspace-wide formatting was intentionally not applied to avoid unrelated edits.
- [x] Run `git diff --check` and inspect the final diff while preserving earlier uncommitted feature changes.
