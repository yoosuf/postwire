# Postwire Rename Design

## Goal

Rename Pine Mail as Postwire across the product and project surfaces in this
repository. The interface, documentation, installers, packaged artifacts, and
runtime metadata should present Postwire as the current name.

## Scope

- Rename Cargo workspace package identities and primary executables to
  `postwire-core`, `postwire-server`, `postwire-mcp`, `postwire`, and
  `postwire-mcp` respectively.
- Rename user-facing UI labels, sample text, MCP server metadata, installer
  output, Docker/Compose service and local image names, release artifact names,
  synthetic test-message identity, and project documentation to Postwire.
- Update defaults and environment variable names to `POSTWIRE_*` where the
  application or installers currently expose `POSTWIRE_*` settings.
- Keep existing `postwire` executable names and `POSTWIRE_*` settings working
  as compatibility aliases when practical. Existing database files and data
  must remain usable; do not migrate or rename user data as part of branding.
- Use `https://github.com/yoosuf/postwire` as the canonical source URL after
  renaming the existing GitHub repository; GitHub redirects the old URL.
- Docker Hub does not allow repository renames. Keep the current published
  image usable until a public `yoosuf/postwire` repository is created and
  images are republished there; do not remove the legacy image.

## Compatibility and behavior

Postwire names take precedence when both new and legacy environment variables
are supplied. Legacy variables remain accepted. Old executable aliases should
invoke the same behavior as their Postwire counterparts. Database defaults
must not silently move existing installations to a new empty database; retain
the current default path or provide a deliberate fallback that discovers the
existing database. HTTP routes, JSON shapes, SMTP behavior, and MCP tool names
remain unchanged.

The rename must preserve all existing in-progress source changes in the
worktree. Do not commit or push source changes. The user has explicitly
authorized renaming the GitHub repository and migrating the Docker Hub
coordinate; do not delete the legacy Docker image.

## Verification

- Search tracked and untracked project files for obsolete user-facing product
  names and stale invocation examples, accounting for intentional compatibility
  aliases and external hosting URLs.
- Check the Cargo workspace and build all new and compatibility executable
  names.
- Run Rust tests and the frontend production build.
- Check installer syntax and release/Docker references for artifact-name
  consistency.
- Confirm the working diff contains no unrelated changes and passes
  `git diff --check`.
