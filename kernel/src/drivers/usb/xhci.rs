//! xHCI (eXtensible Host Controller Interface) Driver
//!
//! Implements USB 3.0/3.1 host controller support.
//!
//! # References
//! - XHCI Specification 1.2
//! - Intel XHCI Programmer's Reference

use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::format;
use core::ptr::{read_volatile, write_volatile};

use crate::println;
use crate::error::UsbError;
use super::{UsbSpeed, DeviceDescriptor};

/// XHCI PCI class code
pub const XHCI_PCI_CLASS: u8 = 0x0C;
/// XHCI PCI subclass
pub const XHCI_PCI_SUBCLASS: u8 = 0x03;
/// XHCI PCI programming interface
pub const XHCI_PCI_PROG_IF: u8 = 0x30;

/// Capability registers offset
const CAP_LENGTH: usize = 0x00;
const HCI_VERSION: usize = 0x02;
const HCS_PARAMS1: usize = 0x04;
const HCS_PARAMS2: usize = 0x08;
const HCS_PARAMS3: usize = 0x0C;
const HCC_PARAMS1: usize = 0x10;
const DBOFF: usize = 0x14;
const RTSOFF: usize = 0x18;

/// Operational registers (offset by cap_length)
const USB_CMD: usize = 0x00;
const USB_STS: usize = 0x04;
const PAGE_SIZE: usize = 0x08;
const DNCTRL: usize = 0x14;
const CRCR: usize = 0x18;
const DCBAAP: usize = 0x30;
const CONFIG: usize = 0x38;

/// Port status registers base
const PORT_STATUS_BASE: usize = 0x400;
const PORTSC_OFFSET: usize = 0x00;
const PORTPMSC_OFFSET: usize = 0x04;
const PORTLI_OFFSET: usize = 0x08;
const PORTHLPMC_OFFSET: usize = 0x0C;

/// Runtime registers (offset by rtsoff)
const MFINDEX: usize = 0x00;

/// Interrupter registers (offset by runtime_regs + 0x20 + n*0x20)
const IMAN: usize = 0x00;
const IMOD: usize = 0x04;
const ERSTSZ: usize = 0x08;
const ERSTBA: usize = 0x10;
const ERDP: usize = 0x18;

/// Doorbell registers (offset by dboff)
/// Doorbell n is at dboff + n * 4

/// USB Command register bits
const CMD_RUN_STOP: u32 = 1 << 0;
const CMD_RESET: u32 = 1 << 1;

/// USB Status register bits
const STS_HALTED: u32 = 1 << 0;
const STS_CNR: u32 = 1 << 11;

/// Port Status Control bits
const PORTSC_CCS: u32 = 1 << 0;     // Current Connect Status
const PORTSC_PED: u32 = 1 << 1;     // Port Enabled/Disabled
const PORTSC_OCA: u32 = 1 << 3;     // Over-current Active
const PORTSC_PR: u32 = 1 << 4;      // Port Reset
const PORTSC_PP: u32 = 1 << 9;      // Port Power
const PORTSC_CSC: u32 = 1 << 17;    // Connect Status Change
const PORTSC_PEC: u32 = 1 << 18;    // Port Enabled Change
const PORTSC_WRC: u32 = 1 << 19;    // Warm Reset Change
const PORTSC_OCC: u32 = 1 << 20;    // Over-current Change
const PORTSC_PRC: u32 = 1 << 21;    // Port Reset Change
const PORTSC_PLC: u32 = 1 << 22;    // Port Link State Change
const PORTSC_CEC: u32 = 1 << 23;    // Config Error Change

/// Speed values from PORTSC
const PORTSC_SPEED_SHIFT: u32 = 10;
const PORTSC_SPEED_MASK: u32 = 0xF;

/// xHCI Capability Registers
#[derive(Debug)]
pub struct CapabilityRegisters {
    /// Capability length and version
    pub cap_length: u8,
    pub hci_version: u16,
    /// Structural parameters
    pub max_slots: u8,
    pub max_interrupts: u16,
    pub max_ports: u8,
    /// Offsets
    pub db_offset: u32,
    pub rt_offset: u32,
}

/// xHCI Operational Registers
#[derive(Debug)]
pub struct OperationalRegisters {
    base: usize,
}

impl OperationalRegisters {
    /// Create new operational registers at base address
    pub fn new(base: usize) -> Self {
        Self { base }
    }

    /// Read USB Command register
    pub fn usb_cmd(&self) -> u32 {
        unsafe { read_volatile((self.base + USB_CMD) as *const u32) }
    }

