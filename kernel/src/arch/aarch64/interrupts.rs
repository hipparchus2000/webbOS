//! ARM64 Interrupt handling
//!
//! This module handles exceptions and interrupts on ARM64 architecture.
//! ARM uses a unified exception model with different exception types.

use core::arch::{asm, global_asm};
use crate::println;

// Include the exception vector table assembly
global_asm!(
    r#"
    .section .exception_vectors, "ax"
    .align 11 // 2KB aligned for VBAR_EL1
    
    .global __exception_vectors_start
    __exception_vectors_start:
    
    // Current EL with SP0
    .align 7
    b vector_sync_sp0
    
    .align 7
    b vector_irq_sp0
    
    .align 7
    b vector_fiq_sp0
    
    .align 7
    b vector_serror_sp0
    
    // Current EL with SPx
    .align 7
    b vector_sync_spx
    
    .align 7
    b vector_irq_spx
    
    .align 7
    b vector_fiq_spx
    
    .align 7
    b vector_serror_spx
    
    // Lower EL using AArch64
    .align 7
    b vector_sync_lower64
    
    .align 7
    b vector_irq_lower64
    
    .align 7
    b vector_fiq_lower64
    
    .align 7
    b vector_serror_lower64
    
    // Lower EL using AArch32
    .align 7
    b vector_sync_lower32
    
    .align 7
    b vector_irq_lower32
    
    .align 7
    b vector_fiq_lower32
    
    .align 7
    b vector_serror_lower32
    "#
);

// Handler functions that delegate to Rust
#[no_mangle]
pub extern "C" fn vector_sync_sp0() {
    panic!("Synchronous exception with SP0");
}

#[no_mangle]
pub extern "C" fn vector_irq_sp0() {
    panic!("IRQ with SP0");
}

#[no_mangle]
pub extern "C" fn vector_fiq_sp0() {
    panic!("FIQ with SP0");
}

#[no_mangle]
pub extern "C" fn vector_serror_sp0() {
    panic!("SError with SP0");
}

#[no_mangle]
pub extern "C" fn vector_sync_spx() {
    vector_handle_exception();
}

#[no_mangle]
pub extern "C" fn vector_irq_spx() {
    vector_handle_irq();
}

#[no_mangle]
pub extern "C" fn vector_fiq_spx() {
    vector_handle_fiq();
}

#[no_mangle]
pub extern "C" fn vector_serror_spx() {
    panic!("SError with SPx");
}

#[no_mangle]
pub extern "C" fn vector_sync_lower64() {
    vector_handle_exception();
}

#[no_mangle]
pub extern "C" fn vector_irq_lower64() {
    vector_handle_irq();
}

#[no_mangle]
pub extern "C" fn vector_fiq_lower64() {
    vector_handle_fiq();
}

#[no_mangle]
pub extern "C" fn vector_serror_lower64() {
    panic!("SError from lower EL (AArch64)");
}

#[no_mangle]
pub extern "C" fn vector_sync_lower32() {
    panic!("Synchronous exception from lower EL (AArch32)");
}

#[no_mangle]
pub extern "C" fn vector_irq_lower32() {
    panic!("IRQ from lower EL (AArch32)");
}

#[no_mangle]
pub extern "C" fn vector_fiq_lower32() {
    panic!("FIQ from lower EL (AArch32)");
}

#[no_mangle]
pub extern "C" fn vector_serror_lower32() {
    panic!("SError from lower EL (AArch32)");
}

fn vector_handle_exception() {
    // TODO: Implement proper exception handling
    println!("Exception occurred!");
    loop { unsafe { asm!("wfi", options(nomem, nostack)); } }
}

fn vector_handle_irq() {
    // TODO: Implement proper IRQ handling
    println!("IRQ occurred!");
}

fn vector_handle_fiq() {
    // TODO: Implement proper FIQ handling
    println!("FIQ occurred!");
}

/// Exception Class (EC) values for ESR_EL1
#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum ExceptionClass {
    Unknown = 0x00,
    WFxTrap = 0x01,
    CP15RTTrap = 0x03,
    CP15RRTrap = 0x04,
    CP14RTTrap = 0x05,
    CP14RRTrap = 0x06,
    CP14DTTrap = 0x07,
    AdvSIMDFPAccessTrap = 0x09,
    FPTrap = 0x08,
    SError = 0x2F,
    Breakpoint = 0x30,
    Step = 0x32,
    Watchpoint = 0x34,
    BKPTInstruction = 0x38,
    SVC64 = 0x15,
    HVC64 = 0x16,
    SMC64 = 0x17,
    SystemRegisterTrap = 0x18,
    InstructionAbort = 0x20,
    PCAlignmentFault = 0x22,
    DataAbort = 0x24,
    SPAlignmentFault = 0x26,
    TrappedFPException = 0x2C,
    IRQSPx = 0x1E,
    IRQLower64 = 0x1F,
}

