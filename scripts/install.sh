#!/usr/bin/env sh
set -eu

repo="${HOARDER_REPO:-Ns2Kracy/hoarder}"
version="${HOARDER_VERSION:-latest}"
install_dir="${HOARDER_INSTALL_DIR:-$HOME/.local/bin}"

os="$(uname -s)"
arch="$(uname -m)"

case "$os" in
  Linux)
    platform="linux"
    ;;
  Darwin)
    platform="macos"
    ;;
  *)
    echo "unsupported operating system: $os" >&2
    exit 1
    ;;
esac

case "$arch" in
  x86_64|amd64)
    cpu="x86_64"
    ;;
  arm64|aarch64)
    if [ "$platform" = "macos" ]; then
      cpu="arm64"
    else
      echo "unsupported architecture for $platform: $arch" >&2
      exit 1
    fi
    ;;
  *)
    echo "unsupported architecture: $arch" >&2
    exit 1
    ;;
esac

archive="hoarder-${platform}-${cpu}.tar.gz"
if [ "$version" = "latest" ]; then
  url="https://github.com/${repo}/releases/latest/download/${archive}"
else
  url="https://github.com/${repo}/releases/download/${version}/${archive}"
fi

tmp_dir="$(mktemp -d)"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT INT TERM

mkdir -p "$install_dir"
echo "Downloading $url"
curl --fail --location --silent --show-error "$url" --output "$tmp_dir/$archive"
tar -xzf "$tmp_dir/$archive" -C "$tmp_dir"
install -m 0755 "$tmp_dir/hoarder" "$install_dir/hoarder"

echo "Installed hoarder to $install_dir/hoarder"
case ":$PATH:" in
  *":$install_dir:"*) ;;
  *) echo "Add $install_dir to PATH to run hoarder from any shell." ;;
esac
