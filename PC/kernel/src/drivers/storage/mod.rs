//! Storage drivers
//!
//! AHCI, NVMe, and other storage controllers.

pub mod ahci;

use alloc::vec::Vec;
use alloc::boxed::Box;
use spin::Mutex;
use lazy_static::lazy_static;
use crate::println;

/// Storage device trait
pub trait StorageDevice: Send + Sync {
    /// Get device name
    fn name(&self) -> &str;
    /// Get device size in bytes
    fn size(&self) -> u64;
    /// Get block size
    fn block_size(&self) -> usize;
    /// Read blocks
    fn read(&self, lba: u64, count: usize, buf: &mut [u8]) -> Result<usize, StorageError>;
    /// Write blocks
    fn write(&self, lba: u64, count: usize, buf: &[u8]) -> Result<usize, StorageError>;
    /// Flush cache
    fn flush(&self) -> Result<(), StorageError>;
}

/// Storage error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageError {
    /// Success
    Success = 0,
    /// Device not found
    NotFound = 1,
    /// Invalid parameter
    InvalidParameter = 2,
    /// Out of memory
    OutOfMemory = 3,
    /// IO error
    IoError = 4,
    /// Device error
    DeviceError = 5,
    /// Not implemented
    NotImplemented = 6,
    /// Timeout
    Timeout = 7,
    /// Unknown error
    Unknown = 255,
}

lazy_static! {
    /// Global storage device list
    static ref STORAGE_DEVICES: Mutex<Vec<Box<dyn StorageDevice>>> = Mutex::new(Vec::new());
}

/// Initialize storage drivers
pub fn init() {
    println!("[storage] Initializing storage drivers...");

    // Initialize AHCI
    ahci::init();

    println!("[storage] Storage drivers initialized");
}

/// Register a storage device
pub fn register_device(device: Box<dyn StorageDevice>) {
    let mut devices = STORAGE_DEVICES.lock();
    println!("[storage] Registered device: {} ({} bytes)", 
        device.name(), device.size());
    devices.push(device);
}

/// Get number of storage devices
pub fn device_count() -> usize {
    STORAGE_DEVICES.lock().len()
}

/// Print storage device list
pub fn print_devices() {
    let devices = STORAGE_DEVICES.lock();
    
    println!("Storage Devices:");
    println!("{:<4} {:<20} {:>15} {:>10}",
        "Idx", "Name", "Size", "Block");
    println!("{}", "-".repeat(55));
    
    for (i, dev) in devices.iter().enumerate() {
        let size_mb = dev.size() / (1024 * 1024);
        println!("{:>4} {:<20} {:>10} MB {:>10}",
            i, dev.name(), size_mb, dev.block_size());
    }
}
