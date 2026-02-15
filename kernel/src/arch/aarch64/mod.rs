//! ARM64 architecture-specific code

pub mod cpu;
pub mod interrupts;
pub mod paging;

/// Initialize ARM64 architecture
pub fn init() {
    println!("[arch] Initializing ARM64 architecture...");
    
    // Initialize CPU features
    cpu::init();
    
    // Initialize interrupt handling
    interrupts::init();
    
    // Initialize memory management
    paging::init();
    
    println!("[arch] ARM64 architecture initialized");
}

/// Architecture-specific panic handler
pub fn panic(info: &core::panic::PanicInfo) -> ! {
    use crate::println;
    
    println!("[PANIC] ARM64 kernel panic!");
    if let Some(location) = info.location() {
        println!("  at {}:{}:{}", 
            location.file(), 
            location.line(), 
            location.column()
        );
    }
    if let Some(message) = info.message() {
        println!("  {}", message);
    }
    
    // Halt the CPU
    loop {
        unsafe {
            core::arch::asm!("wfi", options(nomem, nostack));
        }
    }
}

/// Get the current stack pointer
pub fn current_stack_pointer() -> usize {
    let sp: usize;
    unsafe {
        core::arch::asm!("mov {}, sp", out(reg) sp, options(nomem, nostack));
    }
    sp
}

/// Get the current frame pointer
pub fn current_frame_pointer() -> usize {
    let fp: usize;
    unsafe {
        core::arch::asm!("mov {}, x29", out(reg) fp, options(nomem, nostack));
    }
    fp
}

/// NOP instruction for busy waiting
pub fn nop() {
    unsafe {
        core::arch::asm!("nop", options(nomem, nostack));
    }
}

/// Memory barrier
pub fn barrier() {
    unsafe {
        core::arch::asm!("dsb sy", options(nomem, nostack));
        core::arch::asm!("isb", options(nomem, nostack));
    }
}