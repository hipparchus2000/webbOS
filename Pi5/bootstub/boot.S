// Boot stub for Raspberry Pi 3/4 ARM64
// This file provides the initial entry point when the Pi boots
//
// The GPU firmware loads kernel8.img at address 0x80000 and jumps to it
// with the following state:
// - CPU0 running, other CPUs halted
// - EL2 (Hypervisor mode) or EL3 (Secure mode)
// - MMU disabled, caches disabled
// - x0 = device tree blob physical address

.section ".text.boot"
.balign 8
.global _start

_start:
    // Save DTB pointer
    mov x19, x0

    // Check which EL we're in
    mrs x0, CurrentEL
    lsr x0, x0, #2
    cmp x0, #2
    beq in_el2
    cmp x0, #1
    beq in_el1
    
    // If we're in EL3, set up and drop to EL2
    b setup_el3

setup_el3:
    // Configure SCR_EL3
    mov x0, #0x5b1
    msr SCR_EL3, x0
    
    // Set SPSR_EL3 for EL2
    mov x0, #0x3c9
    msr SPSR_EL3, x0
    
    // Set return address
    adr x0, in_el2
    msr ELR_EL3, x0
    eret

in_el2:
    // Configure HCR_EL2 to route exceptions to EL1
    mov x0, #0x80000000
    orr x0, x0, #0x00000002
    msr HCR_EL2, x0
    
    // Set up SPSR_EL2 for EL1
    mov x0, #0x3c5
    msr SPSR_EL2, x0
    
    // Set return address for EL1
    adr x0, in_el1
    msr ELR_EL2, x0
    eret

in_el1:
    // Now in EL1
    // Restore DTB pointer
    mov x0, x19
    
    // Set up stack
    ldr x1, =0x80000
    mov sp, x1
    
    // Clear BSS
    ldr x1, =__bss_start
    ldr x2, =__bss_end
    sub x2, x2, x1
    cbz x2, bss_done

clear_bss:
    str xzr, [x1], #8
    sub x2, x2, #8
    cbnz x2, clear_bss

bss_done:
    // Jump to Rust main
    b rust_main

// Infinite loop
the_end:
    wfe
    b the_end
