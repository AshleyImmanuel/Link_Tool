#!/usr/bin/env sh
set -eu

REPO_DEFAULT="AshleyImmanuel/Link_Tool"
REPO="${LINKMAP_REPO:-$REPO_DEFAULT}"
VERSION="${LINKMAP_VERSION:-latest}"
INSTALL_DIR="${LINKMAP_INSTALL_DIR:-$HOME/.local/bin}"

echo "warning: linkmap is an experimental hobby project and is still under review. Use at your own risk." >&2
echo "warning: If you find issues, contact Ashley via LinkedIn: https://www.linkedin.com/in/ashley-immanuel-81609731b/" >&2

uname_s="$(uname -s | tr '[:upper:]' '[:lower:]')"
uname_m="$(uname -m)"

case "$uname_s" in
  linux*) platform="linux" ;;
  darwin*) platform="macos" ;;
  *)
    echo "Unsupported OS: $uname_s" >&2
    exit 1
    ;;
esac

case "$uname_m" in
  x86_64|amd64) arch="x86_64" ;;
  *)
    echo "Unsupported architecture: $uname_m (only x86_64 supported in v1 releases)" >&2
    exit 1
    ;;
esac

asset="linkmap-${platform}-${arch}.tar.gz"
base="https://github.com/${REPO}/releases"

if [ "$VERSION" = "latest" ]; then
  url="${base}/latest/download/${asset}"
  sums_url="${base}/latest/download/SHA256SUMS"
else
  url="${base}/download/v${VERSION}/${asset}"
  sums_url="${base}/download/v${VERSION}/SHA256SUMS"
fi

tmp="${TMPDIR:-/tmp}/linkmap-install.$$"
mkdir -p "$tmp"
cleanup() { rm -rf "$tmp"; }
trap cleanup EXIT INT TERM

echo "Downloading ${url}"
curl -fsSL "$url" -o "${tmp}/${asset}"
curl -fsSL "$sums_url" -o "${tmp}/SHA256SUMS"

expected="$(grep " ${asset}\$" "${tmp}/SHA256SUMS" | awk '{print $1}' | head -n 1 || true)"
if [ -z "$expected" ]; then
  echo "Could not find checksum for ${asset} in SHA256SUMS" >&2
  exit 1
fi

if command -v sha256sum >/dev/null 2>&1; then
  actual="$(sha256sum "${tmp}/${asset}" | awk '{print $1}')"
elif command -v shasum >/dev/null 2>&1; then
  actual="$(shasum -a 256 "${tmp}/${asset}" | awk '{print $1}')"
else
  echo "Missing sha256sum/shasum for checksum verification" >&2
  exit 1
fi

if [ "$actual" != "$expected" ]; then
  echo "Checksum mismatch for ${asset}" >&2
  echo "Expected: ${expected}" >&2
  echo "Actual:   ${actual}" >&2
  exit 1
fi

mkdir -p "$INSTALL_DIR"
tar -xzf "${tmp}/${asset}" -C "$tmp"
chmod +x "${tmp}/linkmap"
mv "${tmp}/linkmap" "${INSTALL_DIR}/linkmap"

echo "Installed linkmap to ${INSTALL_DIR}/linkmap"
echo "Run: ${INSTALL_DIR}/linkmap --version"

