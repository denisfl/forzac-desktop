#!/bin/bash
# Wraps a release build as a Debian package for Raspberry Pi OS: the archive layout (scripts/package-linux.sh) under
# /usr. Build it in Debian 13 (ci/debian.Dockerfile) so the binary links that release's GTK and glibc.
#   scripts/package-deb.sh 0.2.0      (after cargo build --release -p forzac-gtk) -> ./forzac_0.2.0_arm64.deb
# The MIME, desktop and icon caches are refreshed by dpkg triggers; postinst only prints the default-reader hint.
set -euo pipefail
version=${1:?usage: scripts/package-deb.sh <version>}
arch=${ARCH:-$(dpkg --print-architecture)}
root=$(cd "$(dirname "$0")/.." && pwd)
work=$(mktemp -d)
tar -C "$work" -xzf "$(cd "$work" && "$root/scripts/package-linux.sh" "forzac-$version" | tail -1)"
pkg=$work/pkg
mkdir -p "$pkg/usr" "$pkg/DEBIAN"
cp -r "$work/forzac-$version"/{bin,share} "$pkg/usr/"
# apt updates it: the app does not check GitHub for a newer release.
: > "$pkg/usr/share/forzac/packaged"
# Debian keeps licences in the copyright file and /usr/share/doc, not /usr/share/licenses.
mkdir -p "$pkg/usr/share/doc/forzac"
mv "$pkg/usr/share/licenses/forzac" "$pkg/usr/share/doc/forzac/licenses"
rmdir "$pkg/usr/share/licenses"
cat > "$pkg/usr/share/doc/forzac/copyright" <<EOF
Forzac, https://forzac.truenrth.app

The code: Mozilla Public License 2.0 (licenses/LICENSE).
The bundled fonts in /usr/share/forzac/fonts: SIL Open Font License 1.1 (licenses/*-OFL.txt).
The hyphenation patterns built into the binary: Russian LPPL 1.2 or later, English free use with the notice kept
(licenses/hyph-*.lic.txt).
EOF
cat > "$pkg/DEBIAN/control" <<EOF
Package: forzac
Version: $version
Architecture: $arch
Maintainer: denisfl
Installed-Size: $(du -sk "$pkg/usr" | cut -f1)
Depends: libgtk-4-1 (>= 4.14), libfontconfig1, libfile-mimeinfo-perl
Section: text
Priority: optional
Homepage: https://forzac.truenrth.app
Description: Offline FB2 and EPUB reader with book-quality typesetting
 Reads FB2, FB3 and EPUB books with hyphenation, justification and no lone
 lines at page edges. Keyboard first, few settings.
EOF
# Printed after the first installation only, not on upgrades.
cat > "$pkg/DEBIAN/postinst" <<'EOF'
#!/bin/sh
set -e
if [ "$1" = configure ] && [ -z "$2" ]; then
  echo "To open FB2, FB3 and EPUB books with Forzac (xdg-open, file managers), run as your user:"
  echo "  xdg-mime default app.truenrth.forzac.desktop application/x-fictionbook+xml application/x-zip-compressed-fb2 application/x-fb3 application/epub+zip"
fi
EOF
chmod 755 "$pkg/DEBIAN/postinst"
dpkg-deb --root-owner-group --build "$pkg" "$PWD/forzac_${version}_$arch.deb" >/dev/null
rm -rf "$work"
echo "$PWD/forzac_${version}_$arch.deb"
