#!/bin/sh
set -eu
cd "$(dirname "$0")/../.."
mkdir -p target/release/universal dist
lipo -create target/aarch64-apple-darwin/release/hermes target/x86_64-apple-darwin/release/hermes -output target/release/universal/hermes
lipo target/release/universal/hermes -verify_arch arm64 x86_64
scripts/bundle-macos.sh target/release/universal/hermes
codesign --verify --deep --strict target/release/Hermes.app
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
ditto target/release/Hermes.app "$stage/Hermes.app"
ln -s /Applications "$stage/Applications"
hdiutil create -volname Hermes -srcfolder "$stage" -ov -format UDZO dist/Hermes.dmg
