//! USB Hub Driver
//!
//! Handles USB hub devices and port management.

use crate::println;
use crate::error::UsbError;
use super::{UsbDriver, UsbDevice, UsbClass};

/// USB Hub driver
pub struct HubDriver {
    name: &'static str,
}

impl HubDriver {
    /// Create new hub driver
    pub const fn new() -> Self {
        Self {
            name: "USB Hub",
        }
    }
}

impl UsbDriver for HubDriver {
    fn name(&self) -> &str {
        self.name
    }

    fn supports(&self, device: &UsbDevice) -> bool {
        // Hub class is 0x09
        device.device_descriptor.class == UsbClass::Hub as u8
    }

    fn init(&mut self, device: &mut UsbDevice) -> Result<(), UsbError> {
        println!("[usb-hub] Initializing hub at address {}", device.address);
        
        // TODO: 
        // 1. Get hub descriptor
        // 2. Power on ports
        // 3. Enable status change endpoint
        
        println!("[usb-hub] Hub initialized");
        Ok(())
    }

    fn disconnect(&mut self, device: &UsbDevice) {
        println!("[usb-hub] Hub disconnected from address {}", device.address);
    }
}

/// Hub descriptor
#[derive(Debug, Clone)]
pub struct HubDescriptor {
    /// Number of downstream ports
    pub num_ports: u8,
    /// Hub characteristics
    pub characteristics: u16,
    /// Power on to power good time (in 2ms intervals)
    pub power_on_delay: u8,
    /// Maximum hub controller current (in mA)
    pub max_current: u8,
}

impl HubDescriptor {
    /// Parse hub descriptor from raw bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self, UsbError> {
        if data.len() < 9 {
            return Err(UsbError::InvalidDescriptor);
        }

        Ok(Self {
            num_ports: data[2],
            characteristics: u16::from_le_bytes([data[3], data[4]]),
            power_on_delay: data[5],
            max_current: data[6],
        })
    }
}

/// Hub port status
#[derive(Debug, Clone, Copy)]
pub struct HubPortStatus {
    /// Port status bits
    pub status: u16,
    /// Port change bits
    pub change: u16,
}

impl HubPortStatus {
    /// Check if device is connected
    pub fn is_connected(&self) -> bool {
        (self.status & 0x0001) != 0
    }

    /// Check if port is enabled
    pub fn is_enabled(&self) -> bool {
        (self.status & 0x0002) != 0
    }

    /// Check if port is suspended
    pub fn is_suspended(&self) -> bool {
        (self.status & 0x0004) != 0
    }

    /// Check if port is in over-current condition
    pub fn is_over_current(&self) -> bool {
        (self.status & 0x0008) != 0
    }

    /// Check if port is reset
    pub fn is_reset(&self) -> bool {
        (self.status & 0x0010) != 0
    }

    /// Get port power status
    pub fn is_powered(&self) -> bool {
        (self.status & 0x0100) != 0
    }

    /// Check for connection status change
    pub fn connection_changed(&self) -> bool {
        (self.change & 0x0001) != 0
    }
}
