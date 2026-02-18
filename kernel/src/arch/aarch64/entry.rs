//! AArch64 kernel entry point

use core::arch::naked_asm;
use super::constants::{
    uart::{self, ascii},
    memory::PHYS_STACK_TOP,
    asm::{STACK_TOP_IMM, STACK_TOP_SHIFT},
};

/// Kernel entry point from bootloader
#[naked]
#[no_mangle]
#[repr(align(16))]
pub unsafe extern "C" fn _start() -> ! {
    naked_asm!(
        // Save boot info pointer
        "mov x19, x0",
        
        // UART at base address - use movz with shift
        "movz x20, {uart_imm}, lsl {uart_shift}",
        
        // Output "AKERNEL"
        "mov w3, {ascii_a}", "str w3, [x20]",
        "mov w3, {ascii_k}", "str w3, [x20]",
        "mov w3, {ascii_e}", "str w3, [x20]",
        "mov w3, {ascii_r}", "str w3, [x20]",
        "mov w3, {ascii_n}", "str w3, [x20]",
        "mov w3, {ascii_e}", "str w3, [x20]",
        "mov w3, {ascii_l}", "str w3, [x20]",
        
        // Set up stack at physical address - load into temp register first
        "movz x1, {stack_imm}, lsl {stack_shift}",
        "mov sp, x1",
        
        // Output 'S' for stack setup
        "mov w3, {ascii_s}", "str w3, [x20]",
        
        // Output 'X' for calling kernel_entry
        "mov w3, {ascii_x}", "str w3, [x20]",
        
        // Call kernel_entry
        "mov x0, x19",
        "bl {kernel_entry}",
        
        // Should not return
        "1: wfi",
        "b 1b",
        
        // Constants
        uart_imm = const uart::MOVZ_UART_IMM,
        uart_shift = const uart::MOVZ_UART_SHIFT,
        stack_imm = const STACK_TOP_IMM,
        stack_shift = const STACK_TOP_SHIFT,
        ascii_a = const ascii::A,
        ascii_k = const ascii::K,
        ascii_e = const ascii::E,
        ascii_r = const ascii::R,
        ascii_n = const ascii::N,
        ascii_l = const ascii::L,
        ascii_s = const ascii::S,
        ascii_x = const ascii::X,
        kernel_entry = sym crate::kernel_entry,
    );
}
