# How the Raspberry Pi 3 Boots

On most boards the CPU starts first and runs a boot loader. On the
Raspberry Pi, the GPU starts first, and it decides what the ARM cores run.
This chapter follows the Raspberry Pi 3's boot up to startup's first
instruction, and shows how QEMU is made to boot the same way.

## The GPU boots the ARM

The BCM2837 has a VideoCore IV GPU and four Cortex-A53 cores. At
power-on, only the GPU runs. Its boot ROM loads `bootcode.bin` from the
SD card (the Raspberry Pi 3 B+ and 3 A+ can also boot over USB and the
network), which loads the GPU's firmware, `start.elf`. The firmware reads
`config.txt` to learn what to do. QNX documents no configuration for the
Raspberry Pi 3; this one does for the BCM2837 what the RPi4 BSP's does
for the BCM2711:

```text
[pi3]
arm_64bit=1
enable_uart=1
gpu_mem=16
kernel=ifs-rpi3.bin
```

- `arm_64bit=1` starts the ARM cores in AArch64;
- `kernel=` names the file to load, the boot image, which the firmware
  loads at 0x80000, the address the build file gives (`[image=0x80000]`);
- `enable_uart=1` sets the mini-UART up on GPIO 14 and 15, and, on the
  Raspberry Pi 3, fixes the GPU's core clock, from which the mini-UART
  takes its baud rate, at 250 MHz;
- `gpu_mem=16` gives the GPU 16 MB of the RAM; the rest is the ARM's.

The firmware also loads the board's device tree (`bcm2710-rpi-3-b.dtb`
for a Pi 3 B, `bcm2710-rpi-3-b-plus.dtb` for a Pi 3 B+; the BCM2710 is
the BCM2837's name in the device trees) and edits it before it starts
the ARM.

## armstub8

The ARM cores start at physical address 0, in EL3, where the firmware has
put a small program, *armstub8*. The Raspberry Pi 3's differs from the
Raspberry Pi 4's: the BCM2837 has no GIC to set up. On each core it

- sets up the BCM2836 local controller's core timer to count the 19.2
  MHz crystal, and writes that frequency to `CNTFRQ_EL0`;
- enables the core's coherency (`CPUECTLR_EL1.SMPEN`);
- drops to non-secure EL2.

Then, as on the Raspberry Pi 4:

- core 0 jumps to the image, at 0x80000, with `x0` holding the device
  tree's address;
- cores 1 to 3 wait, in a loop with `wfe`, until the 64-bit word at
  `0xd8 + 8 × core` holds an address, and jump there with `x0` = 0. This
  is the *spin table*; the device tree gives the words' addresses as each
  CPU's `cpu-release-addr`.

Figure 3.1 shows the sequence, and its QEMU counterpart.

![The Raspberry Pi 3's boot, and QEMU's](images/ch03-rpi3-boot.svg)

*Figure 3.1: From power-on to startup, on the board and in QEMU.*

So startup can count on this: core 0 at EL2, MMU off, `x0` the device
tree, and three cores waiting on the spin table in the first page of RAM.
The first page must survive until the other cores have left it, which is
why the board crate keeps it out of RAM (Chapter 5).

## The same boot in QEMU

QEMU's `raspi3b` machine models the BCM2837's peripherals, but not the
GPU's boot: given an ELF file with `-kernel`, it starts every core at its
entry point in EL3, with nothing in `x0`. To boot the same image as the
board, `tools/qemu/fwstub/fwstub.S` does armstub8's job (Listing 3.1),
and QEMU's loader device puts the image at 0x80000 and the device tree at
0x1f000000 (Listing 2.3).

*Listing 3.1: The firmware stub's handoff, at EL2 (`tools/qemu/fwstub/fwstub.S`).*

```text
{{#include ../../../tools/qemu/fwstub/fwstub.S:fwstub_el2}}
```

The stub is linked at address 0 and occupies the first page, like
armstub8, with the spin table at the same address. Before the code
above, at EL3, it sets up the core timer and the coherency as armstub8
does, and drops to EL2. It leaves `CNTFRQ_EL0` alone: QEMU's system
counter runs at 62.5 MHz whatever the local controller says, and QEMU
has already put that frequency in the register, where startup reads it.

The device tree is the Raspberry Pi firmware's own, downloaded from a
pinned release and checked against its SHA-256 by
`tools/rpi3/fetch-firmware.sh`. Unlike the real firmware, QEMU does not
edit it; QEMU's mailbox answers the questions that matter (Chapter 5).

> [!NOTE]
> With the firmware stub, the same image boots on the board and in QEMU,
> with the same startup options (`-u reg`: the device tree's address is in
> `x0`). An image started by U-Boot's `go` command gets the device tree's
> address as an argument instead, and takes `-u arg`.
