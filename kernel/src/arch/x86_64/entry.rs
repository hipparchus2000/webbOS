//! x86_64 kernel entry point

use core::arch::naked_asm;
use super::constants::{
    serial::{COM1_PORT, ascii},
    memory::{VIRT_STACK_TOP, KERNEL_VIRT_BASE, PHYS_STACK_TOP},
    cr::{CR0_PG, CR4_PAE, EFER_MSR},
    bootinfo::{PAGE_TABLE_OFFSET, OPTION_PHYSADDR_VALUE_OFFSET},
};

/// GDT entry for the long jump to higher half
#[repr(C, packed)]
struct GdtEntry {
    limit_low: u16,
    base_low: u16,
    base_middle: u8,
    access: u8,
    granularity: u8,
    base_high: u8,
}

/// GDT pointer
#[repr(C, packed)]
struct GdtPointer {
    limit: u16,
    base: u64,
}

/// Kernel entry point from bootloader
/// 
/// Enables paging and transitions to higher half.
#[naked]
#[no_mangle]
#[repr(align(16))]
pub unsafe extern "C" fn _start() -> ! {
    // Temporary GDT for long jump to higher half
    // This must be in the entry function so it's at a known physical address
    
    naked_asm!(
        // Save boot info pointer
        "mov r15, rdi",
        
        // Debug: "KERN"
        "mov dx, {com1_port}",
        "mov al, {ascii_k}", "out dx, al",
        "mov al, {ascii_e}", "out dx, al",
        "mov al, {ascii_r}", "out dx, al",
        "mov al, {ascii_n}", "out dx, al",
        
        // For now, just run in physical mode
        // Paging setup is complex and needs careful implementation
        "mov rsp, {phys_stack}",
        "xor rbp, rbp",
        "mov rdi, r15",
        "call {kernel_entry}",
        
        // Should never return
        "cli",
        "99:",
        "hlt",
        "jmp 99b",
        
        // Constants
        com1_port = const COM1_PORT,
        ascii_k = const ascii::K,
        ascii_e = const ascii::E,
        ascii_r = const ascii::R,
        ascii_n = const ascii::N,
        phys_stack = const PHYS_STACK_TOP,
        kernel_entry = sym crate::kernel_entry,
    );
}
