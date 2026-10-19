#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/../.."
version=$(cargo metadata --no-deps --format-version 1 | python3 -c 'import json,sys; print(next(p["version"] for p in json.load(sys.stdin)["packages"] if p["name"] == "typeaside"))')
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
mkdir -p dist
curl --fail --location --retry 3 https://github.com/goreleaser/nfpm/releases/download/v2.48.0/nfpm_2.48.0_Linux_x86_64.tar.gz -o "$stage/nfpm.tar.gz"
printf '%s  %s\n' 6d7f9a28b2f397934403950fcbba83f598b08d02edbe79a0815b3b4897341750 "$stage/nfpm.tar.gz" | sha256sum --check
tar -xzf "$stage/nfpm.tar.gz" -C "$stage" nfpm
VERSION=$version "$stage/nfpm" package -f packaging/linux/nfpm.yaml -p deb -t dist/typeaside.deb
VERSION=$version "$stage/nfpm" package -f packaging/linux/nfpm.yaml -p rpm -t dist/typeaside.rpm
curl --fail --location --retry 3 https://github.com/linuxdeploy/linuxdeploy/releases/download/1-alpha-20251107-1/linuxdeploy-x86_64.AppImage -o "$stage/linuxdeploy.AppImage"
printf '%s  %s\n' c20cd71e3a4e3b80c3483cef793cda3f4e990aca14014d23c544ca3ce1270b4d "$stage/linuxdeploy.AppImage" | sha256sum --check
chmod +x "$stage/linuxdeploy.AppImage"
APPIMAGE_EXTRACT_AND_RUN=1 OUTPUT="$PWD/dist/TypeAside.AppImage" "$stage/linuxdeploy.AppImage" \
    --appdir "$stage/TypeAside.AppDir" \
    --executable target/release/typeaside \
    --desktop-file packaging/typeaside.desktop \
    --icon-file assets/typeaside.svg \
    --output appimage
packaging/sign.sh dist/TypeAside.AppImage dist/typeaside.deb dist/typeaside.rpm
