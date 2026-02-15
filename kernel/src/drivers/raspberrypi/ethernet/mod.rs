//! Ethernet Driver for Raspberry Pi 5
//!
//! Research and skeleton implementation for network interface support.
//!
//! # Raspberry Pi 5 Ethernet Architecture
//!
//! The Raspberry Pi 5 uses a different Ethernet controller than previous models:
//! - **Raspberry Pi 5**: Via PCIe attached Ethernet controller
//! - **Raspberry Pi 4**: Broadcom BCM54213PE Gigabit Ethernet (internal)
//!
//! # Ethernet Controllers
//!
//! ## Possible Controllers on Pi 5:
//! 1. **Realtek RTL8111/8168** - Common PCIe Gigabit Ethernet
//!    - PCI Vendor ID: 0x10EC
//!    - PCI Device ID: 0x8168 (various revisions)
//!    - Speed: 10/100/1000 Mbps
//!    
//! 2. **Intel I210/I211** - Industrial Ethernet
//!    - PCI Vendor ID: 0x8086
//!    - Higher performance, advanced features
//!
//! ## Features Required:
//! - MAC address handling
//! - Packet transmission (Tx)
//! - Packet reception (Rx)
//! - Interrupt handling
//! - Link state detection
//! - Auto-negotiation
//!
//! # Implementation Approach
//!
//! 1. PCIe enumeration to find Ethernet controller
//! 2. Initialize MAC and PHY
//! 3. Set up DMA rings for Tx/Rx
//! 4. Enable interrupts
//! 5. Implement packet send/receive

use crate::hal::{mmio, platform_info, delay};
use crate::println;

/// Ethernet controller types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EtherControllerType {
    /// Realtek RTL8168/8111
    Rtl8168,
    /// Intel I210
    IntelI210,
    /// Intel I211
    IntelI211,
    /// Broadcom BCM54213PE (Pi 4)
    Bcm54213pe,
    /// Unknown controller
    Unknown,
}

/// Ethernet link speed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkSpeed {
    /// 10 Mbps
    Speed10M,
    /// 100 Mbps
    Speed100M,
    /// 1 Gbps
    Speed1G,
    /// Not connected
    Disconnected,
}

impl LinkSpeed {
    /// Get speed in Mbps
    pub fn mbps(&self) -> u32 {
        match self {
            LinkSpeed::Speed10M => 10,
            LinkSpeed::Speed100M => 100,
            LinkSpeed::Speed1G => 1000,
            LinkSpeed::Disconnected => 0,
        }
    }
}

/// Ethernet duplex mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DuplexMode {
    /// Half duplex
    Half,
    /// Full duplex
    Full,
    /// Unknown
    Unknown,
}

/// MAC address (48-bit)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MacAddress([u8; 6]);

impl MacAddress {
    /// Create a MAC address from bytes
    pub const fn new(bytes: [u8; 6]) -> Self {
        Self(bytes)
    }
    
    /// Get the MAC address bytes
    pub fn bytes(&self) -> &[u8; 6] {
        &self.0
    }
    
    /// Format as string (XX:XX:XX:XX:XX:XX)
    pub fn to_string(&self) -> alloc::string::String {
        alloc::format!("{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            self.0[0], self.0[1], self.0[2],
            self.0[3], self.0[4], self.0[5])
    }
    
    /// Check if valid (not all zeros or all ones)
    pub fn is_valid(&self) -> bool {
        let not_all_zeros = self.0.iter().any(|&b| b != 0);
        let not_all_ones = self.0.iter().any(|&b| b != 0xFF);
        not_all_zeros && not_all_ones
    }
    
