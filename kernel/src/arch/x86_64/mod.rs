//! x86_64 architecture-specific code

use crate::println;

pub mod cpu;
pub mod interrupts;
pub mod paging;
pub mod gdt;

/// Initialize x86_64 architecture
pub fn init() {
    println!("[arch] Initializing x86_64 architecture...");
    
    // Initialize CPU features
    cpu::init();
    
    // Initialize interrupt handling
    interrupts::init();
    
    // Initialize memory management
    // TODO: Get actual physical_memory_offset from boot info
    unsafe { paging::init(0xFFFF_8000_0000_0000); } // Typical higher-half offset
    
    println!("[arch] x86_64 architecture initialized");
}

/// Architecture-specific panic handler
pub fn panic(info: &core::panic::PanicInfo) -> ! {
    use crate::println;
    
    println!("[PANIC] x86_64 kernel panic!");
    if let Some(location) = info.location() {
        println!("  at {}:{}:{}", 
            location.file(), 
            location.line(), 
            location.column()
        );
    }
    // Note: info.message() returns fmt::Arguments which can't be printed directly
    // in no_std without proper formatting support
    // if let Some(message) = info.message() {
    //     println!("  {}", message);
    // }
    
    // Disable interrupts and halt
    cpu::disable_interrupts();
    loop {
        unsafe {
            core::arch::asm!("hlt", options(nomem, nostack));
        }
    }
}

/// Get the current stack pointer
pub fn current_stack_pointer() -> usize {
    let sp: usize;
    unsafe {
        core::arch::asm!("mov {}, rsp", out(reg) sp, options(nomem, nostack));
    }
    sp
}

/// Get the current frame pointer
pub fn current_frame_pointer() -> usize {
    let fp: usize;
    unsafe {
        core::arch::asm!("mov {}, rbp", out(reg) fp, options(nomem, nostack));
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
        core::arch::asm!("mfence", options(nomem, nostack));
    }
}