#!/usr/bin/env bash
set -euo pipefail
root_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
if [[ "$EUID" -ne 0 ]]; then
    exec sudo -- "$0" "$@"
fi
kernel_release="$(uname -r)"
make -C "$root_dir/linux"
install -Dm644 "$root_dir/linux/jiaolong_mifs.ko" "/lib/modules/$kernel_release/extra/jiaolong_mifs.ko"
install -Dm644 "$root_dir/linux/70-jiaolong-mifs.rules" /etc/udev/rules.d/70-jiaolong-mifs.rules
printf '%s\n' jiaolong_mifs > /etc/modules-load.d/jiaolong-mifs.conf
depmod -a "$kernel_release"
udevadm control --reload-rules
modprobe jiaolong_mifs
udevadm trigger --subsystem-match=misc --sysname-match=jiaolong-mifs
udevadm settle
printf '%s\n' '已安装 /dev/jiaolong-mifs 固件访问接口。'
