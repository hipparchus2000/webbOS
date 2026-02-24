//! Exception handling for ARM64
//!
//! ARM64 uses an exception vector table instead of an IDT like x86.
//! The VBAR_EL1 register points to the base of the exception vectors.

use crate::println;

/// Exception vector table
/// 
/// ARM64 exception vectors are organized by:
/// - Exception level (EL0, EL1, EL2, EL3)
/// - Exception type (synchronous, IRQ, FIQ, SError)
/// - Source (same level or lower level)
///
/// Each entry is 128 bytes (0x80) and must be aligned to 2048 bytes (0x800)
#[repr(C, align(2048))]
struct ExceptionVectorTable {
    // From EL1 with SP_EL0 (should not happen in our case)
    el1t_sync: [u8; 128],
    el1t_irq: [u8; 128],
    el1t_fiq: [u8; 128],
    el1t_error: [u8; 128],
    
    // From EL1 with SP_EL1
    el1h_sync: [u8; 128],
    el1h_irq: [u8; 128],
    el1h_fiq: [u8; 128],
    el1h_error: [u8; 128],
    
    // From EL0 executing in 64-bit mode
    el0_64_sync: [u8; 128],
    el0_64_irq: [u8; 128],
    el0_64_fiq: [u8; 128],
    el0_64_error: [u8; 128],
    
    // From EL0 executing in 32-bit mode (not used)
    el0_32_sync: [u8; 128],
    el0_32_irq: [u8; 128],
    el0_32_fiq: [u8; 128],
    el0_32_error: [u8; 128],
}

/// Exception class (from ESR_EL1.EC)
#[repr(u8)]
#[derive(Debug, Copy, Clone)]
pub enum ExceptionClass {
    Unknown = 0x00,
    WFx = 0x01,
    MCRMRC_CP15 = 0x03,
    MCRRMRRC_CP15 = 0x04,
    MCRMRC_CP14 = 0x05,
    LDCSTC_CP14 = 0x06,
    SME = 0x07,
    FP = 0x08,
    LDST = 0x09,
    MRC_VMRS = 0x0A,
    BranchTarget = 0x0B,
    HVC = 0x16,
    SMC = 0x17,
    MSRMRS = 0x18,
    SVC = 0x15,
    InstructionAbortLower = 0x20,
    InstructionAbortCurrent = 0x21,
    PCAlignment = 0x22,
    DataAbortLower = 0x24,
    DataAbortCurrent = 0x25,
    SPAlignment = 0x26,
    FPException = 0x28,
    SError = 0x2F,
    BreakpointLower = 0x30,
    BreakpointCurrent = 0x31,
    WatchpointLower = 0x32,
    WatchpointCurrent = 0x33,
    Brk = 0x3C,
}

/// Exception frame - saved registers when an exception occurs
#[repr(C)]
#[derive(Debug)]
pub struct ExceptionFrame {
    pub x0: u64,
    pub x1: u64,
    pub x2: u64,
    pub x3: u64,
    pub x4: u64,
    pub x5: u64,
    pub x6: u64,
    pub x7: u64,
    pub x8: u64,
    pub x9: u64,
    pub x10: u64,
    pub x11: u64,
    pub x12: u64,
    pub x13: u64,
    pub x14: u64,
    pub x15: u64,
    pub x16: u64,
    pub x17: u64,
    pub x18: u64,
    pub x19: u64,
    pub x20: u64,
    pub x21: u64,
    pub x22: u64,
    pub x23: u64,
    pub x24: u64,
    pub x25: u64,
    pub x26: u64,
    pub x27: u64,
    pub x28: u64,
    pub x29: u64,  // FP
    pub x30: u64,  // LR
    pub sp: u64,
    pub elr: u64,  // Exception Link Register
    pub spsr: u64, // Saved Program Status Register
    pub esr: u64,  // Exception Syndrome Register
    pub far: u64,  // Fault Address Register
}