    /// Write USB Command register
    pub fn set_usb_cmd(&self, value: u32) {
        unsafe { write_volatile((self.base + USB_CMD) as *mut u32, value) }
    }

    /// Read USB Status register
    pub fn usb_sts(&self) -> u32 {
        unsafe { read_volatile((self.base + USB_STS) as *const u32) }
    }

    /// Read port status control
    pub fn portsc(&self, port: u8) -> u32 {
        let offset = PORT_STATUS_BASE + (port as usize * 0x10) + PORTSC_OFFSET;
        unsafe { read_volatile((self.base + offset) as *const u32) }
    }

    /// Write port status control
    pub fn set_portsc(&self, port: u8, value: u32) {
        let offset = PORT_STATUS_BASE + (port as usize * 0x10) + PORTSC_OFFSET;
        unsafe { write_volatile((self.base + offset) as *mut u32, value) }
    }

    /// Check if controller is halted
    pub fn is_halted(&self) -> bool {
        (self.usb_sts() & STS_HALTED) != 0
    }

    /// Check if controller is ready
    pub fn is_ready(&self) -> bool {
        (self.usb_sts() & STS_CNR) == 0
    }
}

/// xHCI Event Ring Segment Table Entry
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy)]
pub struct EventRingSegment {
    /// Ring segment base address (64-bit)
    pub ring_segment_base: u64,
    /// Ring segment size
    pub ring_segment_size: u16,
    /// Reserved
    _reserved: [u16; 3],
}

/// Transfer Request Block (TRB) types
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrbType {
    Reserved = 0,
    Normal = 1,
    SetupStage = 2,
    DataStage = 3,
    StatusStage = 4,
    Isoch = 5,
    Link = 6,
    EventData = 7,
    NoOp = 8,
    EnableSlot = 9,
    DisableSlot = 10,
    AddressDevice = 11,
    ConfigureEndpoint = 12,
    EvaluateContext = 13,
    ResetEndpoint = 14,
    StopEndpoint = 15,
    SetTrDequeuePtr = 16,
    ResetDevice = 17,
    ForceEvent = 18,
    NegotiateBandwidth = 19,
    SetLatencyTolerance = 20,
    GetPortBandwidth = 21,
    ForceHeader = 22,
    NoOpCmd = 23,
    GetExtendedProperty = 24,
    SetExtendedProperty = 25,
}

/// Transfer Request Block (TRB)
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
pub struct Trb {
    /// Parameter (depends on TRB type)
    pub parameter: u64,
    /// Status
    pub status: u32,
    /// Control
    pub control: u32,
}

impl Trb {
    /// Create a new TRB
    pub const fn new() -> Self {
        Self {
            parameter: 0,
            status: 0,
            control: 0,
        }
    }

    /// Get TRB type
    pub fn trb_type(&self) -> TrbType {
        let type_num = ((self.control >> 10) & 0x3F) as u8;
        match type_num {
            1 => TrbType::Normal,
            2 => TrbType::SetupStage,
            3 => TrbType::DataStage,
            4 => TrbType::StatusStage,
            6 => TrbType::Link,
            9 => TrbType::EnableSlot,
            10 => TrbType::DisableSlot,
            11 => TrbType::AddressDevice,
            _ => TrbType::Reserved,
        }
    }

    /// Set TRB type
    pub fn set_trb_type(&mut self, trb_type: TrbType) {
        self.control = (self.control & !(0x3F << 10)) | ((trb_type as u32) << 10);
    }

    /// Check if TRB has completion code
    pub fn completion_code(&self) -> u8 {
        ((self.status >> 24) & 0xFF) as u8
    }
}

/// xHCI Event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XhciEvent {
    /// Port connected
    PortConnect { port: u8, speed: UsbSpeed },
    /// Port disconnected
    PortDisconnect { port: u8 },
    /// Transfer completed
    TransferComplete { slot: u8, endpoint: u8, success: bool },
    /// Command completed
    CommandComplete { command: TrbType, success: bool },
    /// Port reset complete
    PortResetComplete { port: u8 },
}

/// xHCI Controller
pub struct XhciController {
    /// MMIO base address
    mmio_base: usize,
    /// Capability registers
    caps: CapabilityRegisters,
    /// Number of ports
    num_ports: u8,
    /// Device slots
    max_slots: u8,
}

