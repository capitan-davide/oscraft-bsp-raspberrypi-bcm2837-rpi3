/*
 * fwstub - stand-in for the Raspberry Pi 3 firmware + armstub8 under QEMU.
 *
 * QEMU's raspi3b machine, given an ELF with -kernel, starts every core at the
 * ELF entry in EL3 and passes nothing in x0. The real board enters the kernel
 * image differently: the VideoCore firmware loads the image at 0x80000 and the
 * DTB somewhere in RAM, then armstub8 (at physical 0x0) drops every core to
 * non-secure EL2, enters the image on core 0 with x0 = DTB address, and parks
 * cores 1-3 on a spin table at 0xd8 + 8 * cpu (the DTB's cpu-release-addr).
 *
 * This stub reproduces that hand-off so the same image boots unchanged in QEMU
 * and on hardware. The image and DTB themselves are placed by QEMU's generic
 * loader device (see tools/qemu/run.sh).
 */

#ifndef KERNEL_ADDR
#define KERNEL_ADDR     0x80000
#endif
#ifndef DTB_ADDR
#define DTB_ADDR        0x1f000000
#endif

/* The BCM2836 local controller: the core timer's source and prescaler. */
#define LOCAL_BASE      0x40000000
#define LOCAL_CONTROL   0x00
#define LOCAL_PRESCALER 0x08

/* SCR_EL3: RW (EL2 is AArch64), HCE (HVC allowed), SMD (no SMC), RES1, NS */
#define SCR_EL3_VAL     0x5b1
/* SPSR_EL3: return to EL2h with D, A, I, F masked */
#define SPSR_EL2H       0x3c9

    .section .text.boot, "ax"
    .global _start
_start:
    b       el3_entry

    /* Spin table, at the same address as armstub8's. */
    .org    0xd8
    .global spin_table
spin_table:
    .quad   0, 0, 0, 0

    .org    0x100
el3_entry:
    /*
     * Like armstub8: the core timer counts the 19.2 MHz crystal, one per
     * tick, and the core takes part in coherency (CPUECTLR_EL1.SMPEN).
     * Unlike armstub8, CNTFRQ_EL0 keeps QEMU's value: armstub8 sets 19.2
     * MHz, the crystal's rate, but QEMU's counter runs at its own rate,
     * 62.5 MHz, whatever the prescaler says.
     */
    ldr     x1, =LOCAL_BASE
    str     wzr, [x1, #LOCAL_CONTROL]
    mov     w0, #0x80000000
    str     w0, [x1, #LOCAL_PRESCALER]
    mrs     x0, s3_1_c15_c2_1
    orr     x0, x0, #(1 << 6)
    msr     s3_1_c15_c2_1, x0

    /* Drop to non-secure EL2. */
    ldr     x0, =SCR_EL3_VAL
    msr     scr_el3, x0
    mov     x0, #SPSR_EL2H
    msr     spsr_el3, x0
    adr     x0, el2_entry
    msr     elr_el3, x0
    eret

/* ANCHOR: fwstub_el2 */
el2_entry:
    mrs     x5, mpidr_el1
    and     x5, x5, #0xff
    cbz     x5, primary

    /* Cores 1-3: wait until startup writes our release address. */
    adr     x6, spin_table
    add     x6, x6, x5, lsl #3
3:  wfe
    ldr     x4, [x6]
    cbz     x4, 3b
    mov     x0, xzr
    b       enter

primary:
    ldr     x0, =DTB_ADDR
    ldr     x4, =KERNEL_ADDR
enter:
    mov     x1, xzr
    mov     x2, xzr
    mov     x3, xzr
    br      x4
/* ANCHOR_END: fwstub_el2 */

    .ltorg
