//! The board's devices in hwinfo, for the drivers' defaults.
//!
//! The watchdog, for `wdtkick`, described as QNX's RPi4 BSP describes the
//! same block on the BCM2711. The Pi 3's other devices need no entry:
//! its Ethernet controller is on USB, and its SD card controller reaches
//! all of the RAM.

use crate::pm;
use oscraft_startup::Startup;
use oscraft_startup::syspage::hwinfo;

/// The PM block's register range.
const PM_SIZE: u32 = 0x114;

// ANCHOR: init_hwinfo
pub fn init_hwinfo(s: &mut Startup) {
    let sp = &mut s.syspage;
    let (hw, st) = (&mut sp.hwinfo, &mut sp.strings);
    let bus = hwinfo::add_default(hw, st);
    let mem = sp.asinfo.find(st, "memory").unwrap_or(0);
    hw.add_item(st, "Device", "wdog", bus, &7u32.to_le_bytes());
    hw.add_location(st, pm::BASE, PM_SIZE, 0, mem);
    hw.add_inputclk(st, 500, 1);
    hw.add_item(st, "Device", "wdt,options", bus, &0u32.to_le_bytes());
    hw.add_regname(st, "write_width", 32);
    hw.add_regname(st, "enable_width", 32);
}
// ANCHOR_END: init_hwinfo
