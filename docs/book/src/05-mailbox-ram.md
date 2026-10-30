# The Mailbox and the RAM

On the Raspberry Pi, the GPU's firmware owns much of the board's
configuration: how the RAM is split between the GPU and the ARM, the
clocks, the board's identity. Startup asks it through a *mailbox*. This
chapter explains the mailbox, then how the board crate describes the RAM
with what it learns.

## The BCM2837's address map

The ARM cores see the BCM2837 as in Table 5.1.

*Table 5.1: The BCM2837's physical address map, as the ARM sees it.*

| Range | Contents |
|---|---|
| `0x0000_0000` - `0x3eff_ffff` | RAM: the ARM's, then at its top the GPU's |
| `0x3f00_0000` - `0x3fff_ffff` | The peripherals: UARTs, mailbox, GPIO, the power manager, the BCM2835 interrupt controller |
| `0x4000_0000` - `0x4003_ffff` | The ARM's local peripherals: the BCM2836 local controller (Chapter 7) |

The peripherals take the last 16 MB of the first gigabyte, so even a
1 GB board gives the ARM at most 1008 MB of RAM; the GPU's share is at
the top of it, below the peripherals. The address map has no room for
more RAM: no Raspberry Pi 3 has more than 1 GB.

## The mailbox

The mailbox is a pair of hardware FIFOs between the ARM and the GPU, at
`0x3f00b880`. The ARM talks to the firmware on its *property channel*
(8) in messages: a buffer in RAM that holds a list of *tags*, each a
request with room for the answer. The ARM writes the buffer's address,
with the channel number in its low 4 bits (so the buffer must be 16-byte
aligned), into mailbox 1; the firmware answers in the same buffer and
posts the address back in mailbox 0.

*Listing 5.1: A mailbox call (`startup-bcm2837-rpi3/src/mbox.rs`).*

```rust
{{#include ../../../startup-bcm2837-rpi3/src/mbox.rs:mbox_call}}
```

Startup runs with the caches off, so the GPU sees the message as the ARM
wrote it, and the ARM sees the answer, with no cache maintenance. The
buffer is in startup's own data, which the GPU can reach.

The board crate asks for four things:

*Table 5.2: What startup asks the firmware.*

| Tag | Answer | Used for |
|---|---|---|
| `0x00010002` board revision | A 32-bit code | RAM size, board name, maker, SoC |
| `0x00010004` board serial | 64 bits | `_CS_HW_SERIAL` |
| `0x00010006` VC memory | Base and size of the GPU's RAM | Reserved, named `vcram` |
| `0x00030002` clock rate, ARM | Hz | The CPUs' speed in cpuinfo |

With `-vvvv`, startup also prints what the firmware reports about the
board, in the form QNX's RPi4 startup prints it (Listing 5.2): useful
when a board misbehaves because of power or temperature. A request the
firmware does not answer (QEMU answers few of them) prints 0.

*Listing 5.2: The firmware's diagnostics (`startup-bcm2837-rpi3/src/mbox.rs`).*

```rust
{{#include ../../../startup-bcm2837-rpi3/src/mbox.rs:diagnostics}}
```

## The board revision

The revision code describes the board in bit fields:
`NOQuuuWuFMMMCCCCPPPPTTTTTTTTRRRR`, where `MMM` is the RAM size, `CCCC`
the maker, `PPPP` the processor (2: the BCM2837) and `TTTTTTTT` the board
type: 0x08 for a Pi 3 B, 0x0d for a Pi 3 B+, 0x0e for a Pi 3 A+, 0x0a and
0x10 for the Compute Modules 3 and 3+, and 0x12 for the Zero 2 W, whose
BCM2710A1 is the same SoC. `revision.rs` decodes it, with unit tests;
QEMU's `raspi3b` reports `0xa02082`, a 1 GB Pi 3 B, revision 1.2.

## The RAM

With the revision's RAM size and the GPU's share from the mailbox,
`init_ram()` (Listing 5.3) describes the RAM.

*Listing 5.3: Describing the RAM, `init_ram()` (`startup-bcm2837-rpi3/src/board.rs`).*

```rust
{{#include ../../../startup-bcm2837-rpi3/src/board.rs:init_ram}}
```

- The RAM is one range, which ends at the peripherals: on a 1 GB board,
  the revision's size runs into them.
- The first page holds armstub8 and the spin table, which the other
  cores read until startup releases them (Chapter 3): it is reserved.
- The GPU's RAM is reserved too, and named `vcram`, as far as it is
  below the peripherals. QEMU reports 64 MB from `0x3c000000`, which runs
  to 1 GB; with `gpu_mem=16`, the board's firmware gives the GPU exactly
  the 16 MB under the peripherals' window.

Unlike the BCM2711's, the BCM2837's DMA engines and SD card controller
reach all of the ARM's RAM, so there is no `below1G`. The library then
turns what is free into `sysram` (the library's book,
[Chapter 5](../startup/05-ram.html)). On QEMU's board the kernel reports
940 MB free of 1008.