    /// Broadcast address
    pub const BROADCAST: Self = Self([0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
}

impl Default for MacAddress {
    fn default() -> Self {
        Self([0, 0, 0, 0, 0, 0])
    }
}

/// Ethernet controller information
#[derive(Debug)]
pub struct EtherControllerInfo {
    /// Controller type
    controller_type: EtherControllerType,
    /// Base address (physical MMIO)
    base_addr: usize,
    /// IRQ number
    irq: u32,
    /// PCI vendor ID
    pci_vendor_id: u16,
    /// PCI device ID
    pci_device_id: u16,
    /// MAC address
    mac_address: MacAddress,
    /// Maximum MTU
    max_mtu: u16,
}

/// RTL8168 register offsets
pub mod rtl8168_regs {
    /// MAC address (0x00-0x05)
    pub const MAC_ADDR: usize = 0x00;
    /// Multicast hash table
    pub const MAR: usize = 0x08;
    /// Transmit descriptors start address
    pub const TNPDS: usize = 0x20;
    /// Transmit priority descriptors
    pub const THPDS: usize = 0x28;
    /// Flash memory read/write
    pub const FLASH: usize = 0x30;
    /// Early transmit threshold
    pub const ERSR: usize = 0x36;
    /// Command register
    pub const CR: usize = 0x37;
    /// Transmit configuration
    pub const TCR: usize = 0x40;
    /// Receive configuration
    pub const RCR: usize = 0x44;
    /// Timer count
    pub const TCTR: usize = 0x48;
    /// Missed packet counter
    pub const MPC: usize = 0x4C;
    /// 9346 command register
    pub const 9346CR: usize = 0x50;
    /// Configuration register 0
    pub const CONFIG0: usize = 0x51;
    /// Configuration register 1
    pub const CONFIG1: usize = 0x52;
    /// Configuration register 2
    pub const CONFIG2: usize = 0x53;
    /// Configuration register 3
    pub const CONFIG3: usize = 0x54;
    /// Configuration register 4
    pub const CONFIG4: usize = 0x55;
    /// Configuration register 5
    pub const CONFIG5: usize = 0x56;
    /// Time interrupt
    pub const TIMERINT: usize = 0x58;
    /// Multiple interrupt select
    pub const MSI: usize = 0x5A;
    /// Interrupt status
    pub const ISR: usize = 0x3E;
    /// Interrupt mask
    pub const IMR: usize = 0x3C;
    /// Receive descriptors address
    pub const RDSAR: usize = 0xE4;
    /// Maximum transmit packet size
    pub const MTPS: usize = 0xEC;
    /// PHY access
    pub const PHYAR: usize = 0x60;
    /// Twister pair status
    pub const TPS: usize = 0x64;
    /// PHY status
    pub const PHYSTATUS: usize = 0x6C;
    /// Wake-on-LAN status
    pub const WOL: usize = 0xA0;
    /// Receive max size
    pub const RMS: usize = 0xDA;
    /// C+ Command
    pub const CPLUSCR: usize = 0xE0;
}

/// RTL8168 Command Register bits
pub mod rtl8168_cr {
    /// Receiver enable
    pub const RE: u8 = 0x08;
    /// Transmitter enable
    pub const TE: u8 = 0x04;
    /// Software reset
    pub const RST: u8 = 0x10;
}

/// RTL8168 Interrupt Status/Mask bits
pub mod rtl8168_isr {
    /// Receive OK
    pub const ROK: u16 = 0x0001;
    /// Receive error
    pub const RER: u16 = 0x0002;
    /// Transmit OK
    pub const TOK: u16 = 0x0004;
    /// Transmit error
    pub const TER: u16 = 0x0008;
    /// Link status change
    pub const LINKCHG: u16 = 0x0020;
    /// Receive descriptor unavailable
    pub const RDU: u16 = 0x0040;
    /// Transmit descriptor unavailable
    pub const TDU: u16 = 0x0080;
    /// Software interrupt
    pub const SWINT: u16 = 0x0100;
    /// Time out
    pub const TOK_TDU: u16 = 0x0200;
    /// System error
    pub const SERR: u16 = 0x8000;
}

/// RTL8168 Receive Configuration bits
pub mod rtl8168_rcr {
    /// Accept all packets
    pub const AAP: u32 = 1 << 0;
    /// Accept physical match packets
    pub const APM: u32 = 1 << 1;
    /// Accept multicast packets
    pub const AM: u32 = 1 << 2;
    /// Accept broadcast packets
    pub const AB: u32 = 1 << 3;
    /// Append FCS
    pub const AR: u32 = 1 << 4;
    /// Accept runt packets
    pub const AER: u32 = 1 << 5;
    /// Accept error packets
    pub const ACPT_ERR: u32 = 1 << 5;
    /// Max DMA burst size (mask)
    pub const MXDMA_MASK: u32 = 0x7 << 8;
    /// Unlimited DMA burst
    pub const MXDMA_UNLIMITED: u32 = 0x7 << 8;
    /// RX buffer length (mask)
    pub const RBLEN_MASK: u32 = 0x3 << 11;
    /// 8K + 16 bytes
    pub const RBLEN_8K: u32 = 0x0 << 11;
    /// 16K + 16 bytes
    pub const RBLEN_16K: u32 = 0x1 << 11;
    /// 32K + 16 bytes
    pub const RBLEN_32K: u32 = 0x2 << 11;
    /// 64K + 16 bytes
    pub const RBLEN_64K: u32 = 0x3 << 11;
    /// No wrapping
    pub const WRAP: u32 = 1 << 7;
}

/// RTL8168 Transmit Configuration bits
pub mod rtl8168_tcr {
    /// Max DMA burst size (mask)
    pub const MXDMA_MASK: u32 = 0x7 << 8;
    /// Unlimited DMA burst
    pub const MXDMA_UNLIMITED: u32 = 0x7 << 8;
    /// TX normal priority clear
    pub const TXNPCLR: u32 = 1 << 25;
    /// TX high priority clear
    pub const TXHPCLR: u32 = 1 << 26;
}

/// Ethernet packet buffer
pub const ETHERNET_MTU: usize = 1500;
pub const ETHERNET_FRAME_SIZE: usize = 1518; // Max frame size without VLAN

/// Ethernet frame header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct EtherHeader {
    /// Destination MAC address
    pub dst: [u8; 6],
    /// Source MAC address
    pub src: [u8; 6],
    /// EtherType (big-endian)
    pub ethertype: u16,
}

impl EtherHeader {
    /// EtherType for IPv4
    pub const ETHERTYPE_IPV4: u16 = 0x0800;
    /// EtherType for IPv6
    pub const ETHERTYPE_IPV6: u16 = 0x86DD;
    /// EtherType for ARP
    pub const ETHERTYPE_ARP: u16 = 0x0806;
    /// EtherType for VLAN
    pub const ETHERTYPE_VLAN: u16 = 0x8100;
}

/// Ethernet driver state
pub struct EthernetDriver {
    /// Controller info
    controller: Option<EtherControllerInfo>,
    /// Link state
    link_up: bool,
    /// Link speed
    link_speed: LinkSpeed,
    /// Duplex mode
    duplex: DuplexMode,
    /// Initialized flag
    initialized: bool,
    /// Tx packet count
    tx_count: u64,
    /// Rx packet count
    rx_count: u64,
    /// Tx error count
    tx_errors: u64,
    /// Rx error count
    rx_errors: u64,
}

/// Global Ethernet driver instance
static mut ETHERNET_DRIVER: EthernetDriver = EthernetDriver {
    controller: None,
    link_up: false,
    link_speed: LinkSpeed::Disconnected,
    duplex: DuplexMode::Unknown,
    initialized: false,
    tx_count: 0,
    rx_count: 0,
    tx_errors: 0,
    rx_errors: 0,
};

/// Initialize Ethernet subsystem
pub fn init() {
    println!("[Ethernet] Initializing Ethernet subsystem...");
    
    // Detect Ethernet controller
    detect_controller();
    
    let drv = driver();
    
    if let Some(ref info) = drv.controller {
        println!("[Ethernet] Found {:?} controller", info.controller_type);
        println!("[Ethernet] MAC: {}", info.mac_address.to_string());
        
        match info.controller_type {
            EtherControllerType::Rtl8168 => {
                if let Err(e) = init_rtl8168(info) {
                    println!("[Ethernet] RTL8168 initialization failed: {:?}", e);
                }
            }
            _ => {
                println!("[Ethernet] Controller type not yet implemented");
            }
        }
    } else {
        println!("[Ethernet] No Ethernet controller detected");
    }
    
    drv.initialized = true;
    println!("[Ethernet] Initialization complete");
}

/// Detect Ethernet controller on the system
fn detect_controller() {
    let drv = driver();
    let info = platform_info();
    
    match info.platform_type {
        crate::hal::PlatformType::RaspberryPi5 => {
            // Pi 5 uses PCIe Ethernet controller
            detect_pcie_ethernet();
        }
        crate::hal::PlatformType::RaspberryPi4 => {
            // Pi 4 has integrated Broadcom Ethernet
            // It's attached via the internal bus, not PCIe
            let ether_info = EtherControllerInfo {
                controller_type: EtherControllerType::Bcm54213pe,
                base_addr: info.ethernet_base,
                irq: 56, // GIC SPI for Ethernet on Pi 4
                pci_vendor_id: 0x14E4, // Broadcom
                pci_device_id: 0x54213,
                mac_address: MacAddress::default(), // Would be read from OTP
                max_mtu: 1500,
            };
            drv.controller = Some(ether_info);
        }
        crate::hal::PlatformType::QemuVirt => {
            // QEMU virt typically uses virtio-net
            println!("[Ethernet] QEMU virt: Would use virtio-net");
        }
        _ => {
            println!("[Ethernet] Unknown platform, no Ethernet controller detected");
        }
    }
}

/// Detect Ethernet controller via PCIe
fn detect_pcie_ethernet() {
    println!("[Ethernet] Scanning PCIe for Ethernet controllers...");
    
    // In a real implementation, this would scan the PCIe bus
    // Looking for Class Code 0x020000 (Network Controller / Ethernet)
    
    // For now, assume RTL8168 at a known address
    let info = platform_info();
    
    let ether_info = EtherControllerInfo {
        controller_type: EtherControllerType::Rtl8168,
        base_addr: info.ethernet_base,
        irq: 88, // Typical GIC SPI for Ethernet on Pi 5
        pci_vendor_id: 0x10EC, // Realtek
        pci_device_id: 0x8168,
        mac_address: MacAddress::default(),
        max_mtu: 1500,
    };
    
    driver().controller = Some(ether_info);
}

/// Initialize RTL8168 controller
fn init_rtl8168(info: &EtherControllerInfo) -> Result<(), EtherError> {
    println!("[Ethernet] Initializing RTL8168 at 0x{:016X}", info.base_addr);
    
    unsafe {
        // Software reset
        mmio::write8(info.base_addr + rtl8168_regs::CR, rtl8168_cr::RST);
        
        // Wait for reset to complete
        let mut timeout = 1000;
        while timeout > 0 {
            let cr = mmio::read8(info.base_addr + rtl8168_regs::CR);
            if cr & rtl8168_cr::RST == 0 {
                break;
            }
            delay::microseconds(10);
            timeout -= 1;
        }
        
        if timeout == 0 {
            return Err(EtherError::ResetTimeout);
        }
        
        // Read MAC address from EEPROM/flash
        let mac = read_mac_address(info)?;
        
        // Write MAC address to registers (for verification)
        for i in 0..6 {
            mmio::write8(info.base_addr + rtl8168_regs::MAC_ADDR + i, mac.bytes()[i]);
        }
        
        println!("[Ethernet] MAC Address: {}", mac.to_string());
        
        // Configure receive
        let rcr = rtl8168_rcr::AAP |    // Accept all packets (promiscuous mode)
                  rtl8168_rcr::APM |    // Accept physical match
                  rtl8168_rcr::AM  |    // Accept multicast
                  rtl8168_rcr::AB  |    // Accept broadcast
                  rtl8168_rcr::MXDMA_UNLIMITED |
                  rtl8168_rcr::RBLEN_64K;
        mmio::write32(info.base_addr + rtl8168_regs::RCR, rcr);
        
        // Configure transmit
        let tcr = rtl8168_tcr::MXDMA_UNLIMITED;
        mmio::write32(info.base_addr + rtl8168_regs::TCR, tcr);
        
        // Enable receiver and transmitter
        mmio::write8(info.base_addr + rtl8168_regs::CR, 
            rtl8168_cr::RE | rtl8168_cr::TE);
        
        // Enable interrupts
        let imr = rtl8168_isr::ROK | rtl8168_isr::TOK | rtl8168_isr::LINKCHG;
        mmio::write16(info.base_addr + rtl8168_regs::IMR, imr);
        
        mmio::memory_barrier();
    }
    
    // Update driver state
    let drv = driver();
    if let Some(ref mut ctrl) = drv.controller {
        ctrl.mac_address = read_mac_address(info).unwrap_or_default();
    }
    
    drv.link_up = check_link_status();
    
    println!("[Ethernet] RTL8168 initialized successfully");
    Ok(())
}

/// Read MAC address from controller
fn read_mac_address(info: &EtherControllerInfo) -> Result<MacAddress, EtherError> {
    unsafe {
        let mut mac = [0u8; 6];
        
        for i in 0..6 {
            mac[i] = mmio::read8(info.base_addr + rtl8168_regs::MAC_ADDR + i);
        }
        
        let addr = MacAddress::new(mac);
        
        if addr.is_valid() {
            Ok(addr)
        } else {
            // Return a default local MAC if none programmed
            Ok(MacAddress::new([0x02, 0x00, 0x00, 0x00, 0x00, 0x01]))
        }
    }
}

/// Check link status
fn check_link_status() -> bool {
    let drv = driver();
    
    if let Some(ref info) = drv.controller {
        unsafe {
            let phystatus = mmio::read8(info.base_addr + rtl8168_regs::PHYSTATUS);
            // Link status bit is typically bit 1
            (phystatus & 0x02) != 0
        }
    } else {
        false
    }
}

/// Get current link speed
fn get_link_speed() -> LinkSpeed {
    let drv = driver();
    
    if let Some(ref info) = drv.controller {
        unsafe {
            let phystatus = mmio::read8(info.base_addr + rtl8168_regs::PHYSTATUS);
            
            // Speed bits are typically in bits 2-3
            let speed_bits = (phystatus >> 2) & 0x3;
            
            match speed_bits {
                0 => LinkSpeed::Speed10M,
                1 => LinkSpeed::Speed100M,
                2 => LinkSpeed::Speed1G,
                _ => LinkSpeed::Disconnected,
            }
        }
    } else {
        LinkSpeed::Disconnected
    }
}

/// Ethernet error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EtherError {
    /// Success
    Success = 0,
    /// Device not found
    NotFound = 1,
    /// Initialization failed
    InitFailed = 2,
    /// Reset timeout
    ResetTimeout = 3,
    /// DMA error
    DmaError = 4,
    /// No memory
    NoMemory = 5,
    /// Invalid MAC address
    InvalidMac = 6,
    /// Link down
    LinkDown = 7,
}

