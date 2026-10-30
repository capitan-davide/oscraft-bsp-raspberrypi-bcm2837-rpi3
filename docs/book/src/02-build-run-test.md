# Building, Running and Testing

This chapter shows how to build the startup and the boot image, boot it
in QEMU, and run the tests.

## What you need

- **The QNX SDP 8.0**, with the Raspberry Pi 4 BSP package installed
  through the QNX Software Center (for its serial driver; Chapter 1).
  Source its environment script before running `make`, from bash:

  ```text
  source <sdp>/qnxsdp-env.sh
  ```

  It sets `QNX_HOST` and `QNX_TARGET` and puts `mkifs` and the SDP's
  compilers on the `PATH`. The Makefile takes everything from there, and
  finds the BSP package's zip in the SDP's `bsp/` directory; `QNX_BSP_ZIP`
  overrides it.
- **Rust**: `rust-toolchain.toml` pins the version and the target,
  `aarch64-unknown-none-softfloat`, which `rustup` installs on first use.
- **`clang` and `ld.lld`**: `ld.lld` links the startup (below), `clang`
  assembles the QEMU firmware stub.
- **QEMU** with the `raspi3b` machine.
- **mdBook**, for the books.

## The Makefile

*Table 2.1: The Makefile's targets.*

| Target | What it does |
|---|---|
| `make startup` | Builds the board crate and links the startup program, `build/startup/startup-bcm2837-rpi3` |
| `make image` | Builds the boot image, `build/rpi3.ifs` |
| `make run` | Boots the image in QEMU, the console on the terminal |
| `make test` | Runs the unit tests, then boots the image in QEMU and checks it |
| `make book` | Checks and builds both books into `build/book/` |

`STARTUP_OPTS` sets the startup's options in the image, for example
`make run STARTUP_OPTS="-vvv -D miniuart -u reg"` for more output.
`PROFILE=dev` builds the startup without optimizations.

The startup library is a git dependency of the board crate, which
`Cargo.lock` pins to one commit; `cargo update -p oscraft-startup` moves
it to the latest. To work on the library and the board at once,
`OSCRAFT_STARTUP=../oscraft-startup` makes every target use a local
checkout of the library instead.

> [!WARNING]
> With `OSCRAFT_STARTUP`, cargo records the local checkout in
> `Cargo.lock`. Commit a lock file only after a build without it, which
> puts the git source back.

## Building the startup

QNX's `mkifs` does not take an executable for startup: it takes a
*relocatable object* and links it itself, at the address where startup
lands in the image (the library's book,
[Chapter 2](../startup/02-image.html)). The Makefile makes one from the
board crate, which is built as a static library, with the library's
linker script, wherever cargo has the library (Listing 2.1).

*Listing 2.1: Building the startup program (`Makefile`).*

```makefile
{{#include ../../../Makefile:make_startup}}
```

## Building the image

`images/rpi3.build` says what goes into the boot image. Listing 2.2 is its
bootstrap section: the startup, with options the Makefile fills in, and
the kernel.

*Listing 2.2: The image's bootstrap section (`images/rpi3.build`).*

```text
{{#include ../../../images/rpi3.build:bootstrap}}
```

The boot script then starts the serial driver for the mini-UART, the pipe
manager, and a shell; the files section lists the libraries and
utilities they need. mkifs looks for every file along `MKIFS_PATH`, which
the Makefile builds from three places, in order: the directory of our
startup, the RPi4 BSP's prebuilt binaries, the SDP's target tree. The
first directory that has a file wins.

## Running in QEMU

QEMU's `raspi3b` machine models the BCM2837, but it does not run the
Raspberry Pi's firmware. A small firmware stub stands in for it (Chapter
3), and `tools/qemu/run.sh` starts QEMU with the stub, the image and the
Raspberry Pi 3 B's device tree (Listing 2.3).

*Listing 2.3: Starting QEMU (`tools/qemu/run.sh`).*

```bash
{{#include ../../../tools/qemu/run.sh:qemu_exec}}
```

The mini-UART, the console, is on the terminal; `Ctrl-A X` quits QEMU.
A boot with `-v` looks like this:

```text
RaspberryPi3B (revision 0xa02082), 4 CPUs at 700 MHz
RAM: 1008 MB, VideoCore 0x3c000000-0x40000000
MMU: TCR_EL1 0x12b5183519, L0 tables at 0xffffff8040200000
Loaded /proc/boot/procnto-smp-instr, entry 0xffffff8060073c80
System page at phys:0x9000 user:0xffffff8040208000 kern:0xffffff8040205000
Starting next program at 0xffffff8060073c80
Welcome to QNX on the Raspberry Pi 3
QNX 8.0.0 on RaspberryPi3B, 4 CPUs
#
```

## Testing

`make test` runs two kinds of test (Listing 2.4).

*Listing 2.4: The tests (`Makefile`).*

```makefile
{{#include ../../../Makefile:make_test}}
```

The **unit tests** of the board crate (decoding the board revision) run
on the host. The library's tests (its binary interface against QNX's
headers, its parsers and builders) are in its repository; see its book,
[Chapter 15](../startup/15-testing.html).

The **QEMU test**, `tools/qemu/test.py`, boots the image and checks the
boot script's banner, the shell prompt, that `pidin info` lists four
processors, that the kernel's idle threads run on all four, and that
`uname -m` reports the board. It types its commands at the shell, so it
also checks the console's interrupts, which come through both of the
board's interrupt controllers (Chapter 7).

## The books

`make book` checks both books before it builds them. Each book has a
checker, `check.py`, which makes sure that every listing still names an
existing anchor in the source, that every anchor is used, that every
source file is in the book's source map, and that each book includes code
only from its own tree. This book's tree is the repository without the
library (Listing 2.5); the library's book's is the library's directory,
so that it can move with the library.

*Listing 2.5: This book's checker configuration (`docs/book/check.py`).*

```python
{{#include ../check.py:check_config}}
```
