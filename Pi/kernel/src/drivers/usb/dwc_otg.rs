//! DWC OTG USB Host Controller Driver
//!
//! Driver for the Synopsys DesignWare Core USB OTG controller
//! used in Raspberry Pi (BCM2835/BCM2836/BCM2837/BCM2711).
//!
//! Base addresses:
//! - Pi 1/2/3 (BCM2835/2836/2837): 0x3F980000
#![allow(dead_code)]

//! - Pi 4 (BCM2711): 0xFE980000

use core::ptr::{read_volatile, write_volatile};
use spin::Mutex;
use lazy_static::lazy_static;

use crate::println;
use super::UsbError;

// =============================================================================
// Hardware Base Addresses
// =============================================================================

/// USB DWC OTG base address for Pi 3
pub const USB_BASE_PI3: u64 = 0x3F98_0000;
/// USB DWC OTG base address for Pi 4
pub const USB_BASE_PI4: u64 = 0xFE98_0000;

/// Current USB base address (detected at runtime)
use core::sync::atomic::{AtomicU64, Ordering};
static USB_BASE: AtomicU64 = AtomicU64::new(USB_BASE_PI3);

/// Get the current USB base address
fn usb_base() -> u64 {
    USB_BASE.load(Ordering::Relaxed)
}

/// Read from DWC OTG register
#[inline]
unsafe fn read_reg(offset: u64) -> u32 {
    read_volatile((usb_base() + offset) as *const u32)
}

/// Write to DWC OTG register
#[inline]
unsafe fn write_reg(offset: u64, value: u32) {
    write_volatile((usb_base() + offset) as *mut u32, value);
}

// =============================================================================
// DWC OTG Register Offsets (Core Global Registers)
// =============================================================================

const REG_GOTGCTL: u64       = 0x000; // OTG Control and Status
const REG_GAHBCFG: u64       = 0x008; // AHB Configuration
const REG_GUSBCFG: u64       = 0x00C; // USB Configuration
const REG_GRSTCTL: u64       = 0x010; // Reset Register
const REG_GINTSTS: u64       = 0x014; // Interrupt Status
const REG_GINTMSK: u64       = 0x018; // Interrupt Mask
const REG_GRXSTSR: u64       = 0x01C; // Receive Status Read
const REG_GRXSTSP: u64       = 0x020; // Receive Status Pop
const REG_GRXFSIZ: u64       = 0x024; // Receive FIFO Size
const REG_GNPTXFSIZ: u64     = 0x028; // Non-periodic TX FIFO Size
const REG_GHWCFG1: u64       = 0x044; // User HW Config 1
const REG_GHWCFG2: u64       = 0x048; // User HW Config 2
const REG_GHWCFG3: u64       = 0x04C; // User HW Config 3
const REG_GHWCFG4: u64       = 0x050; // User HW Config 4

// Host Mode Registers (offset from base)
const REG_HCFG: u64          = 0x400; // Host Configuration
const REG_HFIR: u64          = 0x404; // Host Frame Interval
const REG_HFNUM: u64         = 0x408; // Host Frame Number
const REG_HPTXSTS: u64       = 0x410; // Host Periodic TX FIFO Status
const REG_HAINT: u64         = 0x414; // Host All Channels Interrupt
const REG_HAINTMSK: u64      = 0x418; // Host All Channels Interrupt Mask

// Host Channel Registers (0x500 + channel * 0x20)
const REG_HC_BASE: u64       = 0x500;
const REG_HC_OFFSET: u64     = 0x20;

const REG_HCCHAR: u64        = 0x00; // Channel Characteristics
const REG_HCSPLT: u64        = 0x04; // Channel Split Control
const REG_HCINT: u64         = 0x08; // Channel Interrupt
const REG_HCINTMSK: u64      = 0x0C; // Channel Interrupt Mask
const REG_HCTSIZ: u64        = 0x10; // Channel Transfer Size
const REG_HCDMA: u64         = 0x14; // Channel DMA Address

// =============================================================================
// Register Bit Definitions
// =============================================================================

// GAHBCFG bits
const GAHBCFG_GLBL_INTR_EN: u32 = 1 << 0;
const GAHBCFG_DMA_EN: u32       = 1 << 5;
const GAHBCFG_TX_FIFO_EMTY_LVL: u32 = 1 << 7;