/// Get the Ethernet driver instance
fn driver() -> &'static mut EthernetDriver {
    unsafe { &mut ETHERNET_DRIVER }
}

/// Send a packet (skeleton implementation)
pub fn send_packet(data: &[u8]) -> Result<(), EtherError> {
    let drv = driver();
    
    if !drv.initialized || drv.controller.is_none() {
        return Err(EtherError::NotFound);
    }
    
    if !drv.link_up {
        return Err(EtherError::LinkDown);
    }
    
    // TODO: Implement actual packet transmission
    // 1. Allocate Tx descriptor
    // 2. Copy data to DMA buffer
    // 3. Program descriptor
    // 4. Notify NIC
    // 5. Wait for completion or handle interrupt
    
    println!("[Ethernet] Send packet ({} bytes) - stub", data.len());
    drv.tx_count += 1;
    
    Ok(())
}

/// Receive a packet (skeleton implementation)
pub fn receive_packet(buffer: &mut [u8]) -> Result<usize, EtherError> {
    let drv = driver();
    
    if !drv.initialized || drv.controller.is_none() {
        return Err(EtherError::NotFound);
    }
    
    // TODO: Implement actual packet reception
    // 1. Check Rx descriptors
    // 2. If packet available, copy to buffer
    // 3. Update descriptor ownership
    // 4. Return packet length
    
    // For now, return no packet available
    Err(EtherError::NotFound)
}

