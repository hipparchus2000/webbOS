//! USB Controller Driver for Raspberry Pi 5
//!
//! Research and skeleton implementation for USB 3.0/2.0 support.
//!
//! # Raspberry Pi 5 USB Architecture
//!
//! The Raspberry Pi 5 features significant USB improvements over previous models:
//! - 1x USB 3.0 port (via PCIe XHCI controller)
//! - 3x USB 2.0 ports (via internal hub or separate controllers)
//! - USB Type-C for power and data
//!
//! # USB Controllers
//!
//! ## XHCI (eXtensible Host Controller Interface) - USB 3.0
//! - Standard: USB 3.2 Gen 1 (formerly USB 3.0)
//! - Speed: Up to 5 Gbps
//! - Controller: Typically VIA VL805 or similar PCIe XHCI
//! - BAR: Accessed via PCIe configuration space
//!
//! ## DWC2 (DesignWare Core 2) - USB 2.0
//! - Synopsys DesignWare USB 2.0 controller
//! - Used for USB 2.0 ports
//! - May be connected to an internal hub
//!
//! # Implementation Plan
//!
//! ## Phase 1: USB 2.0 (EHCI/OHCI or DWC2)
//! 1. Implement basic host controller initialization
//! 2. USB device enumeration
//! 3. Control transfers
//! 4. Bulk transfers for mass storage
//!
//! ## Phase 2: USB 3.0 (XHCI)
//! 1. PCIe enumeration to find XHCI controller
//! 2. XHCI initialization and memory structures
//! 3. USB 3.0 device support
//! 4. SuperSpeed transfers
//!
//! # References
//! - XHCI Specification 1.2: https://www.intel.com/content/www/us/en/io/universal-serial-bus/extensible-host-controler-interface-usb-xhci.html
//! - USB 3.2 Specification: https://www.usb.org/usb32
//! - Synopsys DWC2 Databook

use crate::hal::{mmio, platform_info};
use crate::println;

/// USB controller types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbControllerType {
    /// XHCI (USB 3.0)
    Xhci,
    /// EHCI (USB 2.0)
    Ehci,
    /// OHCI (USB 1.1/2.0)
    Ohci,
    /// DWC2 (Synopsys USB 2.0)
    Dwc2,
    /// Unknown controller
    Unknown,
}

/// USB device speed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbSpeed {
    /// Low speed (1.5 Mbps)
    Low,
    /// Full speed (12 Mbps)
    Full,
    /// High speed (480 Mbps)
    High,
    /// SuperSpeed (5 Gbps)
    Super,
    /// SuperSpeed+ (10 Gbps)
    SuperPlus,
}

impl UsbSpeed {
    /// Get speed in Mbps
    pub fn mbps(&self) -> u32 {
        match self {
            UsbSpeed::Low => 2,
            UsbSpeed::Full => 12,
            UsbSpeed::High => 480,
            UsbSpeed::Super => 5000,
            UsbSpeed::SuperPlus => 10000,
        }
    }
}

/// USB controller information
#[derive(Debug)]
pub struct UsbControllerInfo {
    /// Controller type
    pub controller_type: UsbControllerType,
    /// Base address (physical)
    pub base_addr: usize,
    /// IRQ number
    pub irq: u32,
    /// Supported speeds
    pub supported_speeds: &'static [UsbSpeed],
    /// Number of ports
    pub num_ports: u8,
    /// PCI device ID (if applicable)
    pub pci_device_id: Option<u16>,
    /// PCI vendor ID (if applicable)
    pub pci_vendor_id: Option<u16>,
}

/// XHCI register offsets
/// From xHCI Specification 1.2
pub mod xhci_regs {
    /// Capability registers base
    pub const CAP_LENGTH: usize = 0x00;
    /// Host Controller Interface Version Number
    pub const HCI_VERSION: usize = 0x02;
    /// Structural Parameters 1
    pub const HCS_PARAMS1: usize = 0x04;
    /// Structural Parameters 2
    pub const HCS_PARAMS2: usize = 0x08;
    /// Structural Parameters 3
    pub const HCS_PARAMS3: usize = 0x0C;
    /// Capability Parameters 1
    pub const HCC_PARAMS1: usize = 0x10;
    /// Doorbell Offset
    pub const DBOFF: usize = 0x14;
    /// Runtime Register Space Offset
    pub const RTSOFF: usize = 0x18;
    /// Capability Parameters 2
    pub const HCC_PARAMS2: usize = 0x1C;
    
