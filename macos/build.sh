#!/bin/bash
# Builds the macOS shell: the core as a static library, then the Swift app against its C ABI (forzac.h),
# as macos/build/Forzac and as macos/build/Forzac.app.
#   macos/build.sh && macos/build/Forzac book.fb2.zip
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
# The oldest macOS the app runs on (Info.plist LSMinimumSystemVersion), whatever SDK builds it.
export MACOSX_DEPLOYMENT_TARGET=14.0
cargo build --release -p forzac-core --manifest-path "$root/Cargo.toml" --target aarch64-apple-darwin
mkdir -p "$root/macos/build"
swiftc -O -target arm64-apple-macos$MACOSX_DEPLOYMENT_TARGET -import-objc-header "$root/macos/forzac.h" "$root"/macos/*.swift \
  -L "$root/target/aarch64-apple-darwin/release" -lforzac_core \
  -framework AppKit -framework CoreText -framework QuartzCore \
  -o "$root/macos/build/Forzac"
# The same binary as an app bundle, with the icon (scripts/icons.py).
app="$root/macos/build/Forzac.app/Contents"
rm -rf "$root/macos/build/Forzac.app"
install -d "$app/MacOS"
cp "$root/macos/build/Forzac" "$app/MacOS/Forzac"
cp "$root/macos/Info.plist" "$app/Info.plist"
install -d "$app/Resources"
cp "$root/macos/Forzac.icns" "$app/Resources/"
# The fonts read from files (the other book families, the interface font) and the licences of everything bundled.
install -d "$app/Resources/licenses"
cp -r "$root/assets/fonts" "$app/Resources/"
cp "$root/LICENSE" "$root/assets/hyph/hyph-ru.lic.txt" "$root/assets/hyph/hyph-en-us.lic.txt" "$app/Resources/licenses/"
# Signed ad hoc as a whole bundle (Apple Silicon runs nothing unsigned; sealing the resources keeps Gatekeeper's check
# of a download to the first-open step). No Developer ID: see docs/macos-first-open.md.
codesign --force --sign - --identifier app.truenrth.forzac "$root/macos/build/Forzac.app"
