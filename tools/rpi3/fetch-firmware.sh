#!/usr/bin/env bash
# Download one file of the Raspberry Pi firmware from a pinned release and
# check its SHA-256.
#
# Usage: fetch-firmware.sh <name> <output>
#   e.g. fetch-firmware.sh start.elf build/rpi3/fw/start.elf
set -euo pipefail

FIRMWARE_TAG=1.20260915

name=${1:?usage: fetch-firmware.sh <name> <output>}
out=${2:?usage: fetch-firmware.sh <name> <output>}

case $name in
    bcm2710-rpi-3-b.dtb)      sha256=cd922193672e391b6194d6d94e34ffab6fc154205375fcfb362cad191f7f8aae ;;
    bcm2710-rpi-3-b-plus.dtb) sha256=d41256bdc25b09dbba0bcdb27592560561d39752ebb09c64e82a449e3271807d ;;
    LICENCE.broadcom)         sha256=c7283ff51f863d93a275c66e3b4cb08021a5dd4d8c1e7acc47d872fbe52d3d6b ;;
    *) echo "fetch-firmware.sh: no pinned checksum for '$name'" >&2; exit 1 ;;
esac

mkdir -p "$(dirname "$out")"
curl -fsSL -o "$out.tmp" "https://raw.githubusercontent.com/raspberrypi/firmware/${FIRMWARE_TAG}/boot/${name}"
echo "${sha256}  $out.tmp" | sha256sum --check --quiet
mv "$out.tmp" "$out"
