#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
: "${VERSION:?}" "${CLOUDFLARE_API_TOKEN:?}" "${CLOUDFLARE_ACCOUNT_ID:?}"
for file in TypeAside.dmg TypeAside-setup.exe TypeAside.AppImage typeaside.deb typeaside.rpm; do
    test -s "dist/$file"
    minisign -V -p packaging/minisign.pub -m "dist/$file" -x "dist/$file.minisig"
done
for file in TypeAside.dmg TypeAside-setup.exe TypeAside.AppImage typeaside.deb typeaside.rpm; do
    case "$file" in
        *.dmg) content_type=application/x-apple-diskimage ;;
        *.exe) content_type=application/vnd.microsoft.portable-executable ;;
        *.AppImage) content_type=application/vnd.appimage ;;
        *.deb) content_type=application/vnd.debian.binary-package ;;
        *.rpm) content_type=application/x-rpm ;;
    esac
    for key in "v$VERSION/$file" "$file"; do
        bunx wrangler@4 r2 object put "typeaside-releases/$key" --file "dist/$file" --content-type "$content_type" --remote
        bunx wrangler@4 r2 object put "typeaside-releases/$key.minisig" --file "dist/$file.minisig" --content-type text/plain --remote
    done
done
python3 - <<'MANIFEST'
import hashlib
import json
import os
from pathlib import Path

files = ["TypeAside.dmg", "TypeAside-setup.exe", "TypeAside.AppImage", "typeaside.deb", "typeaside.rpm"]
checksums = {}
for name in files:
    with Path("dist", name).open("rb") as installer:
        checksums[name] = hashlib.file_digest(installer, "sha256").hexdigest()
manifest = {"version": os.environ["VERSION"], "files": checksums}
Path("dist/latest.json").write_text(json.dumps(manifest) + "\n")
MANIFEST
packaging/sign.sh dist/latest.json
bunx wrangler@4 r2 object put typeaside-releases/latest.json.minisig --file dist/latest.json.minisig --content-type text/plain --remote
bunx wrangler@4 r2 object put typeaside-releases/latest.json --file dist/latest.json --content-type application/json --remote
