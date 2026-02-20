//! Architecture-specific code
//!
//! Currently supports x86_64 only.

pub mod cpu;
pub mod interrupts;
pub mod paging;
pub mod gdt;
