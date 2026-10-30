//! The board revision code: which Raspberry Pi this is.
//!
//! New-style codes pack the board's description into bits:
//! `NOQuuuWuFMMMCCCCPPPPTTTTTTTTRRRR`, where MMM is the memory size, CCCC
//! the manufacturer, PPPP the processor and TTTTTTTT the board type.

fn field(rev: u32, shift: u32, mask: u32) -> usize {
    ((rev >> shift) & mask) as usize
}

/// The RAM size, in MB.
pub fn memory_mb(rev: u32) -> u64 {
    const SIZES: [u64; 6] = [256, 512, 1024, 2048, 4096, 8192];
    SIZES.get(field(rev, 20, 0x7)).copied().unwrap_or(0)
}

pub fn manufacturer(rev: u32) -> &'static str {
    const NAMES: [&str; 6] = [
        "Sony UK",
        "Egoman",
        "Embest",
        "Sony Japan",
        "Embest",
        "Stadium",
    ];
    NAMES.get(field(rev, 16, 0xf)).copied().unwrap_or("Unknown")
}

pub fn soc(rev: u32) -> &'static str {
    const NAMES: [&str; 4] = ["BCM2835", "BCM2836", "BCM2837", "BCM2711"];
    NAMES.get(field(rev, 12, 0xf)).copied().unwrap_or("Unknown")
}

pub fn board(rev: u32) -> &'static str {
    match field(rev, 4, 0xff) {
        0x08 => "RaspberryPi3B",
        0x0a => "RaspberryPiCM3",
        0x0d => "RaspberryPi3B+",
        0x0e => "RaspberryPi3A+",
        0x10 => "RaspberryPiCM3+",
        0x12 => "RaspberryPiZero2W",
        _ => "Unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_a_raspberry_pi_3() {
        // QEMU's raspi3b: a 1 GB Pi 3 B (revision 1.2) made by Sony UK.
        let rev = 0x00a0_2082;
        assert_eq!(memory_mb(rev), 1024);
        assert_eq!(manufacturer(rev), "Sony UK");
        assert_eq!(soc(rev), "BCM2837");
        assert_eq!(board(rev), "RaspberryPi3B");
        // A Pi 3 B+ (Sony UK), a 512 MB Pi 3 A+ and a Zero 2 W.
        assert_eq!(board(0x00a0_20d3), "RaspberryPi3B+");
        assert_eq!(memory_mb(0x0090_20e0), 512);
        assert_eq!(board(0x0090_20e0), "RaspberryPi3A+");
        assert_eq!(board(0x0090_2120), "RaspberryPiZero2W");
    }
}