/// Get MAC address
pub fn get_mac_address() -> MacAddress {
    let drv = driver();
    
    if let Some(ref info) = drv.controller {
        info.mac_address
    } else {
        MacAddress::default()
    }
}

/// Get link status
pub fn is_link_up() -> bool {
    driver().link_up
}

/// Get current link speed
pub fn get_link_speed_info() -> LinkSpeed {
    driver().link_speed
}

/// Get statistics
pub fn get_stats() -> (u64, u64, u64, u64) {
    let drv = driver();
    (drv.tx_count, drv.rx_count, drv.tx_errors, drv.rx_errors)
}

/// Print Ethernet driver information
pub fn print_info() {
    let drv = driver();
    
    println!("Ethernet Driver Information:");
    println!("  Initialized: {}", drv.initialized);
    
    if let Some(ref info) = drv.controller {
        println!("  Controller: {:?}", info.controller_type);
        println!("  Base Address: 0x{:016X}", info.base_addr);
        println!("  IRQ: {}", info.irq);
        println!("  MAC: {}", info.mac_address.to_string());
        println!("  Max MTU: {}", info.max_mtu);
    }
    
    println!("  Link Status: {}", if drv.link_up { "Up" } else { "Down" });
    println!("  Link Speed: {} Mbps", drv.link_speed.mbps());
    println!("  Duplex: {:?}", drv.duplex);
    
    let (tx, rx, tx_err, rx_err) = get_stats();
    println!("  TX Packets: {}", tx);
    println!("  RX Packets: {}", rx);
    println!("  TX Errors: {}", tx_err);
    println!("  RX Errors: {}", rx_err);
}

