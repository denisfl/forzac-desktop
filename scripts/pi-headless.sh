#!/bin/bash
# Measures the GTK shell on a Raspberry Pi (or any wlroots desktop) without a monitor: starts a second labwc with a
# headless output rendered on the GPU (GLES), then runs the usual measurements for each GTK renderer and takes
# screenshots at fractional scales. The user's own session is not touched.
#   scripts/pi-headless.sh ./forzac book.fb2.zip [out-dir]
set -euo pipefail
bin=$(realpath "${1:?usage: pi-headless.sh forzac book.fb2.zip [out-dir]}")
book=$(realpath "${2:?book}")
out=${3:-$PWD/pi-out}
mkdir -p "$out"
export XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR:-/run/user/$(id -u)}
cfg=$(mktemp -d)   # empty labwc config: no panel, no autostart
WLR_BACKENDS=headless WLR_RENDERER=gles2 WLR_HEADLESS_OUTPUTS=1 WLR_LIBINPUT_NO_DEVICES=1 \
  WAYLAND_DISPLAY= labwc -C "$cfg" >"$out/labwc.log" 2>&1 &
labwc_pid=$!
trap 'kill $labwc_pid 2>/dev/null; rm -rf "$cfg"' EXIT
sleep 2
# The socket created after the config directory: a stale socket name of an earlier compositor may be reused.
socket=$(basename "$(find "$XDG_RUNTIME_DIR" -maxdepth 1 -type s -name 'wayland-*' -newer "$cfg" | head -1)")
export WAYLAND_DISPLAY=$socket
output=$(wlr-randr | head -1 | awk '{print $1}')
echo "compositor: labwc headless on $socket, output $output, $(uname -m), GTK $(dpkg-query -W -f='${Version}' libgtk-4-1 2>/dev/null)"

wlr-randr --output "$output" --custom-mode 1920x1200@120Hz --scale 1
for renderer in default ngl vulkan gl; do
  echo "== GSK_RENDERER=$renderer"
  env_r=(); [ "$renderer" != default ] && env_r=(GSK_RENDERER=$renderer)
  for i in 1 2 3; do env "${env_r[@]}" "$bin" "$book" --measure 2>&1 | grep -E '^startup|rror' || true; done
  env "${env_r[@]}" "$bin" "$book" --chapter "${CHAPTER:-5}" --turns 20 2>&1 | grep -E '^(turns|pages)|rror' || true
done

echo "== idle (default renderer)"
"$bin" "$book" --chapter "${CHAPTER:-5}" >/dev/null 2>&1 & app=$!
sleep 4
a=$(awk '{print $14+$15}' /proc/$app/stat); sleep 10; b=$(awk '{print $14+$15}' /proc/$app/stat)
echo "idle: $((b - a)) CPU ticks in 10 s (100 ticks = 1 s of CPU), $(grep VmRSS /proc/$app/status | tr -s ' ')"
for scale in 1 1.25 1.6 2; do
  wlr-randr --output "$output" --scale "$scale"
  sleep 1.5
  grim "$out/pi-scale-$scale.png"
done
echo "screenshots: $out/pi-scale-*.png"
kill $app
