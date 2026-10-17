#!/bin/bash
# Inside forzac-arch: build and install the AUR packages with makepkg -si (forzac from the working tree, then
# forzac-bin from an archive made from that build), check the desktop integration, then run the app under a headless
# sway at fractional scales and take screenshots with grim.
# Hyprland needs a DRM render node even when headless; containers without /dev/dri can only run sway (pixman).
set -euo pipefail
book=${1:?usage: ci/arch-check.sh book.fb2.zip}
out=${OUT:-/src/target-png}
# perl-file-mimeinfo's mimetype is in /usr/bin/vendor_perl, which login shells (and so desktop sessions) put in PATH.
export PATH=$PATH:/usr/bin/vendor_perl
cd ~
git config --global --add safe.directory /src
ver=$(sed -n 's/^pkgver=//p' /src/packaging/aur/forzac/PKGBUILD)
mkdir -p forzac forzac-bin
# The working tree as the tag's source archive would hold it: tracked and new files, nothing ignored.
git -C /src ls-files -co --exclude-standard -z | (cd /src && xargs -0 ls -d 2>/dev/null) \
  | tar -C /src -T - --transform "s|^|forzac-desktop-$ver/|" -czf "$HOME/forzac/forzac-$ver.tar.gz"

# The source package, as an AUR user installs it: makepkg -si (dependencies from the repositories, then pacman -U).
cp /src/packaging/aur/forzac/{PKGBUILD,forzac.install} forzac/
sed -i "s|^source=.*|source=(\"forzac-$ver.tar.gz\")|" forzac/PKGBUILD
(cd forzac && time makepkg -si --noconfirm >makepkg.log 2>&1) || { tail -30 forzac/makepkg.log; exit 1; }
grep -A3 'post_install\|To open' forzac/makepkg.log || true
pacman -Qi forzac | grep -E '^(Name|Version|Installed Size|Depends On)'
pacman -Ql forzac | awk '{print $2}' | grep -vE '/$'

# Desktop integration: the .desktop file validates, MIME types resolve, Forzac becomes the default.
check_desktop() {
  desktop-file-validate /usr/share/applications/app.truenrth.forzac.desktop && echo "desktop file: valid"
  mkdir -p ~/.local/share/applications ~/.config
  xdg-mime default app.truenrth.forzac.desktop application/x-fictionbook+xml application/x-zip-compressed-fb2 application/x-fb3 application/epub+zip
  for f in book.fb2.zip x.fb2 x.fb3 x.epub; do t=$(xdg-mime query filetype ~/$f); echo "$f is $t, xdg-mime query default: $(xdg-mime query default "$t")"; done
}
cp "$book" ~/book.fb2.zip
printf '<?xml version="1.0"?><FictionBook/>' > ~/x.fb2; cp ~/book.fb2.zip ~/x.fb3; cp ~/book.fb2.zip ~/x.epub
check_desktop

# The binary package from a release archive laid out like CI's (scripts/package-linux.sh), replacing the source one.
(cd forzac-bin && CARGO_TARGET_DIR=~/forzac/src/forzac-desktop-$ver/target /src/scripts/package-linux.sh "forzac-$ver-linux-x86_64" >/dev/null)
cp /src/packaging/aur/forzac-bin/{PKGBUILD,forzac.install} forzac-bin/
sed -i "s|^source_x86_64=.*|source_x86_64=(\"forzac-$ver-linux-x86_64.tar.gz\")|" forzac-bin/PKGBUILD
sudo pacman -R --noconfirm forzac forzac-debug >/dev/null
(cd forzac-bin && makepkg -si --noconfirm >makepkg.log 2>&1) || { tail -30 forzac-bin/makepkg.log; exit 1; }
pacman -Qi forzac-bin | grep -E '^(Name|Version|Provides|Conflicts With)'
pacman -Ql forzac-bin | awk '{print $2}' | grep -cvE '/$' | sed 's/^/forzac-bin files: /'
check_desktop

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
