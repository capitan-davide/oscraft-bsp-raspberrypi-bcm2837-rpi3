//! The VideoCore mailbox: asking the GPU firmware about the board.
//!
//! On the Raspberry Pi the GPU boots first, and its firmware owns much of
//! the board's configuration: how RAM is split between the ARM and the
//! VideoCore, the clocks, the board revision. The ARM asks through the
//! *property channel* (8) of a mailbox: it writes the address of a message
//! of tags into the mailbox, and the firmware answers in the same buffer.

use core::ptr::{read_volatile, write_volatile};

/// The mailbox registers: mailbox 0 is the firmware's (ARM reads), mailbox
/// 1 the ARM's (ARM writes).
const BASE: u64 = 0x3f00_b880;
const MBOX0_READ: u64 = BASE;
const MBOX0_STATUS: u64 = BASE + 0x18;
const MBOX1_WRITE: u64 = BASE + 0x20;
const MBOX1_STATUS: u64 = BASE + 0x38;
const STATUS_FULL: u32 = 1 << 31;
const STATUS_EMPTY: u32 = 1 << 30;
const CHANNEL_PROPERTY: u32 = 8;

const TAG_BOARD_REVISION: u32 = 0x0001_0002;
const TAG_BOARD_SERIAL: u32 = 0x0001_0004;
const TAG_VC_MEMORY: u32 = 0x0001_0006;
const TAG_CLOCK_RATE: u32 = 0x0003_0002;
const TAG_POWER_STATE: u32 = 0x0002_0001;
const TAG_VOLTAGE: u32 = 0x0003_0003;
const TAG_TEMPERATURE: u32 = 0x0003_0006;
const TAG_EXPGPIO_STATE: u32 = 0x0003_0041;
/// The ARM cores' clock, for TAG_CLOCK_RATE.
pub const CLOCK_ARM: u32 = 3;

/// A message with one tag of up to 2 words of value: size, request code,
/// tag, value size, tag code, value, end tag. The address must be 16-byte
/// aligned: its low bits carry the channel.
#[repr(C, align(16))]
struct Message([u32; 8]);

static mut MESSAGE: Message = Message([0; 8]);

// ANCHOR: mbox_call
/// Ask the firmware for `tag`, with `args` as the value, and return the
/// answer's two value words (0 where the firmware fails to answer).
fn call(tag: u32, args: [u32; 2]) -> [u32; 2] {
    let msg = &raw mut MESSAGE;
    // SAFETY: the mailbox registers are device memory, mapped 1:1 while
    // the MMU is off; MESSAGE is startup's, in RAM the firmware can reach,
    // and the boot CPU alone uses it.
    unsafe {
        write_volatile(msg, Message([32, 0, tag, 8, 0, args[0], args[1], 0]));
        while read_volatile(MBOX1_STATUS as *const u32) & STATUS_FULL != 0 {}
        write_volatile(MBOX1_WRITE as *mut u32, msg as u32 | CHANNEL_PROPERTY);
        loop {
            while read_volatile(MBOX0_STATUS as *const u32) & STATUS_EMPTY != 0 {}
            if read_volatile(MBOX0_READ as *const u32) & 0xf == CHANNEL_PROPERTY {
                break;
            }
        }
        let m = read_volatile(msg).0;
        // Bit 31 of the request code: the firmware handled it.
        if m[1] & (1 << 31) == 0 {
            return [0, 0];
        }
        [m[5], m[6]]
    }
}
// ANCHOR_END: mbox_call

/// The board's revision code (see [`crate::revision`]).
pub fn board_revision() -> u32 {
    call(TAG_BOARD_REVISION, [0, 0])[0]
}

/// The board's serial number.
pub fn board_serial() -> u64 {
    let [lo, hi] = call(TAG_BOARD_SERIAL, [0, 0]);
    u64::from(hi) << 32 | u64::from(lo)
}

/// The RAM the VideoCore keeps for itself: (base, size).
pub fn vc_memory() -> (u64, u64) {
    let [base, size] = call(TAG_VC_MEMORY, [0, 0]);
    (u64::from(base), u64::from(size))
}

/// The rate of clock `id`, in Hz.
pub fn clock_rate(id: u32) -> u32 {
    call(TAG_CLOCK_RATE, [id, 0])[1]
}

// ANCHOR: diagnostics
/// The firmware's view of the board, for `-vvvv`, as QNX's RPi4 startup prints
/// it: the SoC's temperature (thousandths of a degree), the voltages
/// (core, SDRAM controller, PHY, I/O), the clocks that are running, the
/// devices' power states and the expander GPIOs (on the Pi 3 B, the
/// LEDs and the camera).
pub fn print_diagnostics() {
    use oscraft_startup::console::print;
    print(format_args!(
        "Temps {}\nVolts",
        call(TAG_TEMPERATURE, [0, 0])[1]
    ));
    for id in 1..=4 {
        print(format_args!(" {id}={}", call(TAG_VOLTAGE, [id, 0])[1]));
    }
    print(format_args!("\nClock"));
    for id in 1..=14 {
        let hz = clock_rate(id);
        if hz != 0 {
            print(format_args!(" {id}={}MHz", hz / 1_000_000));
        }
    }
    print(format_args!("\nPower"));
    for id in 0..=8 {
        print(format_args!(" {id}={}", call(TAG_POWER_STATE, [id, 0])[1]));
    }
    print(format_args!("\neGPIO"));
    for gpio in 128..=133 {
        print(format_args!(
            " {gpio}={}",
            call(TAG_EXPGPIO_STATE, [gpio, 0])[1]
        ));
    }
    print(format_args!("\n"));
}
// ANCHOR_END: diagnostics
