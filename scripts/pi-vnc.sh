#!/bin/bash
# Shows the Raspberry Pi's desktop on this Mac: an SSH tunnel to the Pi's VNC server (wayvnc, listening on the Pi's
# localhost only) and macOS Screen Sharing on top of it. The traffic is encrypted by SSH.
#   PI=user@host scripts/pi-vnc.sh          (PI_SSH_OPTS for extra ssh options, e.g. "-i ~/.ssh/key")
# Pi side, once: VNC on (raspi-config nonint do_vnc 0), and in /etc/wayvnc/config:
#   address=127.0.0.1        relax_encryption=true   (lets macOS Screen Sharing log in; SSH does the encryption)
set -euo pipefail
pi=${PI:?set PI=user@host}
port=${VNC_LOCAL_PORT:-5901}
if ! nc -z 127.0.0.1 "$port" 2>/dev/null; then
  # shellcheck disable=SC2086
  ssh ${PI_SSH_OPTS:-} -f -N -o ExitOnForwardFailure=yes -L "$port:127.0.0.1:5900" "$pi"
fi
open "vnc://localhost:$port"
echo "Screen Sharing: log in with the Pi user's name and password. Stop the tunnel: pkill -f 'L $port:127.0.0.1:5900'"
