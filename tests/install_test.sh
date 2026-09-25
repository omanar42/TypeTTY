#!/bin/sh
set -eu

project_dir="$(CDPATH='' cd -- "$(dirname "$0")/.." && pwd)"
test_root="$(mktemp -d "${TMPDIR:-/tmp}/typetty-install-test.XXXXXX")"
trap 'rm -rf "$test_root"' EXIT HUP INT TERM

target="aarch64-apple-darwin"
release_dir="$test_root/releases/download/v0.1.0"
payload_dir="$test_root/payload"
mkdir -p "$release_dir" "$payload_dir" "$test_root/home"

printf '#!/bin/sh\nprintf "wpm test binary\\n"\n' >"$payload_dir/wpm"
chmod 755 "$payload_dir/wpm"
archive="$release_dir/wpm-$target.tar.gz"
tar -czf "$archive" -C "$payload_dir" wpm
if command -v sha256sum >/dev/null 2>&1; then
  checksum="$(sha256sum "$archive" | awk '{print $1}')"
else
  checksum="$(shasum -a 256 "$archive" | awk '{print $1}')"
fi
printf '%s  %s\n' "$checksum" "$(basename "$archive")" >"$archive.sha256"

HOME="$test_root/home" \
SHELL="/bin/zsh" \
PATH="/usr/bin:/bin" \
WPM_OS="Darwin" \
WPM_ARCH="arm64" \
WPM_VERSION="v0.1.0" \
WPM_RELEASE_BASE_URL="file://$test_root/releases" \
  sh "$project_dir/install.sh" >/dev/null

test -x "$test_root/home/.local/bin/wpm"
test "$("$test_root/home/.local/bin/wpm")" = "wpm test binary"
grep -F "export PATH=\"\$HOME/.local/bin:\$PATH\"" "$test_root/home/.zshrc" >/dev/null

# A second run must not duplicate the shell configuration entry.
HOME="$test_root/home" \
SHELL="/bin/zsh" \
PATH="/usr/bin:/bin" \
WPM_OS="Darwin" \
WPM_ARCH="arm64" \
WPM_VERSION="v0.1.0" \
WPM_RELEASE_BASE_URL="file://$test_root/releases" \
  sh "$project_dir/install.sh" >/dev/null

test "$(grep -c '# TypeTTY' "$test_root/home/.zshrc")" -eq 1
printf 'installer tests passed\n'
