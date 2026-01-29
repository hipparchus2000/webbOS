//! PCI/PCIe bus driver
//!
//! Enumerates PCI devices and provides access to configuration space.

use alloc::vec::Vec;
use lazy_static::lazy_static;
use spin::Mutex;
use crate::println;

/// PCI Configuration Space ports
const CONFIG_ADDRESS: u16 = 0xCF8;
const CONFIG_DATA: u16 = 0xCFC;

/// PCI Device structure
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
    /// Read configuration space
    pub fn read_config(&self, offset: u8) -> u32 {
        let address = pci_address(self.bus, self.device, self.function, offset);
        unsafe {
            // Write address
            core::arch::asm!(
                "out dx, eax",
                in("dx") CONFIG_ADDRESS,
                in("eax") address,
                options(nomem, nostack)
            );
            
            // Read data
            let val: u32;
            core::arch::asm!(
                "in eax, dx",
                in("dx") CONFIG_DATA,
                out("eax") val,
                options(nomem, nostack)
            );
            
            val
        }
    }

    /// Write configuration space
    pub fn write_config(&self, offset: u8, value: u32) {
        let address = pci_address(self.bus, self.device, self.function, offset);
        unsafe {
            // Write address
            core::arch::asm!(
                "out dx, eax",
                in("dx") CONFIG_ADDRESS,
                in("eax") address,
                options(nomem, nostack)
            );
            
            // Write data
            core::arch::asm!(
                "out dx, eax",
                in("dx") CONFIG_DATA,
                in("eax") value,
                options(nomem, nostack)
            );
        }
    }

    /// Get device description
    pub fn description(&self) -> &'static str {
        match (self.class, self.subclass) {
            (0x01, 0x01) => "IDE Controller",
            (0x01, 0x06) => "SATA Controller",
            (0x01, 0x08) => "NVMe Controller",
            (0x02, 0x00) => "Ethernet Controller",
            (0x03, 0x00) => "VGA Controller",
            (0x06, 0x01) => "ISA Bridge",
            (0x06, 0x04) => "PCI-to-PCI Bridge",
            _ => "Unknown Device",
        }
    }

    /// Check if device is valid (has non-zero vendor)
    pub fn is_valid(&self) -> bool {
        self.vendor_id != 0xFFFF && self.vendor_id != 0
    }
}

lazy_static! {
    /// Global PCI device list
    static ref PCI_DEVICES: Mutex<Vec<PciDevice>> = Mutex::new(Vec::new());
}

/// Generate PCI configuration address
fn pci_address(bus: u8, device: u8, function: u8, offset: u8) -> u32 {
    ((bus as u32) << 16) |
    ((device as u32) << 11) |
    ((function as u32) << 8) |
    ((offset as u32) & 0xFC) |
    0x80000000
}

/// Initialize PCI and enumerate devices
pub fn init() {
    println!("[pci] Enumerating PCI bus...");

    let mut devices = PCI_DEVICES.lock();
    devices.clear();

    // Scan all buses (0-255), devices (0-31), functions (0-7)
    for bus in 0..=255u8 {
        for device in 0..32u8 {
            for function in 0..8u8 {
                // Skip functions > 0 if not a multifunction device
                if function > 0 {
                    let header = read_config8(bus, device, 0, 0x0E);
                    if header & 0x80 == 0 {
                        continue;
                    }
                }

                let vendor_id = read_config16(bus, device, function, 0x00);
                
                if vendor_id == 0xFFFF {
                    continue; // No device
                }

                let device_id = read_config16(bus, device, function, 0x02);
                let class = read_config8(bus, device, function, 0x0B);
                let subclass = read_config8(bus, device, function, 0x0A);
                let prog_if = read_config8(bus, device, function, 0x09);
                let header_type = read_config8(bus, device, function, 0x0E);

                let mut bars = [0u32; 6];
                for i in 0..6 {
                    bars[i] = read_config32(bus, device, function, 0x10 + (i as u8 * 4));
                }

                let pci_dev = PciDevice {
                    bus,
                    device,
                    function,
                    vendor_id,
                    device_id,
                    class,
                    subclass,
                    prog_if,
                    header_type,
                    bars,
                };

                println!("[pci] Found {:04X}:{:04X} at {:02X}:{:02X}.{} - {}",
                    vendor_id, device_id, bus, device, function,
                    pci_dev.description());

                devices.push(pci_dev);

                // Only scan function 0 if not multifunction
                if function == 0 && header_type & 0x80 == 0 {
                    break;
                }
            }
        }
    }

    println!("[pci] Found {} PCI devices", devices.len());
}

