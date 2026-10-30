# Preface

This book describes the OsCraft board support package (BSP) for the
Raspberry Pi 3, compatible with QNX OS 8.0: what it takes to boot QNX's
kernel on that board with a startup program written in Rust, and how the
BSP does it. It is not a QNX product: it implements the interfaces QNX's
kernel and tools expect.

QNX has no BSP for the Raspberry Pi 3. This one is derived from the
[OsCraft Raspberry Pi 4 BSP](https://capitan-davide.github.io/oscraft-bsp-raspberrypi-bcm2711-rpi4/),
whose structure it keeps; Chapter 9 lists what the port changed, and why.

It is a companion to [The OsCraft Startup Library](../startup/index.html),
which explains how a QNX startup program works and how the generic
library, `oscraft-startup`, implements it. This book is about the board:
the Raspberry Pi's boot process and firmware, the BCM2837 SoC, and the
board crate that turns the library into `startup-bcm2837-rpi3`. Where a
topic belongs to the library (the system page, callouts, page tables,
starting the CPUs), this book links to the library's book rather than
repeating it.

The book is for readers who know systems programming. Each chapter first
explains the part of the hardware or firmware it deals with, then the
code. Code listings come from the source when the book is built.

## Conventions

- Paths are relative to the repository's root: `startup-bcm2837-rpi3/src/board.rs`.
- *Listings*, *figures* and *tables* are numbered per chapter.
- "The library's book, Chapter 6" refers to the startup library's book.

> [!NOTE]
> Notes give background that is useful but not needed to follow the text.

> [!WARNING]
> Warnings point out things that are easy to get wrong when changing the
> code.
