#!/bin/sh
set -eu
cd "$(dirname "$0")/../.."
mkdir -p dist
lipo target/release/typeaside -verify_arch arm64
scripts/bundle-macos.sh target/release/typeaside
codesign --verify --deep --strict target/release/TypeAside.app
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
ditto target/release/TypeAside.app "$stage/TypeAside.app"
ln -s /Applications "$stage/Applications"
hdiutil create -volname TypeAside -srcfolder "$stage" -ov -format UDZO dist/TypeAside.dmg
packaging/sign.sh dist/TypeAside.dmg
