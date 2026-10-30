# Console, Reboot and Watchdog

Two of the board's devices serve both startup and the kernel: a UART,
which is the console, and the power manager's watchdog, which can reset
the board. This chapter shows how each is used by startup, and
handed to the kernel as callouts.

## The debug UARTs

Startup and the kernel print on one of two UARTs, which the board lists
for the library (Listing 6.1); `-D` chooses one, by name (the library's
book, [Chapter 4](../startup/04-startup-context.html)).

*Listing 6.1: The debug UARTs (`startup-bcm2837-rpi3/src/board.rs`).*

```rust
{{#include ../../../startup-bcm2837-rpi3/src/board.rs:debug_drivers}}
```

*Table 6.1: The debug UARTs.*

| `-D` | UART | Pins | `config.txt` |
|---|---|---|---|
| `miniuart` (default) | UART1, the mini-UART, `0x3f215000` | GPIO 14, 15 | `enable_uart=1` |
| `pl011` | UART0, a PL011, `0x3f201000` | GPIO 14, 15 | `dtoverlay=disable-bt` |

The BCM2837 has one PL011 UART and one simpler *mini-UART* (UART1), part
of its "auxiliary" peripherals; both are the same as the BCM2711's, at
other addresses. On the Raspberry Pi 3, the header's console pins, GPIO
14 and 15, go to the mini-UART; UART0 serves the Bluetooth chip.
`config.txt` changes this: `dtoverlay=disable-bt` gives the pins to
UART0. Which pins a UART uses is set in the GPIO block's function
registers, which the firmware sets up from `config.txt`; startup leaves
them alone. In QEMU, UART0 is the first serial port and the mini-UART the
second (`SERIAL0`, `SERIAL1` in `tools/qemu/run.sh`).

The PL011 uses the library's driver; startup sets its baud rate from its
48 MHz clock. The mini-UART is the board's own
(`startup-bcm2837-rpi3/src/miniuart.rs`): with `enable_uart=1` the
firmware has set it to 115200 baud already, so its `init` does nothing,
and startup only sends bytes, polling the line status register until
the transmitter has room.

> [!WARNING]
> The mini-UART's baud rate is derived from the GPU's core clock, which
> `enable_uart=1` fixes at 250 MHz on the Raspberry Pi 3 (the Raspberry
> Pi 4's runs at 500 MHz). The serial driver the boot script starts,
> `devc-serminiuart`, is told the clock (`-c250000000`): with the Pi 4's
> value, its baud rate is wrong.

## The kernel's debug channel

The kernel prints its own messages (with `procnto -v`, and when it
crashes) through the `debug` callouts of the system page. The library
installs the selected UART's: they do in assembly what startup's driver
does in Rust, at the registers' virtual address. Listing 6.2 shows the
mini-UART's.

*Listing 6.2: The mini-UART's callouts (`startup-bcm2837-rpi3/src/miniuart.rs`).*

```rust
{{#include ../../../startup-bcm2837-rpi3/src/miniuart.rs:miniuart_callouts}}
```

`display_char` waits for room, writes the character, and then waits for
the transmitter to be idle: the kernel may print just before it resets
the board, and the character must leave first. `poll_key` returns a
received byte or -1. The mini-UART cannot detect a break, so
`break_detect` returns 0.

## The power manager and its watchdog

The BCM2837's power manager (PM, at `0x3f100000`) is the BCM2711's: it
has a watchdog, a counter that counts down in ticks of about 15 µs
(65,536 Hz) and, when it expires, resets what `PM_RSTC` says. Its
registers only accept writes with a password, 0x5a, in the top byte.

With `-W`, startup starts the watchdog (Listing 6.3), and something in the
running system must reload it in time: QNX's `wdtkick` utility does, given
the PM's registers and values (Chapter 8).

*Listing 6.3: Starting the watchdog, `watchdog_enable()` (`startup-bcm2837-rpi3/src/pm.rs`).*

```rust
{{#include ../../../startup-bcm2837-rpi3/src/pm.rs:watchdog}}
```

## Reboot

The same watchdog resets the board for the kernel: the `reboot` callout
(Listing 6.4) sets it to 10 ticks, asks for a full reset, and waits.

*Listing 6.4: The reboot callout (`startup-bcm2837-rpi3/src/pm.rs`).*

```rust
{{#include ../../../startup-bcm2837-rpi3/src/pm.rs:reboot_callout}}
```

In QEMU, `shutdown` at the shell reaches this callout, and QEMU, started
with `-no-reboot`, exits.

> [!NOTE]
> QEMU's model of the watchdog resets the machine as soon as the
> watchdog is set up, instead of counting down. So `-W` makes an image
> reset at once in QEMU, and is only for the board.
