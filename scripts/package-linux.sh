#!/bin/bash
# Lays out a release build as an archive that runs where it is unpacked:
#   bin/forzac, share/forzac/fonts, share/applications, share/mime/packages, share/licenses/forzac
#   scripts/package-linux.sh forzac-0.2.0-linux-x86_64   (after cargo build --release -p forzac-gtk)
set -euo pipefail
name=${1:?usage: scripts/package-linux.sh <archive-name>}
root=$(cd "$(dirname "$0")/.." && pwd)
target=${CARGO_TARGET_DIR:-$root/target}
stage=$(mktemp -d)/$name
install -Dm755 "$target/release/forzac" "$stage/bin/forzac"
mkdir -p "$stage/share/forzac" && cp -r "$root/assets/fonts" "$stage/share/forzac/"
install -Dm644 "$root/data/app.truenrth.forzac.desktop" -t "$stage/share/applications/"
install -Dm644 "$root/data/forzac-mime.xml" "$stage/share/mime/packages/forzac.xml"
install -Dm644 "$root/LICENSE" "$root/assets/hyph/hyph-ru.lic.txt" "$root/assets/hyph/hyph-en-us.lic.txt" -t "$stage/share/licenses/forzac/"
for f in pt-serif pt-sans literata source-serif; do
    install -Dm644 "$root/assets/fonts/$f/OFL.txt" "$stage/share/licenses/forzac/$f-OFL.txt"
done
tar -C "$(dirname "$stage")" -czf "$PWD/$name.tar.gz" "$name"
echo "$PWD/$name.tar.gz"
