#!/bin/sh
set -eu
cd "$(dirname "$0")/../.."
mkdir -p target/release/universal dist
lipo -create target/aarch64-apple-darwin/release/typeaside target/x86_64-apple-darwin/release/typeaside -output target/release/universal/typeaside
lipo target/release/universal/typeaside -verify_arch arm64 x86_64
scripts/bundle-macos.sh target/release/universal/typeaside
codesign --verify --deep --strict target/release/TypeAside.app
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
ditto target/release/TypeAside.app "$stage/TypeAside.app"
ln -s /Applications "$stage/Applications"
hdiutil create -volname TypeAside -srcfolder "$stage" -ov -format UDZO dist/TypeAside.dmg
packaging/sign.sh dist/TypeAside.dmg