// GUSBCFG bits
const GUSBCFG_FORCE_HOST_MODE: u32 = 1 << 29;
const GUSBCFG_PHY_IF_16BIT: u32    = 1 << 3;
const GUSBCFG_TRDT_MASK: u32       = 0xF << 10;
const GUSBCFG_TRDT_VAL: u32        = 0x5 << 10;

// GRSTCTL bits
const GRSTCTL_AHB_IDLE: u32    = 1 << 31;
const GRSTCTL_TX_FIFO_FLUSH: u32 = 1 << 5;
const GRSTCTL_RX_FIFO_FLUSH: u32 = 1 << 4;
const GRSTCTL_INT_FLUSH: u32   = 1 << 3;
const GRSTCTL_CORE_SOFT_RST: u32 = 1 << 0;

// GINTSTS/GINTMSK bits
const GINTSTS_MODE_MISMATCH: u32 = 1 << 1;
const GINTSTS_OTG_INT: u32       = 1 << 2;
const GINTSTS_SOF: u32           = 1 << 3;
const GINTSTS_RX_FIFO_NEMPTY: u32 = 1 << 4;
const GINTSTS_NPTXFEMPTY: u32    = 1 << 5;
const GINTSTS_GINNAKEFF: u32     = 1 << 6;
const GINTSTS_GOUTNAKEFF: u32    = 1 << 7;
const GINTSTS_HCH_INT: u32       = 1 << 25; // Host channels interrupt
const GINTSTS_PRT_INT: u32       = 1 << 24; // Host port interrupt
const GINTSTS_DISC_INT: u32      = 1 << 29; // Disconnect detected
const GINTSTS_CMOD: u32          = 1 << 0;  // Current mode (0=device, 1=host)

// HCFG bits
const HCFG_FS_LS_PHY_SEL: u32    = 1 << 2;
const HCFG_FS_LS_SUPPORT: u32    = 1 << 3;

// Host Port Register (HPRT) - at offset 0x440
const REG_HPRT: u64 = 0x440;

const HPRT_PWR: u32           = 1 << 12;
const HPRT_RST: u32           = 1 << 8;
const HPRT_SUSP: u32          = 1 << 7;
const HPRT_RES: u32           = 1 << 6;
const HPRT_ENA: u32           = 1 << 2;
const HPRT_CON_STATE: u32     = 1 << 1;
const HPRT_CON_DETECT: u32    = 1 << 1;
const HPRT_CONN_DET: u32      = 1 << 0;

// HCCHAR bits
const HCCHAR_CHENA: u32       = 1 << 31;
const HCCHAR_CHDIS: u32       = 1 << 30;
const HCCHAR_ODDFRM: u32      = 1 << 29;
const HCCHAR_DEVADDR_SHIFT: u32 = 22;
const HCCHAR_EP_TYPE_SHIFT: u32 = 18;
const HCCHAR_LSPDDEV: u32     = 1 << 17;
const HCCHAR_EP_DIR: u32      = 1 << 15;
const HCCHAR_EP_NUM_SHIFT: u32 = 11;
const HCCHAR_MPS_SHIFT: u32   = 0;

// HCINT bits
const HCINT_XFER_COMPL: u32   = 1 << 0;
const HCINT_CH_HALTED: u32    = 1 << 1;
const HCINT_AHB_ERR: u32      = 1 << 2;
const HCINT_STALL: u32        = 1 << 3;
const HCINT_NAK: u32          = 1 << 4;
const HCINT_ACK: u32          = 1 << 5;
const HCINT_NYET: u32         = 1 << 6;
const HCINT_XACT_ERR: u32     = 1 << 7;
const HCINT_BBL_ERR: u32      = 1 << 8;
const HCINT_FRM_OVRUN: u32    = 1 << 9;
const HCINT_DATA_TOG_ERR: u32 = 1 << 10;

// HCTSIZ bits
const HCTSIZ_DOPING: u32      = 1 << 31;
const HCTSIZ_PID_SHIFT: u32   = 29;
const HCTSIZ_PKT_CNT_SHIFT: u32 = 19;
const HCTSIZ_XFER_SIZE_SHIFT: u32 = 0;

// Packet IDs
const PID_DATA0: u32 = 0;
const PID_DATA1: u32 = 2;
const PID_DATA2: u32 = 1;
const PID_MDATA: u32 = 3;
const PID_SETUP: u32 = 3;

