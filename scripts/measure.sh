#!/bin/bash
# Measures the GTK shell on a real Linux desktop (Omarchy, Raspberry Pi 5), or the AppKit shell on a Mac:
#   scripts/measure.sh ~/Books/book.fb2.zip
# Prints: time to the first presented page (3 runs; the 2nd and 3rd are warm), frame intervals of 20 animated page
# turns at the display's refresh rate, CPU and memory while idle — the budgets of `openspec/specs/performance`. Try
# renderers on the Pi: GSK_RENDERER=ngl or GSK_RENDERER=vulkan.
set -euo pipefail
book=${1:?usage: scripts/measure.sh book.fb2.zip}
cd "$(dirname "$0")/.."
if [ "$(uname)" = Darwin ]; then
  # Display links stop while the display sleeps: keep it awake while measuring. A locked screen gets no display cycles
  # for a while, so the first page waits for them.
  caffeinate -d -u -w $$ & sleep 1
  if ioreg -n Root -d1 | grep '"CGSSessionScreenIsLocked"=Yes' >/dev/null; then
    echo "warning: the screen is locked; the first-page times are not valid, unlock it and run again"
  fi
  macos/build.sh >/dev/null 2>&1 || { echo "macos/build.sh failed"; exit 1; }
  bin=macos/build/Forzac.app/Contents/MacOS/Forzac
  echo "macOS $(sw_vers -productVersion), $(sysctl -n machdep.cpu.brand_string), $(pmset -g batt | grep -o "'.*'" | tr -d "'")"
  for i in 1 2 3; do "$bin" "$book" --measure; done
  "$bin" "$book" --chapter "${CHAPTER:-5}" --turns 20
  "$bin" "$book" --chapter "${CHAPTER:-5}" & pid=$!
  sleep 3
  # `ps` gives the CPU time in hundredths of a second, the ticks of Linux's /proc.
  ticks() { ps -o time= -p $pid | awk -F'[:.]' '{print ($1 * 60 + $2) * 100 + $3}'; }
  a=$(ticks); sleep 10; b=$(ticks)
  echo "idle: $((b - a)) CPU ticks in 10 s (100 ticks = 1 s of CPU), RSS $(ps -o rss= -p $pid | tr -d ' ') kB"
  kill $pid
  exit
fi
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
