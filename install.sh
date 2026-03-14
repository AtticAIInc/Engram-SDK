#!/bin/bash
set -euo pipefail

# Engram CLI installer
# Usage: curl -fsSL https://raw.githubusercontent.com/AtticAIInc/Engram-SDK/main/install.sh | sh
#
# Environment variables:
#   ENGRAM_VERSION   - Version to install (e.g. "v0.3.0" or "latest"). Default: latest
#   ENGRAM_INSTALL   - Installation directory. Default: ~/.engram/bin
#   GITHUB_TOKEN     - GitHub token for API requests (avoids rate limits)

REPO="AtticAIInc/Engram-SDK"
VERSION="${ENGRAM_VERSION:-latest}"
INSTALL_DIR="${ENGRAM_INSTALL:-${HOME}/.engram/bin}"

detect_platform() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"

  case "$os" in
    Linux)  os="linux" ;;
    Darwin) os="darwin" ;;
    *)
      echo "Error: Unsupported OS: $os"
      echo "Engram supports Linux and macOS."
      exit 1
      ;;
  esac

  case "$arch" in
    x86_64|amd64)  arch="x86_64" ;;
    aarch64|arm64) arch="aarch64" ;;
    *)
      echo "Error: Unsupported architecture: $arch"
      echo "Engram supports x86_64 and aarch64/arm64."
      exit 1
      ;;
  esac

  case "${os}-${arch}" in
    linux-x86_64)   TARGET="x86_64-unknown-linux-musl" ;;
    linux-aarch64)  TARGET="aarch64-unknown-linux-musl" ;;
    darwin-x86_64)  TARGET="x86_64-apple-darwin" ;;
    darwin-aarch64) TARGET="aarch64-apple-darwin" ;;
  esac
}

resolve_version() {
  if [ "$VERSION" = "latest" ]; then
    local auth_header=""
    if [ -n "${GITHUB_TOKEN:-}" ]; then
      auth_header="-H Authorization: Bearer ${GITHUB_TOKEN}"
    fi

    VERSION=$(curl -sL \
      -H "Accept: application/vnd.github+json" \
      ${auth_header} \
      "https://api.github.com/repos/${REPO}/releases/latest" \
      | grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/')

    if [ -z "$VERSION" ]; then
      echo "Error: Failed to resolve latest release version."
      echo "This may be due to GitHub API rate limits."
      echo "Try: ENGRAM_VERSION=v0.3.0 $0"
      echo "  or: GITHUB_TOKEN=ghp_... $0"
      exit 1
    fi
  fi
}

install_binary() {
  local url="$1"
  mkdir -p "$INSTALL_DIR"

  local tmp_tarball="${INSTALL_DIR}/engram.tar.gz"

  echo "Downloading engram ${VERSION} for ${TARGET}..."
  echo "  ${url}"

  local http_code
  http_code=$(curl -sL -o "$tmp_tarball" -w "%{http_code}" "$url")

  if [ "$http_code" != "200" ]; then
    rm -f "$tmp_tarball"
    echo ""
    echo "Error: Download failed with HTTP ${http_code}"
    echo ""
    echo "Possible causes:"
    echo "  - Version ${VERSION} has no release yet"
    echo "  - No binary for platform ${TARGET}"
    echo ""
    echo "Check available releases:"
    echo "  https://github.com/${REPO}/releases"
    echo ""
    echo "Or build from source:"
    echo "  cargo install --git https://github.com/${REPO}.git engram-cli"
    exit 1
  fi

  tar xzf "$tmp_tarball" -C "$INSTALL_DIR"
  rm -f "$tmp_tarball"
  chmod +x "${INSTALL_DIR}/engram"
}

setup_path() {
  # Check if already on PATH
  if echo "$PATH" | tr ':' '\n' | grep -qx "$INSTALL_DIR" 2>/dev/null; then
    return 0
  fi

  local shell_name rc_file
  shell_name="$(basename "${SHELL:-/bin/sh}")"

  case "$shell_name" in
    zsh)  rc_file="$HOME/.zshrc" ;;
    bash) rc_file="$HOME/.bashrc" ;;
    fish) rc_file="$HOME/.config/fish/config.fish" ;;
    *)    rc_file="$HOME/.profile" ;;
  esac

  echo ""
  echo "Add engram to your PATH:"
  echo ""
  if [ "$shell_name" = "fish" ]; then
    echo "  fish_add_path ${INSTALL_DIR}"
  else
    echo "  echo 'export PATH=\"${INSTALL_DIR}:\$PATH\"' >> ${rc_file}"
  fi
  echo ""
  echo "Then restart your shell or run:"
  echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
}

main() {
  detect_platform
  resolve_version

  local url="https://github.com/${REPO}/releases/download/${VERSION}/engram-${TARGET}.tar.gz"
  install_binary "$url"

  echo ""
  echo "Installed engram ${VERSION} to ${INSTALL_DIR}/engram"
  "${INSTALL_DIR}/engram" version 2>/dev/null || true

  setup_path
}

main
