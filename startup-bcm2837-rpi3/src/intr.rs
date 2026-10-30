//! The interrupt controllers: the BCM2837 has no GIC.
//!
//! Interrupts reach a core through two controllers:
//!
//! - the *local* controller (the BCM2836's "ARM control" block), one set
//!   of registers per core: each core's generic timers, its four
//!   mailboxes (the IPIs), the PMU, and the GPU's interrupt, which goes
//!   to one core;
//! - the *ARM interrupt controller* of the BCM2835, behind the GPU's
//!   interrupt: the 64 GPU (peripheral) interrupts and 8 of the ARM's own.
//!
//! To the kernel, they are two `intrinfo` entries. The local controller's
//! vectors 0-9 are its pending bits; they are per CPU (`local_stride`),
//! like a GIC's PPIs. The ARM controller is cascaded into vector 8, the
//! GPU's interrupt, with vectors from [`ARMCTRL_VECTOR_BASE`]: the 64 GPU
//! interrupts, then the 8 basic ones. Neither controller acknowledges an
//! interrupt the way a GIC does: they show which sources are asserted.

use oscraft_startup::Startup;
use oscraft_startup::abi::syspage::{
    INTR_GENFLAG_LOAD_INTRMASK, INTR_NOT_CASCADED, IntrgenData, IntrinfoEntry,
};

/// The local controller.
const LOCAL_BASE: u64 = 0x4000_0000;
/// Which core gets the GPU's interrupt (bits 0-1), as an IRQ.
const GPU_INT_ROUTING: u64 = 0x0c;
/// PMU interrupt routing: set (IRQ to core n: bit n) and clear.
const PMU_ROUTING_CLEAR: u64 = 0x14;
/// Per core n, at + 4 * n: which timer interrupts are IRQs (bits 0-3),
/// which mailbox interrupts are IRQs (bits 0-3), and the pending sources.
const TIMER_CNTRL: u64 = 0x40;
const MAILBOX_CNTRL: u64 = 0x50;
/// Per core n, at + 16 * n: mailbox 0 to 3, write-set and write-clear.
const MAILBOX0_CLEAR: u64 = 0xc0;

/// The ARM interrupt controller's page, and its registers in it.
const ARMCTRL_BASE: u64 = 0x3f00_b000;
const FIQ_CONTROL: u64 = 0x20c;
const DISABLE_1: u64 = 0x21c;
const DISABLE_2: u64 = 0x220;
const DISABLE_BASIC: u64 = 0x224;

/// The local controller's vectors: the pending bits.
const LOCAL_VECTORS: u32 = 10;
/// The virtual timer: the system tick. (Vector 4, mailbox 0, is the IPI.)
pub const CNTV_VECTOR: i32 = 3;
/// The GPU's interrupt, into which the ARM controller is cascaded.
const GPU_VECTOR: u32 = 8;
/// The ARM controller's vectors: GPU interrupt n is vector base + n, ARM
/// (basic) interrupt n is base + 64 + n.
pub const ARMCTRL_VECTOR_BASE: u32 = 0x40;
const ARMCTRL_VECTORS: u32 = 72;

fn write(addr: u64, value: u32) {
    // SAFETY: interrupt controller registers, mapped 1:1 while the MMU is
    // off.
    unsafe { core::ptr::write_volatile(addr as *mut u32, value) }
}

