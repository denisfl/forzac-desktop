#!/bin/bash
# Measures the GTK shell on a real Linux desktop (Omarchy, Raspberry Pi 5):
#   scripts/measure.sh ~/Books/book.fb2.zip
# Prints: time to the first presented page (3 runs), frame intervals of 20 animated page turns at the display's
# refresh rate, CPU and memory while idle. Try renderers on the Pi: GSK_RENDERER=ngl or GSK_RENDERER=vulkan.
set -euo pipefail
book=${1:?usage: scripts/measure.sh book.fb2.zip}
cd "$(dirname "$0")/.."
cargo build --release -q -p forzac-gtk
bin=target/release/forzac
echo "renderer: ${GSK_RENDERER:-default}, $(uname -m), $(pkg-config --modversion gtk4 2>/dev/null || echo 'gtk4 ?')"
for i in 1 2 3; do "$bin" "$book" --measure; done
"$bin" "$book" --chapter "${CHAPTER:-5}" --turns 20
"$bin" "$book" --chapter "${CHAPTER:-5}" & pid=$!
sleep 3
a=$(awk '{print $14+$15}' /proc/$pid/stat); sleep 10; b=$(awk '{print $14+$15}' /proc/$pid/stat)
echo "idle: $((b - a)) CPU ticks in 10 s (100 ticks = 1 s of CPU), $(grep VmRSS /proc/$pid/status | tr -s ' ')"
kill $pid
