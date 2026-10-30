/* fwstub occupies the first page of RAM, like armstub8 on the real board. */
ENTRY(_start)

PHDRS
{
    boot PT_LOAD FLAGS(7);
}

SECTIONS
{
    . = 0x0;
    .text : {
        KEEP(*(.text.boot))
        *(.text*)
        *(.rodata*)
        *(.data*)
    } :boot

    ASSERT(SIZEOF(.text) <= 0x1000, "fwstub must fit in the first page")
}
