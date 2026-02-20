//! CPU context management
//!
//! Handles saving and restoring CPU registers during context switches.

use crate::println;

/// CPU context for x86_64
///
/// This structure contains all registers that need to be saved/restored
/// during a context switch.
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct Context {
    // General purpose registers
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rbp: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rdx: u64,
    pub rcx: u64,
    pub rbx: u64,
    pub rax: u64,
    // Stack pointer
    pub rsp: u64,
    // Instruction pointer
    pub rip: u64,
    // Flags register
    pub rflags: u64,
    // Segment selectors
    pub cs: u64,
    pub ss: u64,
}

impl Context {
    /// Create a new empty context
    pub const fn new() -> Self {
        Self {
            r15: 0,
            r14: 0,
            r13: 0,
            r12: 0,
            r11: 0,
            r10: 0,
            r9: 0,
            r8: 0,
            rbp: 0,
            rdi: 0,
            rsi: 0,
            rdx: 0,
            rcx: 0,
            rbx: 0,
            rax: 0,
            rsp: 0,
            rip: 0,
            rflags: 0x202, // Interrupt enable
            cs: 0x08,       // Kernel code segment
            ss: 0x10,       // Kernel data segment
        }
    }

    /// Create context for a new kernel thread
    pub fn new_kernel_thread(entry: fn() -> !, stack_top: u64) -> Self {
        let mut ctx = Self::new();
        ctx.rip = entry as u64;
        ctx.rsp = stack_top;
        ctx
    }

    /// Create context for a new user thread
    pub fn new_user_thread(entry: u64, stack_top: u64, user_code_seg: u64, user_data_seg: u64) -> Self {
        let mut ctx = Self::new();
        ctx.rip = entry;
        ctx.rsp = stack_top;
        ctx.cs = user_code_seg | 3; // Ring 3
        ctx.ss = user_data_seg | 3; // Ring 3
        ctx.rflags = 0x202;         // Interrupt enable, IOPL=0
        ctx
    }
}

/// Save current context to the given Context structure
/// 
/// # Safety
/// This is unsafe because it manipulates CPU registers directly.
#[naked]
pub unsafe extern "C" fn save_context(ctx: *mut Context) {
    core::arch::naked_asm!(
        // Save all registers to the context structure
        "mov [rdi + 0x00], r15",
        "mov [rdi + 0x08], r14",
        "mov [rdi + 0x10], r13",
        "mov [rdi + 0x18], r12",
        "mov [rdi + 0x20], r11",
        "mov [rdi + 0x28], r10",
        "mov [rdi + 0x30], r9",
        "mov [rdi + 0x38], r8",
        "mov [rdi + 0x40], rbp",
        "mov [rdi + 0x48], rdi",
        "mov [rdi + 0x50], rsi",
        "mov [rdi + 0x58], rdx",
        "mov [rdi + 0x60], rcx",
        "mov [rdi + 0x68], rbx",
        "mov [rdi + 0x70], rax",
        // Save stack pointer (from interrupt frame)
        "mov rax, rsp",
        "add rax, 8", // Skip return address
        "mov [rdi + 0x78], rax",
        "ret",
    );
}

/// Restore context from the given Context structure
///
/// # Safety
/// This is unsafe because it manipulates CPU registers directly.
#[naked]
pub unsafe extern "C" fn restore_context(ctx: *const Context) -> ! {
    core::arch::naked_asm!(
        // Restore all registers from the context structure
        "mov r15, [rdi + 0x00]",
        "mov r14, [rdi + 0x08]",
        "mov r13, [rdi + 0x10]",
        "mov r12, [rdi + 0x18]",
        "mov r11, [rdi + 0x20]",
        "mov r10, [rdi + 0x28]",
        "mov r9, [rdi + 0x30]",
        "mov r8, [rdi + 0x38]",
        "mov rbp, [rdi + 0x40]",
        // Skip rdi for now (we need it)
        "mov rsi, [rdi + 0x50]",
        "mov rdx, [rdi + 0x58]",
        "mov rcx, [rdi + 0x60]",
        "mov rbx, [rdi + 0x68]",
        "mov rax, [rdi + 0x70]",
        // Now restore rdi
        "mov rdi, [rdi + 0x48]",
        // Switch to new stack
        "mov rsp, [rsp + 0x78 - 0x48]", // rsp offset - rdi offset
        // Jump to new instruction pointer
        "ret",
    );
}

/// Switch context from old to new
///
/// # Safety
/// This is unsafe because it manipulates CPU registers and stack directly.
pub unsafe fn switch_context(old: *mut Context, new: *const Context) {
    // Save current context
    save_context(old);
    // Restore new context
    restore_context(new);
}

/// Initialize a kernel thread's stack
///
/// Sets up the initial stack frame for a new kernel thread.
pub unsafe fn init_kernel_stack(stack_top: u64, entry: fn() -> !, arg: u64) -> u64 {
    let mut rsp = stack_top;

    // Push return address (entry point)
    rsp -= 8;
    core::ptr::write(rsp as *mut u64, entry as u64);

    // Push argument
    rsp -= 8;
    core::ptr::write(rsp as *mut u64, arg);

    // Push dummy values for other registers
    for _ in 0..15 {
        rsp -= 8;
        core::ptr::write(rsp as *mut u64, 0);
    }

    rsp
}

/// Print context for debugging
pub fn print_context(ctx: &Context) {
    println!("Context:");
    println!("  RAX={:016X} RBX={:016X} RCX={:016X} RDX={:016X}",
        ctx.rax, ctx.rbx, ctx.rcx, ctx.rdx);
    println!("  RSI={:016X} RDI={:016X} RBP={:016X} RSP={:016X}",
        ctx.rsi, ctx.rdi, ctx.rbp, ctx.rsp);
    println!("  R8 ={:016X} R9 ={:016X} R10={:016X} R11={:016X}",
        ctx.r8, ctx.r9, ctx.r10, ctx.r11);
    println!("  R12={:016X} R13={:016X} R14={:016X} R15={:016X}",
        ctx.r12, ctx.r13, ctx.r14, ctx.r15);
    println!("  RIP={:016X} RFLAGS={:016X} CS={:04X} SS={:04X}",
        ctx.rip, ctx.rflags, ctx.cs, ctx.ss);
}