impl XhciController {
    /// Initialize xHCI controller
    pub fn init() -> Result<Self, UsbError> {
        println!("[xhci] Looking for xHCI controller...");

        // Find xHCI controller via PCI
        let mmio_base = Self::find_controller()?;
        println!("[xhci] Found controller at MMIO base: {:016X}", mmio_base);

        // Read capability registers
        let caps = Self::read_capabilities(mmio_base);
        let max_ports = caps.max_ports;
        let max_slots = caps.max_slots;
        println!("[xhci] Version: {:04X}, Max slots: {}, Max ports: {}",
            caps.hci_version, max_slots, max_ports);

        let mut controller = Self {
            mmio_base,
            caps,
            num_ports: max_ports,
            max_slots: max_slots,
        };

        // Reset and initialize
        controller.reset()?;
        controller.init_memory_structures()?;
        controller.start()?;

        println!("[xhci] Controller initialized successfully");
        Ok(controller)
    }

    /// Find xHCI controller via PCI
    fn find_controller() -> Result<usize, UsbError> {
        // Try to find XHCI controller in PCI
        if let Some(device) = crate::drivers::pci::find_device(XHCI_PCI_CLASS, XHCI_PCI_SUBCLASS) {
            // Read BAR0 for MMIO base address
            let bar0 = crate::drivers::pci::read_config32(device.bus, device.device, device.function, 0x10);
            
            // Check if BAR is memory mapped (bit 0 should be 0)
            if bar0 & 1 == 0 {
                // Mask off lower bits to get base address
                let base = (bar0 & 0xFFFFFFF0) as usize;
                
                // For x86_64, we need to map this physical address
                // For now, assume it's already mapped or in the identity map
                return Ok(base);
            }
        }

        // Try known addresses for QEMU or other platforms
        // QEMU xHCI typically at 0xFE900000 or similar
        let known_addresses = [0xFE900000usize, 0xFEA00000, 0xFEB00000];
        
        for &addr in &known_addresses {
            // Check if this looks like a valid xHCI controller
            // by reading the capability length and version
            unsafe {
                let cap_length = read_volatile(addr as *const u8);
                let version = read_volatile((addr + 2) as *const u16);
                
                // Valid xHCI should have version 1.0+ and reasonable cap length
                if version >= 0x0100 && cap_length >= 0x20 && cap_length <= 0xFF {
                    println!("[xhci] Found potential controller at {:016X} (ver {:04X})", addr, version);
                    return Ok(addr);
                }
            }
        }

        Err(UsbError::ControllerNotFound)
    }

    /// Read capability registers
    fn read_capabilities(base: usize) -> CapabilityRegisters {
        unsafe {
            let cap_length = read_volatile(base as *const u8);
            let version = read_volatile((base + 2) as *const u16);
            
            let hcs_params1 = read_volatile((base + HCS_PARAMS1) as *const u32);
            let max_slots = (hcs_params1 & 0xFF) as u8;
            let max_interrupts = ((hcs_params1 >> 8) & 0x7FF) as u16;
            let max_ports = ((hcs_params1 >> 24) & 0xFF) as u8;
            
            let db_offset = read_volatile((base + DBOFF) as *const u32);
            let rt_offset = read_volatile((base + RTSOFF) as *const u32);

            CapabilityRegisters {
                cap_length,
                hci_version: version,
                max_slots,
                max_interrupts,
                max_ports,
                db_offset,
                rt_offset,
            }
        }
    }

    /// Get operational registers base
    fn op_base(&self) -> usize {
        self.mmio_base + self.caps.cap_length as usize
    }

    /// Reset controller
    fn reset(&mut self) -> Result<(), UsbError> {
        println!("[xhci] Resetting controller...");

        let op = OperationalRegisters::new(self.op_base());

        // Make sure controller is stopped
        let cmd = op.usb_cmd();
        if cmd & CMD_RUN_STOP != 0 {
            println!("[xhci] Stopping controller...");
            op.set_usb_cmd(cmd & !CMD_RUN_STOP);
            
            // Wait for halt
            let mut timeout = 1000;
            while !op.is_halted() && timeout > 0 {
                timeout -= 1;
                core::hint::spin_loop();
            }
            
            if timeout == 0 {
                return Err(UsbError::ControllerError("Timeout stopping controller".to_string()));
            }
        }

        // Reset controller
        println!("[xhci] Issuing reset...");
        op.set_usb_cmd(cmd | CMD_RESET);

        // Wait for reset complete
        let mut timeout = 1000;
        loop {
            let cmd = op.usb_cmd();
            if cmd & CMD_RESET == 0 {
                break;
            }
            timeout -= 1;
            if timeout == 0 {
                return Err(UsbError::ControllerError("Reset timeout".to_string()));
            }
            core::hint::spin_loop();
        }

        // Wait for ready
        let mut timeout = 1000;
        while !op.is_ready() && timeout > 0 {
            timeout -= 1;
            core::hint::spin_loop();
        }

        if timeout == 0 {
            return Err(UsbError::ControllerError("Controller not ready after reset".to_string()));
        }

        println!("[xhci] Controller reset complete");
        Ok(())
    }

