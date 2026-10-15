#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
if [ "$(uname -s)" != Darwin ]; then
    echo 'Run this script on macOS.' >&2
    exit 1
fi
cargo build --release
bundle="target/release/Hermes.app"
mkdir -p "$bundle/Contents/MacOS"
cp target/release/hermes "$bundle/Contents/MacOS/hermes"
cp packaging/macos/Info.plist "$bundle/Contents/Info.plist"
# A stable bundle identity lets TCC associate permissions with the app.
codesign --force --deep --sign "${HERMES_SIGNING_IDENTITY:--}" "$bundle"
printf '%s\n' "$bundle"
