#!/usr/bin/env bash
# Boot a QNX image file system (IFS) in QEMU's raspi3b machine.
#
# The image is loaded at 0x80000 and the DTB at 0x1f000000, and fwstub hands
# control to the image the way the real firmware does (see fwstub/fwstub.S).
#
# Usage: run.sh [--gdb] [--log <file>] <image> [extra qemu args...]
#   --gdb         wait for a debugger on tcp::1234 before running
#   --log <file>  write QEMU's interrupt/guest-error trace to <file>
#
# Environment:
#   BUILD    build directory holding qemu/fwstub.elf and qemu/*.dtb (default: build)
#   SERIAL0  QEMU -serial spec for the PL011 (UART0)     (default: null)
#   SERIAL1  QEMU -serial spec for the mini-UART (UART1) (default: mon:stdio)
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
build=${BUILD:-$root/build}/qemu

qemu_args=()
while [[ $# -gt 0 ]]; do
    case $1 in
        --gdb) qemu_args+=(-s -S); shift ;;
        --log) qemu_args+=(-d int,guest_errors,unimp -D "$2"); shift 2 ;;
        *) break ;;
    esac
done
image=${1:?usage: run.sh [--gdb] [--log <file>] <image> [qemu args...]}
shift

for f in "$build/fwstub.elf" "$build/bcm2710-rpi-3-b.dtb"; do
    [[ -f $f ]] || { echo "missing $f - run 'make qemu' first" >&2; exit 1; }
done

# ANCHOR: qemu_exec
exec qemu-system-aarch64 \
    -M raspi3b \
    -kernel "$build/fwstub.elf" \
    -device loader,file="$image",addr=0x80000,force-raw=on \
    -device loader,file="$build/bcm2710-rpi-3-b.dtb",addr=0x1f000000,force-raw=on \
    -serial "${SERIAL0:-null}" \
    -serial "${SERIAL1:-mon:stdio}" \
    -display none \
    "${qemu_args[@]}" \
    "$@"
# ANCHOR_END: qemu_exec
