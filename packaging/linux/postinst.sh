#!/bin/sh
set -e
modprobe uinput || true
udevadm control --reload-rules || true
udevadm trigger --subsystem-match=input --subsystem-match=misc || true
