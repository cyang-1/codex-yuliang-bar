#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DESKTOP_DIR="$ROOT_DIR/apps/desktop-tauri"
TARGET="x86_64-pc-windows-msvc"

missing=()
tool_bins=()

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    missing+=("$1")
  fi
}

resolve_tool() {
  local tool="$1"
  local formula="$2"
  local found=""

  if found="$(command -v "$tool" 2>/dev/null)"; then
    tool_bins+=("$(dirname "$found")")
    return 0
  fi

  if command -v brew >/dev/null 2>&1; then
    local prefix
    prefix="$(brew --prefix "$formula" 2>/dev/null || true)"
    if [[ -n "$prefix" && -x "$prefix/bin/$tool" ]]; then
      tool_bins+=("$prefix/bin")
      return 0
    fi
  fi

  for prefix in /opt/homebrew/opt/"$formula" /usr/local/opt/"$formula"; do
    if [[ -x "$prefix/bin/$tool" ]]; then
      tool_bins+=("$prefix/bin")
      return 0
    fi
  done

  missing+=("$tool")
}

require_command pnpm
require_command cargo-xwin
resolve_tool llvm-lib llvm
resolve_tool lld-link lld

if [[ ${#missing[@]} -gt 0 ]]; then
  echo "Missing tools for macOS -> Windows cross build:" >&2
  printf '  - %s\n' "${missing[@]}" >&2
  echo >&2
  echo "Install the expected toolchain:" >&2
  echo "  cargo install cargo-xwin" >&2
  echo "  brew install llvm lld" >&2
  exit 1
fi

if ! rustup target list --installed | grep -qx "$TARGET"; then
  echo "Installing Rust target $TARGET..."
  rustup target add "$TARGET"
fi

for bin_dir in "${tool_bins[@]}"; do
  export PATH="$bin_dir:$PATH"
done

pnpm --dir "$DESKTOP_DIR" exec tauri build \
  --runner cargo-xwin \
  --target "$TARGET" \
  --no-bundle

echo
echo "Built Windows app:"
echo "  $ROOT_DIR/target/$TARGET/release/codexbar-desktop-tauri.exe"
