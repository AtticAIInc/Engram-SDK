#!/bin/bash
set -euo pipefail

# Install the engram CLI binary from GitHub Releases.
# Expected environment variables:
#   ENGRAM_VERSION  - Tag to install (e.g. "v0.1.0" or "latest")
#   RUNNER_OS       - GitHub Actions runner OS (Linux, macOS)
#   RUNNER_ARCH     - GitHub Actions runner arch (X64, ARM64)
#   GITHUB_PATH     - GitHub Actions path file (for adding to PATH)

REPO="AtticAIInc/Engram-SDK"
VERSION="${ENGRAM_VERSION:-latest}"
OS="${RUNNER_OS:-Linux}"
ARCH="${RUNNER_ARCH:-X64}"

# Map GitHub Actions runner environment to Rust target triples
case "${OS}-${ARCH}" in
  Linux-X64)    TARGET="x86_64-unknown-linux-musl" ;;
  Linux-ARM64)  TARGET="aarch64-unknown-linux-musl" ;;
  macOS-X64)    TARGET="x86_64-apple-darwin" ;;
  macOS-ARM64)  TARGET="aarch64-apple-darwin" ;;
  *)
    echo "::error::Unsupported platform: ${OS}-${ARCH}"
    exit 1
    ;;
esac

# Resolve "latest" to an actual tag
if [ "$VERSION" = "latest" ]; then
  VERSION=$(curl -sL \
    -H "Accept: application/vnd.github+json" \
    "https://api.github.com/repos/${REPO}/releases/latest" \
    | grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/')

  if [ -z "$VERSION" ]; then
    echo "::error::Failed to resolve latest release version"
    exit 1
  fi
fi

URL="https://github.com/${REPO}/releases/download/${VERSION}/engram-${TARGET}.tar.gz"
echo "Installing engram ${VERSION} for ${TARGET}..."
echo "  URL: ${URL}"

INSTALL_DIR="${HOME}/.engram/bin"
mkdir -p "$INSTALL_DIR"

HTTP_CODE=$(curl -sL -o "${INSTALL_DIR}/engram.tar.gz" -w "%{http_code}" "$URL")
if [ "$HTTP_CODE" != "200" ]; then
  echo "::error::Download failed with HTTP ${HTTP_CODE}: ${URL}"
  rm -f "${INSTALL_DIR}/engram.tar.gz"
  exit 1
fi

tar xzf "${INSTALL_DIR}/engram.tar.gz" -C "$INSTALL_DIR"
rm -f "${INSTALL_DIR}/engram.tar.gz"
chmod +x "${INSTALL_DIR}/engram"

# Add to PATH for subsequent steps
echo "${INSTALL_DIR}" >> "$GITHUB_PATH"

echo "Installed engram ${VERSION} to ${INSTALL_DIR}"
"${INSTALL_DIR}/engram" version || true