    /// Operational registers offset from capability base
    pub const OP_OFFSET: usize = 0x00; // Determined by CAP_LENGTH
    /// USB Command
    pub const USB_CMD: usize = 0x00;
    /// USB Status
    pub const USB_STS: usize = 0x04;
    /// Page Size
    pub const PAGE_SIZE: usize = 0x08;
    /// Device Notification Control
    pub const DNCTRL: usize = 0x14;
    /// Command Ring Control
    pub const CRCR: usize = 0x18;
    /// Device Context Base Address Array Pointer
    pub const DCBAAP: usize = 0x30;
    /// Configure
    pub const CONFIG: usize = 0x38;
    
    /// Port status/control registers start at 0x400
    pub const PORT_STATUS_BASE: usize = 0x400;
    /// Port status register offset (0x400 + port * 0x10)
    pub const PORTSC: usize = 0x00;
    /// Port Power Management Status and Control
    pub const PORTPMSC: usize = 0x04;
    /// Port Link Info
    pub const PORTLI: usize = 0x08;
    /// Port Hardware LPM Control
    pub const PORTHLPMC: usize = 0x0C;
}

/// XHCI USB Command register bits
pub mod xhci_usbcmd {
    /// Run/Stop
    pub const RS: u32 = 1 << 0;
    /// Host Controller Reset
    pub const HCRST: u32 = 1 << 1;
    /// Interrupter Enable
    pub const INTE: u32 = 1 << 2;
    /// Host System Error Enable
    pub const HSEE: u32 = 1 << 3;
}

/// XHCI USB Status register bits
pub mod xhci_usbsts {
    /// HCHalted
    pub const HCH: u32 = 1 << 0;
    /// Host System Error
    pub const HSE: u32 = 1 << 2;
    /// Event Interrupt
    pub const EINT: u32 = 1 << 3;
    /// Port Change Detect
    pub const PCD: u32 = 1 << 4;
    /// Save State Status
    pub const SSS: u32 = 1 << 8;
    /// Restore State Status
    pub const RSS: u32 = 1 << 9;
    /// Save/Restore Error
    pub const SRE: u32 = 1 << 10;
    /// Controller Not Ready
    pub const CNR: u32 = 1 << 11;
    /// Host Controller Error
    pub const HCE: u32 = 1 << 12;
}

/// XHCI Port Status register bits
pub mod xhci_portsc {
    /// Current Connect Status
    pub const CCS: u32 = 1 << 0;
    /// Port Enabled/Disabled
    pub const PED: u32 = 1 << 1;
    /// Port Reset
    pub const PR: u32 = 1 << 4;
    /// Port Link State (mask)
    pub const PLS_MASK: u32 = 0xF << 5;
    /// Port Power
    pub const PP: u32 = 1 << 9;
    /// Port Speed (mask)
    pub const PORT_SPEED_MASK: u32 = 0xF << 10;
    /// Port Indicator Control (mask)
    pub const PIC_MASK: u32 = 0x3 << 14;
    /// Port Link State Write Strobe
    pub const LWS: u32 = 1 << 16;
    /// Connect Status Change
    pub const CSC: u32 = 1 << 17;
    /// Port Enabled/Disabled Change
    pub const PEC: u32 = 1 << 18;
    /// Warm Port Reset Change
    pub const WRC: u32 = 1 << 19;
    /// Over-current Change
    pub const OCC: u32 = 1 << 20;
    /// Port Reset Change
    pub const PRC: u32 = 1 << 21;
    /// Port Link State Change
    pub const PLC: u32 = 1 << 22;
    /// Port Config Error Change
    pub const CEC: u32 = 1 << 23;
    /// Cold Attach Status
    pub const CAS: u32 = 1 << 24;
    /// Wake on Connect Enable
    pub const WCE: u32 = 1 << 25;
    /// Wake on Disconnect Enable
    pub const WDE: u32 = 1 << 26;
    /// Wake on Over-current Enable
    pub const WOE: u32 = 1 << 27;
    /// Device Removable
    pub const DR: u32 = 1 << 30;
    /// Warm Port Reset
    pub const WPR: u32 = 1 << 31;
}

