#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
if [ "$(uname -s)" != Darwin ]; then
    echo 'Run this script on macOS.' >&2
    exit 1
fi
binary="${1:-target/release/typeaside}"
if [ "$#" -eq 0 ]; then
    cargo build --locked --release
fi
bundle="target/release/TypeAside.app"
rm -rf "$bundle"
mkdir -p "$bundle/Contents/MacOS"
cp "$binary" "$bundle/Contents/MacOS/typeaside"
cp packaging/macos/Info.plist "$bundle/Contents/Info.plist"
version=$(cargo metadata --no-deps --format-version 1 | python3 -c 'import json,sys; print(next(p["version"] for p in json.load(sys.stdin)["packages"] if p["name"] == "typeaside"))')
/usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $version" "$bundle/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Add :CFBundleVersion string $version" "$bundle/Contents/Info.plist"
/usr/libexec/PlistBuddy -c 'Add :LSMinimumSystemVersion string 13.0' "$bundle/Contents/Info.plist"
# A stable bundle identity lets TCC associate permissions with the app.
codesign --force --deep --sign "${TYPEASIDE_SIGNING_IDENTITY:--}" "$bundle"
printf '%s\n' "$bundle"