// =============================================================================
// USB Constants
// =============================================================================

/// Maximum number of host channels
const MAX_CHANNELS: usize = 8;

/// USB standard request types
pub const REQ_TYPE_STANDARD: u8 = 0x00;
pub const REQ_TYPE_CLASS: u8    = 0x20;
pub const REQ_TYPE_VENDOR: u8   = 0x40;

/// USB standard request codes
pub const REQ_GET_STATUS: u8        = 0x00;
pub const REQ_CLEAR_FEATURE: u8     = 0x01;
pub const REQ_SET_FEATURE: u8       = 0x03;
pub const REQ_SET_ADDRESS: u8       = 0x05;
pub const REQ_GET_DESCRIPTOR: u8    = 0x06;
pub const REQ_SET_DESCRIPTOR: u8    = 0x07;
pub const REQ_GET_CONFIGURATION: u8 = 0x08;
pub const REQ_SET_CONFIGURATION: u8 = 0x09;
pub const REQ_GET_INTERFACE: u8     = 0x0A;
pub const REQ_SET_INTERFACE: u8     = 0x0B;

/// USB descriptor types
pub const DESC_DEVICE: u8        = 0x01;
pub const DESC_CONFIGURATION: u8 = 0x02;
pub const DESC_STRING: u8        = 0x03;
pub const DESC_INTERFACE: u8     = 0x04;
pub const DESC_ENDPOINT: u8      = 0x05;
pub const DESC_HID: u8           = 0x21;
pub const DESC_HID_REPORT: u8    = 0x22;

/// USB device classes
pub const CLASS_HID: u8    = 0x03;
pub const CLASS_MASS: u8   = 0x08;
pub const CLASS_HUB: u8    = 0x09;

/// Endpoint types
pub const EP_TYPE_CONTROL: u8     = 0;
pub const EP_TYPE_ISOCHRONOUS: u8 = 1;
pub const EP_TYPE_BULK: u8        = 2;
pub const EP_TYPE_INTERRUPT: u8   = 3;

// =============================================================================
// Data Structures
// =============================================================================

/// USB Device Descriptor
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct DeviceDescriptor {
    pub b_length: u8,
    pub b_descriptor_type: u8,
    pub bcd_usb: u16,
    pub b_device_class: u8,
    pub b_device_subclass: u8,
    pub b_device_protocol: u8,
    pub b_max_packet_size0: u8,
    pub id_vendor: u16,
    pub id_product: u16,
    pub bcd_device: u16,
    pub i_manufacturer: u8,
    pub i_product: u8,
    pub i_serial_number: u8,
    pub b_num_configurations: u8,
}

/// USB Configuration Descriptor
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct ConfigDescriptor {
    pub b_length: u8,
    pub b_descriptor_type: u8,
    pub w_total_length: u16,
    pub b_num_interfaces: u8,
    pub b_configuration_value: u8,
    pub i_configuration: u8,
    pub bm_attributes: u8,
    pub b_max_power: u8,
}

/// USB Interface Descriptor
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct InterfaceDescriptor {
    pub b_length: u8,
    pub b_descriptor_type: u8,
    pub b_interface_number: u8,
    pub b_alternate_setting: u8,
    pub b_num_endpoints: u8,
    pub b_interface_class: u8,
    pub b_interface_subclass: u8,
    pub b_interface_protocol: u8,
    pub i_interface: u8,
}

/// USB Endpoint Descriptor
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct EndpointDescriptor {
    pub b_length: u8,
    pub b_descriptor_type: u8,
    pub b_endpoint_address: u8,
    pub bm_attributes: u8,
    pub w_max_packet_size: u16,
    pub b_interval: u8,
}

/// USB HID Descriptor
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct HidDescriptor {
    pub b_length: u8,
    pub b_descriptor_type: u8,
    pub bcd_hid: u16,
    pub b_country_code: u8,
    pub b_num_descriptors: u8,
    pub b_class_descriptor_type: u8,
    pub w_descriptor_length: u16,
}

/// USB Setup Packet
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct SetupPacket {
    pub bm_request_type: u8,
    pub b_request: u8,
    pub w_value: u16,
    pub w_index: u16,
    pub w_length: u16,
}

