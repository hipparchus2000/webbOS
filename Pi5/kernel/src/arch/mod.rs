//! Architecture-specific code for ARM64 (AArch64)
//!
//! This module provides ARM64-specific implementations for:
//! - CPU initialization and features
//! - Exception handling (replaces x86 IDT)
//! - MMU management (replaces x86 paging)
//!
//! Note: ARM64 does not use GDT/TSS like x86.

pub mod cpu;
pub mod exceptions;
pub mod mmu;

// GDT is not needed on ARM64 - removed
// pub mod gdt;
