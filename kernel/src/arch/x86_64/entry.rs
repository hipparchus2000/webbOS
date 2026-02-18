//! x86_64 kernel entry point

use core::arch::naked_asm;
use webbos_shared::bootinfo::BOOTINFO_PAGE_TABLE_OFFSET;

/// Kernel entry point from bootloader
#[naked]
#[no_mangle]
#[repr(align(16))]
pub unsafe extern "C" fn _start() -> ! {
    naked_asm!(
        // Save boot info pointer
        "mov r15, rdi",
        
        // Debug: "K"
        "mov dx, 0x3F8",
        "mov al, 0x4B", "out dx, al",
        
        // Check if rdi (boot_info) is null - output '0'
        "test rdi, rdi",
        "jnz 1f",
        "mov al, 0x30", "out dx, al",
        "jmp 9f",
        
        "1:",
        // Get page table address from boot_info (skip Option discriminant)
        "mov r14, [r15 + {page_table_offset} + 8]",
        
        // Output '1' - got page table
        "mov al, 0x31", "out dx, al",
        
        // Enable PAE
        "mov rax, cr4",
        "or al, 0x20",
        "mov cr4, rax",
        
        // Load page table
        "mov cr3, r14",
        
        // Enable long mode
        "mov ecx, 0xC0000080",
        "rdmsr",
        "or ah, 0x01",
        "wrmsr",
        
        // Enable paging
        "mov rax, cr0",
        "mov rbx, 0x80000000",
        "or rax, rbx",
        "mov cr0, rax",
        
        // Output '2' - paging enabled
        "mov al, 0x32", "out dx, al",
        
        // Set virtual stack
        "mov rsp, {virt_stack}",
        
        // Jump to virtual address
        "lea rax, [rip + 3f]",
        "mov rbx, 0xFFFF800000000000",
        "add rax, rbx",
        "jmp rax",
        
        "3:",
        // Output '3' - in virtual mode
        "mov al, 0x33", "out dx, al",
        "xor rbp, rbp",
        "mov rdi, r15",
        "call {kernel_entry}",
        
        "cli",
        "2: hlt",
        "jmp 2b",
        
        // Error
        "9:",
        "mov al, 0x21", "out dx, al",
        "jmp 9b",
        
        virt_stack = const 0xFFFF800000500000u64,
        kernel_entry = sym crate::kernel_entry,
        page_table_offset = const BOOTINFO_PAGE_TABLE_OFFSET,
    );
}
