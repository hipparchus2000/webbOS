//! Panic handler for kernel

use core::panic::PanicInfo;
use crate::println;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // Disable interrupts (architecture-specific)
    #[cfg(target_arch = "x86_64")]
    unsafe { core::arch::asm!("cli") };
    #[cfg(target_arch = "aarch64")]
    unsafe { core::arch::asm!("msr daifset, #0b1111", options(nomem, nostack)) };
    
    println!("\n╔══════════════════════════════════════════════════╗");
    println!("║              KERNEL PANIC                        ║");
    println!("╚══════════════════════════════════════════════════╝");
    
    if let Some(location) = info.location() {
        println!("Location: {}:{}:{}", 
            location.file(), 
            location.line(), 
            location.column()
        );
    }
    
    println!("Message: {:?}", info.message());
    
    println!("\nSystem halted.");
    
    // Halt forever (architecture-specific)
    loop {
        #[cfg(target_arch = "x86_64")]
        unsafe { core::arch::asm!("hlt") };
        #[cfg(target_arch = "aarch64")]
        unsafe { core::arch::asm!("wfi", options(nomem, nostack)) };
    }
}