impl SetupPacket {
    pub fn new(request_type: u8, request: u8, value: u16, index: u16, length: u16) -> Self {
        Self {
            bm_request_type: request_type,
            b_request: request,
            w_value: value,
            w_index: index,
            w_length: length,
        }
    }
}

/// Channel state
#[derive(Debug, Clone, Copy, PartialEq)]
enum ChannelState {
    Idle,
    Busy,
    Complete,
    Error,
}

/// Channel information
#[derive(Clone, Copy)]
struct Channel {
    state: ChannelState,
    device_addr: u8,
    endpoint_num: u8,
    endpoint_type: u8,
    max_packet_size: u16,
    pid: u32, // DATA0/DATA1 toggle
}

impl Channel {
    const fn new() -> Self {
        Self {
            state: ChannelState::Idle,
            device_addr: 0,
            endpoint_num: 0,
            endpoint_type: 0,
            max_packet_size: 8,
            pid: PID_DATA0,
        }
    }
}

/// DWC OTG Controller State
struct DwcOtgState {
    initialized: bool,
    channels: [Channel; MAX_CHANNELS],
    next_device_addr: u8,
}

impl DwcOtgState {
    const fn new() -> Self {
        Self {
            initialized: false,
            channels: [Channel::new(); MAX_CHANNELS],
            next_device_addr: 1,
        }
    }
}

lazy_static! {
    static ref DWC_OTG: Mutex<DwcOtgState> = Mutex::new(DwcOtgState::new());
}

// =============================================================================
// Initialization
// =============================================================================

/// Initialize the DWC OTG USB controller
pub fn init() -> Result<(), UsbError> {
    println!("[usb/dwc_otg] Initializing DWC OTG USB controller...");
    
    let mut state = DWC_OTG.lock();
    
    unsafe {
        // Detect Pi version by checking hardware
        let hwcfg2 = read_reg(REG_GHWCFG2);
        let op_mode = (hwcfg2 >> 0) & 0x7;
        println!("[usb/dwc_otg] HWCFG2 = 0x{:08X}, op_mode = {}", hwcfg2, op_mode);
        
        // Force host mode
        let mut usbcfg = read_reg(REG_GUSBCFG);
        usbcfg |= GUSBCFG_FORCE_HOST_MODE;
        write_reg(REG_GUSBCFG, usbcfg);
        
        // Wait for mode switch
        for _ in 0..100000 {
            core::arch::asm!("nop", options(nomem, nostack));
        }
        
        // Perform core soft reset
        println!("[usb/dwc_otg] Resetting core...");
        reset_core()?;
        
        // Configure PHY
        let mut usbcfg = read_reg(REG_GUSBCFG);
        usbcfg &= !GUSBCFG_TRDT_MASK;
        usbcfg |= GUSBCFG_TRDT_VAL;
        write_reg(REG_GUSBCFG, usbcfg);
        
        // Initialize host mode
        init_host_mode()?;
        
        // Enable global interrupts
        write_reg(REG_GINTMSK, GINTSTS_HCH_INT | GINTSTS_PRT_INT | GINTSTS_DISC_INT);
        write_reg(REG_GAHBCFG, GAHBCFG_GLBL_INTR_EN);
        
        // Enable power to port
        let mut hprt = read_reg(REG_HPRT);
        hprt &= !(HPRT_PWR | HPRT_RST | HPRT_ENA); // Clear these bits
        hprt |= HPRT_PWR; // Enable power
        write_reg(REG_HPRT, hprt);
        
        // Wait for power stabilization
        for _ in 0..100000 {
            core::arch::asm!("nop", options(nomem, nostack));
        }
        
        println!("[usb/dwc_otg] Core initialized, waiting for device connection...");
    }
    
    state.initialized = true;
    Ok(())
}

/// Reset the DWC OTG core
unsafe fn reset_core() -> Result<(), UsbError> {
    // Wait for AHB idle
    let mut timeout = 100000;
    while (read_reg(REG_GRSTCTL) & GRSTCTL_AHB_IDLE) == 0 {
        timeout -= 1;
        if timeout == 0 {
            return Err(UsbError::Timeout);
        }
    }
    
    // Soft reset
    write_reg(REG_GRSTCTL, GRSTCTL_CORE_SOFT_RST);
    
    // Wait for reset to complete
    timeout = 100000;
    while (read_reg(REG_GRSTCTL) & GRSTCTL_CORE_SOFT_RST) != 0 {
        timeout -= 1;
        if timeout == 0 {
            return Err(UsbError::Timeout);
        }
    }
    
    // Wait a bit after reset
    for _ in 0..10000 {
        core::arch::asm!("nop", options(nomem, nostack));
    }
    
    Ok(())
}

