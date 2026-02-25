//! PCI/PCIe bus driver (stub for ARM/Raspberry Pi)
//!
//! The Raspberry Pi does not have traditional PCI/PCIe buses.
//! Instead, it uses:
//! - USB for external devices
//! - SDIO for WiFi/SD card
//! - Dedicated MMIO for on-chip peripherals
//!
#![allow(dead_code)]

//! This module provides a stub implementation for compatibility.

use alloc::vec::Vec;
use lazy_static::lazy_static;
use spin::Mutex;
use crate::println;

/// PCI Device structure (stub)
#[derive(Debug, Clone, Copy)]
pub struct PciDevice {
    /// Bus number
    pub bus: u8,
    /// Device number
    pub device: u8,
    /// Function number
    pub function: u8,
    /// Vendor ID
    pub vendor_id: u16,
    /// Device ID
    pub device_id: u16,
    /// Class code
    pub class: u8,
    /// Subclass
    pub subclass: u8,
    /// Programming interface
    pub prog_if: u8,
    /// Header type
    pub header_type: u8,
    /// Base address registers
    pub bars: [u32; 6],
}

impl PciDevice {
    /// Read configuration space (stub)
    pub fn read_config(&self, _offset: u8) -> u32 {
        0xFFFF_FFFF // No device
    }

    /// Write configuration space (stub)
    pub fn write_config(&self, _offset: u8, _value: u32) {
        // No-op on Pi
    }

    /// Get device description
    pub fn description(&self) -> &'static str {
        "Not Available on ARM"
    }

    /// Check if device is valid
    pub fn is_valid(&self) -> bool {
        false // No PCI devices on Pi
    }
}

lazy_static! {
    /// Global PCI device list (empty on Pi)
    static ref PCI_DEVICES: Mutex<Vec<PciDevice>> = Mutex::new(Vec::new());
}

/// Initialize PCI and enumerate devices (stub on Pi)
pub fn init() {
    println!("[pci] PCI not available on Raspberry Pi");
    println!("[pci] Using native drivers for USB, SDIO, and MMIO devices");
}

/// Read 8-bit value from PCI config space (stub)
pub fn read_config8(_bus: u8, _device: u8, _function: u8, _offset: u8) -> u8 {
    0xFF
}

/// Read 16-bit value from PCI config space (stub)
pub fn read_config16(_bus: u8, _device: u8, _function: u8, _offset: u8) -> u16 {
    0xFFFF
}

/// Read 32-bit value from PCI config space (stub)
pub fn read_config32(_bus: u8, _device: u8, _function: u8, _offset: u8) -> u32 {
    0xFFFF_FFFF
}

/// Find device by class/subclass (stub - always returns None on Pi)
pub fn find_device(_class: u8, _subclass: u8) -> Option<PciDevice> {
    None
}

/// Find device by vendor/device ID (stub - always returns None on Pi)
pub fn find_device_by_id(_vendor_id: u16, _device_id: u16) -> Option<PciDevice> {
    None
}

/// Get all devices (empty list on Pi)
pub fn get_devices() -> Vec<PciDevice> {
    Vec::new()
}

/// Print PCI device list (shows message on Pi)
pub fn print_devices() {
    println!("PCI Devices:");
    println!("  (PCI not available on Raspberry Pi)");
    println!("  Use native drivers for USB, SDIO, and MMIO devices");
}

/// Common PCI class codes (kept for reference)
pub mod class {
    pub const MASS_STORAGE: u8 = 0x01;
    pub const NETWORK: u8 = 0x02;
    pub const DISPLAY: u8 = 0x03;
    pub const MULTIMEDIA: u8 = 0x04;
    pub const MEMORY: u8 = 0x05;
    pub const BRIDGE: u8 = 0x06;
    pub const SERIAL: u8 = 0x0C;
}

/// Common PCI subclass codes (kept for reference)
pub mod subclass {
    pub const IDE: u8 = 0x01;
    pub const SATA: u8 = 0x06;
    pub const NVME: u8 = 0x08;
    pub const ETHERNET: u8 = 0x00;
    pub const VGA: u8 = 0x00;
}

/// Common PCI vendor IDs (kept for reference)
pub mod vendor {
    pub const INTEL: u16 = 0x8086;
    pub const AMD: u16 = 0x1022;
    pub const NVIDIA: u16 = 0x10DE;
    pub const REALTEK: u16 = 0x10EC;
    pub const QEMU: u16 = 0x1234;
    pub const RED_HAT: u16 = 0x1AF4; // VirtIO
    pub const VMWARE: u16 = 0x15AD;
    pub const VIA: u16 = 0x1106;
}
