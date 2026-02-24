/* WebbOS Pi Bootloader Linker Script */

ENTRY(_start)

SECTIONS
{
    /* Bootloader loaded at 0x80000 by Pi firmware */
    . = 0x80000;
    
    .text : {
        *(.text.boot)
        *(.text .text.*)
    }
    
    .rodata : {
        *(.rodata .rodata.*)
    }
    
    .data : {
        *(.data .data.*)
    }
    
    .bss : {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        __bss_end = .;
    }
    
    /* Stack grows down from here */
    __stack_top = . + 0x10000;
}