/// USB driver state
pub struct UsbDriver {
    /// List of detected controllers
    controllers: [Option<UsbControllerInfo>; 4],
    /// Number of controllers
    num_controllers: usize,
    /// Initialized flag
    initialized: bool,
}

/// Global USB driver instance
static mut USB_DRIVER: UsbDriver = UsbDriver {
    controllers: [None, None, None, None],
    num_controllers: 0,
    initialized: false,
};

/// Initialize USB subsystem
pub fn init() {
    println!("[USB] Initializing USB subsystem...");
    
    let drv = driver();
    
    // Detect USB controllers
    detect_controllers();
    
    println!("[USB] Found {} USB controller(s)", drv.num_controllers);
    
    // Initialize each controller
    for i in 0..drv.num_controllers {
        if let Some(ref info) = drv.controllers[i] {
            println!("[USB] Controller {}: {:?}", i, info.controller_type);
            
            match info.controller_type {
                UsbControllerType::Xhci => {
                    if let Err(e) = init_xhci(info) {
                        println!("[USB] XHCI initialization failed: {:?}", e);
                    }
                }
                _ => {
                    println!("[USB] Controller type not yet implemented");
                }
            }
        }
    }
    
    drv.initialized = true;
    println!("[USB] Initialization complete");
}

/// Detect USB controllers on the system
fn detect_controllers() {
    let drv = driver();
    let info = platform_info();
    
    match info.platform_type {
        crate::hal::PlatformType::RaspberryPi5 => {
            // Pi 5 has XHCI via PCIe
            // We need to scan PCIe to find the XHCI controller
            detect_pcie_xhci();
            
            // Also check for DWC2 USB 2.0 controller
            detect_dwc2();
        }
        crate::hal::PlatformType::RaspberryPi4 => {
            // Pi 4 uses VIA VL805 PCIe USB 3.0 controller
            detect_pcie_xhci();
        }
        crate::hal::PlatformType::QemuVirt => {
            // QEMU may have XHCI via PCIe
            println!("[USB] QEMU virt: Checking for XHCI...");
            // In QEMU, we can configure XHCI
            if let Some(xhci_info) = create_qemu_xhci_info() {
                add_controller(xhci_info);
            }
        }
        _ => {
            println!("[USB] Unknown platform, no USB controllers detected");
        }
    }
}

/// Detect XHCI controller via PCIe
fn detect_pcie_xhci() {
    // This would scan the PCIe bus for USB controllers
    // Looking for Class Code 0x0C0330 (Serial Bus Controller / USB / XHCI)
    println!("[USB] Scanning PCIe for XHCI controllers...");
    
    // For now, assume XHCI at a known address on Pi 5
    // In reality, this would be discovered via PCIe enumeration
    let info = platform_info();
    
    let xhci_info = UsbControllerInfo {
        controller_type: UsbControllerType::Xhci,
        base_addr: info.usb_xhci_base,
        irq: 144, // Typical GIC SPI for XHCI on Pi 5
        supported_speeds: &[UsbSpeed::Super, UsbSpeed::High, UsbSpeed::Full, UsbSpeed::Low],
        num_ports: 4,
        pci_device_id: Some(0x3483), // VIA VL805
        pci_vendor_id: Some(0x1106), // VIA
    };
    
    add_controller(xhci_info);
}