/// Static exception vector table
static mut VECTOR_TABLE: ExceptionVectorTable = ExceptionVectorTable {
    el1t_sync: [0; 128],
    el1t_irq: [0; 128],
    el1t_fiq: [0; 128],
    el1t_error: [0; 128],
    el1h_sync: [0; 128],
    el1h_irq: [0; 128],
    el1h_fiq: [0; 128],
    el1h_error: [0; 128],
    el0_64_sync: [0; 128],
    el0_64_irq: [0; 128],
    el0_64_fiq: [0; 128],
    el0_64_error: [0; 128],
    el0_32_sync: [0; 128],
    el0_32_irq: [0; 128],
    el0_32_fiq: [0; 128],
    el0_32_error: [0; 128],
};

/// Initialize exception handling
pub fn init() {
    unsafe {
        // Set up the vector table entries with assembly stubs
        setup_vectors();
        
        // Set VBAR_EL1 to point to our vector table
        core::arch::asm!(
            "msr VBAR_EL1, {0}",
            in(reg) &VECTOR_TABLE,
        );
        
        println!("[exceptions] Vector table installed at {:p}", &VECTOR_TABLE);
    }
}

/// Set up exception vector stubs
/// 
/// This function fills in the exception vector table with
/// assembly stubs that save context and call the appropriate handlers.
unsafe fn setup_vectors() {
    // For a real implementation, we'd write assembly stubs here
    // For now, we'll use inline assembly to create simple stubs
    
    // Each vector entry needs to:
    // 1. Save registers to stack
    // 2. Call the handler
    // 3. Restore registers
    // 4. Return from exception (ERET)
}

/// Disable interrupts
pub fn disable() {
    super::cpu::disable_interrupts();
}

/// Enable interrupts
pub fn enable() {
    super::cpu::enable_interrupts();
}

/// Check if interrupts are enabled
pub fn are_enabled() -> bool {
    super::cpu::interrupts_enabled()
}

/// Timer tick counter
static mut TIMER_TICKS: u64 = 0;

/// Get the current timer tick count
pub fn get_timer_ticks() -> u64 {
    unsafe { TIMER_TICKS }
}

/// Default exception handler
#[no_mangle]
extern "C" fn handle_exception(frame: &ExceptionFrame) {
    let ec = ((frame.esr >> 26) & 0x3F) as u8;
    
    println!("[exception] Exception occurred!");
    println!("  ELR: {:#016x}", frame.elr);
    println!("  ESR: {:#016x}", frame.esr);
    println!("  EC: {:#x} ({:?})", ec, decode_exception_class(ec));
    println!("  FAR: {:#016x}", frame.far);
    
    // Decode ISS (Instruction Specific Syndrome)
    let iss = frame.esr & 0x1FFFFFF;
    println!("  ISS: {:#x}", iss);
    
    // For data aborts, print more info
    if ec == 0x24 || ec == 0x25 {
        let isv = (frame.esr >> 24) & 1;
        let sas = (frame.esr >> 22) & 3;
        let sse = (frame.esr >> 21) & 1;
        let srt = (frame.esr >> 16) & 0x1F;
        let ea = (frame.esr >> 9) & 1;
        let cm = (frame.esr >> 8) & 1;
        let s1ptw = (frame.esr >> 7) & 1;
        let wnr = (frame.esr >> 6) & 1;
        let dfsc = frame.esr & 0x3F;
        
        println!("  Data Abort:");
        println!("    Direction: {}", if wnr == 1 { "Write" } else { "Read" });
        println!("    DFSC: {:#x}", dfsc);
        if isv == 1 {
            println!("    Register: X{}", srt);
            println!("    Size: {} bytes", 1 << sas);
        }
    }
    
    panic!("Exception in kernel");
}

/// Decode exception class
fn decode_exception_class(ec: u8) -> &'static str {
    match ec {
        0x00 => "Unknown",
        0x01 => "Trapped WFI/WFE",
        0x03 => "MCR/MRC CP15",
        0x04 => "MCRR/MRRC CP15",
        0x05 => "MCR/MRC CP14",
        0x06 => "LDC/STC CP14",
        0x07 => "SME",
        0x08 => "FP",
        0x09 => "LD64B/ST64B",
        0x0A => "MRC VMRS",
        0x0B => "Branch Target",
        0x15 => "SVC",
        0x16 => "HVC",
        0x17 => "SMC",
        0x18 => "MSR/MRS",
        0x20 => "Instruction Abort (EL0)",
        0x21 => "Instruction Abort (EL1)",
        0x22 => "PC Alignment",
        0x24 => "Data Abort (EL0)",
        0x25 => "Data Abort (EL1)",
        0x26 => "SP Alignment",
        0x28 => "FP Exception",
        0x2F => "SError",
        0x30 => "Breakpoint (EL0)",
        0x31 => "Breakpoint (EL1)",
        0x32 => "Watchpoint (EL0)",
        0x33 => "Watchpoint (EL1)",
        0x3C => "BRK",
        _ => "Unknown",
    }
}