    /// Initialize memory structures
    fn init_memory_structures(&mut self) -> Result<(), UsbError> {
        println!("[xhci] Initializing memory structures...");
        
        // This is where we would:
        // 1. Allocate Device Context Base Address Array (DCBAA)
        // 2. Set up command ring
        // 3. Set up event ring
        // 4. Allocate scratchpad buffers if needed
        
        // For now, we'll do minimal setup
        println!("[xhci] Memory structures initialized (minimal)");
        Ok(())
    }

    /// Start controller
    fn start(&mut self) -> Result<(), UsbError> {
        println!("[xhci] Starting controller...");

        let op = OperationalRegisters::new(self.op_base());

        // Clear status bits
        let status = op.usb_sts();
        unsafe {
            write_volatile((self.op_base() + USB_STS) as *mut u32, status);
        }

        // Set run bit
        let cmd = op.usb_cmd();
        op.set_usb_cmd(cmd | CMD_RUN_STOP);

        // Wait for running
        let mut timeout = 1000;
        while op.is_halted() && timeout > 0 {
            timeout -= 1;
            core::hint::spin_loop();
        }

        if timeout == 0 {
            return Err(UsbError::ControllerError("Controller failed to start".to_string()));
        }

        println!("[xhci] Controller running");
        Ok(())
    }

    /// Poll for events
    pub fn poll_event(&mut self) -> Option<XhciEvent> {
        // Check all ports for connect/disconnect
        for port in 0..self.num_ports {
            let op = OperationalRegisters::new(self.op_base());
            let portsc = op.portsc(port);

            // Check for connect status change
            if portsc & PORTSC_CSC != 0 {
                // Clear the change bit
                op.set_portsc(port, portsc | PORTSC_CSC);

                if portsc & PORTSC_CCS != 0 {
                    // Device connected
                    let speed = Self::decode_speed((portsc >> PORTSC_SPEED_SHIFT) & PORTSC_SPEED_MASK);
                    return Some(XhciEvent::PortConnect { port, speed });
                } else {
                    // Device disconnected
                    return Some(XhciEvent::PortDisconnect { port });
                }
            }
        }

        None
    }

    /// Decode speed from PORTSC value
    fn decode_speed(speed: u32) -> UsbSpeed {
        match speed {
            1 => UsbSpeed::Full,
            2 => UsbSpeed::Low,
            3 => UsbSpeed::High,
            4 => UsbSpeed::Super,
            5 => UsbSpeed::SuperPlus,
            _ => UsbSpeed::Full,
        }
    }

    /// Get number of ports
    pub fn num_ports(&self) -> u8 {
        self.num_ports
    }

    /// Print controller status
    pub fn print_status(&self) {
        println!("xHCI Controller Status:");
        println!("  MMIO Base: {:016X}", self.mmio_base);
        println!("  Version: {:04X}", self.caps.hci_version);
        println!("  Max Slots: {}", self.max_slots);
        println!("  Max Ports: {}", self.num_ports);
        
        let op = OperationalRegisters::new(self.op_base());
        let cmd = op.usb_cmd();
        let sts = op.usb_sts();
        
        println!("  Running: {}", cmd & CMD_RUN_STOP != 0);
        println!("  Halted: {}", sts & STS_HALTED != 0);
        
        // Print port status
        for port in 0..self.num_ports {
            let portsc = op.portsc(port);
            let connected = portsc & PORTSC_CCS != 0;
            let enabled = portsc & PORTSC_PED != 0;
            let speed = (portsc >> PORTSC_SPEED_SHIFT) & PORTSC_SPEED_MASK;
            
            if connected {
                println!("  Port {}: Connected, Enabled={}, Speed={}",
                    port, enabled, speed);
            }
        }
    }
}

impl Drop for XhciController {
    fn drop(&mut self) {
        // Stop controller on drop
        let op = OperationalRegisters::new(self.op_base());
        let cmd = op.usb_cmd();
        op.set_usb_cmd(cmd & !CMD_RUN_STOP);
    }
}
