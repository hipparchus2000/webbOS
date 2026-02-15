/* ARM64 boot and exception vector assembly code */

.section .text.boot, "ax"
.global _start
.type _start, @function

_start:
    /* Entry point for ARM64 kernel */
    
    /* Check if we're the primary CPU (CPU0) */
    mrs x0, mpidr_el1
    and x0, x0, #0xFF        /* Extract CPU ID */
    cbz x0, primary_cpu      /* If CPU0, continue */
    
    /* Secondary CPUs: wait for initialization */
secondary_spin:
    wfi                     /* Wait for interrupt */
    b secondary_spin
    
primary_cpu:
    /* Set stack pointer */
    ldr x0, =__stack_top
    mov sp, x0
    
    /* Clear BSS section */
    ldr x0, =__bss_start
    ldr x1, =__bss_end
    sub x1, x1, x0
    cbz x1, bss_clear_done
    
bss_clear_loop:
    str xzr, [x0], #8
    subs x1, x1, #8
    b.gt bss_clear_loop
    
bss_clear_done:
    /* Call Rust kernel main */
    bl kernel_main
    
    /* If kernel_main returns, halt */
    b halt_system

halt_system:
    wfi
    b halt_system

.size _start, . - _start

/* Exception vector table */
.section .exception_vectors, "ax"
.align 11 /* 2KB aligned for VBAR_EL1 */

.global __exception_vectors_start
__exception_vectors_start:

/* Current EL with SP0 */
.align 7
sync_sp0:
    b sync_sp0_handler

.align 7
irq_sp0:
    b irq_sp0_handler

.align 7
fiq_sp0:
    b fiq_sp0_handler

.align 7
serror_sp0:
    b serror_sp0_handler

/* Current EL with SPx */
.align 7
sync_spx:
    b sync_spx_handler

.align 7
irq_spx:
    b irq_spx_handler

.align 7
fiq_spx:
    b fiq_spx_handler

.align 7
serror_spx:
    b serror_spx_handler

/* Lower EL using AArch64 */
.align 7
sync_lower64:
    b sync_lower64_handler

.align 7
irq_lower64:
    b irq_lower64_handler

.align 7
fiq_lower64:
    b fiq_lower64_handler

.align 7
serror_lower64:
    b serror_lower64_handler

/* Lower EL using AArch32 */
.align 7
sync_lower32:
    b sync_lower32_handler

.align 7
irq_lower32:
    b irq_lower32_handler

.align 7
fiq_lower32:
    b fiq_lower32_handler

.align 7
serror_lower32:
    b serror_lower32_handler

/* Exception handlers */
sync_sp0_handler:
    /* Save context */
    stp x0, x1, [sp, #-16]!
    stp x2, x3, [sp, #-16]!
    stp x4, x5, [sp, #-16]!
    stp x6, x7, [sp, #-16]!
    stp x8, x9, [sp, #-16]!
    stp x10, x11, [sp, #-16]!
    stp x12, x13, [sp, #-16]!
    stp x14, x15, [sp, #-16]!
    stp x16, x17, [sp, #-16]!
    stp x18, x19, [sp, #-16]!
    stp x20, x21, [sp, #-16]!
    stp x22, x23, [sp, #-16]!
    stp x24, x25, [sp, #-16]!
    stp x26, x27, [sp, #-16]!
    stp x28, x29, [sp, #-16]!
    
    /* Get exception information */
    mrs x0, esr_el1
    mrs x1, far_el1
    mrs x2, elr_el1
    mrs x3, spsr_el1
    
    /* Call Rust exception handler */
    bl exception_handler_wrapper
    
    /* Restore context */
    ldp x28, x29, [sp], #16
    ldp x26, x27, [sp], #16
    ldp x24, x25, [sp], #16
    ldp x22, x23, [sp], #16
    ldp x20, x21, [sp], #16
    ldp x18, x19, [sp], #16
    ldp x16, x17, [sp], #16
    ldp x14, x15, [sp], #16
    ldp x12, x13, [sp], #16
    ldp x10, x11, [sp], #16
    ldp x8, x9, [sp], #16
    ldp x6, x7, [sp], #16
    ldp x4, x5, [sp], #16
    ldp x2, x3, [sp], #16
    ldp x0, x1, [sp], #16
    
    /* Return from exception */
    eret

/* Other exception handlers (simplified for now) */
irq_sp0_handler:
fiq_sp0_handler:
serror_sp0_handler:
sync_spx_handler:
irq_spx_handler:
fiq_spx_handler:
serror_spx_handler:
sync_lower64_handler:
irq_lower64_handler:
fiq_lower64_handler:
serror_lower64_handler:
sync_lower32_handler:
irq_lower32_handler:
fiq_lower32_handler:
serror_lower32_handler:
    /* For now, all go to generic handler */
    b sync_sp0_handler

/* Wrapper to call Rust exception handler */
.extern exception_handler
.type exception_handler_wrapper, @function
exception_handler_wrapper:
    /* Save link register */
    stp x30, x0, [sp, #-16]!
    
    /* Prepare arguments for Rust function */
    /* x0 already has ESR */
    /* x1 already has FAR */
    /* x2 already has ELR */
    /* x3 already has SPSR */
    
    /* Create ExceptionContext on stack */
    sub sp, sp, #256
    
    /* Save general purpose registers */
    stp x0, x1, [sp, #0]
    stp x2, x3, [sp, #16]
    stp x4, x5, [sp, #32]
    stp x6, x7, [sp, #48]
    stp x8, x9, [sp, #64]
    stp x10, x11, [sp, #80]
    stp x12, x13, [sp, #96]
    stp x14, x15, [sp, #112]
    stp x16, x17, [sp, #128]
    stp x18, x19, [sp, #144]
    stp x20, x21, [sp, #160]
    stp x22, x23, [sp, #176]
    stp x24, x25, [sp, #192]
    stp x26, x27, [sp, #208]
    stp x28, x29, [sp, #224]
    
    /* Save SP, PC, and other registers */
    add x4, sp, #256        /* Original SP */
    mrs x5, elr_el1         /* PC */
    mrs x6, spsr_el1        /* Processor state */
    mrs x7, esr_el1         /* Exception syndrome */
    mrs x8, far_el1         /* Fault address */
    
    stp x4, x5, [sp, #240]
    str x6, [sp, #256]
    str x7, [sp, #264]
    str x8, [sp, #272]
    
    /* Call Rust exception handler */
    mov x0, sp
    bl exception_handler
    
    /* Restore stack */
    add sp, sp, #256
    
    /* Restore link register */
    ldp x30, x0, [sp], #16
    
    ret

.size exception_handler_wrapper, . - exception_handler_wrapper

/* UART functions for early debugging */
.global uart_putc
.type uart_putc, @function
uart_putc:
    /* x0: character to send */
    /* For now, just return - UART will be implemented separately */
    ret
.size uart_putc, . - uart_putc

.global uart_puts
.type uart_puts, @function
uart_puts:
    /* x0: string pointer */
    /* For now, just return */
    ret
.size uart_puts, . - uart_puts