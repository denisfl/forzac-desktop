#!/bin/bash
# Builds the macOS shell: the core as a static library, then the Swift app against its C ABI (forzac.h).
#   macos/build.sh && macos/build/Forzac book.fb2.zip
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
cargo build --release -p forzac-core --manifest-path "$root/Cargo.toml" --target aarch64-apple-darwin
mkdir -p "$root/macos/build"
swiftc -O -import-objc-header "$root/macos/forzac.h" "$root/macos/main.swift" \
  -L "$root/target/aarch64-apple-darwin/release" -lforzac_core \
  -framework AppKit -framework CoreText -framework QuartzCore \
  -o "$root/macos/build/Forzac"
