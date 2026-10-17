#!/bin/bash
# Pins the released .dmg checksum into the cask and pushes it to the tap.
set -euo pipefail
: "${VERSION:?}"
here="$(cd "$(dirname "$0")" && pwd)"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
git clone -q git@github.com:adityamiskin/homebrew-tap.git "$work/tap"
dmg="${DMG:-$work/TypeAside.dmg}"
[[ -n "${DMG:-}" ]] || curl --fail --location --retry 3 -o "$dmg" "https://typeaside.app/releases/v$VERSION/TypeAside.dmg"
sum=$(shasum -a 256 "$dmg" | cut -d' ' -f1)
mkdir -p "$work/tap/Casks"
sed -e "s/^  version .*/  version \"$VERSION\"/" -e "s/^  sha256 .*/  sha256 \"$sum\"/" \
    "$here/typeaside.rb" > "$work/tap/Casks/typeaside.rb"
cd "$work/tap"
git add Casks/typeaside.rb
git diff --cached --quiet && { echo "typeaside already at $VERSION"; exit 0; }
git -c user.name="Aditya Miskin" -c user.email="adityamiskin98@gmail.com" commit -qm "typeaside $VERSION"
git push -q origin HEAD