/// Research notes on Ethernet implementation
pub mod research {
    //! # Ethernet Implementation Research Notes
    //!
    //! ## RTL8168 Initialization Sequence:
    //!
    //! 1. Software reset (CR.RST = 1)
    //! 2. Wait for reset complete
    //! 3. Read MAC address from EEPROM
    //! 4. Configure Rx/Tx descriptors
    //! 5. Program RCR and TCR registers
    //! 6. Enable receiver and transmitter
    //! 7. Enable interrupts
    //!
    //! ## DMA Ring Management:
    //!
    //! ### Receive Ring:
    //! - Circular buffer of Rx descriptors
    //! - Each descriptor points to a receive buffer
    //! - Hardware writes received packets to buffers
    //! - Software processes and releases descriptors
    //!
    //! ### Transmit Ring:
    //! - Circular buffer of Tx descriptors
    //! - Software fills descriptors with packet data
    //! - Hardware transmits and marks complete
    //!
    //! ## PHY Communication:
    //! - Access via MII/GMII registers
    //! - Read PHY status for link state
    //! - Auto-negotiation for speed/duplex
    //!
    //! ## Interrupt Handling:
    //! - ROK: Packet received
    //! - TOK: Packet transmitted
    //! - LINKCHG: Link status changed
    //! - RDU: Rx descriptor unavailable
    //!
    //! ## Key Challenges:
    //! - PCIe enumeration and BAR mapping
    //! - DMA memory allocation and cache coherency
    //! - Descriptor ring management
    //! - PHY auto-negotiation
    //! - Performance optimization
}

/// Helper functions for MMIO
mod mmio_helpers {
    use super::*;
    
    pub unsafe fn read8(addr: usize) -> u8 {
        mmio::read32(addr) as u8
    }
    
    pub unsafe fn read16(addr: usize) -> u16 {
        mmio::read32(addr) as u16
    }
    
    pub unsafe fn write8(addr: usize, value: u8) {
        let current = mmio::read32(addr & !0x3);
        let shift = (addr & 0x3) * 8;
        let mask = 0xFF << shift;
        let new_value = (current & !mask) | ((value as u32) << shift);
        mmio::write32(addr & !0x3, new_value);
    }
    
    pub unsafe fn write16(addr: usize, value: u16) {
        let current = mmio::read32(addr & !0x3);
        let shift = (addr & 0x2) * 8;
        let mask = 0xFFFF << shift;
        let new_value = (current & !mask) | ((value as u32) << shift);
        mmio::write32(addr & !0x3, new_value);
    }
}

use mmio_helpers::*;
