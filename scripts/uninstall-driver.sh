#!/usr/bin/env bash
set -euo pipefail
if [[ "$EUID" -ne 0 ]]; then exec sudo -- "$0" "$@"; fi
modprobe -r jiaolong_mifs
rm -f "/lib/modules/$(uname -r)/extra/jiaolong_mifs.ko" /etc/udev/rules.d/70-jiaolong-mifs.rules /etc/modules-load.d/jiaolong-mifs.conf
depmod -a
udevadm control --reload-rules
