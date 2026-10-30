# Appendix A: Source Map

Every file of the BSP outside the startup library, and the chapter that
explains it. The library's files are in its own book's source map.

*Table A.1: The source files.*

| File | Contents | Chapter |
|---|---|---|
| `startup-bcm2837-rpi3/src/lib.rs` | The board crate's root | 4 |
| `startup-bcm2837-rpi3/src/board.rs` | `main()`, the debug UARTs, the RAM layout, the spin table | 4, 5, 6, 7 |
| `startup-bcm2837-rpi3/src/mbox.rs` | The VideoCore mailbox | 5 |
| `startup-bcm2837-rpi3/src/revision.rs` | Decoding the board revision | 5 |
| `startup-bcm2837-rpi3/src/miniuart.rs` | The mini-UART and its callouts | 6 |
| `startup-bcm2837-rpi3/src/pm.rs` | The watchdog and the reboot callout | 6 |
| `startup-bcm2837-rpi3/src/intr.rs` | The interrupt controllers and the IPIs | 7 |
| `startup-bcm2837-rpi3/src/hwinfo.rs` | The devices in hwinfo | 8 |
| `images/rpi3.build` | The boot image's build file | 2 |
| `tools/qemu/fwstub/fwstub.S` | The firmware stub for QEMU | 3 |
| `tools/qemu/fwstub/fwstub.ld` | Its linker script: the first page | 3 |
| `tools/qemu/run.sh` | Booting an image in QEMU | 2 |
| `tools/qemu/test.py` | The QEMU boot test | 2 |
| `tools/rpi3/fetch-firmware.sh` | Fetching the device trees | 3 |
| `Makefile` | Building, running, testing, the books | 2 |
