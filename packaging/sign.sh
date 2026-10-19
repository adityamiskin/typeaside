#!/bin/bash
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
key=${TYPEASIDE_SIGNING_KEY_FILE:-$HOME/.config/typeaside-release/minisign.key}
if [[ -n ${TYPEASIDE_SIGNING_KEY:-} ]]; then
    key=$(mktemp)
    trap 'rm -f "$key"' EXIT
    chmod 600 "$key"
    printf '%s\n' "$TYPEASIDE_SIGNING_KEY" > "$key"
fi
if [[ $# -eq 0 ]]; then
    echo "Usage: packaging/sign.sh artifact..." >&2
    exit 1
fi
for artifact in "$@"; do
    minisign -S -s "$key" -m "$artifact" -x "$artifact.minisig"
    minisign -V -p "$root/packaging/minisign.pub" -m "$artifact" -x "$artifact.minisig"
done
