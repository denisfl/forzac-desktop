#!/bin/bash
# Inside forzac-arch: build the package from the committed tree with makepkg, install it, check the desktop
# integration, then run the app under a headless sway at fractional scales and take screenshots with grim.
# Hyprland needs a DRM render node even when headless; containers without /dev/dri can only run sway (pixman).
set -euo pipefail
book=${1:?usage: ci/arch-check.sh book.fb2.zip}
out=${OUT:-/src/target-png}
cd ~
git config --global --add safe.directory /src
git -C /src archive --prefix=forzac-desktop-0.1.0/ -o "$HOME/forzac-0.1.0.tar.gz" HEAD
cp /src/data/PKGBUILD .
sed -i 's|^source=.*|source=("forzac-0.1.0.tar.gz")|' PKGBUILD
time makepkg -f --noconfirm >makepkg.log 2>&1 || { tail -30 makepkg.log; exit 1; }
sudo pacman -U --noconfirm ./forzac-0.1.0-1-*.pkg.tar.zst >/dev/null
pacman -Qi forzac | grep -E '^(Name|Version|Installed Size|Depends On)'
pacman -Ql forzac | awk '{print $2}' | grep -vE '/$'

# Desktop integration: the .desktop file validates, MIME types resolve, Forzac becomes the default.
desktop-file-validate /usr/share/applications/app.truenrth.forzac.desktop && echo "desktop file: valid"
mkdir -p ~/.local/share/applications ~/.config
xdg-mime default app.truenrth.forzac.desktop application/epub+zip application/x-fictionbook+xml application/x-zip-compressed-fb2 application/x-fb3
cp "$book" ~/book.fb2.zip
echo "book.fb2.zip is $(xdg-mime query filetype ~/book.fb2.zip), opens with $(xdg-mime query default "$(xdg-mime query filetype ~/book.fb2.zip)")"
printf '<?xml version="1.0"?><FictionBook/>' > ~/x.fb2; cp ~/book.fb2.zip ~/x.fb3
for f in x.fb2 x.fb3; do t=$(xdg-mime query filetype ~/$f); echo "$f is $t, opens with $(xdg-mime query default "$t")"; done

export XDG_RUNTIME_DIR=/tmp/xdg; mkdir -p -m 700 $XDG_RUNTIME_DIR
for scale in 1 1.25 1.6 2; do
  printf 'output HEADLESS-1 mode 1600x1000 scale %s\ndefault_border none\n' "$scale" > ~/sway.conf
  WLR_BACKENDS=headless WLR_RENDERER=pixman WLR_LIBINPUT_NO_DEVICES=1 sway -c ~/sway.conf >sway.log 2>&1 &
  sway_pid=$!
  sleep 2
  export WAYLAND_DISPLAY=$(ls $XDG_RUNTIME_DIR | grep -m1 '^wayland-[0-9]*$') SWAYSOCK=$(ls $XDG_RUNTIME_DIR/sway-ipc.* | head -1)
  forzac ~/book.fb2.zip --chapter 93 --measure 2>/dev/null | sed "s/^/scale $scale: /"
  forzac ~/book.fb2.zip --chapter 93 >/dev/null 2>&1 & app=$!
  sleep 3
  grim "$out/sway-$scale.png"
  swaymsg -t get_tree | python3 -c "import json,sys
def walk(n):
    if n.get('app_id'): print('  window: app_id', n['app_id'], 'size', n['rect']['width'], 'x', n['rect']['height'])
    for c in n.get('nodes', []) + n.get('floating_nodes', []): walk(c)
walk(json.load(sys.stdin))"
  kill $app $sway_pid; wait $sway_pid 2>/dev/null || true
done