// ANCHOR: local_callouts
// The local controller's callouts. Each core has its own registers, so a
// callout finds its core's in MPIDR_EL1 (Aff0 is the core, 0-3), and does
// for its core what a GIC's banked registers do: id and mask act on the
// CPU that runs them.
//
// id: the lowest pending source of this core, in x19 (-1 if none). If it
// is mailbox 0, the IPI, clear the mailbox now, so that another IPI can
// be sent while the kernel handles this one.
oscraft_startup::callout!(pub static LOCAL_ID, "bcm2837_local_id", patches = 1,
    "mov     x7, #0", "movk x7, #0, lsl #16", "movk x7, #0, lsl #32", "movk x7, #0, lsl #48",
    "mrs     x0, mpidr_el1",
    "and     x0, x0, #3",
    "add     x1, x7, x0, lsl #2",
    "ldr     w2, [x1, #0x60]",      // the core's pending sources
    "and     w2, w2, #0x3ff",
    "mov     x19, #-1",
    "cbz     w2, 0f",
    "rbit    w2, w2",
    "clz     w19, w2",              // the lowest set bit
    "cmp     w19, #4",              // mailbox 0, the IPI
    "b.ne    0f",
    "add     x1, x7, x0, lsl #4",
    "mov     w2, #-1",
    "str     w2, [x1, #0xc0]",      // MAILBOX0_CLEAR
    "0:",
);

// eoi: nothing to do. The timer's interrupt goes away when the kernel
// programs the next tick, the IPI's when id clears the mailbox.
oscraft_startup::callout!(pub static LOCAL_EOI, "bcm2837_local_eoi", patches = 0,
    "nop",
);

// mask/unmask(syspage, vector), on this core: a timer (0-3) in
// TIMER_CNTRL, a mailbox (4-7) in MAILBOX_CNTRL, the PMU (9) in its
// routing. The GPU's interrupt (8) stays routed: the ARM controller masks
// its sources. Returns -1 for a vector beyond 9. IRQs are off while the
// control register is read and written.
macro_rules! local_mask_callout {
    ($name:ident, $sym:literal, $op:literal, $pmu:literal) => {
        oscraft_startup::callout!(pub static $name, $sym, patches = 1,
            "mov     x7, #0", "movk x7, #0, lsl #16", "movk x7, #0, lsl #32", "movk x7, #0, lsl #48",
            "mrs     x2, mpidr_el1",
            "and     x2, x2, #3",
            "mov     x0, #-1",
            "cmp     w1, #10",
            "b.hs    9f",
            "mov     x0, #0",
            "cmp     w1, #8",
            "b.eq    9f",
            "b.hi    2f",
            "mrs     x6, daif",
            "msr     daifset, #2",
            "add     x4, x7, x2, lsl #2",
            "add     x4, x4, #0x40",        // TIMER_CNTRL
            "cmp     w1, #4",
            "b.lo    1f",
            "add     x4, x4, #0x10",        // MAILBOX_CNTRL
            "sub     w1, w1, #4",
            "1: mov  w3, #1",
            "lsl     w3, w3, w1",
            "ldr     w5, [x4]",
            concat!($op, "     w5, w5, w3"),
            "str     w5, [x4]",
            "msr     daif, x6",
            "b       9f",
            "2: mov  w3, #1",               // the PMU
            "lsl     w3, w3, w2",
            concat!("str     w3, [x7, #", $pmu, "]"),
            "9: ret",
        );
    };
}
local_mask_callout!(LOCAL_MASK, "bcm2837_local_mask", "bic", "0x14");
local_mask_callout!(LOCAL_UNMASK, "bcm2837_local_unmask", "orr", "0x10");

// config(syspage, intrinfo, vector): mailbox 0 is the IPI.
oscraft_startup::callout!(pub static LOCAL_CONFIG, "bcm2837_local_config", patches = 0,
    "cmp     x2, #4",               // mailbox 0
    "mov     w0, #4",               // INTR_CONFIG_FLAG_IPI
    "csel    w0, w0, wzr, eq",
    "ret",
);

