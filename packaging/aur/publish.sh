#!/bin/bash
# Pins the released .deb checksum into the PKGBUILD and pushes it to the AUR.
set -euo pipefail
: "${VERSION:?}"
here="$(cd "$(dirname "$0")" && pwd)"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
git clone -q ssh://aur@aur.archlinux.org/typeaside-bin.git "$work/aur"
deb="${DEB:-$work/typeaside.deb}"
[[ -n "${DEB:-}" ]] || curl --fail --location --retry 3 -o "$deb" "https://typeaside.app/releases/v$VERSION/typeaside.deb"
sum=$(sha256sum "$deb" | cut -d' ' -f1)
sed -e "s/^pkgver=.*/pkgver=$VERSION/" -e "s/^pkgrel=.*/pkgrel=1/" -e "s/^sha256sums=.*/sha256sums=('$sum')/" \
    "$here/PKGBUILD" > "$work/aur/PKGBUILD"
cd "$work/aur"
makepkg --printsrcinfo > .SRCINFO
git add PKGBUILD .SRCINFO
git diff --cached --quiet && { echo "typeaside-bin already at $VERSION"; exit 0; }
git -c user.name="Aditya Miskin" -c user.email="adityamiskin98@gmail.com" commit -qm "Update to $VERSION"
git push -q origin HEAD:master
