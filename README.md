# OsCraft BSP for the Raspberry Pi 3 (BCM2837)

A board support package for the Raspberry Pi 3, compatible with QNX OS
8.0, with a startup program written in Rust. QNX's unmodified kernel,
`procnto-smp-instr`, comes up on all four cores and runs the boot script
to a shell. QNX has no BSP for the Raspberry Pi 3; this one is derived
from the [OsCraft BSP for the Raspberry Pi 4](https://github.com/capitan-davide/oscraft-bsp-raspberrypi-bcm2711-rpi4).

- `startup-bcm2837-rpi3/`: the board crate, which builds
  `startup-bcm2837-rpi3`. It uses the startup library,
  [oscraft-startup](https://github.com/capitan-davide/oscraft-startup), a
  Rust counterpart of QNX's `libstartup.a`, as a git dependency;
  `make OSCRAFT_STARTUP=<dir> ...` uses a local checkout of it instead.
- `images/rpi3.build`: the boot image.
- `tools/qemu/`: booting and testing the image in QEMU's `raspi3b`.
- `docs/book/`: the BSP's book.

## Building

You need the QNX SDP 8.0 with the Raspberry Pi 4 BSP package (from the
QNX Software Center, for its serial driver), Rust (the toolchain in
`rust-toolchain.toml`), `clang` and `ld.lld`, QEMU, and mdBook for the
books. From bash:

```text
source <sdp>/qnxsdp-env.sh
make image          # build/rpi3.ifs
make run            # boot it in QEMU; Ctrl-A X quits
make test           # unit tests, then a boot in QEMU on 4 CPUs
make book           # both books, in build/book/
```

The books explain everything else. They are published at
<https://capitan-davide.github.io/oscraft-bsp-raspberrypi-bcm2837-rpi3/>
(the BSP's and the startup library's); `make book` builds them locally,
into `build/book/`.

## Status

Tested in QEMU's `raspi3b` (a Raspberry Pi 3 B): QNX boots to the shell
on 4 CPUs, with interrupts through the BCM2836 and BCM2835 controllers.
Not yet run on the board; the book's "Porting from the Raspberry Pi 4"
chapter lists what is left to check there.
