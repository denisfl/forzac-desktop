#!/bin/bash
# Fetches the pinned PDFium for Linux (pdf-reading) and checks it: bblanchon/pdfium-binaries, one build per release.
#   scripts/pdfium.sh [x86_64|aarch64] -> prints the folder with lib/libpdfium.so and licenses/, in target/pdfium.
# To update: the build here and in packaging/aur/forzac/PKGBUILD, with its checksums; pdfium-render's API feature with it.
set -euo pipefail
build=8125
case "${1:-$(uname -m)}" in
  x86_64 | amd64) arch=x64 sha256=e1c3ce6e270cb5e0c20816b379ad9f2c1309972697c505f508de92bf5e7d6659 ;;
  aarch64 | arm64) arch=arm64 sha256=8b8818f2ec57d1b213d952ca6d7f0ffeca9d0110487d0e20ad83ba918bc92efa ;;
  *) echo "no PDFium for $1" >&2; exit 1 ;;
esac
root=$(cd "$(dirname "$0")/.." && pwd)
dir=${CARGO_TARGET_DIR:-$root/target}/pdfium/$build-$arch
if [ ! -f "$dir/lib/libpdfium.so" ]; then
  mkdir -p "$dir"
  curl -fsSL -o "$dir/pdfium.tgz" "https://github.com/bblanchon/pdfium-binaries/releases/download/chromium/$build/pdfium-linux-$arch.tgz"
  echo "$sha256  $dir/pdfium.tgz" | sha256sum -c --quiet
  tar -xzf "$dir/pdfium.tgz" -C "$dir" lib licenses
  rm "$dir/pdfium.tgz"
  strip "$dir/lib/libpdfium.so" 2>/dev/null || true
fi
echo "$dir"
