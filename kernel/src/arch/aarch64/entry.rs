//! AArch64 kernel entry point

use core::arch::naked_asm;

/// Kernel entry point from bootloader
#[naked]
#[no_mangle]
#[repr(align(16))]
pub unsafe extern "C" fn _start() -> ! {
    naked_asm!(
        // Save boot info pointer
        "mov x19, x0",
        
        // UART at 0x09000000
        "mov x20, 0x09000000",
        
        // "AKERNEL"
        "mov w3, 0x41", "str w3, [x20]",  // A
        "mov w3, 0x4B", "str w3, [x20]",  // K
        "mov w3, 0x45", "str w3, [x20]",  // E
        "mov w3, 0x52", "str w3, [x20]",  // R
        "mov w3, 0x4E", "str w3, [x20]",  // N
        "mov w3, 0x45", "str w3, [x20]",  // E
        "mov w3, 0x4C", "str w3, [x20]",  // L
        
        // Set up stack at 0x500000 (5MB)
        "mov sp, 0x500000",
        
        // "S" for stack setup
        "mov w3, 0x53", "str w3, [x20]",
        
        // Call kernel_entry
        "mov x0, x19",
        "bl {kernel_entry}",
        
        // Should not return
        "1: wfi",
        "b 1b",
        
        kernel_entry = sym crate::kernel_entry,
    );
}
