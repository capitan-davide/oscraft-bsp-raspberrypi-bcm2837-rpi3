//! The board's `main()`.

use crate::{intr, mbox, miniuart, pm, revision};
use oscraft_startup::cmdline::{Opt, parse_number};
use oscraft_startup::console::{DebugDriver, DebugPort};
use oscraft_startup::pl011;
use oscraft_startup::syspage::strings;
use oscraft_startup::{Startup, kprintln};

/// Options of this board, besides the common ones: `-W [ms]` starts the
/// watchdog.
const BOARD_OPTIONS: &str = "W:";

// ANCHOR: debug_drivers
/// The UARTs `-D` can choose for debug output; the first is the default.
/// The mini-UART is on GPIO 14/15 with `enable_uart=1`. UART0, a PL011, is
/// there instead with `dtoverlay=disable-bt` (and is QEMU's first serial
/// port); without it, UART0 drives the Bluetooth chip. It runs from a
/// 48 MHz clock.
static DEBUG_DRIVERS: [DebugDriver; 2] = [
    miniuart::DRIVER,
    pl011::driver(
        "pl011",
        DebugPort {
            base: 0x3f20_1000,
            shift: 0,
            baud: 115_200,
            clk: 48_000_000,
        },
    ),
];
// ANCHOR_END: debug_drivers

/// The BCM2837's four Cortex-A53 cores.
const NUM_CPUS: usize = 4;

const MB: u64 = 1 << 20;

// ANCHOR: spin_table
/// The firmware's spin table: secondary core n waits until the 64-bit word
/// at 0xd8 + 8 * n holds the address to start at (the DTB's
/// cpu-release-addr). The first page also holds the firmware's armstub.
const SPIN_TABLE: u64 = 0xd8;
const FIRMWARE_PAGE: u64 = 0x1000;

/// Let core `cpu` run from `entry`: post the address and wake the cores
/// waiting in WFE.
fn release_cpu(cpu: usize, entry: u64) {
    let slot = (SPIN_TABLE + 8 * cpu as u64) as *mut u64;
    // SAFETY: the spin table is RAM in the first page, which startup
    // reserved; the core reads it with its MMU off, as startup writes it.
    unsafe {
        slot.write_volatile(entry);
        core::arch::asm!("dsb sy", "sev");
    }
}
// ANCHOR_END: spin_table

// ANCHOR: init_ram
/// Where the peripherals start: the ARM sees no RAM from here up, even on
/// a board with 1 GB.
const PERIPHERALS: u64 = 0x3f00_0000;

/// Describe the RAM: its size from the board revision, up to the
/// peripherals, less the part the VideoCore keeps (`vcram`, at the top).
fn init_ram(s: &mut Startup, rev: u32) {
    let ram = match revision::memory_mb(rev) * MB {
        0 => {
            kprintln!("startup: unknown RAM size, using 256 MB");
            256 * MB
        }
        size => size.min(PERIPHERALS),
    };
    s.add_ram(0, ram);
    // The armstub and spin table stay: the secondary cores read the table.
    s.ram.reserve(0, FIRMWARE_PAGE);
    let (vc_base, vc_size) = mbox::vc_memory();
    let vc_end = (vc_base + vc_size).min(ram);
    if vc_size != 0 && vc_base < vc_end {
        s.ram.reserve(vc_base, vc_end - vc_base);
        s.syspage
            .as_add_containing(vc_base, vc_end - 1, "vcram", "ram");
    }
    if s.debug_level > 0 {
        kprintln!(
            "RAM: {} MB, VideoCore {:#x}-{:#x}",
            ram / MB,
            vc_base,
            vc_base + vc_size
        );
    }
}
// ANCHOR_END: init_ram

// ANCHOR: board_main
fn main(s: &mut Startup) -> ! {
    s.init_debug(&DEBUG_DRIVERS);
    let mut watchdog = None;
    for opt in s.options(BOARD_OPTIONS) {
        match opt {
            Opt::Flag('W', ms) => watchdog = Some(ms.map_or(0, |ms| parse_number(ms).0)),
            other => s.handle_common_option(&other),
        }
    }
    if let Some(ms) = watchdog {
        pm::watchdog_enable(ms);
    }
    let rev = mbox::board_revision();
    let cpu_mhz = mbox::clock_rate(mbox::CLOCK_ARM) / 1_000_000;
    if s.debug_level > 0 {
        kprintln!(
            "{} (revision {rev:#x}), {NUM_CPUS} CPUs at {cpu_mhz} MHz",
            revision::board(rev)
        );
    }

    if s.debug_level > 3 {
        mbox::print_diagnostics();
    }

    s.init_smp(NUM_CPUS, release_cpu);
    init_ram(s, rev);
    s.init_mmu();
    intr::init(s);
    s.init_qtime(intr::CNTV_VECTOR);
    s.init_boot_cpu(cpu_mhz);

    // The kernel's way to reboot. (Its debug output is the console's
    // device, which boot() sets up.)
    let pm_regs = s.map_device(pm::BASE, 0x1000);
    let sp = &mut s.syspage;
    sp.callout.reboot = sp.callout(&pm::REBOOT, &[pm_regs]);

    // What confstr() and uname report.
    sp.add_typed_string(strings::CS_MACHINE, revision::board(rev));
    sp.add_typed_string(strings::CS_HW_PROVIDER, revision::manufacturer(rev));
    sp.add_typed_string(strings::CS_ARCHITECTURE, revision::soc(rev));
    let mut serial = [0u8; 16];
    sp.add_typed_string(
        strings::CS_HW_SERIAL,
        hex(mbox::board_serial(), &mut serial),
    );
    crate::hwinfo::init_hwinfo(s);
    s.boot()
}
// ANCHOR_END: board_main

/// `value` as 16 hexadecimal digits.
fn hex(value: u64, buf: &mut [u8; 16]) -> &str {
    for (i, b) in buf.iter_mut().enumerate() {
        let digit = (value >> (60 - 4 * i)) & 0xf;
        *b = b"0123456789abcdef"[digit as usize];
    }
    core::str::from_utf8(buf).unwrap_or("")
}

oscraft_startup::entry!(main);
