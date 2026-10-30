//! The mini-UART (UART1), the console on GPIO 14/15: startup's output,
//! and the kernel's debug channel through the callouts below.

use oscraft_startup::console::{DebugDriver, DebugPort};

/// The mini-UART's registers, in the BCM2837's auxiliary peripherals.
pub const BASE: u64 = 0x3f21_5000;
const AUX_MU_IO: u64 = 0x40;
const AUX_MU_LSR: u64 = 0x54;
const LSR_TX_EMPTY: u32 = 1 << 5;

/// Nothing to set up: the firmware configures the mini-UART when
/// `config.txt` has `enable_uart=1`.
fn init(_: &DebugPort) {}

fn putc(port: &DebugPort, byte: u8) {
    // SAFETY: the mini-UART registers are device memory, mapped 1:1
    // while the MMU is off.
    unsafe {
        while core::ptr::read_volatile((port.base + AUX_MU_LSR) as *const u32) & LSR_TX_EMPTY == 0 {
        }
        core::ptr::write_volatile((port.base + AUX_MU_IO) as *mut u32, u32::from(byte));
    }
}

/// The mini-UART as a debug device, `-D miniuart`.
pub const DRIVER: DebugDriver = DebugDriver {
    name: "miniuart",
    defaults: DebugPort {
        base: BASE,
        shift: 0,
        baud: 115_200,
        clk: 0,
    },
    init,
    putc,
    size: 0x100,
    display_char: &DISPLAY_CHAR,
    poll_key: &POLL_KEY,
    break_detect: &BREAK_DETECT,
};

// ANCHOR: miniuart_callouts
// The kernel's debug callouts, patched with the registers' virtual
// address. display_char(syspage, c) waits for room, sends c and waits for
// it to leave; poll_key(syspage) returns a received byte or -1.
oscraft_startup::callout!(pub static DISPLAY_CHAR, "bcm2837_display_char_miniuart", patches = 1,
    "mov     x7, #0", "movk x7, #0, lsl #16", "movk x7, #0, lsl #32", "movk x7, #0, lsl #48",
    "0: ldr  w2, [x7, #0x54]",      // AUX_MU_LSR: transmitter empty?
    "tbz     w2, #5, 0b",
    "and     w2, w1, #0xff",
    "str     w2, [x7, #0x40]",      // AUX_MU_IO
    "1: ldr  w2, [x7, #0x54]",      // transmitter idle?
    "tbz     w2, #6, 1b",
    "ret",
);

oscraft_startup::callout!(pub static POLL_KEY, "bcm2837_poll_key_miniuart", patches = 1,
    "mov     x7, #0", "movk x7, #0, lsl #16", "movk x7, #0, lsl #32", "movk x7, #0, lsl #48",
    "ldrb    w1, [x7, #0x54]",      // AUX_MU_LSR: data ready?
    "mov     x0, #-1",
    "tst     w1, #1",
    "b.eq    1f",
    "ldrb    w0, [x7, #0x40]",
    "1: ret",
);

oscraft_startup::callout!(pub static BREAK_DETECT, "bcm2837_break_detect_miniuart", patches = 0,
    "mov     x0, #0",
    "ret",
);
// ANCHOR_END: miniuart_callouts
