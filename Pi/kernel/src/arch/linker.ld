/* WebbOS Kernel Linker Script for ARM64/Raspberry Pi */

ENTRY(_start)

/* Kernel virtual base address (higher half) */
KERNEL_VIRT_BASE = 0xFFFF000000000000;
/* Kernel physical load address */
KERNEL_PHYS_BASE = 0x100000;

SECTIONS
{
    /* Virtual address for code */
    . = KERNEL_VIRT_BASE + KERNEL_PHYS_BASE;
    
    /* Entry code and all sections */
    .text : AT(KERNEL_PHYS_BASE) {
        *(.text.boot)
        *(.text .text.*)
    }
    
    .rodata : AT(LOADADDR(.text) + SIZEOF(.text)) {
        *(.rodata .rodata.*)
    }
    
    .data : AT(LOADADDR(.rodata) + SIZEOF(.rodata)) {
        *(.data .data.*)
    }
    
    /* BSS - not loaded from file */
    .bss : AT(LOADADDR(.data) + SIZEOF(.data)) {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        __bss_end = .;
    }
    
    /* Runtime kernel end */
    __kernel_end = .;

    /DISCARD/ :
    {
        *(.comment)
        *(.note*)
    }
}
