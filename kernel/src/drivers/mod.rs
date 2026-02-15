//! Device drivers
//!
//! Hardware-specific drivers for various devices.

pub mod timer;
pub mod pci;
pub mod storage;
pub mod vesa;
pub mod input;

// Raspberry Pi specific drivers (ARM64 only)
#[cfg(target_arch = "aarch64")]
pub mod raspberrypi;

// Driver tests
pub mod tests;

use crate::println;

/// Initialize all drivers
pub fn init() {
    println!("[drivers] Initializing device drivers...");
    
    // Initialize HAL first (ARM64 only)
    #[cfg(target_arch = "aarch64")]
    {
        crate::hal::init();
    }
    
    timer::init();
    pci::init();
    // Storage drivers initialized separately after PCI enumeration
    
    // Initialize Raspberry Pi specific drivers (ARM64 only)
    #[cfg(target_arch = "aarch64")]
    {
        raspberrypi::init();
    }
    
    println!("[drivers] Device drivers initialized");
}

/// Run driver self-tests
pub fn run_tests() {
    println!("[drivers] Running driver tests...");
    
    tests::run_all_tests();
    
    #[cfg(target_arch = "aarch64")]
    {
        raspberrypi::run_tests();
    }
    
    println!("[drivers] Driver tests complete");
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