/// Read 8-bit value from PCI config space
pub fn read_config8(bus: u8, device: u8, function: u8, offset: u8) -> u8 {
    let address = pci_address(bus, device, function, offset);
    unsafe {
        core::arch::asm!(
            "out dx, eax",
            in("dx") CONFIG_ADDRESS,
            in("eax") address,
            options(nomem, nostack)
        );
        
        let val: u32;
        core::arch::asm!(
            "in eax, dx",
            in("dx") CONFIG_DATA,
            out("eax") val,
            options(nomem, nostack)
        );
        
        (val >> ((offset & 3) * 8)) as u8
    }
}

/// Read 16-bit value from PCI config space
pub fn read_config16(bus: u8, device: u8, function: u8, offset: u8) -> u16 {
    let address = pci_address(bus, device, function, offset);
    unsafe {
        core::arch::asm!(
            "out dx, eax",
            in("dx") CONFIG_ADDRESS,
            in("eax") address,
            options(nomem, nostack)
        );
        
        let val: u16;
        core::arch::asm!(
            "in eax, dx",
            in("dx") CONFIG_DATA,
            out("eax") val,
            options(nomem, nostack)
        );
        
        (val >> ((offset & 2) * 8)) as u16
    }
}

/// Read 32-bit value from PCI config space
pub fn read_config32(bus: u8, device: u8, function: u8, offset: u8) -> u32 {
    let address = pci_address(bus, device, function, offset);
    unsafe {
        core::arch::asm!(
            "out dx, eax",
            in("dx") CONFIG_ADDRESS,
            in("eax") address,
            options(nomem, nostack)
        );
        
        let val: u32;
        core::arch::asm!(
            "in eax, dx",
            in("dx") CONFIG_DATA,
            out("eax") val,
            options(nomem, nostack)
        );
        
        val
    }
}

/// Find device by class/subclass
pub fn find_device(class: u8, subclass: u8) -> Option<PciDevice> {
    let devices = PCI_DEVICES.lock();
    
    for dev in devices.iter() {
        if dev.class == class && dev.subclass == subclass {
            return Some(*dev);
        }
    }
    
    None
}

/// Find device by vendor/device ID
pub fn find_device_by_id(vendor_id: u16, device_id: u16) -> Option<PciDevice> {
    let devices = PCI_DEVICES.lock();
    
    for dev in devices.iter() {
        if dev.vendor_id == vendor_id && dev.device_id == device_id {
            return Some(*dev);
        }
    }
    
    None
}

/// Get all devices
pub fn get_devices() -> Vec<PciDevice> {
    PCI_DEVICES.lock().clone()
}

/// Print PCI device list
pub fn print_devices() {
    let devices = PCI_DEVICES.lock();
    
    println!("PCI Devices:");
    println!("Bus  Dev Fn   Vendor Device Description");
    println!("------------------------------------------------");
    
    for dev in devices.iter() {
        println!("{:02X}:{:02X} {:02X}   {:04X}   {:04X}   {}",
            dev.bus, dev.device, dev.function,
            dev.vendor_id, dev.device_id,
            dev.description());
    }
}

/// Common PCI class codes
pub mod class {
    pub const MASS_STORAGE: u8 = 0x01;
    pub const NETWORK: u8 = 0x02;
    pub const DISPLAY: u8 = 0x03;
    pub const MULTIMEDIA: u8 = 0x04;
    pub const MEMORY: u8 = 0x05;
    pub const BRIDGE: u8 = 0x06;
    pub const SERIAL: u8 = 0x0C;
}

/// Common PCI subclass codes
pub mod subclass {
    pub const IDE: u8 = 0x01;
    pub const SATA: u8 = 0x06;
    pub const NVME: u8 = 0x08;
    pub const ETHERNET: u8 = 0x00;
    pub const VGA: u8 = 0x00;
}

/// Common PCI vendor IDs
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
