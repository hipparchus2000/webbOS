//! Simple Disk Block Device
//!
//! A simple block device that uses the storage module directly.

#![allow(dead_code)]

use alloc::boxed::Box;

use crate::storage::{BlockDevice, StorageError};

/// Simple disk that wraps a storage device index
pub struct SimpleDisk {
    device_index: usize,
}

impl SimpleDisk {
    /// Create new simple disk
    pub fn new(device_index: usize) -> Self {
        Self { device_index }
    }
}

impl BlockDevice for SimpleDisk {
    fn name(&self) -> &str {
        "simple_disk"
    }

    fn block_size(&self) -> usize {
        512
    }

    fn block_count(&self) -> u64 {
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

/// Create a simple disk for the boot device
pub fn create_boot_disk() -> Option<Box<SimpleDisk>> {
    if crate::storage::device_count() > 0 {
        Some(Box::new(SimpleDisk::new(0)))
    } else {
        None
    }
}
