#!/bin/bash
# Builds the GTK shell for aarch64 in a container on this machine, copies it to a Raspberry Pi and opens a book in
# the Pi's desktop session (watch it with scripts/pi-vnc.sh). The previous instance is closed first: Forzac keeps one
# process per user, so a running old build would otherwise receive the book and show old code.
#   PI=user@host scripts/pi-dev.sh book.fb2.zip [forzac flags]     (PI_SSH_OPTS for extra ssh options)
# Needs Docker here; the Pi runs Raspberry Pi OS 13 (trixie) with its desktop. Logs: ~/forzac-dev/forzac.log on the Pi.
set -euo pipefail
pi=${PI:?set PI=user@host}
book=${1:?usage: PI=user@host scripts/pi-dev.sh book.fb2.zip [forzac flags]}
shift
root=$(cd "$(dirname "$0")/.." && pwd)
# PI_SSH_OPTS is split on purpose (several options); remote commands expand on the Pi on purpose.
# shellcheck disable=SC2086,SC2029
ssh_pi() { ssh ${PI_SSH_OPTS:-} "$pi" "$@"; }

docker image inspect forzac-debian >/dev/null 2>&1 || docker build -q -t forzac-debian -f "$root/ci/debian.Dockerfile" "$root/ci"
docker run --rm -v "$root":/src -v forzac-pi-target:/target -w /tmp forzac-debian bash -c \
  'cargo build --release -q -p forzac-gtk --manifest-path /src/Cargo.toml && /src/scripts/package-linux.sh forzac-dev >/dev/null && cp forzac-dev.tar.gz /src/target/'

ssh_pi 'pkill -f "[f]orzac-dev/bin/forzac" || true; rm -rf ~/forzac-dev && mkdir -p ~/forzac-dev/books'
ssh_pi 'tar xzf - -C ~' < "$root/target/forzac-dev.tar.gz"
# shellcheck disable=SC2094  # reads the local book, writes a copy on the Pi
ssh_pi "cat > ~/forzac-dev/books/$(basename "$book")" < "$book"
# Extra flags for forzac, quoted for the remote shell; nothing at all when there are none (printf would emit '').
args=""
if [ $# -gt 0 ]; then args=$(printf ' %q' "$@"); fi
# Separate statements: a trailing `&` after `a && b && c` backgrounds the whole list, and that subshell would keep the
# SSH channel open.
ssh_pi "cd ~/forzac-dev; export XDG_RUNTIME_DIR=/run/user/\$(id -u); \
  export WAYLAND_DISPLAY=\$(cd \$XDG_RUNTIME_DIR && ls wayland-? | head -1); \
  setsid nohup \$HOME/forzac-dev/bin/forzac books/$(printf %q "$(basename "$book")")$args > forzac.log 2>&1 < /dev/null & sleep 1; cat forzac.log"
echo "running on $pi; log: ~/forzac-dev/forzac.log"