// send_ipi(syspage, target, cmd, &ipicmds[cpu]): record the command and,
// if none was pending, set a bit of the target core's mailbox 0. The
// kernel takes target from gic_map, which holds the core's number.
oscraft_startup::callout!(pub static SEND_IPI, "bcm2837_send_ipi", patches = 1,
    "mov     x5, #0", "movk x5, #0, lsl #16", "movk x5, #0, lsl #32", "movk x5, #0, lsl #48",
    "mrs     x0, daif",
    "msr     daifset, #2",
    "0: ldxr w7, [x3]",
    "orr     w6, w7, w2",
    "stxr    w4, w6, [x3]",
    "cbnz    w4, 0b",
    "dsb     sy",
    "cbnz    w7, 1f",
    "add     x5, x5, w1, uxtw #4",
    "mov     w6, #1",
    "str     w6, [x5, #0x80]",      // the target's mailbox 0, write-set
    "1: msr  daif, x0",
    "ret",
);
// ANCHOR_END: local_callouts

// ANCHOR: armctrl_callouts
// The ARM controller's callouts. Its registers, from 0x200: the basic
// pending sources (0x200), GPU pending 1 and 2 (0x204, 0x208), then three
// banks of enable (0x210: GPU 0-31, 0x214: GPU 32-63, 0x218: basic) and
// disable (0x21c, 0x220, 0x224) registers, in the order of the vectors.
// A device's interrupt stays asserted until its driver clears it, so id
// disables the source it finds and eoi enables it again, unless the
// kernel keeps it masked (x22, the mask count, is not 0).
//
// id: the lowest enabled, pending source, in x19 (-1 if none).
oscraft_startup::callout!(pub static ARMCTRL_ID, "bcm2837_armctrl_id", patches = 1,
    "mov     x7, #0", "movk x7, #0, lsl #16", "movk x7, #0, lsl #32", "movk x7, #0, lsl #48",
    "mov     x19, #-1",
    "mov     x2, #0",               // the bank: GPU 0-31, GPU 32-63, basic
    "0: add  x1, x7, x2, lsl #2",
    "ldr     w0, [x1, #0x204]",     // GPU pending 1, 2 (or, for bank 2, ...)
    "cmp     x2, #2",
    "b.ne    1f",
    "ldr     w0, [x7, #0x200]",     // ... the basic pending sources
    "and     w0, w0, #0xff",
    "1: ldr  w1, [x1, #0x210]",     // the bank's enable register
    "ands    w0, w0, w1",
    "b.ne    2f",
    "add     x2, x2, #1",
    "cmp     x2, #3",
    "b.lo    0b",
    "b       3f",
    "2: rbit w0, w0",
    "clz     w0, w0",
    "mov     w1, #1",
    "lsl     w1, w1, w0",
    "add     x0, x0, x2, lsl #5",
    "mov     x19, x0",
    "add     x0, x7, x2, lsl #2",
    "str     w1, [x0, #0x21c]",     // the bank's disable register
    "3:",
);

oscraft_startup::callout!(pub static ARMCTRL_EOI, "bcm2837_armctrl_eoi", patches = 1,
    "mov     x7, #0", "movk x7, #0, lsl #16", "movk x7, #0, lsl #32", "movk x7, #0, lsl #48",
    "cbnz    x22, 0f",
    "cmp     w19, #72",
    "b.hs    0f",
    "and     w0, w19, #31",
    "mov     w1, #1",
    "lsl     w1, w1, w0",
    "lsr     w0, w19, #5",
    "add     x0, x7, x0, lsl #2",
    "str     w1, [x0, #0x210]",     // the bank's enable register
    "0:",
);

// mask/unmask(syspage, vector): set the vector's bit in its bank's
// disable / enable register; -1 for a vector beyond 71.
macro_rules! armctrl_mask_callout {
    ($name:ident, $sym:literal, $reg:literal) => {
        oscraft_startup::callout!(pub static $name, $sym, patches = 1,
            "mov     x7, #0", "movk x7, #0, lsl #16", "movk x7, #0, lsl #32", "movk x7, #0, lsl #48",
            "mov     x0, #-1",
            "cmp     w1, #72",
            "b.hs    0f",
            "and     w2, w1, #31",
            "mov     w3, #1",
            "lsl     w3, w3, w2",
            "lsr     w2, w1, #5",
            "add     x2, x7, x2, lsl #2",
            concat!("str     w3, [x2, #", $reg, "]"),
            "mov     x0, #0",
            "0: ret",
        );
    };
}
armctrl_mask_callout!(ARMCTRL_MASK, "bcm2837_armctrl_mask", "0x21c");
armctrl_mask_callout!(ARMCTRL_UNMASK, "bcm2837_armctrl_unmask", "0x210");
// ANCHOR_END: armctrl_callouts

