//! AHCI (Advanced Host Controller Interface) driver

use crate::println;
//
// Driver for SATA controllers in AHCI mode.

use crate::drivers::pci::{self, PciDevice};
use crate::drivers::pci::class;

/// AHCI BAR5 size
const AHCI_BAR5_SIZE: usize = 0x1100;

/// HBA memory registers
#[repr(C)]
struct HbaMemory {
    // Host capability
    cap: u32,
    // Global host control
    ghc: u32,
    // Interrupt status
    is: u32,
    // Ports implemented
    pi: u32,
    // Version
    vs: u32,
    // Command completion coalescing control
    ccc_ctl: u32,
    // Command completion coalescing ports
    ccc_pts: u32,
    // Enclosure management location
    em_loc: u32,
    // Enclosure management control
    em_ctl: u32,
    // Host capabilities extended
    cap2: u32,
    // BIOS/OS handoff control and status
    bohc: u32,
    // Reserved
    _reserved: [u8; 0x74],
    // Vendor specific
    _vendor: [u8; 0x60],
    // Port registers (0-31)
    ports: [HbaPort; 32],
}

/// HBA Port registers
#[repr(C)]
struct HbaPort {
    // Command list base address
    clb: u32,
    // Command list base address upper 32-bits
    clbu: u32,
    // FIS base address
    fb: u32,
    // FIS base address upper 32-bits
    fbu: u32,
    // Interrupt status
    is: u32,
    // Interrupt enable
    ie: u32,
    // Command and status
    cmd: u32,
    // Reserved
    _reserved0: u32,
    // Task file data
    tfd: u32,
    // Signature
    sig: u32,
    // SATA status
    ssts: u32,
    // SATA control
    sctl: u32,
    // SATA error
    serr: u32,
    // SATA active
    sact: u32,
    // Command issue
    ci: u32,
    // SATA notification
    sntf: u32,
    // FIS-based switch control
    fbs: u32,
    // Device sleep
    devslp: u32,
    // Reserved
    _reserved1: [u32; 10],
    // Vendor specific
    _vendor: [u32; 4],
}

/// AHCI controller state
struct AhciController {
    /// PCI device
    pci_dev: PciDevice,
    /// HBA memory base address
    hba_base: u64,
    /// Number of ports
    num_ports: u32,
}

/// Initialize AHCI controllers
pub fn init() {
    println!("[ahci] Looking for AHCI controllers...");

    // Find SATA controllers in AHCI mode
    let devices = pci::get_devices();
    
    for dev in devices.iter() {
        if dev.class == class::MASS_STORAGE && dev.subclass == 0x06 {
            println!("[ahci] Found AHCI controller at {:02X}:{:02X}.{}: {:04X}:{:04X}",
                dev.bus, dev.device, dev.function,
                dev.vendor_id, dev.device_id);
            
            // TODO: Initialize controller
            // For now just print that we found it
            
            if let Err(e) = init_controller(dev) {
                println!("[ahci] Failed to initialize controller: {:?}", e);
            }
        }
    }
}

/// Initialize a single AHCI controller
fn init_controller(dev: &PciDevice) -> Result<(), AhciError> {
    // Get BAR5 (HBA memory registers)
    let bar5 = dev.bars[5] as u64;
    
    if bar5 == 0 {
        return Err(AhciError::NoBar5);
    }

    // 32-bit BAR (bars[6] doesn't exist in our structure)
    let hba_base = (bar5 & 0xFFFFFFF0) as u64;

    println!("[ahci] HBA base address: {:016X}", hba_base);

    // TODO: Map HBA memory and initialize ports
    // This requires proper MMIO support

    Ok(())
}

/// Check if port has a device connected
fn check_port(_port: &HbaPort) -> bool {
    // TODO: Check port status
    false
}

/// AHCI error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AhciError {
    /// No BAR5 found
    NoBar5,
    /// Memory mapping failed
    MapFailed,
    /// No devices found
    NoDevices,
    /// Port error
    PortError,
}

/// Port signature values
mod sig {
    pub const SATA: u32 = 0x00000101;
    pub const SATAPI: u32 = 0xEB140101;
    pub const SEMB: u32 = 0xC33C0101;
    pub const PM: u32 = 0x96690101;
}

/// Command flags
mod cmd {
    pub const START: u32 = 1 << 0;
    pub const SPIN_UP: u32 = 1 << 1;
    pub const POWER_ON: u32 = 1 << 2;
    pub const CLO: u32 = 1 << 3;
    pub const FRE: u32 = 1 << 4;
    pub const FR: u32 = 1 << 14;
    pub const CR: u32 = 1 << 15;
}

/// Task file data flags
mod tfd {
    pub const ERR: u32 = 1 << 0;
    pub const DRQ: u32 = 1 << 3;
    pub const BSY: u32 = 1 << 7;
}
