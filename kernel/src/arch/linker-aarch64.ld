/* WebbOS Kernel Linker Script for ARM64 (Raspberry Pi 5) */

ENTRY(_start)

/* Raspberry Pi 5 loads bare metal applications at 0x80000 by default */
/* For Linux kernels, it loads at 0x200000, but we're bare metal */
KERNEL_BASE = 0x80000;

/* Memory layout for ARM64 with 48-bit virtual address space */
/* We'll use the higher half mapping similar to x86_64 */
KERNEL_VIRTUAL_BASE = 0xFFFF800000000000;

SECTIONS
{
    /* Start at the physical load address */
    . = KERNEL_BASE;
    
    __kernel_start = .;
    
    .text : AT(ADDR(.text))
    {
        KEEP(*(.text.boot))  /* Boot code must come first */
        *(.text .text.*)
    }
    
    . = ALIGN(4096);
    __rodata_start = .;
    
    .rodata : AT(ADDR(.rodata))
    {
        *(.rodata .rodata.*)
        *(.got .got.*)
    }
    
    . = ALIGN(4096);
    __data_start = .;
    
    .data : AT(ADDR(.data))
    {
        *(.data .data.*)
    }
    
    . = ALIGN(4096);
    __bss_start = .;
    
    .bss : AT(ADDR(.bss))
    {
        *(.bss .bss.*)
        *(COMMON)
    }
    
    . = ALIGN(4096);
    __bss_end = .;
    
    /* Stack space - 128KB for kernel */
    . = ALIGN(4096);
    __stack_bottom = .;
    . += 128 * 1024;  /* 128KB stack */
    __stack_top = .;
    
    /* Heap space - 4MB for kernel */
    . = ALIGN(4096);
    __heap_start = .;
    . += 4 * 1024 * 1024;  /* 4MB heap */
    __heap_end = .;
    
    __kernel_end = .;
    
    /* Discard unnecessary sections */
    /DISCARD/ :
    {
        *(.comment)
        *(.note*)
        *(.eh_frame*)
    }
}

/* Symbol definitions for the kernel */
__kernel_size = __kernel_end - __kernel_start;
__bss_size = __bss_end - __bss_start;