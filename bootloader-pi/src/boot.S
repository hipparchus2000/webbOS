/* Raspberry Pi Bootloader Assembly Stub */

.section .text.boot, "ax"
.global _start
.type _start, @function

/* Entry point - loaded by GPU firmware at 0x80000 */
_start:
    /* Save registers from firmware */
    mov x19, x0        /* Save DTB pointer */
    mov x20, x1        /* Save other registers */
    mov x21, x2
    mov x22, x3
    
    /* Set up stack pointer (grow down from 0x80000) */
    ldr x0, =__stack_top
    mov sp, x0
    
    /* Clear frame pointer */
    mov x29, xzr
    mov x30, xzr
    
    /* Check current exception level */
    mrs x0, CurrentEL
    lsr x0, x0, #2
    cmp x0, #2
    b.eq el2_setup
    cmp x0, #3
    b.eq el3_setup
    
    /* Already at EL1 */
    b el1_entry

/* EL3 (Secure) setup */
el3_setup:
    /* Set up SCR_EL3 */
    mov x0, #0
    msr scr_el3, x0
    
    /* Set up SCTLR_EL2 */
    mrs x0, sctlr_el2
    msr sctlr_el2, x0
    
    /* Return to EL2 */
    mov x0, #0b01001    /* EL2h */
    msr spsr_el3, x0
    adr x0, el2_setup
    msr elr_el3, x0
    eret

/* EL2 (Hypervisor) setup */  
el2_setup:
    /* Enable AArch64 in EL1 */
    mov x0, #(1 << 31)  /* RW bit */
    orr x0, x0, #(1 << 1)   /* SWIO hardwired on */
    msr hcr_el2, x0
    
    /* Set up SCTLR_EL1 */
    mrs x0, sctlr_el1
    msr sctlr_el1, x0
    
    /* Return to EL1 */
    mov x0, #0b00101    /* EL1h, interrupts masked */
    msr spsr_el2, x0
    adr x0, el1_entry
    msr elr_el2, x0
    eret

/* EL1 Entry */
el1_entry:
    /* Restore DTB pointer */
    mov x0, x19
    
    /* Jump to Rust code */
    bl bootloader_main
    
    /* Should not return */
1:  wfe
    b 1b

.size _start, . - _start

/* Stack space */
.section .bss.stack, "aw", @nobits
.align 16
__stack_bottom:
    .space 65536        /* 64KB stack */
__stack_top:
