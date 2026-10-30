# To do

Work planned for the BSP and its startup library, roughly in order.

## Startup: finished

Done (2026-10-28): compressed images (UCL, the only kind QNX's startup
boots), the common options QNX's startup library acts on, pointer
authentication, and the board's diagnostics at `-vvvv`. Not done, by
choice: `-Q` (QNX Hypervisor: the kernel at EL2) and `-I`/`-i` (restoring
an image file system kept across a warm reset), which need a product or a
setup this project does not have; revisit if needed. The rule: provide
what QNX's original startup does, and weigh anything beyond it.

## Board bring-up (when the Raspberry Pi 3 B+ is available)

- [ ] `make sdcard`: a bootable SD card image with the GPU firmware
      (`bootcode.bin`, `start.elf`, `fixup.dat`, the device tree),
      `config.txt` and the image, fetched pinned and checked like the
      device tree.
- [ ] The QEMU test, `tools/qemu/test.py`, over a serial port, so that the
      same checks run on the board.
- [ ] A bring-up checklist in the book, and what each failure means.
- [ ] Check on the board what QEMU cannot (the book's Chapter 9): that the
      ARM interrupt controller's enable registers read back, the mailbox
      with the message's ARM physical address, the 19.2 MHz system
      counter and the tick, the watchdog counting down and reboot, the
      3 B+'s revision, `-u arg` under U-Boot, caches and timing on real
      hardware.

## Housekeeping

- [ ] A `LICENSE` file (Apache-2.0) in both repositories.
- [ ] Tag `oscraft-startup` v0.1.0 and pin the BSP to it.
- [x] The books checked, built and published on GitHub Pages, in both
      repositories.
- [ ] Continuous integration for the code too, in both repositories: unit
      tests, clippy, formatting (none needs the QNX SDP).

## The rest of the BSP: drivers

Today the image uses one of QNX's prebuilt drivers from its Raspberry Pi
4 BSP, the console's, which works on the BCM2837's mini-UART too. QNX's
other Raspberry Pi 4 drivers are for the BCM2711's devices (its SD card
controller, EMMC2, the BCM2837 does not have); the Raspberry Pi 3 needs
its own, for the SD card first. Writing them in Rust is a separate
decision, to take later.

| Driver | Does | C size | Built on (QNX library) |
|---|---|---|---|
| `devc-serminiuart` | the console, mini-UART | ~1,000 | `io-char` |
| `devc-serpl011` | the PL011 UARTs | ~1,800 | `io-char` |
| `i2c-bcm2711` | I2C | ~1,200 | I2C master framework |
| `spi-bcm2711` | SPI | ~1,700 | SPI master framework |
| `devb-sdmmc-bcm2711` | the SD card | ~24,000 | `io-blk`, CAM |
| `gpio-bcm2711`, `mbox-bcm2711` | GPIO, the GPU mailbox | ~1,000 | resource manager |
| DMA library | DMA for the above | ~2,400 | |
| PCI module, Ethernet, USB | | prebuilt only | PCI server, `io-sock` |

The choice, as for startup and `libstartup.a`:

1. keep QNX's drivers;
2. Rust hardware layers on QNX's C frameworks, through FFI;
3. all Rust, the frameworks too (a resource manager crate, a terminal
   crate, a block device crate): several times the startup's work.

Before any of them:

- [ ] Rust for QNX programs: `aarch64-unknown-nto-qnx800` is a tier-3
      Rust target, with no prebuilt standard library. QNX ships a Rust
      toolchain package for SDP 8.0 (`com.qnx.qnx800.host.linux.x86_64.rust`),
      not installed yet.
- [ ] Rust bindings to QNX's APIs: resource managers, interrupts, mapping
      device memory.
- [ ] A pilot: `mbox-bcm2711` (small, no framework under it), then decide
      between 2 and 3, driver by driver; serial next, the SD card last.
- Most drivers can only be tested on the board: QEMU's models of I2C, SPI
  and the SD card are partial.