impl ExceptionClass {
    /// Convert ESR_EL1.EC field to ExceptionClass
    pub fn from_ec(ec: u32) -> Self {
        match ec {
            0x01 => Self::WFxTrap,
            0x03 => Self::CP15RTTrap,
            0x04 => Self::CP15RRTrap,
            0x05 => Self::CP14RTTrap,
            0x06 => Self::CP14RRTrap,
            0x07 => Self::CP14DTTrap,
            0x08 => Self::FPTrap,
            0x09 => Self::AdvSIMDFPAccessTrap,
            0x15 => Self::SVC64,
            0x16 => Self::HVC64,
            0x17 => Self::SMC64,
            0x18 => Self::SystemRegisterTrap,
            0x1E => Self::IRQSPx,
            0x1F => Self::IRQLower64,
            0x20 => Self::InstructionAbort,
            0x22 => Self::PCAlignmentFault,
            0x24 => Self::DataAbort,
            0x26 => Self::SPAlignmentFault,
            0x2C => Self::TrappedFPException,
            0x2F => Self::SError,
            0x30 => Self::Breakpoint,
            0x32 => Self::Step,
            0x34 => Self::Watchpoint,
            0x38 => Self::BKPTInstruction,
            _ => Self::Unknown,
        }
    }
}

/// Exception Syndrome Register (ESR_EL1) structure
#[derive(Debug)]
pub struct ExceptionSyndrome {
    /// Exception Class
    pub ec: ExceptionClass,
    /// Instruction Specific Syndrome
    pub iss: u32,
    /// Data Fault Status Code (for data aborts)
    pub dfsc: u8,
    /// Instruction Fault Status Code (for instruction aborts)
    pub ifsc: u8,
    /// Write not Read (for data aborts)
    pub wnrw: bool,
    /// Syndrome Valid
    pub valid: bool,
}

impl ExceptionSyndrome {
    /// Read from ESR_EL1
    pub fn read() -> Self {
        let esr: u64;
        unsafe {
            asm!("mrs {}, esr_el1", out(reg) esr, options(nomem, nostack));
        }
        
        Self::from_bits(esr as u32)
    }
    
    /// Parse ESR value
    fn from_bits(esr: u32) -> Self {
        let ec = (esr >> 26) & 0x3F;
        let iss = esr & 0x1FFFFFF;
        
        Self {
            ec: ExceptionClass::from_ec(ec),
            iss,
            dfsc: (iss & 0x3F) as u8,
            ifsc: ((iss >> 6) & 0x3F) as u8,
            wnrw: ((iss >> 6) & 1) != 0,
            valid: ec != 0,
        }
    }
}

/// Exception vector table entries
#[repr(u64)]
#[derive(Debug, Clone, Copy)]
pub enum ExceptionVector {
    /// Synchronous exception from current EL with SP0
    SynchronousSP0 = 0x000,
    /// IRQ from current EL with SP0
    IRQSP0 = 0x080,
    /// FIQ from current EL with SP0
    FIQSP0 = 0x100,
    /// SError from current EL with SP0
    SErrorSP0 = 0x180,
    
    /// Synchronous exception from current EL with SPx
    SynchronousSPx = 0x200,
    /// IRQ from current EL with SPx
    IRQSPx = 0x280,
    /// FIQ from current EL with SPx
    FIQSPx = 0x300,
    /// SError from current EL with SPx
    SErrorSPx = 0x380,
    
    /// Synchronous exception from lower EL (AArch64)
    SynchronousLower64 = 0x400,
    /// IRQ from lower EL (AArch64)
    IRQLower64 = 0x480,
    /// FIQ from lower EL (AArch64)
    FIQLower64 = 0x500,
    /// SError from lower EL (AArch64)
    SErrorLower64 = 0x580,
    
    /// Synchronous exception from lower EL (AArch32)
    SynchronousLower32 = 0x600,
    /// IRQ from lower EL (AArch32)
    IRQLower32 = 0x680,
    /// FIQ from lower EL (AArch32)
    FIQLower32 = 0x700,
    /// SError from lower EL (AArch32)
    SErrorLower32 = 0x780,
}

/// Exception context saved on stack
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ExceptionContext {
    /// General purpose registers x0-x30
    pub gp_regs: [u64; 31],
    /// Stack pointer
    pub sp: u64,
    /// Program counter
    pub pc: u64,
    /// Processor state
    pub pstate: u64,
    /// Exception syndrome register
    pub esr: u64,
    /// Fault address register
    pub far: u64,
    /// Exception link register
    pub elr: u64,
    /// Saved processor state
    pub spsr: u64,
}

