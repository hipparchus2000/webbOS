//! ARM64 CPU context management for Raspberry Pi 5
//!
//! Handles saving and restoring CPU registers during context switches.

#![allow(dead_code)]

use crate::println;

/// CPU context for ARM64
///
/// This structure contains all registers that need to be saved/restored
/// during a context switch on ARM64 (AArch64).
#[derive(Debug, Clone, Copy, Default)]
#[repr(C, align(16))]
pub struct Context {
    // General purpose registers x0-x30
    pub x: [u64; 31],
    // Stack pointer (x31)
    pub sp: u64,
    // Program counter
    pub pc: u64,
    // Processor state (flags)
    pub pstate: u64,
    // Floating point registers (optional - for now we don't save/restore)
    // pub v: [u128; 32],
}

impl Context {
    /// Create a new empty context
    pub const fn new() -> Self {
        Self {
            x: [0; 31],
            sp: 0,
            pc: 0,
            pstate: 0x345, // EL1, IRQs enabled: DAIF=0, M[3:0]=0b0101 (EL1h)
        }
    }

    /// Create context for a new kernel thread
    pub fn new_kernel_thread(entry: fn() -> !, stack_top: u64) -> Self {
        let mut ctx = Self::new();
        ctx.pc = entry as u64;
        ctx.sp = stack_top;
        ctx.x[30] = exit_thread as u64; // LR = exit handler
        ctx
    }

    /// Create context for a new user thread
    pub fn new_user_thread(entry: u64, stack_top: u64) -> Self {
        let mut ctx = Self::new();
        ctx.pc = entry;
        ctx.sp = stack_top;
        ctx.pstate = 0x345; // EL0, IRQs enabled: M[3:0]=0b0000 (EL0t)
        ctx.x[30] = 0; // Will be set up by syscall return
        ctx
    }
}

/// Thread exit handler
extern "C" fn exit_thread() -> ! {
    use super::{ThreadState, THREADS};
    use super::scheduler;
    
    // Get current thread and mark as terminated
    let current_tid = scheduler::current_thread();
    if let Some(tid) = current_tid {
        let mut threads = THREADS.lock();
        if let Some(thread) = threads.get_mut(&tid.as_u64()) {
            thread.state = ThreadState::Terminated;
        }
    }
    
    // Schedule next thread
    unsafe {
        scheduler::schedule_next();
    }
    
    // Should never reach here
    loop {
        unsafe { core::arch::asm!("wfe", options(nomem, nostack)); }
    }
}

/// Save current context and restore new context
///
/// # Safety
/// This function is unsafe because it performs a context switch.
/// The caller must ensure:
/// - old_ctx is a valid, writable pointer
/// - new_ctx is a valid, readable pointer
/// - Interrupts are disabled before calling
#[naked]
pub unsafe extern "C" fn switch_context(old_ctx: *mut Context, new_ctx: *const Context) {
    core::arch::naked_asm!(
        // Save general purpose registers x19-x29 (callee-saved)
        "stp x19, x20, [x0, #0x00]",
        "stp x21, x22, [x0, #0x10]",
        "stp x23, x24, [x0, #0x20]",
        "stp x25, x26, [x0, #0x30]",
        "stp x27, x28, [x0, #0x40]",
        "stp x29, x30, [x0, #0x50]",
        
        // Save SP and PC
        "mov x2, sp",
        "str x2, [x0, #0xF8]",      // sp offset
        "adr x2, 1f",               // Get PC (return address)
        "str x2, [x0, #0xF0]",      // pc offset
        
        // Save PSTATE (using MRS)
        "mrs x2, SPSR_EL1",
        "str x2, [x0, #0x100]",     // pstate offset
        
        // Restore context from new_ctx (x1)
        "1:",
        "ldp x19, x20, [x1, #0x00]",
        "ldp x21, x22, [x1, #0x10]",
        "ldp x23, x24, [x1, #0x20]",
        "ldp x25, x26, [x1, #0x30]",
        "ldp x27, x28, [x1, #0x40]",
        "ldp x29, x30, [x1, #0x50]",
        
        // Restore SP
        "ldr x2, [x1, #0xF8]",
        "mov sp, x2",
        
        // Restore PC (branch to it)
        "ldr x2, [x1, #0xF0]",
        "br x2",
    );
}

/// Save current context without switching
///
/// # Safety
/// ctx must be a valid, writable pointer
#[naked]
pub unsafe extern "C" fn save_context(ctx: *mut Context) {
    core::arch::naked_asm!(
        // Save callee-saved registers
        "stp x19, x20, [x0, #0x00]",
        "stp x21, x22, [x0, #0x10]",
        "stp x23, x24, [x0, #0x20]",
        "stp x25, x26, [x0, #0x30]",
        "stp x27, x28, [x0, #0x40]",
        "stp x29, x30, [x0, #0x50]",
        
        // Save SP
        "mov x1, sp",
        "str x1, [x0, #0xF8]",
        
        // Save PC (return address)
        "adr x1, 1f",
        "str x1, [x0, #0xF0]",
        
        // Save PSTATE
        "mrs x1, SPSR_EL1",
        "str x1, [x0, #0x100]",
        
        "1:",
        "ret",
    );
}

/// Restore context and return
///
/// # Safety
/// ctx must be a valid, readable pointer
#[naked]
pub unsafe extern "C" fn restore_context(ctx: *const Context) -> ! {
    core::arch::naked_asm!(
        // Restore registers
        "ldp x19, x20, [x0, #0x00]",
        "ldp x21, x22, [x0, #0x10]",
        "ldp x23, x24, [x0, #0x20]",
        "ldp x25, x26, [x0, #0x30]",
        "ldp x27, x28, [x0, #0x40]",
        "ldp x29, x30, [x0, #0x50]",
        
        // Restore SP
        "ldr x1, [x0, #0xF8]",
        "mov sp, x1",
        
        // Restore PC and jump
        "ldr x1, [x0, #0xF0]",
        "br x1",
    );
}

/// Initialize a kernel thread's stack
///
/// Sets up the initial stack frame for a new kernel thread.
pub unsafe fn init_kernel_stack(stack_top: u64, _entry: fn() -> !, _arg: u64) -> u64 {
    let mut sp = stack_top;
    
    // On ARM64, we don't need to set up the stack for the initial switch
    // because switch_context expects to save current context first
    // The thread will start at 'entry' when first scheduled
    
    // Align stack to 16 bytes (AArch64 ABI requirement)
    sp = sp & !0xF;
    
    sp
}

/// Print context for debugging
pub fn print_context(ctx: &Context) {
    println!("ARM64 Context:");
    for i in (0..31).step_by(2) {
        println!("  X{:02}={:016X} X{:02}={:016X}", 
            i, ctx.x[i], i+1, ctx.x[i+1]);
    }
    println!("  SP ={:016X} PC ={:016X} PSTATE={:016X}",
        ctx.sp, ctx.pc, ctx.pstate);
}

/// Enable interrupts
pub fn enable_interrupts() {
    unsafe {
        core::arch::asm!("msr DAIFClr, #0xF");
    }
}

/// Disable interrupts
pub fn disable_interrupts() {
    unsafe {
        core::arch::asm!("msr DAIFSet, #0xF");
    }
}

/// Check if interrupts are enabled
pub fn interrupts_enabled() -> bool {
    let daif: u64;
    unsafe {
        core::arch::asm!("mrs {}, DAIF", out(reg) daif);
    }
    // Bits 7:0 are I, F, A, D (IRQ, FIQ, SError, Debug)
    (daif & 0xC0) == 0 // I and F bits clear means interrupts enabled
}
