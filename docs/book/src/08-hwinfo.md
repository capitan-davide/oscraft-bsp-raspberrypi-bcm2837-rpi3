# Devices in hwinfo

The system page's `hwinfo` section tells drivers about the board's
devices, so that they can start without board-specific options (the
library's book, [Chapter 6](../startup/06-system-page.html)). This
chapter shows what the board crate puts there.

## The devices

*Table 8.1: The devices in hwinfo.*

| Device | Tags | Read by |
|---|---|---|
| `wdog` | registers (the PM's), a 500 Hz input clock | `wdtkick` |
| `wdt,options` | register widths: `write_width`, `enable_width` (32) | `wdtkick` |

The watchdog is the BCM2711's, so the entries are those QNX's RPi4 BSP
writes for it, with the PM's address on the BCM2837.

*Listing 8.1: The devices, `init_hwinfo()` (`startup-bcm2837-rpi3/src/hwinfo.rs`).*

```rust
{{#include ../../../startup-bcm2837-rpi3/src/hwinfo.rs:init_hwinfo}}
```

The Raspberry Pi 4's other two entries have no counterpart here:

- **The Ethernet controller.** The Raspberry Pi 3's is on USB (a
  LAN9514 on the 3 B, a LAN7515 on the 3 B+), which its driver finds by
  itself; the Raspberry Pi 4's GENET is on the SoC, and its driver takes
  its registers and MAC address from hwinfo.
- **The SD card controller's DMA window.** The Raspberry Pi 4's EMMC2
  only reaches the first gigabyte of RAM; the Raspberry Pi 3's
  controllers reach all of it.
