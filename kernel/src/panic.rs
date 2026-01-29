//! Panic handler for kernel

use core::panic::PanicInfo;
use crate::println;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // Disable interrupts
    unsafe { core::arch::asm!("cli") };
    
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
    
    // Halt forever
    loop {
        unsafe { core::arch::asm!("hlt") };
    }
}
