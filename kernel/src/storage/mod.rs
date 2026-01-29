//! Storage subsystem
//!
//! Block device drivers and storage management.

use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;
use alloc::string::String;
use spin::Mutex;
use lazy_static::lazy_static;

pub mod ata;
pub mod ahci;
pub mod nvme;

use crate::drivers::pci::PciDevice;
use crate::println;

/// Block device trait
pub trait BlockDevice: Send + Sync {
    /// Get device name
    fn name(&self) -> &str;
    /// Get block size (usually 512 bytes)
    fn block_size(&self) -> usize;
    /// Get total number of blocks
    fn block_count(&self) -> u64;
    /// Read blocks from device
    fn read_blocks(&self, start: u64, count: usize, buf: &mut [u8]) -> Result<(), StorageError>;
    /// Write blocks to device
    fn write_blocks(&self, start: u64, count: usize, buf: &[u8]) -> Result<(), StorageError>;
    /// Flush write cache
    fn flush(&self) -> Result<(), StorageError>;
}

/// Storage error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageError {
    /// Success
    Success = 0,
    /// Device not found
    NotFound = 1,
    /// Invalid argument
    InvalidArgument = 2,
    /// IO error
    IoError = 3,
    /// Timeout
    Timeout = 4,
    /// No media
    NoMedia = 5,
    /// Write protected
    WriteProtected = 6,
    /// Busy
    Busy = 7,
    /// Unknown error
    Unknown = 255,
}

/// Storage device info
#[derive(Debug, Clone)]
pub struct StorageInfo {
    /// Device name
    pub name: String,
    /// Block size in bytes
    pub block_size: usize,
    /// Total blocks
    pub block_count: u64,
    /// Total size in bytes
    pub total_size: u64,
    /// Model name
    pub model: String,
    /// Serial number
    pub serial: String,
}

/// Global block device list
lazy_static! {
    static ref BLOCK_DEVICES: Mutex<Vec<Box<dyn BlockDevice>>> = Mutex::new(Vec::new());
}

/// Initialize storage subsystem
pub fn init() {
    println!("[storage] Initializing storage subsystem...");

    // Try to initialize NVMe first (modern)
    nvme::init();

    // Try AHCI/SATA next
    ahci::init();

    // Fall back to ATA/IDE
    ata::init();

    println!("[storage] Storage subsystem initialized");
}

/// Register block device
pub fn register_device(device: Box<dyn BlockDevice>) {
    let mut devices = BLOCK_DEVICES.lock();
    let idx = devices.len();
    
    println!("[storage] Registered block device {}: {} ({} blocks, {} MB)",
        idx,
        device.name(),
        device.block_count(),
        (device.block_count() * device.block_size() as u64) / (1024 * 1024)
    );
    
    devices.push(device);
}

/// Get number of block devices
pub fn device_count() -> usize {
    BLOCK_DEVICES.lock().len()
}

/// Get block device by index
pub fn get_device(idx: usize) -> Option<Box<dyn BlockDevice>> {
    BLOCK_DEVICES.lock().get(idx).map(|d| {
        // Create a simple wrapper - in reality we'd use Arc or similar
        // For now, just return None since we can't easily clone Box<dyn BlockDevice>
        // The actual usage would be through the global list
        None
    }).flatten()
}

/// Read from block device
pub fn read(idx: usize, start: u64, count: usize, buf: &mut [u8]) -> Result<(), StorageError> {
    let devices = BLOCK_DEVICES.lock();
    if let Some(device) = devices.get(idx) {
        device.read_blocks(start, count, buf)
    } else {
        Err(StorageError::NotFound)
    }
}

/// Write to block device
pub fn write(idx: usize, start: u64, count: usize, buf: &[u8]) -> Result<(), StorageError> {
    let devices = BLOCK_DEVICES.lock();
    if let Some(device) = devices.get(idx) {
        device.write_blocks(start, count, buf)
    } else {
        Err(StorageError::NotFound)
    }
}

/// Print storage device list
pub fn print_devices() {
    let devices = BLOCK_DEVICES.lock();

    println!("Block Devices:");
    println!("{:<4} {:<16} {:<12} {:<16} {}",
        "Idx", "Name", "Block Size", "Blocks", "Size (MB)");
    println!("{}", "-".repeat(70));

    for (i, device) in devices.iter().enumerate() {
        let size_mb = (device.block_count() * device.block_size() as u64) / (1024 * 1024);
        println!("{:<4} {:<16} {:<12} {:<16} {}",
            i,
            device.name(),
            device.block_size(),
            device.block_count(),
            size_mb
        );
    }
}
