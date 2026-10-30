# Interrupts, Timer and Cores

The library provides the generic timer and the SMP protocol, and a GICv2,
which the BCM2837 does not have: its interrupts come through two older
controllers of Broadcom's, which the board crate describes itself. This
chapter covers them, then the timer and the cores.

## Two controllers, no GIC

An interrupt reaches a Cortex-A53 core of the BCM2837 through two
controllers (Figure 7.1 shows the path):

- the **local controller** of the BCM2836 (the "ARM control" block, at
  `0x40000000`), which has a set of registers for each core. It routes
  to each core that core's four generic timers, its four *mailboxes*
  (32-bit registers that any core can set bits in, to interrupt it), the
  PMU, and, to one core, the GPU's interrupt. Each core has a register
  of pending sources, at `0x60 + 4 × core`.
- the **ARM interrupt controller** of the BCM2835 (at `0x3f00b200`),
  behind the GPU's interrupt: it gathers the 64 interrupts of the
  peripherals (the GPU's side of the SoC, numbered 0 to 63) and 8 of the
  ARM's own (its "basic" interrupts), each with a bit to enable it and a
  bit to disable it.

```text
UART, USB, ... ──► ARM interrupt controller (BCM2835) ──┐ GPU interrupt
                                                        ▼ (to core 0)
core n's timers, mailboxes, PMU ────────► local controller (BCM2836) ──► core n's IRQ
```

*Figure 7.1: An interrupt's way to a core.*

Neither controller acknowledges an interrupt the way a GIC does: their
registers show which sources are asserted, and a source stays asserted
until its device, or its mailbox, is cleared.

## What the kernel needs

QNX's kernel can run on such a board because, unless the build file loads
an interrupt controller module (`[module=gicv2]`, for GICs only), it does
not drive the controller itself: it runs the callouts in the system
page's `intrinfo` entries (the library's book,
[Chapter 8](../startup/08-gic.html)). `/proc/config` then says
`intrctlr:N/A`. Three more things are generic, not GIC-specific:

- **Per-CPU interrupts**: an entry with a non-zero `local_stride` holds
  interrupts that each CPU has its own of, as a GIC's PPIs; the kernel
  attaches CPU n's at `vector + n × local_stride`.
- **The IPI**: the kernel finds its vector through the entry's `config`
  callout, which returns `INTR_CONFIG_FLAG_IPI` for it, and sends IPIs
  through the `smp.send_ipi` callout, with the target from `gic_map`.
- **Cascades**: an entry's `cascade_vector` names the vector of another
  entry that its controller signals through.

## The vectors

The board crate describes the two controllers as two entries
(Table 7.1).

*Table 7.1: The interrupt vectors.*

| Vectors | Controller | Sources |
|---|---|---|
| 0-3 | local, per CPU | the core's timers: secure and non-secure physical, hypervisor, virtual |
| 4-7 | local, per CPU | the core's mailboxes; 4, mailbox 0, is the IPI |
| 8 | local | the GPU's interrupt: the ARM controller's cascade, on core 0 |
| 9 | local, per CPU | the PMU |
| 0x40-0x7f | ARM controller | the peripherals' interrupts 0-63 |
| 0x80-0x87 | ARM controller | its basic interrupts 0-7 (the ARM timer, its mailbox, ...) |

A device's vector is 0x40 plus its interrupt number on the GPU's side:
93 for the mini-UART (interrupt 29), the number the boot script gives
`devc-serminiuart`.

*Listing 7.1: Setting up the controllers, `init()` (`startup-bcm2837-rpi3/src/intr.rs`).*

```rust
{{#include ../../../startup-bcm2837-rpi3/src/intr.rs:init_intr}}
```

`init()` disables every source of the ARM controller, routes the GPU's
interrupt to core 0 as an IRQ, and nothing as a FIQ. It then describes
the two entries, sets `send_ipi`, and registers each core's part of the
setup with the library (Listing 7.2), which runs it on the boot core in
`init_boot_cpu()` and on each other core as it starts.

