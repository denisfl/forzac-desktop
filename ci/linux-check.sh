#!/bin/bash
# Build the GTK shell and run it under a headless Weston: startup time, a PNG of the first page, page-turn frames,
# CPU while idle. Software rendering in a container says nothing about real speed; it checks that things work.
# On a real Linux desktop, use scripts/measure.sh instead.
set -euo pipefail
book=${1:?usage: ci/linux-check.sh book.fb2.zip}
cargo build --release -q -p forzac-gtk
export XDG_RUNTIME_DIR=$(mktemp -d) WAYLAND_DISPLAY=wayland-1
weston --backend=headless --renderer=${WESTON_RENDERER:-gl} --socket=$WAYLAND_DISPLAY --width=1600 --height=1200 --scale=${SCALE:-1} >/tmp/weston.log 2>&1 &
sleep 1
bin=$CARGO_TARGET_DIR/release/forzac
for i in 1 2 3; do "$bin" "$book" --measure; done
"$bin" "$book" --png /src/target-png/page.png
"$bin" "$book" --turns 10
"$bin" "$book" & pid=$!
sleep 3
a=$(awk '{print $14+$15}' /proc/$pid/stat); sleep 10; b=$(awk '{print $14+$15}' /proc/$pid/stat)
echo "idle: $((b - a)) CPU ticks in 10 s (100 ticks = 1 s of CPU), $(grep VmRSS /proc/$pid/status)"
kill $pid