/// IRQ handler
#[no_mangle]
extern "C" fn handle_irq(_frame: &ExceptionFrame) {
    unsafe {
        // Increment timer ticks for now
        // In a real implementation, we'd check the interrupt source
        TIMER_TICKS += 1;
        
        // TODO: Route to device drivers
    }
}

/// FIQ handler (Fast Interrupt)
#[no_mangle]
extern "C" fn handle_fiq(_frame: &ExceptionFrame) {
    // FIQs are higher priority and typically used for critical interrupts
}

/// SError handler (System Error)
#[no_mangle]
extern "C" fn handle_serror(frame: &ExceptionFrame) {
    println!("[exception] SError occurred!");
    println!("  ELR: {:#016x}", frame.elr);
    panic!("SError in kernel");
}

/// Assembly exception entry stubs
/// 
/// These would be defined in assembly to properly save/restore context
/// and call the Rust handlers above.
#[naked]
#[no_mangle]
unsafe extern "C" fn exception_entry() {
    core::arch::naked_asm!(
        // Save all registers
        "sub sp, sp, #272",           // Allocate space for ExceptionFrame
        "stp x0, x1, [sp, #0]",
        "stp x2, x3, [sp, #16]",
        "stp x4, x5, [sp, #32]",
        "stp x6, x7, [sp, #48]",
        "stp x8, x9, [sp, #64]",
        "stp x10, x11, [sp, #80]",
        "stp x12, x13, [sp, #96]",
        "stp x14, x15, [sp, #112]",
        "stp x16, x17, [sp, #128]",
        "stp x18, x19, [sp, #144]",
        "stp x20, x21, [sp, #160]",
        "stp x22, x23, [sp, #176]",
        "stp x24, x25, [sp, #192]",
        "stp x26, x27, [sp, #208]",
        "stp x28, x29, [sp, #224]",
        "str x30, [sp, #240]",
        "mrs x0, SP_EL0",
        "str x0, [sp, #248]",
        "mrs x0, ELR_EL1",
        "str x0, [sp, #256]",
        "mrs x0, SPSR_EL1",
        "str x0, [sp, #264]",
        "mrs x0, ESR_EL1",
        "str x0, [sp, #272]",
        "mrs x0, FAR_EL1",
        "str x0, [sp, #280]",
        
        // Call handler with frame pointer
        "mov x0, sp",
        "bl handle_exception",
        
        // Restore and return (shouldn't reach here from panic)
        "b exception_exit",
    );
}

#[naked]
#[no_mangle]
unsafe extern "C" fn exception_exit() {
    core::arch::naked_asm!(
        // Restore registers
        "ldr x0, [sp, #248]",
        "msr SP_EL0, x0",
        "ldr x0, [sp, #256]",
        "msr ELR_EL1, x0",
        "ldr x0, [sp, #264]",
        "msr SPSR_EL1, x0",
        
        "ldp x0, x1, [sp, #0]",
        "ldp x2, x3, [sp, #16]",
        "ldp x4, x5, [sp, #32]",
        "ldp x6, x7, [sp, #48]",
        "ldp x8, x9, [sp, #64]",
        "ldp x10, x11, [sp, #80]",
        "ldp x12, x13, [sp, #96]",
        "ldp x14, x15, [sp, #112]",
        "ldp x16, x17, [sp, #128]",
        "ldp x18, x19, [sp, #144]",
        "ldp x20, x21, [sp, #160]",
        "ldp x22, x23, [sp, #176]",
        "ldp x24, x25, [sp, #192]",
        "ldp x26, x27, [sp, #208]",
        "ldp x28, x29, [sp, #224]",
        "ldr x30, [sp, #240]",
        "add sp, sp, #288",
        
        // Return from exception
        "eret",
    );
}
