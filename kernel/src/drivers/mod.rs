//! Device drivers
//!
//! Hardware-specific drivers for various devices.

pub mod timer;
pub mod pci;
pub mod storage;
pub mod vesa;
pub mod input;

use crate::println;

/// Initialize all drivers
pub fn init() {
    println!("[drivers] Initializing device drivers...");
    
    timer::init();
    pci::init();
    // Storage drivers initialized separately after PCI enumeration
    
    println!("[drivers] Device drivers initialized");
}

/// Driver error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverError {
    /// Success
    Success = 0,
    /// Device not found
    NotFound = 1,
    /// Initialization failed
    InitFailed = 2,
    /// IO error
    IoError = 3,
    /// Unsupported operation
    Unsupported = 4,
    /// Timeout
    Timeout = 5,
    /// Unknown error
    Unknown = 255,
}

/// Result type for driver operations
pub type DriverResult<T> = Result<T, DriverError>;
