//! CPU context management
//!
//! Handles saving and restoring CPU registers during context switches.

use crate::println;

/// CPU context for x86_64
///
/// This structure matches the layout of registers saved on the stack
/// during a context switch.
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct Context {
    // Callee-saved registers (System V AMD64 ABI)
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub rbx: u64,
    pub rbp: u64,
    
    // Return address (RIP) pushed by call instruction
    pub rip: u64,
}

impl Context {
    /// Create a new empty context
    pub const fn new() -> Self {
        Self {
            r15: 0,
            r14: 0,
            r13: 0,
            r12: 0,
            rbx: 0,
            rbp: 0,
            rip: 0,
        }
    }
}

/// Switch context from old to new
///
/// # Safety
/// This is unsafe because it performs a raw context switch, changing the
/// stack pointer and instruction pointer.
/// 
/// Arguments:
/// - rdi: *mut Context (old) - pointer to where we save current context
/// - rsi: *const Context (new) - pointer to context we want to restore
#[naked]
#[no_mangle]
pub unsafe extern "C" fn switch_context(old_rsp: *mut u64, new_rsp: u64) {
    core::arch::naked_asm!(
        // Save callee-saved registers
        "push rbp",
        "push rbx",
        "push r12",
        "push r13",
        "push r14",
        "push r15",
        
        // Save current stack pointer to the memory location pointed to by RDI (old_rsp)
        // RDI contains &mut u64 (pointer to Thread.kernel_stack)
        "mov [rdi], rsp",
        
        // Switch to new stack (RSI contains new_rsp value)
        "mov rsp, rsi",
        
        // Restore callee-saved registers from new stack
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop rbx",
        "pop rbp",
        
        // Return to the address on the new stack
        "ret"
    );
}

/// Initialize a kernel thread's stack
///
/// Sets up the initial stack frame for a new kernel thread.
/// 
/// Stack layout (growing down):
/// [entry_point]  <- rip (ret will jump here)
/// [rbp]          <- Initial values for registers
/// [rbx]
/// ...
/// [r15]          <- New RSP points here
pub unsafe fn init_kernel_stack(stack_top: u64, entry: fn() -> !) -> u64 {
    let mut rsp = stack_top;

    // Align stack to 16 bytes
    rsp = rsp & !0xF;

    // Push return address (entry point)
    rsp -= 8;
    core::ptr::write(rsp as *mut u64, entry as u64);

    // Push initial values for callee-saved registers (6 registers)
    // rbp, rbx, r12, r13, r14, r15
    for _ in 0..6 {
        rsp -= 8;
        core::ptr::write(rsp as *mut u64, 0);
    }

    rsp
}

/// Print context for debugging (Stub)
pub fn print_context(_ctx: &Context) {
    println!("Context printing not implemented implementation opaqueness");
}
