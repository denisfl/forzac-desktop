#!/bin/bash
# Builds the macOS shell: the core as a static library, then the Swift app against its C ABI (forzac.h),
# as macos/build/Forzac and as macos/build/Forzac.app.
#   macos/build.sh && macos/build/Forzac book.fb2.zip
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
cargo build --release -p forzac-core --manifest-path "$root/Cargo.toml" --target aarch64-apple-darwin
mkdir -p "$root/macos/build"
swiftc -O -import-objc-header "$root/macos/forzac.h" "$root/macos/main.swift" \
  -L "$root/target/aarch64-apple-darwin/release" -lforzac_core \
  -framework AppKit -framework CoreText -framework QuartzCore \
  -o "$root/macos/build/Forzac"
# The same binary as an app bundle (unsigned until the macos-app change adds signing).
app="$root/macos/build/Forzac.app/Contents"
rm -rf "$root/macos/build/Forzac.app"
install -d "$app/MacOS"
cp "$root/macos/build/Forzac" "$app/MacOS/Forzac"
cp "$root/macos/Info.plist" "$app/Info.plist"
