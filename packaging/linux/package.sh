#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/../.."
version=$(cargo metadata --no-deps --format-version 1 | python3 -c 'import json,sys; print(next(p["version"] for p in json.load(sys.stdin)["packages"] if p["name"] == "hermes"))')
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
mkdir -p dist "$stage/deb/DEBIAN" "$stage/deb/usr/bin" "$stage/deb/usr/share/applications" "$stage/deb/usr/share/icons/hicolor/scalable/apps"
install -m755 target/release/hermes "$stage/deb/usr/bin/hermes"
install -m644 packaging/hermes.desktop "$stage/deb/usr/share/applications/hermes.desktop"
install -m644 assets/hermes.svg "$stage/deb/usr/share/icons/hicolor/scalable/apps/hermes.svg"
cat > "$stage/deb/DEBIAN/control" <<CONTROL
Package: hermes
Version: $version
Architecture: amd64
Maintainer: Aditya Miskin <adityamiskin@users.noreply.github.com>
Section: utils
Priority: optional
Depends: libasound2, libudev1, libwayland-client0, libwayland-cursor0, libwayland-egl1, libxkbcommon0, libxkbcommon-x11-0, libfontconfig1, libfreetype6, libxcb1, libxcb-xkb1, libxcb-xfixes0, libxcb-randr0, libx11-xcb1, libvulkan1, mesa-vulkan-drivers, libstdc++6, libc6 (>= 2.35)
Description: Native voice dictation for Wayland
CONTROL
dpkg-deb --root-owner-group --build "$stage/deb" dist/hermes.deb
curl --fail --location --retry 3 https://github.com/linuxdeploy/linuxdeploy/releases/download/1-alpha-20251107-1/linuxdeploy-x86_64.AppImage -o "$stage/linuxdeploy.AppImage"
printf '%s  %s\n' c20cd71e3a4e3b80c3483cef793cda3f4e990aca14014d23c544ca3ce1270b4d "$stage/linuxdeploy.AppImage" | sha256sum --check
chmod +x "$stage/linuxdeploy.AppImage"
APPIMAGE_EXTRACT_AND_RUN=1 OUTPUT="$PWD/dist/Hermes.AppImage" "$stage/linuxdeploy.AppImage" \
    --appdir "$stage/Hermes.AppDir" \
    --executable target/release/hermes \
    --desktop-file packaging/hermes.desktop \
    --icon-file assets/hermes.svg \
    --output appimage