/// Initialize host mode
unsafe fn init_host_mode() -> Result<(), UsbError> {
    println!("[usb/dwc_otg] Initializing host mode...");
    
    // Check if we're in host mode
    let gintsts = read_reg(REG_GINTSTS);
    if (gintsts & GINTSTS_CMOD) == 0 {
        println!("[usb/dwc_otg] Warning: Not in host mode!");
    }
    
    // Configure host mode (full-speed)
    let hcfg = HCFG_FS_LS_PHY_SEL | HCFG_FS_LS_SUPPORT;
    write_reg(REG_HCFG, hcfg);
    
    // Set frame interval
    write_reg(REG_HFIR, 48000); // 48MHz / 1000 = 1ms frame interval
    
    // Configure FIFO sizes
    // RX FIFO: 256 words (1KB)
    write_reg(REG_GRXFSIZ, 256);
    
    // Non-periodic TX FIFO: 256 words (1KB)
    write_reg(REG_GNPTXFSIZ, (256 << 16) | 256);
    
    // Host periodic TX FIFO status
    write_reg(REG_HPTXSTS, (256 << 16) | 0x0100);
    
    // Enable host channel interrupts
    write_reg(REG_HAINTMSK, 0xFFFF); // Enable all channels
    
    // Initialize all channels as disabled
    for ch in 0..MAX_CHANNELS {
        let ch_base = REG_HC_BASE + (ch as u64) * REG_HC_OFFSET;
        write_reg(ch_base + REG_HCCHAR, HCCHAR_CHDIS);
        write_reg(ch_base + REG_HCINTMSK, 0);
    }
    
    println!("[usb/dwc_otg] Host mode initialized");
    Ok(())
}

// =============================================================================
// Port Control
// =============================================================================

/// Check if a device is connected
pub fn is_device_connected() -> bool {
    unsafe {
        let hprt = read_reg(REG_HPRT);
        (hprt & HPRT_CONN_DET) != 0
    }
}

/// Reset the USB port
pub fn reset_port() -> Result<(), UsbError> {
    println!("[usb/dwc_otg] Resetting port...");
    
    unsafe {
        // Start reset
        let mut hprt = read_reg(REG_HPRT);
        hprt |= HPRT_RST;
        write_reg(REG_HPRT, hprt);
        
        // Wait 50ms (USB spec requires at least 10ms)
        crate::drivers::timer::sleep_ms(50);
        
        // Clear reset
        hprt = read_reg(REG_HPRT);
        hprt &= !HPRT_RST;
        write_reg(REG_HPRT, hprt);
        
        // Wait for reset to complete
        for _ in 0..100000 {
            core::arch::asm!("nop", options(nomem, nostack));
        }
        
        // Check if port is enabled
        hprt = read_reg(REG_HPRT);
        if (hprt & HPRT_ENA) == 0 {
            return Err(UsbError::DeviceNotResponding);
        }
        
        println!("[usb/dwc_otg] Port reset complete, port enabled");
    }
    
    Ok(())
}

// =============================================================================
// Channel Management
// =============================================================================

/// Allocate a host channel
fn alloc_channel() -> Result<usize, UsbError> {
    let mut state = DWC_OTG.lock();
    
    for i in 0..MAX_CHANNELS {
        if state.channels[i].state == ChannelState::Idle {
            state.channels[i].state = ChannelState::Busy;
            return Ok(i);
        }
    }
    
    Err(UsbError::NoChannels)
}

/// Free a host channel
fn free_channel(ch_num: usize) {
    if ch_num >= MAX_CHANNELS {
        return;
    }
    
    let mut state = DWC_OTG.lock();
    state.channels[ch_num] = Channel::new();
    
    unsafe {
        let ch_base = REG_HC_BASE + (ch_num as u64) * REG_HC_OFFSET;
        write_reg(ch_base + REG_HCCHAR, HCCHAR_CHDIS);
        write_reg(ch_base + REG_HCINTMSK, 0);
    }
}

