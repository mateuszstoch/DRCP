#!/bin/sh
# Simple install script for DRCP
# Downloads the latest release binary from GitHub based on OS and Architecture

set -e

# Repository configuration
GITHUB_REPO="mateuszstoch/DRCP"
BINARY_NAME="drcp"

# Detect OS and Architecture
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$OS" in
  darwin)
    PLATFORM="apple-darwin"
    ;;
  linux)
    PLATFORM="unknown-linux-gnu"
    ;;
  *)
    echo "Error: Unsupported OS: $OS"
    exit 1
    ;;
esac

case "$ARCH" in
  x86_64)
    TARGET_ARCH="x86_64"
    ;;
  arm64|aarch64)
    TARGET_ARCH="aarch64"
    ;;
  *)
    echo "Error: Unsupported architecture: $ARCH"
    exit 1
    ;;
esac

# Construct download URL
TARBALL_NAME="${BINARY_NAME}-${TARGET_ARCH}-${PLATFORM}.tar.gz"
DOWNLOAD_URL="https://github.com/${GITHUB_REPO}/releases/latest/download/${TARBALL_NAME}"

echo "Fetching latest release for ${OS}-${ARCH}..."
echo "URL: ${DOWNLOAD_URL}"

# Create a temp directory for extraction
TMP_DIR=$(mktemp -d)
clean_up() {
  rm -rf "$TMP_DIR"
}
trap clean_up EXIT

# Download tarball
curl -sSfL "$DOWNLOAD_URL" -o "${TMP_DIR}/${TARBALL_NAME}"

# Extract binary
tar -xzf "${TMP_DIR}/${TARBALL_NAME}" -C "$TMP_DIR"

# Install binary to /usr/local/bin (requires sudo) or fallback to current directory
INSTALL_DIR="/usr/local/bin"

if [ -w "$INSTALL_DIR" ]; then
  mv "${TMP_DIR}/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"
  echo "Successfully installed ${BINARY_NAME} to ${INSTALL_DIR}/${BINARY_NAME}"
else
  echo "Permission denied for ${INSTALL_DIR}. Installing to current directory instead..."
  mv "${TMP_DIR}/${BINARY_NAME}" "./${BINARY_NAME}"
  echo "Successfully installed ${BINARY_NAME} in current directory: $(pwd)/${BINARY_NAME}"
fi
