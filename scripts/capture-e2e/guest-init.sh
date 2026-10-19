#!/bin/bash
set -euxo pipefail
export PATH=/usr/bin:/bin
export HOME=/tmp/home XDG_RUNTIME_DIR=/run/capture-e2e
export XDG_CONFIG_HOME=$HOME/.config XDG_DATA_HOME=$HOME/.local/share XDG_CACHE_HOME=$HOME/.cache
unset WAYLAND_DISPLAY DISPLAY NIRI_SOCKET DBUS_SESSION_BUS_ADDRESS
mkdir -p "$HOME" "$XDG_RUNTIME_DIR" /dev/pts /run/udev
chmod 700 "$XDG_RUNTIME_DIR"
mount -t devpts devpts /dev/pts
/usr/lib/systemd/systemd-udevd --daemon
udevadm trigger --action=add
udevadm settle
export GALLIUM_DRIVER=virpipe
export LIBSEAT_BACKEND=seatd
export VK_DRIVER_FILES=/mnt/lvp_icd.json
export __EGL_VENDOR_LIBRARY_FILENAMES=/usr/share/glvnd/egl_vendor.d/50_mesa.json
seatd > /mnt/seatd.log 2>&1 &
sleep 1
dbus-run-session -- python3 /mnt/guest.py > /mnt/guest.log 2>&1 || true
sync
/usr/lib/initcpio/busybox poweroff -f
