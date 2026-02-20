//! Raspberry Pi ARM64 Boot Entry Point
//!
//! This is the very first code executed when the Raspberry Pi boots.
//! The GPU firmware loads kernel8.img at address 0x80000 and jumps to it.
//! 
//! On entry:
//! - x0 = Device tree blob (DTB) physical address (0x100 for older Pi, varies for Pi4)
//! - We are in EL2 (Hypervisor mode) on Pi3, or EL2 on Pi4
//! - MMU is disabled
//! - Caches are disabled

use core::arch::global_asm;

// Assembly entry point - must be at the very beginning of kernel8.img
global_asm!(
    r#"
    // Entry point at 0x80000
    .section .text.boot, "ax"
    .global _start
    .balign 8

_start:
    // Save DTB pointer (x0) across initialization
    mov x19, x0

    // Check which EL (Exception Level) we're in
    mrs x0, CurrentEL
    lsr x0, x0, #2
    and x0, x0, #3
    
    // If we're in EL2, set up EL1
    cmp x0, #2
    beq setup_el1
    
    // If already in EL1, skip setup
    cmp x0, #1
    beq el1_entry
    
    // If in EL3 (unlikely on Pi), configure and drop to EL2 then EL1
    b setup_el3

// Configure EL3 and drop to EL2
setup_el3:
    // Set up SCR_EL3 (Secure Configuration Register)
    // NS=1 (Non-secure), HCE=1 (HVC enabled), SMD=0 (SMC disabled)
    mov x0, #0x5b1
    msr SCR_EL3, x0
    
    // Set up SPSR_EL3 (Saved Program Status Register)
    // DAIF masked, EL2h mode
    mov x0, #0x3c9
    msr SPSR_EL3, x0
    
    // Set ELR_EL3 to return to setup_el1
    adr x0, setup_el1
    msr ELR_EL3, x0
    eret

// Configure EL2 and drop to EL1
setup_el1:
    // Configure Hypervisor Configuration Register
    // RW=1 (AArch64 for lower ELs), SWIO=1
    mov x0, #0x80000000
    orr x0, x0, #0x00000002
    msr HCR_EL2, x0
    
    // Configure EL2 to EL1 transition
    // M[3:0] = 0b0101 (EL1h), all interrupts masked
    mov x0, #0x3c5
    msr SPSR_EL2, x0
    
    // Set ELR_EL2 to el1_entry
    adr x0, el1_entry
    msr ELR_EL2, x0
    eret

// Now in EL1, set up basic environment
el1_entry:
    // Restore DTB pointer
    mov x0, x19
    
    // Set up stack pointer (temporary stack in low memory)
    // Stack grows down from 0x80000 (our load address)
    ldr x1, =0x80000
    mov sp, x1
    
    // Clear BSS section
    ldr x1, =__bss_start
    ldr x2, =__bss_end
    sub x2, x2, x1
    cbz x2, bss_clear_done

bss_clear_loop:
    str xzr, [x1], #8
    sub x2, x2, #8
    cbnz x2, bss_clear_loop

bss_clear_done:
    // Jump to Rust main with DTB pointer in x0
    b rust_main

    // Infinite loop if rust_main returns
halt:
    wfe
    b halt

    .balign 8
    .global __bss_start
    .global __bss_end
    "#
);

extern "C" {
    static __bss_start: u64;
    static __bss_end: u64;
}