/// Configure a channel
fn configure_channel(
    ch_num: usize,
    device_addr: u8,
    endpoint_num: u8,
    endpoint_type: u8,
    max_packet_size: u16,
    is_in: bool,
) -> Result<(), UsbError> {
    if ch_num >= MAX_CHANNELS {
        return Err(UsbError::InvalidChannel);
    }
    
    let mut state = DWC_OTG.lock();
    
    state.channels[ch_num].device_addr = device_addr;
    state.channels[ch_num].endpoint_num = endpoint_num;
    state.channels[ch_num].endpoint_type = endpoint_type;
    state.channels[ch_num].max_packet_size = max_packet_size;
    
    unsafe {
        let ch_base = REG_HC_BASE + (ch_num as u64) * REG_HC_OFFSET;
        
        // Disable channel first
        write_reg(ch_base + REG_HCCHAR, HCCHAR_CHDIS);
        
        // Wait for channel to disable
        let mut timeout = 10000;
        while (read_reg(ch_base + REG_HCCHAR) & HCCHAR_CHENA) != 0 && timeout > 0 {
            timeout -= 1;
        }
        
        // Configure channel characteristics
        let mut hcchar = ((device_addr as u32) << HCCHAR_DEVADDR_SHIFT)
            | ((endpoint_num as u32) << HCCHAR_EP_NUM_SHIFT)
            | ((endpoint_type as u32) << HCCHAR_EP_TYPE_SHIFT)
            | ((max_packet_size as u32) & 0x7FF);
        
        if is_in {
            hcchar |= HCCHAR_EP_DIR;
        }
        
        if endpoint_type == EP_TYPE_INTERRUPT {
            hcchar |= HCCHAR_ODDFRM;
        }
        
        write_reg(ch_base + REG_HCCHAR, hcchar);
        
        // Enable channel interrupts
        write_reg(ch_base + REG_HCINTMSK, 
            HCINT_XFER_COMPL | HCINT_CH_HALTED | HCINT_STALL | 
            HCINT_NAK | HCINT_ACK | HCINT_XACT_ERR);
    }
    
    Ok(())
}

/// Start a transfer on a channel
unsafe fn start_transfer(
    ch_num: usize,
    pid: u32,
    data: Option<&[u8]>,
    length: usize,
) -> Result<(), UsbError> {
    let ch_base = REG_HC_BASE + (ch_num as u64) * REG_HC_OFFSET;
    let state = DWC_OTG.lock();
    let max_packet = state.channels[ch_num].max_packet_size as usize;
    drop(state);
    
    // Calculate packet count
    let pkt_cnt = if length == 0 {
        1
    } else {
        ((length + max_packet - 1) / max_packet).max(1)
    };
    
    // Set up transfer size
    let hctsiz = (pid << HCTSIZ_PID_SHIFT)
        | ((pkt_cnt as u32) << HCTSIZ_PKT_CNT_SHIFT)
        | ((length as u32) & 0x7FFFF);
    write_reg(ch_base + REG_HCTSIZ, hctsiz);
    
    // Set up DMA address (for now, we don't use DMA)
    write_reg(ch_base + REG_HCDMA, 0);
    
    // If this is an OUT transfer, write data to FIFO
    if let Some(data) = data {
        // For non-DMA mode, we need to write data to the FIFO
        // The FIFO address is at base + 0x1000
        let fifo_addr = (usb_base() + 0x1000) as *mut u32;
        
        // Write data in 32-bit words
        let words = (length + 3) / 4;
        for i in 0..words {
            let mut word: u32 = 0;
            for j in 0..4 {
                let idx = i * 4 + j;
                if idx < length {
                    word |= (data[idx] as u32) << (j * 8);
                }
            }
            write_volatile(fifo_addr, word);
        }
    }
    
    // Enable the channel
    let hcchar = read_reg(ch_base + REG_HCCHAR) | HCCHAR_CHENA;
    write_reg(ch_base + REG_HCCHAR, hcchar);
    
    Ok(())
}

