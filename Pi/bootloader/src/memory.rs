//! Simple memory utilities for bootloader

use webbos_shared::types::PhysAddr;

/// Zero a memory region
pub unsafe fn zero_memory(addr: PhysAddr, size: usize) {
    core::ptr::write_bytes(addr.as_mut_ptr::<u8>(), 0, size);
}

/// Copy memory from source to destination
pub unsafe fn copy_memory(src: *const u8, dst: *mut u8, size: usize) {
    core::ptr::copy_nonoverlapping(src, dst, size);
}

/// Align address up to boundary
pub fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}

/// Align address down to boundary
pub fn align_down(addr: usize, align: usize) -> usize {
    addr & !(align - 1)
}