/// Initialize interrupt handling
pub fn init() {
    println!("[Interrupts] Initializing ARM64 interrupt handling...");
    
    unsafe {
        // Set exception vector table address
        set_vector_table();
        
        // Note: Don't enable interrupts here - they will be enabled after all handlers are set up
    }
    
    println!("[Interrupts] Interrupt handling initialized");
}

/// Enable interrupts (wrapper for compatibility with x86_64 API)
pub fn enable() {
    enable_interrupts();
}

/// Disable interrupts (wrapper for compatibility with x86_64 API)
pub fn disable() {
    disable_interrupts();
}

/// Set the exception vector table address
unsafe fn set_vector_table() {
    extern "C" {
        static __exception_vectors_start: u8;
    }
    
    let vector_table_addr = &__exception_vectors_start as *const _ as u64;
    
    // Set VBAR_EL1 (Vector Base Address Register)
    asm!("msr vbar_el1, {}", in(reg) vector_table_addr, options(nomem, nostack));
    
    // Ensure the write is complete
    asm!("isb", options(nomem, nostack));
}

/// Enable interrupts (IRQ and FIQ)
pub fn enable_interrupts() {
    unsafe {
        // Clear DAIF bits (D=Debug, A=SError, I=IRQ, F=FIQ)
        asm!("msr daifclr, #0b1111", options(nomem, nostack));
    }
}

/// Disable interrupts
pub fn disable_interrupts() {
    unsafe {
        // Set DAIF bits
        asm!("msr daifset, #0b1111", options(nomem, nostack));
    }
}

/// Check if interrupts are enabled
pub fn interrupts_enabled() -> bool {
    let daif: u64;
    unsafe {
        asm!("mrs {}, daif", out(reg) daif, options(nomem, nostack));
    }
    // Check if I (IRQ) and F (FIQ) bits are clear (0 = enabled)
    (daif & ((1 << 7) | (1 << 6))) == 0
}

/// Generic exception handler
#[no_mangle]
pub extern "C" fn exception_handler(context: &ExceptionContext) {
    let esr = ExceptionSyndrome::read();
    
    println!("[EXCEPTION] Exception occurred!");
    println!("  ESR: EC={:?}, ISS=0x{:x}", esr.ec, esr.iss);
    println!("  FAR: 0x{:016x}", context.far);
    println!("  ELR: 0x{:016x}", context.elr);
    println!("  PC:  0x{:016x}", context.pc);
    println!("  SP:  0x{:016x}", context.sp);
    
    match esr.ec {
        ExceptionClass::SVC64 => {
            println!("  Type: Supervisor Call (SVC)");
            handle_syscall(context);
        }
        ExceptionClass::IRQSPx | ExceptionClass::IRQLower64 => {
            println!("  Type: IRQ");
            handle_irq(context);
        }
        ExceptionClass::DataAbort => {
            println!("  Type: Data Abort");
            println!("  DFSC: 0x{:x}", esr.dfsc);
            println!("  WnR: {}", esr.wnrw);
            handle_data_abort(context, &esr);
        }
        ExceptionClass::InstructionAbort => {
            println!("  Type: Instruction Abort");
            println!("  IFSC: 0x{:x}", esr.ifsc);
            handle_instruction_abort(context, &esr);
        }
        ExceptionClass::SError => {
            println!("  Type: System Error (SError)");
            handle_serror(context);
        }
        _ => {
            println!("  Type: Unknown/Unhandled");
            handle_unknown(context, &esr);
        }
    }
    
    // For now, just halt
    println!("[EXCEPTION] Halting system...");
    unsafe {
        asm!("wfi", options(nomem, nostack));
    }
}

/// Handle system call
fn handle_syscall(_context: &ExceptionContext) {
    println!("[SYSCALL] System call handler not yet implemented");
    // TODO: Implement system call handling
}

/// Handle IRQ
fn handle_irq(_context: &ExceptionContext) {
    println!("[IRQ] IRQ handler not yet implemented");
    // TODO: Implement IRQ handling
    // This would check GIC (Generic Interrupt Controller) and handle the interrupt
}

