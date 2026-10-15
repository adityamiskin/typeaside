#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
: "${VERSION:?}" "${CLOUDFLARE_API_TOKEN:?}" "${CLOUDFLARE_ACCOUNT_ID:?}"
for file in Hermes.dmg Hermes-setup.exe Hermes.AppImage hermes.deb; do
    test -s "dist/$file"
done
for file in Hermes.dmg Hermes-setup.exe Hermes.AppImage hermes.deb; do
    case "$file" in
        *.dmg) content_type=application/x-apple-diskimage ;;
        *.exe) content_type=application/vnd.microsoft.portable-executable ;;
        *.AppImage) content_type=application/vnd.appimage ;;
        *.deb) content_type=application/vnd.debian.binary-package ;;
    esac
    for key in "v$VERSION/$file" "$file"; do
        bunx wrangler@4 r2 object put "hermes-releases/$key" --file "dist/$file" --content-type "$content_type" --remote
    done
done
