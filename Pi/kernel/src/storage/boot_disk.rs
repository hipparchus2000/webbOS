//! Boot Disk Block Device
//!
//! Provides access to the boot disk (the disk the OS was loaded from).
//! This is typically the first ATA/IDE drive (primary master).

#![allow(dead_code)]

use alloc::sync::Arc;
use spin::Mutex;

use crate::storage::{BlockDevice, StorageError};
use crate::println;

/// Boot disk instance - wraps the first available block device
pub struct BootDisk {
    /// Device index in the global BLOCK_DEVICES list
    device_index: usize,
    /// Device name
    name: &'static str,
}

impl BootDisk {
    /// Create new boot disk wrapper
    pub fn new(device_index: usize) -> Self {
        Self {
            device_index,
            name: "boot_disk",
        }
    }
}

impl BlockDevice for BootDisk {
    fn name(&self) -> &str {
        self.name
    }

    fn block_size(&self) -> usize {
        512
    }

    fn block_count(&self) -> u64 {
        // Query from underlying device
        crate::storage::device_block_count(self.device_index).unwrap_or(0)
    }

    fn read_blocks(&self, start: u64, count: usize, buf: &mut [u8]) -> Result<(), StorageError> {
        crate::storage::read(self.device_index, start, count, buf)
    }

    fn write_blocks(&self, start: u64, count: usize, buf: &[u8]) -> Result<(), StorageError> {
        crate::storage::write(self.device_index, start, count, buf)
    }

    fn flush(&self) -> Result<(), StorageError> {
        Ok(())
    }
}

/// Global boot disk instance
static BOOT_DISK: Mutex<Option<Arc<BootDisk>>> = Mutex::new(None);

/// Initialize boot disk
/// 
/// This should be called after storage subsystem is initialized and
/// at least one block device is registered.
pub fn init() {
    println!("[boot_disk] Initializing boot disk...");

    // Check if we have any block devices registered
    let device_count = crate::storage::device_count();
    
    if device_count == 0 {
        println!("[boot_disk] No block devices available, boot disk not initialized");
        return;
    }

    // Use the first registered device as the boot disk
    println!("[boot_disk] Using device 0 as boot disk");
    let boot_disk = Arc::new(BootDisk::new(0));
    
    *BOOT_DISK.lock() = Some(boot_disk);
    
    println!("[boot_disk] Boot disk initialized");
}

/// Get boot disk handle
pub fn get_boot_disk() -> Option<Arc<BootDisk>> {
    BOOT_DISK.lock().clone()
}

/// Check if boot disk is available
pub fn is_available() -> bool {
    BOOT_DISK.lock().is_some()
}

/// Read from boot disk
pub fn read(start: u64, count: usize, buf: &mut [u8]) -> Result<(), StorageError> {
    if let Some(disk) = BOOT_DISK.lock().as_ref() {
        disk.read_blocks(start, count, buf)
    } else {
        Err(StorageError::NotFound)
    }
}

/// Write to boot disk
pub fn write(start: u64, count: usize, buf: &[u8]) -> Result<(), StorageError> {
    if let Some(disk) = BOOT_DISK.lock().as_ref() {
        disk.write_blocks(start, count, buf)
    } else {
        Err(StorageError::NotFound)
    }
}
