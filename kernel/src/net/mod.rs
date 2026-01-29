//! Network stack
//!
//! TCP/IP network implementation for WebbOS.

use alloc::vec::Vec;
use alloc::boxed::Box;
use spin::Mutex;
use lazy_static::lazy_static;

pub mod drivers;
pub mod tcp;
pub mod udp;
pub mod ip;
pub mod arp;
pub mod dhcp;
pub mod dns;
pub mod socket;

use crate::println;

/// MAC address (48-bit)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MacAddress([u8; 6]);

impl MacAddress {
    /// Create MAC address from bytes
    pub const fn new(bytes: [u8; 6]) -> Self {
        Self(bytes)
    }

    /// Broadcast MAC address
    pub const fn broadcast() -> Self {
        Self([0xFF; 6])
    }

    /// Check if broadcast
    pub fn is_broadcast(&self) -> bool {
        self.0 == [0xFF; 6]
    }

    /// Get bytes
    pub fn as_bytes(&self) -> &[u8; 6] {
        &self.0
    }

    /// Format as string
    pub fn format(&self) -> [u8; 17] {
        let mut buf = [0u8; 17];
        for i in 0..6 {
            let byte = self.0[i];
            buf[i * 3] = hex_nibble(byte >> 4);
            buf[i * 3 + 1] = hex_nibble(byte & 0xF);
            if i < 5 {
                buf[i * 3 + 2] = b':';
            }
        }
        buf
    }
}

fn hex_nibble(n: u8) -> u8 {
    if n < 10 {
        b'0' + n
    } else {
        b'A' + (n - 10)
    }
}

/// IPv4 address
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ipv4Address([u8; 4]);

impl Ipv4Address {
    /// Create IPv4 address from bytes
    pub const fn new(bytes: [u8; 4]) -> Self {
        Self(bytes)
    }

    /// Create from octets
    pub const fn from_octets(a: u8, b: u8, c: u8, d: u8) -> Self {
        Self([a, b, c, d])
    }

    /// Unspecified address (0.0.0.0)
    pub const fn unspecified() -> Self {
        Self([0, 0, 0, 0])
    }

    /// Loopback address (127.0.0.1)
    pub const fn loopback() -> Self {
        Self([127, 0, 0, 1])
    }

    /// Broadcast address (255.255.255.255)
    pub const fn broadcast() -> Self {
        Self([255, 255, 255, 255])
    }

    /// Get bytes
    pub fn as_bytes(&self) -> &[u8; 4] {
        &self.0
    }

    /// Convert to u32
    pub fn as_u32(&self) -> u32 {
        u32::from_be_bytes(self.0)
    }

    /// Format as string
    pub fn format(&self) -> [u8; 15] {
        let mut buf = [0u8; 15];
        let mut pos = 0;
        for i in 0..4 {
            let num = self.0[i];
            if num >= 100 {
                buf[pos] = b'0' + num / 100;
                pos += 1;
            }
            if num >= 10 {
                buf[pos] = b'0' + (num % 100) / 10;
                pos += 1;
            }
            buf[pos] = b'0' + num % 10;
            pos += 1;
            if i < 3 {
                buf[pos] = b'.';
                pos += 1;
            }
        }
        buf
    }

    /// Check if in same subnet
    pub fn in_same_subnet(&self, other: Ipv4Address, netmask: Ipv4Address) -> bool {
        let a = self.as_u32() & netmask.as_u32();
        let b = other.as_u32() & netmask.as_u32();
        a == b
    }
}

/// IPv6 address
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ipv6Address([u8; 16]);

impl Ipv6Address {
    /// Create IPv6 address from bytes
    pub const fn new(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// Loopback address (::1)
    pub const fn loopback() -> Self {
        let mut bytes = [0u8; 16];
        bytes[15] = 1;
        Self(bytes)
    }

    /// Unspecified address (::)
    pub const fn unspecified() -> Self {
        Self([0; 16])
    }
}

/// IP address (v4 or v6)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IpAddress {
    V4(Ipv4Address),
    V6(Ipv6Address),
}