*Listing 7.2: A core's part, `init_cpu()` (`startup-bcm2837-rpi3/src/intr.rs`).*

```rust
{{#include ../../../startup-bcm2837-rpi3/src/intr.rs:init_cpu}}
```

## The local controller's callouts

Each core has its own registers in the local controller, and the
callouts find their core's in `MPIDR_EL1`. So an `id` or `mask` callout
acts on the core that runs it, as a GIC's banked registers do for its
PPIs (Listing 7.3).

*Listing 7.3: The local controller's callouts (`startup-bcm2837-rpi3/src/intr.rs`).*

```rust
{{#include ../../../startup-bcm2837-rpi3/src/intr.rs:local_callouts}}
```

- `id` returns the lowest pending source. For the IPI, it clears mailbox
  0 at once, so that another IPI can be sent while the kernel handles
  this one; `send_ipi` only sets the mailbox when the target had no
  command pending, as the library's GIC callout only raises its SGI then.
- `eoi` has nothing to do: the timer's interrupt goes away when the
  kernel programs the next tick.
- `mask` and `unmask` clear or set the source's bit in the core's timer
  or mailbox control register, with IRQs off around the read and the
  write. The GPU's interrupt is never masked here: the ARM controller
  masks its sources one by one.
- `config` marks vector 4 as the IPI.

## The ARM controller's callouts

The ARM controller's registers come in three banks (the peripherals'
interrupts 0-31, 32-63, and the basic interrupts), in the order of the
vectors, so the callouts compute a bank from the vector and index its
registers (Listing 7.4).

*Listing 7.4: The ARM controller's callouts (`startup-bcm2837-rpi3/src/intr.rs`).*

```rust
{{#include ../../../startup-bcm2837-rpi3/src/intr.rs:armctrl_callouts}}
```

A device keeps its interrupt asserted until its driver has handled it,
which happens later, in a thread. So `id` disables the source it finds,
and `eoi` enables it again, unless the kernel keeps it masked: with
`INTR_GENFLAG_LOAD_INTRMASK`, the kernel loads the interrupt's mask
count into `x22` for `eoi`. This is the Raspberry Pi 4's MSI controller's
scheme (the OsCraft RPi4 BSP's book, Chapter 7).

> [!WARNING]
> `id` takes only sources that are both pending and enabled: it reads the
> enable registers back. QEMU's model of the controller reads them back,
> and its pending registers already leave out disabled sources; that the
> board reads them back too is still to be checked on the board.

## The timer

The system counter runs at 19.2 MHz on the board (armstub8 writes the
frequency to `CNTFRQ_EL0`) and 62.5 MHz in QEMU; the library reads it
there. The system tick is the virtual timer, vector 3, which the kernel
attaches on each core at `3 + n × 0x8000`. In QEMU, a 10-second wait in
the shell (`time waitfor /none 10`) takes 10.0 seconds on the host's
clock.

## The cores

The four Cortex-A53 cores wait on the firmware's spin table (Chapter 3).
The library calls the board's `release_cpu()` (Listing 7.5) for each in
turn, with the physical address of its entry code.

*Listing 7.5: Releasing a core (`startup-bcm2837-rpi3/src/board.rs`).*

```rust
{{#include ../../../startup-bcm2837-rpi3/src/board.rs:spin_table}}
```

The write goes straight to memory, since startup runs with the caches off,
and the waiting core reads it with its own caches off. `dsb sy` makes sure
the write is done before `sev` wakes the cores from `wfe`; each core
rereads its word, and the one whose word is now set leaves the loop.

The core then runs the library's code for secondary CPUs (the library's
book, [Chapter 12](../startup/12-smp.html)), which runs `init_cpu()` on
it. On QEMU, `pidin -f bl` shows the kernel's four idle threads, one on
each core, and other threads moving between them.
