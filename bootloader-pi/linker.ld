/* Raspberry Pi Bootloader Linker Script */

ENTRY(_start)

SECTIONS {
    /* Loaded by GPU firmware at 0x80000 */
    . = 0x80000;
    
    .text : {
        KEEP(*(.text.boot))
        *(.text .text.*)
    }
    
    .rodata : {
        *(.rodata .rodata.*)
    }
    
    .data : {
        *(.data .data.*)
    }
    
    .bss (NOLOAD) : {
        . = ALIGN(16);
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        . = ALIGN(16);
        __bss_end = .;
    }
    
    /* Stack grows down from here */
    __stack_top = 0x80000;
    
    /DISCARD/ : {
        *(.comment)
        *(.gnu.*)
        *(.note*)
        *(.eh_frame*)
    }
}
