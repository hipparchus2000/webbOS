//! Panic handler for kernel

use core::panic::PanicInfo;
use crate::println;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // Disable interrupts (ARM64: DAIF set)
    unsafe { 
        core::arch::asm!(
            "msr DAIFSet, #0xF",  // Disable all interrupts
            options(nomem, nostack)
        );
    };
    
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
    
    // Halt forever (ARM64: wfe - wait for event)
    loop {
        unsafe { 
            core::arch::asm!(
                "wfe",  // Wait for event (low power halt)
                options(nomem, nostack)
            );
        };
    }
}
