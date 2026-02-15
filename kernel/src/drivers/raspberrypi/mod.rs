//! Raspberry Pi Specific Drivers
//!
//! This module contains drivers specific to the Raspberry Pi family of devices.
//! Currently supports:
//! - Raspberry Pi 5 (with RP1 I/O controller)
//! - Raspberry Pi 4 (BCM2711)
//!
//! # Architecture Notes
//!
//! ## Raspberry Pi 5 Changes
//! The Pi 5 uses a new I/O architecture with the RP1 chip handling GPIO, UART, and other
//! peripherals. This is accessed via PCIe or a dedicated high-speed bus, rather than
//! being memory-mapped directly on the SoC like previous models.
//!
//! ## Backward Compatibility
//! The drivers in this module attempt to maintain compatibility with Pi 4 by detecting
//! the platform and using appropriate register offsets and initialization sequences.

pub mod gpio;
pub mod uart;
pub mod usb;
pub mod ethernet;

use crate::println;

/// Initialize all Raspberry Pi specific drivers
pub fn init() {
    println!("[RaspberryPi] Initializing Pi-specific drivers...");
    
    // Initialize GPIO
    gpio::init();
    
    // Initialize UART (already done early for serial console)
    uart::init();
    
    // Initialize USB subsystem
    usb::init();
    
    // Initialize Ethernet
    ethernet::init();
    
    println!("[RaspberryPi] All drivers initialized");
}

/// Run driver self-tests
pub fn run_tests() {
    println!("[RaspberryPi] Running driver tests...");
    
    // GPIO test
    gpio::print_info();
    
    // UART test
    uart::print_info();
    uart::test_pattern();
    
    // USB info
    usb::print_info();
    
    // Ethernet info
    ethernet::print_info();
    
    println!("[RaspberryPi] Driver tests complete");
}

/// Driver version information
pub const DRIVER_VERSION: &str = "0.1.0";
pub const DRIVER_DATE: &str = "2026-02-15";

/// Print driver version information
pub fn print_version() {
    println!("Raspberry Pi Driver Module");
    println!("  Version: {}", DRIVER_VERSION);
    println!("  Date: {}", DRIVER_DATE);
    println!("  Supported platforms:");
    println!("    - Raspberry Pi 5 (RP1 I/O controller)");
    println!("    - Raspberry Pi 4 (BCM2711)");
    println!("    - QEMU virt machine (test target)");
}
