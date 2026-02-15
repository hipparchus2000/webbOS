//! Memory management for bootloader
//!
//! Simple allocation utilities for bootloader use.

use uefi::boot::{allocate_pages, AllocateType, MemoryType};
use webbos_shared::types::PhysAddr;

/// Allocate a contiguous block of pages
pub fn alloc_pages(count: usize) -> uefi::Result<PhysAddr, ()> {
    let pages = allocate_pages(
        AllocateType::AnyPages,
        MemoryType::LOADER_DATA,
        count,
    )?;
    
    Ok(PhysAddr::new(pages.as_ptr() as u64))
}

/// Zero a memory region
pub unsafe fn zero_memory(addr: PhysAddr, size: usize) {
    core::ptr::write_bytes(addr.as_mut_ptr::<u8>(), 0, size);
}

/// Copy memory from source to destination
pub unsafe fn copy_memory(src: *const u8, dst: *mut u8, size: usize) {
    core::ptr::copy_nonoverlapping(src, dst, size);
}