/// Wait for a channel to complete
fn wait_channel(ch_num: usize, timeout_ms: u64) -> Result<usize, UsbError> {
    let start = crate::drivers::timer::get_ticks();
    
    loop {
        unsafe {
            let ch_base = REG_HC_BASE + (ch_num as u64) * REG_HC_OFFSET;
            let hcint = read_reg(ch_base + REG_HCINT);
            
            // Check for completion
            if (hcint & HCINT_XFER_COMPL) != 0 {
                // Clear interrupt
                write_reg(ch_base + REG_HCINT, hcint);
                
                // Read transfer size to determine bytes transferred
                let hctsiz = read_reg(ch_base + REG_HCTSIZ);
                let remaining = (hctsiz & 0x7FFFF) as usize;
                
                let state = DWC_OTG.lock();
                let total = state.channels[ch_num].max_packet_size as usize;
                drop(state);
                
                return Ok(total - remaining);
            }
            
            // Check for errors
            if (hcint & (HCINT_STALL | HCINT_XACT_ERR | HCINT_BBL_ERR)) != 0 {
                write_reg(ch_base + REG_HCINT, hcint);
                
                if (hcint & HCINT_STALL) != 0 {
                    return Err(UsbError::Stall);
                }
                return Err(UsbError::TransferError);
            }
            
            // Check for timeout
            if crate::drivers::timer::get_ticks() - start > timeout_ms {
                return Err(UsbError::Timeout);
            }
            
            // Small delay
            for _ in 0..100 {
                core::arch::asm!("nop", options(nomem, nostack));
            }
        }
    }
}

// =============================================================================
// USB Transfers
// =============================================================================

/// Perform a control transfer (SETUP, DATA, STATUS phases)
pub fn control_transfer(
    device_addr: u8,
    endpoint: u8,
    setup: &SetupPacket,
    data: Option<&mut [u8]>,
    data_len: usize,
) -> Result<usize, UsbError> {
    let ch = alloc_channel()?;
    
    // Configure for control endpoint (EP0)
    configure_channel(ch, device_addr, endpoint, EP_TYPE_CONTROL, 64, false)?;
    
    unsafe {
        // SETUP phase
        let setup_data: [u8; 8] = [
            setup.bm_request_type,
            setup.b_request,
            (setup.w_value & 0xFF) as u8,
            (setup.w_value >> 8) as u8,
            (setup.w_index & 0xFF) as u8,
            (setup.w_index >> 8) as u8,
            (setup.w_length & 0xFF) as u8,
            (setup.w_length >> 8) as u8,
        ];
        
        start_transfer(ch, PID_SETUP, Some(&setup_data), 8)?;
        let _ = wait_channel(ch, 1000)?;
        
        // DATA phase (if any)
        let mut bytes_transferred = 0;
        if data_len > 0 && data.is_some() {
            let is_in = (setup.bm_request_type & 0x80) != 0;
            configure_channel(ch, device_addr, endpoint, EP_TYPE_CONTROL, 64, is_in)?;
            
            if is_in {
                // IN transfer
                start_transfer(ch, PID_DATA1, None, data_len)?;
                
                // Read data from FIFO
                let fifo_addr = (usb_base() + 0x1000) as *const u32;
                let words = (data_len + 3) / 4;
                
                if let Some(buffer) = data {
                    for i in 0..words {
                        let word = read_volatile(fifo_addr);
                        for j in 0..4 {
                            let idx = i * 4 + j;
                            if idx < buffer.len() && idx < data_len {
                                buffer[idx] = (word >> (j * 8)) as u8;
                            }
                        }
                    }
                }
            } else {
                // OUT transfer
                if let Some(buffer) = data {
                    start_transfer(ch, PID_DATA1, Some(&buffer[..data_len]), data_len)?;
                }
            }
            
            bytes_transferred = wait_channel(ch, 1000)?;
        }
        
        // STATUS phase
        let is_in_status = (setup.bm_request_type & 0x80) == 0;
        configure_channel(ch, device_addr, endpoint, EP_TYPE_CONTROL, 64, is_in_status)?;
        
        if is_in_status {
            start_transfer(ch, PID_DATA1, None, 0)?;
        } else {
            start_transfer(ch, PID_DATA1, Some(&[]), 0)?;
        }
        
        let _ = wait_channel(ch, 1000)?;
        
        free_channel(ch);
        Ok(bytes_transferred)
    }
}

