#!/bin/sh
set -eu

repository="omanar42/TypeTTY"
release_base_url="${WPM_RELEASE_BASE_URL:-https://github.com/$repository/releases}"
install_dir="${WPM_INSTALL_DIR:-$HOME/.local/bin}"
version="${WPM_VERSION:-latest}"

say() {
  printf '%s\n' "$*"
}

fail() {
  printf 'TypeTTY installer: %s\n' "$*" >&2
  exit 1
}

command -v curl >/dev/null 2>&1 || fail "curl is required"
command -v tar >/dev/null 2>&1 || fail "tar is required"

operating_system="${WPM_OS:-$(uname -s)}"
architecture="${WPM_ARCH:-$(uname -m)}"

case "$operating_system" in
  Darwin) platform="apple-darwin" ;;
  Linux) platform="unknown-linux-musl" ;;
  *) fail "unsupported operating system: $operating_system" ;;
esac

case "$architecture" in
  x86_64 | amd64) architecture="x86_64" ;;
  arm64 | aarch64) architecture="aarch64" ;;
  *) fail "unsupported architecture: $architecture" ;;
esac

target="$architecture-$platform"
archive="wpm-$target.tar.gz"
if [ "$version" = "latest" ]; then
  download_url="$release_base_url/latest/download"
else
  download_url="$release_base_url/download/$version"
fi

temporary_dir="$(mktemp -d "${TMPDIR:-/tmp}/typetty.XXXXXX")"
trap 'rm -rf "$temporary_dir"' EXIT HUP INT TERM

say "Downloading TypeTTY for $target..."
curl -fsSL "$download_url/$archive" -o "$temporary_dir/$archive"
curl -fsSL "$download_url/$archive.sha256" -o "$temporary_dir/$archive.sha256"

expected_checksum="$(awk '{print $1}' "$temporary_dir/$archive.sha256")"
if command -v sha256sum >/dev/null 2>&1; then
  actual_checksum="$(sha256sum "$temporary_dir/$archive" | awk '{print $1}')"
elif command -v shasum >/dev/null 2>&1; then
  actual_checksum="$(shasum -a 256 "$temporary_dir/$archive" | awk '{print $1}')"
else
  fail "sha256sum or shasum is required to verify the download"
fi
[ "$expected_checksum" = "$actual_checksum" ] || fail "download checksum did not match"

tar -xzf "$temporary_dir/$archive" -C "$temporary_dir"
[ -f "$temporary_dir/wpm" ] || fail "release archive does not contain wpm"
mkdir -p "$install_dir"
temporary_binary="$install_dir/.wpm.new.$$"
cp "$temporary_dir/wpm" "$temporary_binary"
chmod 755 "$temporary_binary"
mv "$temporary_binary" "$install_dir/wpm"

path_updated=false
if [ -z "${WPM_SKIP_PATH_UPDATE:-}" ] && [ "$install_dir" = "$HOME/.local/bin" ]; then
  case ":${PATH:-}:" in
    *":$install_dir:"*) ;;
    *)
      shell_name="$(basename "${SHELL:-sh}")"
      case "$shell_name" in
        zsh) shell_file="$HOME/.zshrc" ;;
        bash) shell_file="$HOME/.bashrc" ;;
        *) shell_file="$HOME/.profile" ;;
      esac
      if ! grep -F '.local/bin' "$shell_file" >/dev/null 2>&1; then
        {
          printf '\n# TypeTTY\n'
          printf '%s\n' "export PATH=\"\$HOME/.local/bin:\$PATH\""
        } >>"$shell_file"
        path_updated=true
      fi
      ;;
  esac
fi

say "Installed wpm to $install_dir/wpm"
if [ "$path_updated" = true ]; then
  say "Updated your shell PATH. Open a new terminal or run:"
  say "  export PATH=\"\$HOME/.local/bin:\$PATH\""
elif ! command -v wpm >/dev/null 2>&1; then
  say "Add $install_dir to PATH, then run: wpm"
else
  say "Run: wpm"
fi
