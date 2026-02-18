//! x86_64 kernel entry point

use core::arch::naked_asm;
use super::constants::{
    serial::{COM1_PORT, ascii},
    memory::{VIRT_STACK_TOP, PHYS_STACK_TOP},
    cr::{EFER_MSR},
    bootinfo::{PAGE_TABLE_OFFSET, OPTION_PHYSADDR_VALUE_OFFSET},
};

/// Kernel entry point from bootloader
#[naked]
#[no_mangle]
#[repr(align(16))]
pub unsafe extern "C" fn _start() -> ! {
    naked_asm!(
        // Save boot info pointer
        "mov r15, rdi",
        
        // IMMEDIATE: Output 'S' (start)
        "mov dx, 0x3F8",
        "mov al, 0x53",  // 'S'
        "out dx, al",
        
        // Check if boot_info is null first
        "test rdi, rdi",
        "jnz 8f",
        // boot_info is null - output '!'
        "mov dx, {com1_port}",
        "mov al, {ascii_excl}", "out dx, al",
        "jmp 99f",
        
        "8:",
        // boot_info valid - output '0'
        "mov dx, {com1_port}",
        "mov al, {ascii_0}", "out dx, al",
        
        // Check page_table_addr discriminant
        "mov rax, [r15 + {page_table_offset}]",
        "test rax, rax",
        "jnz 7f",
        // None - output 'N'
        "mov dx, {com1_port}",
        "mov al, 0x4E", "out dx, al",
        "jmp 99f",
        
        "7:",
        // Some - output 'S'
        "mov dx, {com1_port}",
        "mov al, 0x53", "out dx, al",
        
        // Debug: "KERN"
        "mov al, {ascii_k}", "out dx, al",
        "mov al, {ascii_e}", "out dx, al",
        "mov al, {ascii_r}", "out dx, al",
        "mov al, {ascii_n}", "out dx, al",
        
        // Get page table address from boot_info
        "mov r14, [r15 + {page_table_offset} + {option_physaddr_offset}]",
        
        // Output '1'
        "mov dx, {com1_port}",
        "mov al, {ascii_1}", "out dx, al",
        
        // Load page table into CR3 (PAE should already be enabled by bootloader)
        "mov cr3, r14",
        
        // Output '2'
        "mov dx, {com1_port}",
        "mov al, {ascii_2}", "out dx, al",
        
        // Enable long mode (EFER.LME = bit 8)
        "mov ecx, {efer_msr}",
        "rdmsr",
        "or ah, 0x01",  // Bit 8 is in AH (bits 8-15)
        "wrmsr",
        
        // Output '3'
        "mov dx, {com1_port}",
        "mov al, {ascii_3}", "out dx, al",
        
        // Enable paging (CR0.PG = bit 31)
        "mov rax, cr0",
        "mov rbx, 0x80000000",  // PG bit
        "or rax, rbx",
        "mov cr0, rax",
        
        // Output '4'
        "mov dx, {com1_port}",
        "mov al, {ascii_4}", "out dx, al",
        
        // Set virtual stack
        "mov rsp, {virt_stack}",
        
        // Far jump to higher half using ret trick
        // Push virtual return address and use ret to jump
        "lea rax, [rip + 88f]",
        "mov rbx, 0xFFFF800000000000",  // Virtual base
        "add rax, rbx",
        "push rax",
        "ret",
        
        // Higher half entry point
        "88:",
        // Output '5'
        "mov dx, {com1_port}",
        "mov al, {ascii_5}", "out dx, al",
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
        ascii_excl = const ascii::EXCLAMATION,
        ascii_0 = const ascii::ZERO,
        ascii_1 = const ascii::ONE,
        ascii_2 = const ascii::TWO,
        ascii_3 = const ascii::THREE,
        ascii_4 = const ascii::FOUR,
        ascii_5 = const ascii::FIVE,
        efer_msr = const EFER_MSR,
        virt_stack = const VIRT_STACK_TOP,
        kernel_entry = sym crate::kernel_entry,
        page_table_offset = const PAGE_TABLE_OFFSET,
        option_physaddr_offset = const OPTION_PHYSADDR_VALUE_OFFSET,
    );
}
