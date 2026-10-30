//! The power management block: the watchdog, and reboot through it.
//!
//! Writes need a password in the top byte. The watchdog counts down in
//! ticks of about 15 us (67 per millisecond); when it expires, RSTC says
//! what to reset.

/// The power management registers.
pub const BASE: u64 = 0x3f10_0000;
const RSTC: u64 = BASE + 0x1c;
const WDOG: u64 = BASE + 0x24;
const PASSWORD: u32 = 0x5a00_0000;
const RSTC_WRCFG_MASK: u32 = 0x30;
const RSTC_WRCFG_FULL_RESET: u32 = 0x20;
const RSTC_STOP: u32 = 0x102;
const TICKS_PER_MS: u64 = 67;
const MAX_TICKS: u64 = 0x000f_ffff;
const DEFAULT_TICKS: u32 = 0x0001_0000;

fn read(addr: u64) -> u32 {
    // SAFETY: PM registers, mapped 1:1 while the MMU is off.
    unsafe { core::ptr::read_volatile(addr as *const u32) }
}

fn write(addr: u64, value: u32) {
    // SAFETY: as in read().
    unsafe { core::ptr::write_volatile(addr as *mut u32, value) }
}

// ANCHOR: watchdog
/// Start the watchdog with a timeout of `ms` milliseconds (about 1 s if
/// `ms` is 1000 or less): the system resets unless something (`wdtkick`)
/// reloads it.
pub fn watchdog_enable(ms: u64) {
    write(RSTC, PASSWORD | RSTC_STOP);
    let ticks = if ms <= 1000 {
        DEFAULT_TICKS
    } else {
        (ms * TICKS_PER_MS).min(MAX_TICKS) as u32
    };
    write(WDOG, PASSWORD | ticks);
    let rstc = read(RSTC) & !RSTC_WRCFG_MASK;
    write(RSTC, rstc | PASSWORD | RSTC_WRCFG_FULL_RESET);
}
// ANCHOR_END: watchdog

// ANCHOR: reboot_callout
// reboot(syspage, flags): set the watchdog to 10 ticks and a full reset,
// and wait. Patched with the PM registers' virtual address.
oscraft_startup::callout!(pub static REBOOT, "bcm2837_reboot", patches = 1,
    "mov     x5, #0", "movk x5, #0, lsl #16", "movk x5, #0, lsl #32", "movk x5, #0, lsl #48",
    "mov     w1, #0x5a000000",      // PASSWORD
    "mov     w0, #10",
    "orr     w0, w0, w1",
    "str     w0, [x5, #0x24]",      // WDOG
    "ldr     w0, [x5, #0x1c]",      // RSTC
    "bic     w0, w0, #0x30",
    "orr     w0, w0, w1",
    "orr     w0, w0, #0x20",        // full reset
    "str     w0, [x5, #0x1c]",
    "0: b    0b",
);
// ANCHOR_END: reboot_callout
