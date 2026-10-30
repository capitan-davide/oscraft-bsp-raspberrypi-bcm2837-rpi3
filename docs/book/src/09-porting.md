# Porting from the Raspberry Pi 4

The OsCraft Raspberry Pi 4 BSP can check its startup against QNX's own
for the same board: it boots the same image with either and compares the
system pages. QNX has no startup for the Raspberry Pi 3, so this BSP has
no such reference. This chapter lists what the port from the Raspberry
Pi 4 changed, what was checked instead, and what is left for the board.

## What changed

*Table 9.1: The Raspberry Pi 3 against the Raspberry Pi 4.*

| | Raspberry Pi 4 (BCM2711) | Raspberry Pi 3 (BCM2837) |
|---|---|---|
| Cores | 4 Cortex-A72 | 4 Cortex-A53 |
| Peripherals | `0xfe000000` | `0x3f000000`: the same mailbox, mini-UART, PL011 and PM, at other addresses |
| Interrupts | GIC-400, the library's `gic_v2` | the BCM2836 local controller and the BCM2835 ARM controller, the board's `intr.rs` (Chapter 7) |
| IPIs | SGI 0 | mailbox 0 of the local controller |
| The tick | PPI 27 | local vector 3 |
| The mini-UART's vector | 125 (SPI 93) | 93 (0x40 + 29) |
| RAM | up to 8 GB, in two ranges; `below1G` | up to 1008 MB, one range |
| The system counter | 54 MHz | 19.2 MHz (62.5 MHz in QEMU) |
| The core clock with `enable_uart=1` | 500 MHz | 250 MHz |
| PCI Express (MSIs) | yes, `msi.rs` | none |
| hwinfo | Ethernet, watchdog, SD card's DMA window | watchdog |
| armstub8 | sets up the GIC | sets up the local controller's core timer and `CNTFRQ_EL0` |
| UARTs for `-D` | mini-UART, UART0, UART3 | mini-UART, UART0 |

The spin table, the boot image's address, the mailbox protocol, the
watchdog and reboot, and everything the library does are the same.

One change was needed in the library: it set up each core's GIC CPU
interface itself, which fails without a GIC. A board now registers its
controller's part of each core's setup with `set_cpu_intr_init()`, and
`gic_v2::init()` registers the GIC's (the library's book,
[Chapter 8](../startup/08-gic.html)).

## Checked in QEMU

QEMU's `raspi3b` models a 1 GB Raspberry Pi 3 B with both interrupt
controllers. With the image of `make image`:

- QNX boots to the shell on all 4 cores, and `make test` passes: the
  shell's input comes through the mini-UART's interrupt, so through both
  controllers and the cascade.
- `/proc/config` says `intrctlr:N/A`: the kernel uses the board's
  callouts. `pidin irqs` shows the tick attached on each core (vector 3)
  and `devc-serminiuart` on vector 0x5d (93).
- Time: a 10-second wait in the shell takes 10.0 seconds on the host's
  clock; a wait on core 3 (`on -C 3`) takes its own tick.
- `shutdown` reaches the reboot callout, and `-W` resets at once, as
  QEMU's model of the watchdog does (Chapter 6).
- `-vvvv` prints the firmware's diagnostics: QEMU answers the
  temperature and the clocks, and 0 for the voltages and power states.

## Left for the board

- **The interrupt controllers' reads**: that the ARM controller's enable
  registers read back on the BCM2837 (Chapter 7), and that the pending
  sources look as QEMU shows them.
- **The mailbox's address**: startup passes the message's ARM physical
  address, as the RPi4 BSP does; QEMU accepts it, the board's firmware
  is expected to.
- **The system counter at 19.2 MHz**, which QEMU does not run at, and the
  tick with it.
- **The watchdog counting down** (`-W` with `wdtkick`), and reboot.
- **The Raspberry Pi 3 B+**: its revision code is decoded, but QEMU has
  no model of it.
- **QNX's RPi4 drivers** on the BCM2837 besides the console's: the SD
  card driver is for the BCM2711's EMMC2, which the BCM2837 does not
  have, so the image runs from RAM.

> [!WARNING]
> When the BSP runs on the board, check the `display_char` callout's
> output first (`procnto -v` prints through it): a wrong mini-UART address
> or clock shows as silence after "Starting next program".
