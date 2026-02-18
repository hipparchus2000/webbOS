//! x86_64 kernel entry point

use core::arch::naked_asm;
use super::constants::{
    serial::{COM1_PORT, ascii},
    memory::{PHYS_STACK_TOP},
    bootinfo::{PAGE_TABLE_OFFSET, OPTION_PHYSADDR_VALUE_OFFSET},
};

/// Kernel entry point from bootloader
/// 
/// For now, runs in physical mode without paging.
/// Paging setup will be added in a future update.
#[naked]
#[no_mangle]
#[repr(align(16))]
pub unsafe extern "C" fn _start() -> ! {
    naked_asm!(
        // Save boot info pointer
        "mov r15, rdi",
        
        // Debug: "KERNEL"
        "mov dx, {com1_port}",
        "mov al, {ascii_k}", "out dx, al",
        "mov al, {ascii_e}", "out dx, al",
        "mov al, {ascii_r}", "out dx, al",
        "mov al, {ascii_n}", "out dx, al",
        "mov al, {ascii_e}", "out dx, al",
        "mov al, {ascii_l}", "out dx, al",
        
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
        
        // Constants
        com1_port = const COM1_PORT,
        ascii_k = const ascii::K,
        ascii_e = const ascii::E,
        ascii_r = const ascii::R,
        ascii_n = const ascii::N,
        ascii_l = const ascii::L,
        phys_stack = const PHYS_STACK_TOP,
        kernel_entry = sym crate::kernel_entry,
    );
}
