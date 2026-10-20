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
# Sparkle installs updates (macos-auto-update): its release, fetched once and checked, gives the framework to link and
# embed and bin/sign_update for scripts/appcast.sh.
sparkle=2.10.0
sparkle_sha256=c2bf58aa8387266ac179357b1415d6f2635f044da8be41042af32425dae6da0c
if [ "$(cat "$root/macos/build/sparkle/version" 2>/dev/null)" != $sparkle ]; then
  archive="$root/macos/build/Sparkle-$sparkle.tar.xz"
  curl -fsSL -o "$archive" "https://github.com/sparkle-project/Sparkle/releases/download/$sparkle/Sparkle-$sparkle.tar.xz"
  echo "$sparkle_sha256  $archive" | shasum -a 256 -c --quiet
  rm -rf "$root/macos/build/sparkle"
  mkdir "$root/macos/build/sparkle"
  tar -xJf "$archive" -C "$root/macos/build/sparkle"
  rm "$archive"
  echo $sparkle > "$root/macos/build/sparkle/version"
fi
# The bundle finds the framework in Contents/Frameworks, the bare macos/build/Forzac next to itself.
ln -sfn sparkle/Sparkle.framework "$root/macos/build/Sparkle.framework"
swiftc -O -target arm64-apple-macos$MACOSX_DEPLOYMENT_TARGET -import-objc-header "$root/macos/forzac.h" "$root"/macos/*.swift \
  -L "$root/target/aarch64-apple-darwin/release" -lforzac_core \
  -F "$root/macos/build/sparkle" -framework Sparkle \
  -Xlinker -rpath -Xlinker @executable_path/../Frameworks -Xlinker -rpath -Xlinker @executable_path \
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
# Sparkle as its release signed it (ad hoc; its XPC services stay unused: Forzac is not sandboxed).
install -d "$app/Frameworks"
ditto "$root/macos/build/sparkle/Sparkle.framework" "$app/Frameworks/Sparkle.framework"
cp "$root/macos/build/sparkle/LICENSE" "$app/Resources/licenses/Sparkle-LICENSE"
# Signed ad hoc as a whole bundle (Apple Silicon runs nothing unsigned; sealing the resources keeps Gatekeeper's check
# of a download to the first-open step). No Developer ID: see docs/macos-first-open.md.
codesign --force --sign - --identifier app.truenrth.forzac "$root/macos/build/Forzac.app"
