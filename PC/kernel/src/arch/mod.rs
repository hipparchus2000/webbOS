//! Architecture-specific code
//!
//! Currently supports x86_64 only.

#![allow(dead_code)]

pub mod constants;
pub mod cpu;
pub mod interrupts;
pub mod paging;
pub mod gdt;
