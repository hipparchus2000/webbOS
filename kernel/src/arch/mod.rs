//! Architecture-specific code
//!
//! Supports x86_64 and ARM64 architectures.

#[cfg(target_arch = "x86_64")]
pub mod cpu;
#[cfg(target_arch = "x86_64")]
pub mod interrupts;
#[cfg(target_arch = "x86_64")]
pub mod paging;
#[cfg(target_arch = "x86_64")]
pub mod gdt;

#[cfg(target_arch = "aarch64")]
pub mod aarch64;