// ANCHOR: init_intr
/// Set up both controllers, describe them in the system page and register
/// each CPU's part of the setup. Every source starts disabled; the GPU's
/// interrupt goes to core 0, as IRQ, and nothing is a FIQ.
pub fn init(s: &mut Startup) {
    for reg in [DISABLE_1, DISABLE_2, DISABLE_BASIC] {
        write(ARMCTRL_BASE + reg, !0);
    }
    write(ARMCTRL_BASE + FIQ_CONTROL, 0);
    write(LOCAL_BASE + GPU_INT_ROUTING, 0);
    write(LOCAL_BASE + PMU_ROUTING_CLEAR, 0xff);

    let local = s.map_device(LOCAL_BASE, 0x1000);
    let armctrl = s.map_device(ARMCTRL_BASE, 0x1000);
    let sp = &mut s.syspage;
    let local_entry = IntrinfoEntry {
        vector_base: 0,
        num_vectors: LOCAL_VECTORS,
        cascade_vector: INTR_NOT_CASCADED,
        id: IntrgenData {
            rtn: sp.callout(&LOCAL_ID, &[local]),
            ..Default::default()
        },
        eoi: IntrgenData {
            rtn: sp.callout(&LOCAL_EOI, &[]),
            ..Default::default()
        },
        mask: sp.callout(&LOCAL_MASK, &[local]),
        unmask: sp.callout(&LOCAL_UNMASK, &[local]),
        config: sp.callout(&LOCAL_CONFIG, &[]),
        // Each core has its own: the same stride as a GIC's PPIs.
        local_stride: 0x8000,
        ..Default::default()
    };
    let armctrl_entry = IntrinfoEntry {
        vector_base: ARMCTRL_VECTOR_BASE,
        num_vectors: ARMCTRL_VECTORS,
        cascade_vector: GPU_VECTOR,
        id: IntrgenData {
            rtn: sp.callout(&ARMCTRL_ID, &[armctrl]),
            ..Default::default()
        },
        eoi: IntrgenData {
            genflags: INTR_GENFLAG_LOAD_INTRMASK,
            rtn: sp.callout(&ARMCTRL_EOI, &[armctrl]),
            ..Default::default()
        },
        mask: sp.callout(&ARMCTRL_MASK, &[armctrl]),
        unmask: sp.callout(&ARMCTRL_UNMASK, &[armctrl]),
        ..Default::default()
    };
    sp.intrinfo.push(local_entry);
    sp.intrinfo.push(armctrl_entry);
    sp.smp.send_ipi = sp.callout(&SEND_IPI, &[local]);
    s.set_cpu_intr_init(init_cpu);
}
// ANCHOR_END: init_intr

// ANCHOR: init_cpu
/// This core's part: its timers' interrupts off (the kernel enables the
/// tick's), mailbox 0's on and empty. The kernel sends core n its IPIs
/// through `gic_map[n]`, which is n.
fn init_cpu(gic_map: &mut [u64], cpu: usize, _num_cpu: u16) {
    let n = cpu as u64;
    write(LOCAL_BASE + TIMER_CNTRL + 4 * n, 0);
    write(LOCAL_BASE + MAILBOX0_CLEAR + 16 * n, !0);
    write(LOCAL_BASE + MAILBOX_CNTRL + 4 * n, 1);
    gic_map[cpu] = n;
}
// ANCHOR_END: init_cpu