/// Perform an interrupt IN transfer (for HID devices)
pub fn interrupt_in_transfer(
    device_addr: u8,
    endpoint: u8,
    max_packet_size: u16,
    buffer: &mut [u8],
) -> Result<usize, UsbError> {
    let ch = alloc_channel()?;
    
    configure_channel(ch, device_addr, endpoint, EP_TYPE_INTERRUPT, max_packet_size, true)?;
    
    unsafe {
        start_transfer(ch, PID_DATA0, None, buffer.len())?;
        
        // Read data from FIFO
        let fifo_addr = (usb_base() + 0x1000) as *const u32;
        let words = (buffer.len() + 3) / 4;
        
        for i in 0..words {
            let word = read_volatile(fifo_addr);
            for j in 0..4 {
                let idx = i * 4 + j;
                if idx < buffer.len() {
                    buffer[idx] = (word >> (j * 8)) as u8;
                }
            }
        }
        
        let result = wait_channel(ch, 100);
        free_channel(ch);
        
        result
    }
}

/// Set device address
pub fn set_address(device_addr: u8, new_addr: u8) -> Result<(), UsbError> {
    let setup = SetupPacket::new(
        0x00, // OUT, standard, device
        REQ_SET_ADDRESS,
        new_addr as u16,
        0,
        0,
    );
    
    control_transfer(device_addr, 0, &setup, None, 0)?;
    
    // Wait for address to take effect
    unsafe {
        for _ in 0..10000 {
            core::arch::asm!("nop", options(nomem, nostack));
        }
    }
    
    Ok(())
}

/// Get device descriptor
pub fn get_device_descriptor(device_addr: u8) -> Result<DeviceDescriptor, UsbError> {
    let mut desc: DeviceDescriptor = unsafe { core::mem::zeroed() };
    let setup = SetupPacket::new(
        0x80, // IN, standard, device
        REQ_GET_DESCRIPTOR,
        (DESC_DEVICE as u16) << 8,
        0,
        18,
    );
    
    let bytes = control_transfer(
        device_addr,
        0,
        &setup,
        Some(unsafe { 
            core::slice::from_raw_parts_mut(
                &mut desc as *mut _ as *mut u8,
                18
            )
        }),
        18,
    )?;
    
    if bytes < 8 {
        return Err(UsbError::InvalidDescriptor);
    }
    
    Ok(desc)
}

/// Get configuration descriptor
pub fn get_config_descriptor(device_addr: u8, buffer: &mut [u8]) -> Result<usize, UsbError> {
    let len = buffer.len();
    let setup = SetupPacket::new(
        0x80, // IN, standard, device
        REQ_GET_DESCRIPTOR,
        (DESC_CONFIGURATION as u16) << 8,
        0,
        len as u16,
    );
    
    control_transfer(device_addr, 0, &setup, Some(buffer), len)
}

/// Set configuration
pub fn set_configuration(device_addr: u8, config_value: u8) -> Result<(), UsbError> {
    let setup = SetupPacket::new(
        0x00, // OUT, standard, device
        REQ_SET_CONFIGURATION,
        config_value as u16,
        0,
        0,
    );
    
    control_transfer(device_addr, 0, &setup, None, 0)?;
    Ok(())
}

/// Get next available device address
pub fn alloc_device_address() -> u8 {
    let mut state = DWC_OTG.lock();
    let addr = state.next_device_addr;
    if state.next_device_addr < 127 {
        state.next_device_addr += 1;
    }
    addr
}

/// Poll for USB interrupts and handle them
pub fn poll() {
    unsafe {
        let gintsts = read_reg(REG_GINTSTS);
        let gintmsk = read_reg(REG_GINTMSK);
        let pending = gintsts & gintmsk;
        
        if pending == 0 {
            return;
        }
        
        // Handle port interrupt
        if (pending & GINTSTS_PRT_INT) != 0 {
            let hprt = read_reg(REG_HPRT);
            println!("[usb/dwc_otg] Port interrupt: HPRT=0x{:08X}", hprt);
            
            // Clear interrupt bits (write 1 to clear)
            write_reg(REG_HPRT, hprt);
        }
        
        // Handle channel interrupts
        if (pending & GINTSTS_HCH_INT) != 0 {
            let haint = read_reg(REG_HAINT);
            
            for ch in 0..MAX_CHANNELS {
                if (haint & (1 << ch)) != 0 {
                    let ch_base = REG_HC_BASE + (ch as u64) * REG_HC_OFFSET;
                    let hcint = read_reg(ch_base + REG_HCINT);
                    
                    // Clear interrupts
                    write_reg(ch_base + REG_HCINT, hcint);
                }
            }
        }
        
        // Clear global interrupts
        write_reg(REG_GINTSTS, pending);
    }
}
