// Simple test to verify ARM64 assembly syntax

#![no_std]
#![no_main]

use core::arch::asm;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Test ARM64 assembly
    unsafe {
        // Simple NOP
        asm!("nop");
        
        // Read system register
        let mut value: u64;
        asm!("mrs {}, currentel", out(reg) value);
        
        // Write system register
        asm!("msr daifset, #0b1111");
        
        // Memory barrier
        asm!("dsb sy");
        asm!("isb");
        
        // WFI (Wait For Interrupt)
        asm!("wfi");
    }
    
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}