//! x86_64 kernel entry point

use core::arch::naked_asm;

/// Kernel entry point from bootloader (PHYSICAL mode)
/// 
/// For now, we run in physical mode without paging.
/// Paging will be added in a future update.
#[naked]
#[no_mangle]
#[repr(align(16))]
pub unsafe extern "C" fn _start() -> ! {
    naked_asm!(
        // Save boot info pointer (in RDI from bootloader)
        "mov r15, rdi",
        
        // Debug: Write "KERNEL" to serial port (COM1)
        "mov dx, 0x3F8",
        "mov al, 0x4B", "out dx, al",
        "mov al, 0x45", "out dx, al",
        "mov al, 0x52", "out dx, al",
        "mov al, 0x4E", "out dx, al",
        "mov al, 0x45", "out dx, al",
        "mov al, 0x4C", "out dx, al",
        
        // Set up physical stack
        "mov rsp, {phys_stack}",
        "xor rbp, rbp",
        
        // Call kernel entry
        "mov rdi, r15",
        "call {kernel_entry}",
        
        // Should never return
        "cli",
        "2: hlt",
        "jmp 2b",
        
        phys_stack = const 0x500000u64,
        kernel_entry = sym crate::kernel_entry,
    );
}
