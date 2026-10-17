#!/bin/sh
set -e
case "$1" in
    remove|purge|0) udevadm control --reload-rules || true ;;
esac
