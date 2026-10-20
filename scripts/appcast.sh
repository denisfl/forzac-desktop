#!/bin/bash
# Writes the appcast the macOS app installs updates from (macos-auto-update): one item for this version, whose
# enclosure is the release's disk image with its length and EdDSA signature. The private key comes from
# SPARKLE_ED_PRIVATE_KEY (CI's secret) or else from the login Keychain (account "forzac", see docs/publishing.md).
# Needs Sparkle's sign_update, which macos/build.sh unpacks.
#   scripts/appcast.sh 0.11.0 Forzac-0.11.0-macos-arm64.dmg > appcast.xml
#   scripts/appcast.sh 0.11.0 image.dmg http://localhost:8000/image.dmg > appcast.xml    # a test feed
set -euo pipefail
version=${1:?usage: scripts/appcast.sh <version> <dmg> [url]}
dmg=${2:?usage: scripts/appcast.sh <version> <dmg> [url]}
url=${3:-https://github.com/denisfl/forzac-desktop/releases/download/v$version/Forzac-macos-arm64.dmg}
root=$(cd "$(dirname "$0")/.." && pwd)
sign=$root/macos/build/sparkle/bin/sign_update
# Prints `sparkle:edSignature="…" length="…"`.
if [ -n "${SPARKLE_ED_PRIVATE_KEY:-}" ]; then
  signature=$(printf '%s' "$SPARKLE_ED_PRIVATE_KEY" | "$sign" --ed-key-file - "$dmg")
else
  signature=$("$sign" --account forzac "$dmg")
fi
cat <<EOF
<?xml version="1.0" encoding="utf-8"?>
<rss version="2.0" xmlns:sparkle="http://www.andymatuschak.org/xml-namespaces/sparkle">
  <channel>
    <title>Forzac</title>
    <item>
      <title>Forzac $version</title>
      <pubDate>$(LC_ALL=C date -u '+%a, %d %b %Y %H:%M:%S +0000')</pubDate>
      <sparkle:version>$version</sparkle:version>
      <sparkle:shortVersionString>$version</sparkle:shortVersionString>
      <sparkle:minimumSystemVersion>14.0</sparkle:minimumSystemVersion>
      <enclosure url="$url" type="application/octet-stream" $signature/>
    </item>
  </channel>
</rss>
EOF
