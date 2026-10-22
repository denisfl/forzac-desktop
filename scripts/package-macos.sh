#!/bin/bash
# Wraps the built app (macos/build.sh) as the download for forzac.app: a disk image with Forzac.app and a link
# to /Applications, and its SHA-256 checksum in the format `shasum -a 256 -c` reads.
#   macos/build.sh && scripts/package-macos.sh 0.2.0
#     -> ./Forzac-0.2.0-macos-arm64.dmg, ./Forzac-0.2.0-macos-arm64.dmg.sha256
set -euo pipefail
version=${1:?usage: scripts/package-macos.sh <version>}
root=$(cd "$(dirname "$0")/.." && pwd)
name=Forzac-$version-macos-arm64.dmg
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
ditto "$root/macos/build/Forzac.app" "$stage/Forzac.app"
# The committed Info.plist keeps the development version; the release's goes into the copy, which is signed again
# (CFBundleVersion is what Sparkle compares).
/usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $version" -c "Set :CFBundleVersion $version" \
  "$stage/Forzac.app/Contents/Info.plist"
codesign --force --sign - --identifier app.truenrth.forzac "$stage/Forzac.app"
codesign --verify --deep --strict "$stage/Forzac.app"
ln -s /Applications "$stage/Applications"
rm -f "$name" "$name.sha256"
hdiutil create -quiet -volname Forzac -srcfolder "$stage" -fs HFS+ -format UDZO -o "$name"
shasum -a 256 "$name" > "$name.sha256"
echo "$PWD/$name"
