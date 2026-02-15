/* WebbOS Kernel Linker Script */

ENTRY(_start)

KERNEL_OFFSET = 0xFFFF800000000000;

SECTIONS
{
    . = KERNEL_OFFSET + 0x100000;

    .text : AT(ADDR(.text) - KERNEL_OFFSET)
    {
        *(.text .text.*)
    }

    .rodata : AT(ADDR(.rodata) - KERNEL_OFFSET)
    {
        *(.rodata .rodata.*)
    }

    .data : AT(ADDR(.data) - KERNEL_OFFSET)
    {
        *(.data .data.*)
    }

    .bss : AT(ADDR(.bss) - KERNEL_OFFSET)
    {
        *(.bss .bss.*)
        *(COMMON)
    }

    /DISCARD/ :
    {
        *(.comment)
        *(.note*)
    }
}