/// Port number
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Port(u16);

impl Port {
    pub const fn new(num: u16) -> Self {
        Self(num)
    }

    pub fn as_u16(&self) -> u16 {
        self.0
    }
}

/// Socket address (IP + port)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SocketAddr {
    pub ip: IpAddress,
    pub port: Port,
}

impl SocketAddr {
    pub fn new(ip: IpAddress, port: Port) -> Self {
        Self { ip, port }
    }

    pub fn new_v4(ip: Ipv4Address, port: Port) -> Self {
        Self {
            ip: IpAddress::V4(ip),
            port,
        }
    }
}

/// Ethernet frame types
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EtherType {
    Ipv4 = 0x0800,
    Arp = 0x0806,
    Ipv6 = 0x86DD,
}

impl EtherType {
    pub fn from_u16(val: u16) -> Option<Self> {
        match val {
            0x0800 => Some(Self::Ipv4),
            0x0806 => Some(Self::Arp),
            0x86DD => Some(Self::Ipv6),
            _ => None,
        }
    }
}

/// IP protocol types
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpProtocol {
    Icmp = 1,
    Tcp = 6,
    Udp = 17,
}

impl IpProtocol {
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            1 => Some(Self::Icmp),
            6 => Some(Self::Tcp),
            17 => Some(Self::Udp),
            _ => None,
        }
    }
}

/// Network interface
pub trait NetworkInterface: Send + Sync {
    /// Get interface name
    fn name(&self) -> &str;
    /// Get MAC address
    fn mac_address(&self) -> MacAddress;
    /// Get MTU
    fn mtu(&self) -> usize;
    /// Send packet
    fn send(&self, data: &[u8]) -> Result<usize, NetError>;
    /// Receive packet (non-blocking)
    fn receive(&self, buf: &mut [u8]) -> Result<usize, NetError>;
    /// Check if link is up
    fn is_link_up(&self) -> bool;
}

/// Network error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetError {
    /// Success
    Success = 0,
    /// No such device
    NoDevice = 1,
    /// Device busy
    Busy = 2,
    /// No buffer space
    NoBuffer = 3,
    /// Packet too large
    PacketTooLarge = 4,
    /// Checksum error
    Checksum = 5,
    /// Timeout
    Timeout = 6,
    /// Not connected
    NotConnected = 7,
    /// Connection refused
    ConnectionRefused = 8,
    /// Connection reset
    ConnectionReset = 9,
    /// Unknown error
    Unknown = 255,
}

/// Global network interfaces
lazy_static! {
    static ref INTERFACES: Mutex<Vec<Box<dyn NetworkInterface>>> = Mutex::new(Vec::new());
    static ref DEFAULT_INTERFACE: Mutex<Option<usize>> = Mutex::new(None);
}

/// Initialize network stack
pub fn init() {
    println!("[net] Initializing network stack...");

    // Initialize drivers
    drivers::init();

    println!("[net] Network stack initialized");
}

/// Register network interface
pub fn register_interface(iface: Box<dyn NetworkInterface>) {
    let mut interfaces = INTERFACES.lock();
    let idx = interfaces.len();
    
    println!("[net] Registered interface {}: {} (MAC: {:?})",
        idx, iface.name(), iface.mac_address());
    
    interfaces.push(iface);

    // Set as default if first interface
    let mut default = DEFAULT_INTERFACE.lock();
    if default.is_none() {
        *default = Some(idx);
    }
}

/// Get number of interfaces
pub fn interface_count() -> usize {
    INTERFACES.lock().len()
}

/// Get default interface
pub fn default_interface() -> Option<usize> {
    *DEFAULT_INTERFACE.lock()
}

