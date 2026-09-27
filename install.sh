#!/bin/sh
# Postwire Installer Script for macOS and Linux
# Usage: curl -fsSL https://raw.githubusercontent.com/yoosuf/postwire/main/install.sh | sh
set -e

REPO="yoosuf/postwire"

# Determine download tool
downloader() {
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$1"
    elif command -v wget >/dev/null 2>&1; then
        wget -qO- "$1"
    else
        echo "Error: Neither curl nor wget is installed." >&2
        exit 1
    fi
}

download_file() {
    url="$1"
    dest="$2"
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$url" -o "$dest"
    elif command -v wget >/dev/null 2>&1; then
        wget -qO "$dest" "$url"
    else
        echo "Error: Neither curl nor wget is installed." >&2
        exit 1
    fi
}

# Determine installation directory
if [ -n "$POSTWIRE_INSTALL_DIR" ]; then
    INSTALL_DIR="$POSTWIRE_INSTALL_DIR"
elif [ -n "$POSTWIRE_INSTALL_DIR" ]; then
    INSTALL_DIR="$POSTWIRE_INSTALL_DIR"
elif [ -w "/usr/local/bin" ]; then
    INSTALL_DIR="/usr/local/bin"
else
    INSTALL_DIR="$HOME/.local/bin"
fi

mkdir -p "$INSTALL_DIR"

# Detect OS & Architecture
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux)
        OS_TARGET="unknown-linux-gnu"
        ;;
    Darwin)
        OS_TARGET="apple-darwin"
        ;;
    *)
        echo "Error: Unsupported operating system $OS" >&2
        exit 1
        ;;
esac

case "$ARCH" in
    x86_64|amd64)
        ARCH_TARGET="x86_64"
        ;;
    aarch64|arm64)
        ARCH_TARGET="aarch64"
        ;;
    *)
        echo "Error: Unsupported architecture $ARCH" >&2
        exit 1
        ;;
esac

TARGET="${ARCH_TARGET}-${OS_TARGET}"

echo "Postwire Official Installer"
echo "Target Platform: ${TARGET}"
echo "Install Directory: ${INSTALL_DIR}"

# Fetch release version
if [ -z "${POSTWIRE_VERSION:-${POSTWIRE_VERSION:-}}" ]; then
    RELEASE_JSON=$(downloader "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null || echo "")
    TAG=$(echo "$RELEASE_JSON" | grep '"tag_name":' | head -n 1 | sed -E 's/.*"([^"]+)".*/\1/')
    if [ -z "$TAG" ]; then
        TAG="v0.1.0"
    fi
else
    TAG="${POSTWIRE_VERSION:-$POSTWIRE_VERSION}"
fi

echo "Installing version ${TAG}..."

TMP_DIR=$(mktemp -d 2>/dev/null || mktemp -d -t 'postwire')
trap 'rm -rf "$TMP_DIR"' EXIT

ARCHIVE_NAME="postwire-${TAG}-${TARGET}.tar.gz"
URL="https://github.${REPO_HOST:-com}/${REPO}/releases/download/${TAG}/${ARCHIVE_NAME}"
TARBALL_PATH="$TMP_DIR/$ARCHIVE_NAME"

echo "Downloading release payload from ${URL}..."
if ! download_file "$URL" "$TARBALL_PATH"; then
    ARCHIVE_NAME="postwire-${TAG}-${TARGET}.tar.gz"
    URL="https://github.${REPO_HOST:-com}/${REPO}/releases/download/${TAG}/${ARCHIVE_NAME}"
    TARBALL_PATH="$TMP_DIR/$ARCHIVE_NAME"
    echo "Postwire archive unavailable; trying legacy Pine Mail release..."
    download_file "$URL" "$TARBALL_PATH"
fi

echo "Extracting binary payload..."
tar -xzf "$TARBALL_PATH" -C "$TMP_DIR"

# Older tagged releases only contain the legacy command names. Promote them in
# the extracted payload so a clean install receives the new command names too.
if [ ! -f "$TMP_DIR/postwire" ] && [ -f "$TMP_DIR/postwire" ]; then
    cp "$TMP_DIR/postwire" "$TMP_DIR/postwire"
fi
if [ ! -f "$TMP_DIR/postwire-mcp" ] && [ -f "$TMP_DIR/postwire-mcp" ]; then
    cp "$TMP_DIR/postwire-mcp" "$TMP_DIR/postwire-mcp"
fi

for binary in postwire postwire-mcp postwire postwire-mcp; do
    if [ -f "$TMP_DIR/$binary" ]; then
        mv "$TMP_DIR/$binary" "$INSTALL_DIR/$binary"
        chmod 0755 "$INSTALL_DIR/$binary"
    fi
done
chmod 0755 "$INSTALL_DIR/postwire" "$INSTALL_DIR/postwire-mcp" "$INSTALL_DIR/postwire" "$INSTALL_DIR/postwire-mcp"

echo "✅ Postwire ${TAG} successfully installed to ${INSTALL_DIR}"

# PATH warning check
case ":$PATH:" in
    *":$INSTALL_DIR:"*) ;;
    *)
        echo ""
        echo "⚠️  WARNING: ${INSTALL_DIR} is not currently in your PATH environment variable."
        echo "   Add it to your shell configuration (e.g. ~/.bashrc or ~/.zshrc):"
        echo "     export PATH=\"${INSTALL_DIR}:\$PATH\""
        ;;
esac

echo ""
echo "Quick Start:"
echo "  postwire          # Starts SMTP on :1025 and Web UI / REST API on :8025"
echo "  postwire-mcp      # Starts MCP stdio server for AI agents"
echo "  postwire / postwire-mcp remain available as compatibility commands"