/// Detect DWC2 USB 2.0 controller
fn detect_dwc2() {
    println!("[USB] Checking for DWC2 controller...");
    
    // DWC2 is typically at a fixed address on Raspberry Pi
    // It's used for USB 2.0 ports
    let info = UsbControllerInfo {
        controller_type: UsbControllerType::Dwc2,
        base_addr: 0x1F00000000 + 0x980000, // Typical DWC2 address
        irq: 73, // GIC SPI for DWC2
        supported_speeds: &[UsbSpeed::High, UsbSpeed::Full, UsbSpeed::Low],
        num_ports: 1,
        pci_device_id: None,
        pci_vendor_id: None,
    };
    
    add_controller(info);
}

/// Create XHCI info for QEMU virt machine
fn create_qemu_xhci_info() -> Option<UsbControllerInfo> {
    // QEMU virt machine XHCI is typically at 0x09000000
    Some(UsbControllerInfo {
        controller_type: UsbControllerType::Xhci,
        base_addr: 0x09000000,
        irq: 112,
        supported_speeds: &[UsbSpeed::Super, UsbSpeed::High, UsbSpeed::Full, UsbSpeed::Low],
        num_ports: 4,
        pci_device_id: Some(0x0001),
        pci_vendor_id: Some(0x1B36), // QEMU
    })
}

/// Add a controller to the driver
fn add_controller(info: UsbControllerInfo) {
    let drv = driver();
    
    if drv.num_controllers < drv.controllers.len() {
        drv.controllers[drv.num_controllers] = Some(info);
        drv.num_controllers += 1;
    }
}

/// Initialize an XHCI controller
fn init_xhci(info: &UsbControllerInfo) -> Result<(), UsbError> {
    println!("[USB] Initializing XHCI controller at 0x{:016X}", info.base_addr);
    
    unsafe {
        // Read capability registers
        let cap_length = mmio::read32(info.base_addr + xhci_regs::CAP_LENGTH) as usize;
        let hci_version = mmio::read16(info.base_addr + xhci_regs::HCI_VERSION);
        let hcs_params1 = mmio::read32(info.base_addr + xhci_regs::HCS_PARAMS1);
        let hcs_params2 = mmio::read32(info.base_addr + xhci_regs::HCS_PARAMS2);
        let hcc_params1 = mmio::read32(info.base_addr + xhci_regs::HCC_PARAMS1);
        
        println!("[USB] XHCI Version: {}.{}", hci_version >> 8, hci_version & 0xFF);
        println!("[USB] Capability length: {}", cap_length);
        
        let max_slots = hcs_params1 & 0xFF;
        let max_intrs = (hcs_params1 >> 8) & 0x7FF;
        let max_ports = (hcs_params1 >> 24) & 0xFF;
        
        println!("[USB] Max slots: {}, Max intrs: {}, Max ports: {}", 
                 max_slots, max_intrs, max_ports);
        
        // Operational registers base
        let op_base = info.base_addr + cap_length;
        
        // Check if controller is halted
        let usbsts = mmio::read32(op_base + xhci_regs::USB_STS);
        if usbsts & xhci_usbsts::HCH == 0 {
            println!("[USB] XHCI is running, stopping...");
            // Clear run bit
            let usbcmd = mmio::read32(op_base + xhci_regs::USB_CMD);
            mmio::write32(op_base + xhci_regs::USB_CMD, usbcmd & !xhci_usbcmd::RS);
            
            // Wait for halt
            let mut timeout = 1000;
            while timeout > 0 {
                let sts = mmio::read32(op_base + xhci_regs::USB_STS);
                if sts & xhci_usbsts::HCH != 0 {
                    break;
                }
                timeout -= 1;
                crate::hal::delay::microseconds(10);
            }
            
            if timeout == 0 {
                return Err(UsbError::Timeout);
            }
        }
        
        // Reset the controller
        println!("[USB] Resetting XHCI...");
        let usbcmd = mmio::read32(op_base + xhci_regs::USB_CMD);
        mmio::write32(op_base + xhci_regs::USB_CMD, usbcmd | xhci_usbcmd::HCRST);
        
        // Wait for reset to complete
        let mut timeout = 1000;
        while timeout > 0 {
            let cmd = mmio::read32(op_base + xhci_regs::USB_CMD);
            if cmd & xhci_usbcmd::HCRST == 0 {
                break;
            }
            timeout -= 1;
            crate::hal::delay::microseconds(10);
        }
        
        if timeout == 0 {
            return Err(UsbError::Timeout);
        }
        
        // Check controller not ready bit
        let sts = mmio::read32(op_base + xhci_regs::USB_STS);
        if sts & xhci_usbsts::CNR != 0 {
            return Err(UsbError::ControllerNotReady);
        }
        
        println!("[USB] XHCI reset complete");
        
        // TODO: Initialize memory structures
        // - Device Context Base Address Array
        // - Command Ring
        // - Event Ring
        // - DCBAA
        
        // TODO: Start the controller
        // - Set up interrupts
        // - Start the controller (set RS bit)
        
        println!("[USB] XHCI initialization stub - full implementation needed");
    }
    
    Ok(())
}

