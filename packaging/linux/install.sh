#!/usr/bin/env bash
# Installs Downlink for the current user: the binary, its launcher entry and icons. No sudo; the
# udev rule is a separate step (see README). Usage: packaging/linux/install.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BIN_DIR="${XDG_BIN_HOME:-$HOME/.local/bin}"
DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}"
EXE="$BIN_DIR/downlink"

cargo build --release -p downlink --manifest-path "$ROOT/Cargo.toml"
install -Dm755 "$ROOT/target/release/downlink" "$EXE"

ICONS="$(mktemp -d)"
trap 'rm -rf "$ICONS"' EXIT
cargo run --release -p downlink --bin make-icons --manifest-path "$ROOT/Cargo.toml" -- "$ICONS"
for size in 16 32 128 256 512; do
  install -Dm644 "$ICONS/icon_${size}x${size}.png" \
    "$DATA_DIR/icons/hicolor/${size}x${size}/apps/downlink.png"
done

# Starting from the launcher shows the Layout window. On Omarchy, omarchy-launch-or-focus focuses
# an open Downlink window instead; the pattern matches the window class exactly, not any window
# whose title mentions downlink. The Exec value escapes `$` as the Desktop Entry spec requires.
if command -v omarchy-launch-or-focus >/dev/null; then
  EXEC_LINE=$(printf 'omarchy-launch-or-focus "^downlink\\\\$" "%s --show-layout"' "$EXE")
else
  EXEC_LINE="$EXE --show-layout"
fi
mkdir -p "$DATA_DIR/applications"
while IFS= read -r line; do
  if [[ $line == "Exec=__EXEC__" ]]; then
    printf 'Exec=%s\n' "$EXEC_LINE"
  else
    printf '%s\n' "$line"
  fi
done < "$ROOT/packaging/linux/downlink.desktop" > "$DATA_DIR/applications/downlink.desktop"
chmod 644 "$DATA_DIR/applications/downlink.desktop"

echo "Installed $EXE"
