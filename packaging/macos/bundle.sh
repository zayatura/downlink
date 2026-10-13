#!/usr/bin/env bash
# Builds target/bundle/Downlink.app (ad-hoc signed). Usage: packaging/macos/bundle.sh [output-dir]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT="${1:-$ROOT/target/bundle}"
APP="$OUT/Downlink.app"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -n 1)"

cargo build --release -p downlink --manifest-path "$ROOT/Cargo.toml"

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$ROOT/target/release/downlink" "$APP/Contents/MacOS/downlink"
sed "s/__VERSION__/$VERSION/g" "$ROOT/packaging/macos/Info.plist" > "$APP/Contents/Info.plist"

ICONSET="$(mktemp -d)/Downlink.iconset"
cargo run --release -p downlink --bin make-icons --manifest-path "$ROOT/Cargo.toml" -- "$ICONSET"
iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/Downlink.icns"

codesign --force --deep --sign - "$APP"
echo "Built $APP (version $VERSION)"