/// USB error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbError {
    /// Success
    Success = 0,
    /// Device not found
    NotFound = 1,
    /// Initialization failed
    InitFailed = 2,
    /// Timeout
    Timeout = 3,
    /// Controller not ready
    ControllerNotReady = 4,
    /// Invalid parameter
    InvalidParameter = 5,
    /// Memory allocation failed
    NoMemory = 6,
    /// Transfer error
    TransferError = 7,
    /// Stall error
    Stall = 8,
}

/// Get the USB driver instance
fn driver() -> &'static mut UsbDriver {
    unsafe { &mut USB_DRIVER }
}

/// Print USB driver information
pub fn print_info() {
    let drv = driver();
    
    println!("USB Driver Information:");
    println!("  Initialized: {}", drv.initialized);
    println!("  Controllers: {}", drv.num_controllers);
    
    for i in 0..drv.num_controllers {
        if let Some(ref info) = drv.controllers[i] {
            println!("  Controller {}:", i);
            println!("    Type: {:?}", info.controller_type);
            println!("    Base: 0x{:016X}", info.base_addr);
            println!("    IRQ: {}", info.irq);
            println!("    Ports: {}", info.num_ports);
            if let (Some(vid), Some(did)) = (info.pci_vendor_id, info.pci_device_id) {
                println!("    PCI: {:04X}:{:04X}", vid, did);
            }
        }
    }
}

/// Research notes on USB implementation
pub mod research {
    //! # USB Implementation Research Notes
    //!
    //! ## XHCI Memory Structures Required:
    //!
    //! ### 1. Device Context Base Address Array (DCBAA)
    //! - Array of pointers to device contexts
    //! - One entry per possible device (up to 255 slots)
    //! - Must be 64-byte aligned
    //!
    //! ### 2. Command Ring
    //! - Circular queue of TRBs (Transfer Request Blocks)
    //! - Used to send commands to the controller
    //! - Must be 64-byte aligned, max 64K TRBs
    //!
    //! ### 3. Event Ring
    //! - Circular queue of TRBs from controller to software
    //! - Used for completion notifications
    //! - Requires Event Ring Segment Table (ERST)
    //!
    //! ### 4. Transfer Rings
    //! - One per endpoint
    //! - Queue of TRBs for data transfers
    //!
    //! ## USB Device Enumeration Steps:
    //!
    //! 1. Detect device connection (port status change)
    //! 2. Reset the port
    //! 3. Determine device speed
    //! 4. Assign slot and allocate resources
    //! 5. Send SET_ADDRESS request
    //! 6. Get device descriptor
    //! 7. Get configuration descriptor
    //! 8. Set configuration
    //! 9. Load and initialize device driver
    //!
    //! ## Key Challenges:
    //!
    //! - PCIe enumeration required for XHCI
    //! - Complex memory structure management
    //! - DMA and cache coherency
    //! - Interrupt handling
    //! - Hub support for multiple devices
}

/// Helper function to read 16-bit register
unsafe fn mmio_read16(addr: usize) -> u16 {
    mmio::read32(addr) as u16
}