/// Handle data abort
fn handle_data_abort(context: &ExceptionContext, esr: &ExceptionSyndrome) {
    println!("[DATA ABORT] Fault address: 0x{:016x}", context.far);
    
    // Check fault status code
    match esr.dfsc {
        0b000000 => println!("  Cause: Address size fault, level 0"),
        0b000001 => println!("  Cause: Address size fault, level 1"),
        0b000010 => println!("  Cause: Address size fault, level 2"),
        0b000011 => println!("  Cause: Address size fault, level 3"),
        0b000100 => println!("  Cause: Translation fault, level 0"),
        0b000101 => println!("  Cause: Translation fault, level 1"),
        0b000110 => println!("  Cause: Translation fault, level 2"),
        0b000111 => println!("  Cause: Translation fault, level 3"),
        0b001001 => println!("  Cause: Access flag fault, level 1"),
        0b001010 => println!("  Cause: Access flag fault, level 2"),
        0b001011 => println!("  Cause: Access flag fault, level 3"),
        0b001101 => println!("  Cause: Permission fault, level 1"),
        0b001110 => println!("  Cause: Permission fault, level 2"),
        0b001111 => println!("  Cause: Permission fault, level 3"),
        0b010000 => println!("  Cause: Synchronous external abort"),
        0b010001 => println!("  Cause: Synchronous parity error on memory access"),
        0b010100 => println!("  Cause: Synchronous external abort on translation table walk"),
        0b010101 => println!("  Cause: Synchronous parity error on translation table walk"),
        0b011000 => println!("  Cause: Alignment fault"),
        0b110001 => println!("  Cause: TLB conflict abort"),
        _ => println!("  Cause: Unknown data abort"),
    }
}

/// Handle instruction abort
fn handle_instruction_abort(context: &ExceptionContext, esr: &ExceptionSyndrome) {
    println!("[INSTRUCTION ABORT] Fault address: 0x{:016x}", context.far);
    
    // Check fault status code
    match esr.ifsc {
        0b000000 => println!("  Cause: Address size fault, level 0"),
        0b000001 => println!("  Cause: Address size fault, level 1"),
        0b000010 => println!("  Cause: Address size fault, level 2"),
        0b000011 => println!("  Cause: Address size fault, level 3"),
        0b000100 => println!("  Cause: Translation fault, level 0"),
        0b000101 => println!("  Cause: Translation fault, level 1"),
        0b000110 => println!("  Cause: Translation fault, level 2"),
        0b000111 => println!("  Cause: Translation fault, level 3"),
        0b001001 => println!("  Cause: Access flag fault, level 1"),
        0b001010 => println!("  Cause: Access flag fault, level 2"),
        0b001011 => println!("  Cause: Access flag fault, level 3"),
        0b001101 => println!("  Cause: Permission fault, level 1"),
        0b001110 => println!("  Cause: Permission fault, level 2"),
        0b001111 => println!("  Cause: Permission fault, level 3"),
        0b010000 => println!("  Cause: Synchronous external abort"),
        0b010001 => println!("  Cause: Synchronous parity error on memory access"),
        0b010100 => println!("  Cause: Synchronous external abort on translation table walk"),
        0b010101 => println!("  Cause: Synchronous parity error on translation table walk"),
        _ => println!("  Cause: Unknown instruction abort"),
    }
}

/// Handle system error
fn handle_serror(_context: &ExceptionContext) {
    println!("[SERROR] System error - unrecoverable");
}

/// Handle unknown exception
fn handle_unknown(_context: &ExceptionContext, esr: &ExceptionSyndrome) {
    println!("[UNKNOWN] Unhandled exception type: {:?}", esr.ec);
}

/// Send End of Interrupt (EOI) to interrupt controller
pub fn send_eoi(_irq_num: u32) {
    // TODO: Implement EOI for GIC
    println!("[EOI] End of Interrupt not yet implemented");
}

/// Register an interrupt handler
pub fn register_handler(_irq_num: u32, _handler: fn()) {
    // TODO: Implement interrupt handler registration
    println!("[IRQ] Interrupt handler registration not yet implemented");
}

/// Timer interrupt handler
pub fn timer_handler() {
    println!("[TIMER] Timer interrupt");
    // TODO: Handle timer interrupt and schedule next tick
}

/// Timer tick counter (static for get_timer_ticks)
static mut TIMER_TICKS: u64 = 0;

/// Get the current timer tick count
/// 
/// This provides a compatible API with x86_64
pub fn get_timer_ticks() -> u64 {
    unsafe { TIMER_TICKS }
}

/// Increment timer ticks (called from timer interrupt handler)
pub unsafe fn increment_timer_ticks() {
    TIMER_TICKS += 1;
}

/// Unmask a specific IRQ (for compatibility with x86_64 API)
/// 
/// On ARM64, this would configure the GIC (Generic Interrupt Controller).
/// For now, this is a no-op placeholder.
/// 
/// # Safety
/// Should only be called after the interrupt handler is registered
pub unsafe fn unmask_irq(_irq: u8) {
    // TODO: Implement GIC interrupt unmasking for ARM64
    // For now, just a placeholder
}