/// Print network interface list
pub fn print_interfaces() {
    let interfaces = INTERFACES.lock();
    let default = DEFAULT_INTERFACE.lock();

    println!("Network Interfaces:");
    println!("{:<4} {:<10} {:<20} {:<8} {}",
        "Idx", "Name", "MAC", "MTU", "Status");
    println!("{}", "-".repeat(60));

    for (i, iface) in interfaces.iter().enumerate() {
        let default_mark = if Some(i) == *default { "*" } else { " " };
        let mac = iface.mac_address();
        let mac_str = mac.format();
        let mac_str = core::str::from_utf8(&mac_str).unwrap_or("?");
        
        println!("{}{:<3} {:<10} {:<20} {:<8} {}",
            default_mark, i, iface.name(), mac_str,
            iface.mtu(),
            if iface.is_link_up() { "UP" } else { "DOWN" });
    }
}

/// Send packet on interface
pub fn send_packet(iface_idx: usize, data: &[u8]) -> Result<usize, NetError> {
    let interfaces = INTERFACES.lock();
    if let Some(iface) = interfaces.get(iface_idx) {
        iface.send(data)
    } else {
        Err(NetError::NoDevice)
    }
}

/// Receive packet from interface
pub fn receive_packet(iface_idx: usize, buf: &mut [u8]) -> Result<usize, NetError> {
    let interfaces = INTERFACES.lock();
    if let Some(iface) = interfaces.get(iface_idx) {
        iface.receive(buf)
    } else {
        Err(NetError::NoDevice)
    }
}

/// Process received packet
pub fn process_packet(data: &[u8]) {
    if data.len() < 14 {
        return; // Too short for Ethernet header
    }

    // Parse Ethernet header
    let dst_mac = MacAddress::new([data[0], data[1], data[2], data[3], data[4], data[5]]);
    let src_mac = MacAddress::new([data[6], data[7], data[8], data[9], data[10], data[11]]);
    let ether_type = u16::from_be_bytes([data[12], data[13]]);

    let payload = &data[14..];

    match EtherType::from_u16(ether_type) {
        Some(EtherType::Ipv4) => {
            ip::process_ipv4_packet(payload);
        }
        Some(EtherType::Arp) => {
            arp::process_arp_packet(src_mac, payload);
        }
        Some(EtherType::Ipv6) => {
            // IPv6 not yet implemented
        }
        None => {
            // Unknown ether type
        }
    }
}

/// Network configuration
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    /// IP address
    pub ip: Ipv4Address,
    /// Netmask
    pub netmask: Ipv4Address,
    /// Gateway
    pub gateway: Ipv4Address,
    /// DNS server
    pub dns: Ipv4Address,
}

impl NetworkConfig {
    /// Create empty configuration
    pub const fn empty() -> Self {
        Self {
            ip: Ipv4Address::unspecified(),
            netmask: Ipv4Address::unspecified(),
            gateway: Ipv4Address::unspecified(),
            dns: Ipv4Address::unspecified(),
        }
    }

    /// Check if configured
    pub fn is_configured(&self) -> bool {
        self.ip.as_u32() != 0
    }
}

/// Global network configuration
lazy_static! {
    static ref NET_CONFIG: Mutex<NetworkConfig> = Mutex::new(NetworkConfig::empty());
}

/// Get network configuration
pub fn get_config() -> NetworkConfig {
    NET_CONFIG.lock().clone()
}

/// Set network configuration
pub fn set_config(config: NetworkConfig) {
    let ip_str = config.ip.format();
    let ip_str = core::str::from_utf8(&ip_str).unwrap_or("?");
    let nm_str = config.netmask.format();
    let nm_str = core::str::from_utf8(&nm_str).unwrap_or("?");
    let gw_str = config.gateway.format();
    let gw_str = core::str::from_utf8(&gw_str).unwrap_or("?");
    
    println!("[net] Configured: IP={}/{} GW={}", ip_str, nm_str, gw_str);
    *NET_CONFIG.lock() = config;
}

/// Print network statistics
pub fn print_stats() {
    let config = NET_CONFIG.lock();
    
    println!("Network Statistics:");
    println!("  Interfaces: {}", interface_count());
    
    if config.is_configured() {
        let ip_str = config.ip.format();
        let ip_str = core::str::from_utf8(&ip_str).unwrap_or("?");
        println!("  IP: {}", ip_str);
    } else {
        println!("  IP: Not configured");
    }

    tcp::print_stats();
    udp::print_stats();
}
