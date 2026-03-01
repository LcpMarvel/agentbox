#!/bin/sh
set -e

# AgentBox installer — detects OS/arch and downloads from GitHub Releases.
# Usage: curl -fsSL https://raw.githubusercontent.com/<owner>/agent-box/master/install.sh | sh

REPO="LcpMarvel/agentbox"
INSTALL_DIR="$HOME/.local/bin"
BINARY_NAME="agentbox"

info() { printf "\033[1;34m==>\033[0m %s\n" "$1"; }
error() { printf "\033[1;31merror:\033[0m %s\n" "$1" >&2; exit 1; }

detect_platform() {
    OS=$(uname -s)
    ARCH=$(uname -m)

    case "$OS" in
        Darwin) OS="apple-darwin" ;;
        Linux)  OS="unknown-linux-gnu" ;;
        *)      error "Unsupported OS: $OS" ;;
    esac

    case "$ARCH" in
        x86_64)  ARCH="x86_64" ;;
        aarch64|arm64) ARCH="aarch64" ;;
        *)       error "Unsupported architecture: $ARCH" ;;
    esac

    TARGET="${ARCH}-${OS}"
}

get_latest_version() {
    VERSION=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
        | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')
    [ -z "$VERSION" ] && error "Could not determine latest version"
}

download_and_install() {
    ARCHIVE="${BINARY_NAME}-${VERSION}-${TARGET}.tar.gz"
    URL="https://github.com/${REPO}/releases/download/${VERSION}/${ARCHIVE}"

    info "Downloading AgentBox ${VERSION} for ${TARGET}..."
    TMPDIR=$(mktemp -d)
    trap 'rm -rf "$TMPDIR"' EXIT

    curl -fsSL "$URL" -o "${TMPDIR}/${ARCHIVE}" || error "Download failed: ${URL}"
    tar xzf "${TMPDIR}/${ARCHIVE}" -C "$TMPDIR"

    mkdir -p "$INSTALL_DIR"
    mv "${TMPDIR}/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"
    chmod +x "${INSTALL_DIR}/${BINARY_NAME}"
}

setup_path() {
    case ":$PATH:" in
        *":${INSTALL_DIR}:"*) return ;;
    esac

    info "Adding ${INSTALL_DIR} to PATH..."
    for rc in "$HOME/.zshrc" "$HOME/.bashrc" "$HOME/.profile"; do
        if [ -f "$rc" ]; then
            echo "" >> "$rc"
            echo "# AgentBox" >> "$rc"
            echo "export PATH=\"${INSTALL_DIR}:\$PATH\"" >> "$rc"
        fi
    done
}

main() {
    info "Installing AgentBox..."
    detect_platform
    get_latest_version
    download_and_install
    setup_path

    info "AgentBox ${VERSION} installed to ${INSTALL_DIR}/${BINARY_NAME}"
    echo ""
    echo "  Get started:"
    echo "    agentbox register my-agent \"echo hello\""
    echo "    agentbox run my-agent"
    echo "    agentbox daemon start"
    echo ""
}

main
