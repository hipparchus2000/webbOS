//! ARP (Address Resolution Protocol)
//!
//! Maps IP addresses to MAC addresses.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use spin::Mutex;
use lazy_static::lazy_static;

use crate::net::{Ipv4Address, MacAddress, EtherType};
use crate::println;

/// ARP hardware types
const ARP_HW_ETHERNET: u16 = 1;

/// ARP operation codes
const ARP_OP_REQUEST: u16 = 1;
const ARP_OP_REPLY: u16 = 2;

/// ARP packet header
#[repr(C, packed)]
struct ArpPacket {
    /// Hardware type
    hw_type: u16,
    /// Protocol type
    proto_type: u16,
    /// Hardware address length
    hw_len: u8,
    /// Protocol address length
    proto_len: u8,
    /// Operation
    op: u16,
    /// Sender hardware address
    sender_mac: [u8; 6],
    /// Sender protocol address
    sender_ip: [u8; 4],
    /// Target hardware address
    target_mac: [u8; 6],
    /// Target protocol address
    target_ip: [u8; 4],
}

impl ArpPacket {
    fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 28 {
            return None;
        }

        Some(Self {
            hw_type: u16::from_be_bytes([data[0], data[1]]),
            proto_type: u16::from_be_bytes([data[2], data[3]]),
            hw_len: data[4],
            proto_len: data[5],
            op: u16::from_be_bytes([data[6], data[7]]),
            sender_mac: [data[8], data[9], data[10], data[11], data[12], data[13]],
            sender_ip: [data[14], data[15], data[16], data[17]],
            target_mac: [data[18], data[19], data[20], data[21], data[22], data[23]],
            target_ip: [data[24], data[25], data[26], data[27]],
        })
    }

    fn to_bytes(&self) -> [u8; 28] {
        let mut buf = [0u8; 28];
        buf[0..2].copy_from_slice(&self.hw_type.to_be_bytes());
        buf[2..4].copy_from_slice(&self.proto_type.to_be_bytes());
        buf[4] = self.hw_len;
        buf[5] = self.proto_len;
        buf[6..8].copy_from_slice(&self.op.to_be_bytes());
        buf[8..14].copy_from_slice(&self.sender_mac);
        buf[14..18].copy_from_slice(&self.sender_ip);
        buf[18..24].copy_from_slice(&self.target_mac);
        buf[24..28].copy_from_slice(&self.target_ip);
        buf
    }
}

/// ARP cache entry
struct ArpEntry {
    mac: MacAddress,
    timestamp: u64,
    pending: bool,
}

/// ARP cache
lazy_static! {
    static ref ARP_CACHE: Mutex<BTreeMap<Ipv4Address, ArpEntry>> = Mutex::new(BTreeMap::new());
}

/// ARP timeout (5 minutes in ms)
const ARP_TIMEOUT_MS: u64 = 300_000;

/// Process incoming ARP packet
pub fn process_arp_packet(src_mac: MacAddress, data: &[u8]) {
    let packet = match ArpPacket::from_bytes(data) {
        Some(p) => p,
        None => return,
    };

    // Verify it's Ethernet/IPv4 ARP
    if packet.hw_type != ARP_HW_ETHERNET ||
       packet.proto_type != EtherType::Ipv4 as u16 ||
       packet.hw_len != 6 ||
       packet.proto_len != 4 {
        return;
    }

    let sender_ip = Ipv4Address::new(packet.sender_ip);
    let target_ip = Ipv4Address::new(packet.target_ip);

    // Update cache with sender's info
    {
        let mut cache = ARP_CACHE.lock();
        cache.insert(sender_ip, ArpEntry {
            mac: src_mac,
            timestamp: crate::drivers::timer::elapsed_ms(),
            pending: false,
        });
    }

    match packet.op {
        ARP_OP_REQUEST => {
            // Check if it's asking for our IP
            let config = super::get_config();
            if config.is_configured() && target_ip == config.ip {
                // Send ARP reply
                send_arp_reply(src_mac, sender_ip);
            }
        }
        ARP_OP_REPLY => {
            // Cache is already updated above
        }
        _ => {}
    }
}

/// Send ARP request
pub fn send_arp_request(target_ip: Ipv4Address) {
    let config = super::get_config();
    if !config.is_configured() {
        return;
    }

    // Add pending entry
    {
        let mut cache = ARP_CACHE.lock();
        cache.insert(target_ip, ArpEntry {
            mac: MacAddress::broadcast(),
            timestamp: crate::drivers::timer::elapsed_ms(),
            pending: true,
        });
    }

    let packet = ArpPacket {
        hw_type: ARP_HW_ETHERNET,
        proto_type: EtherType::Ipv4 as u16,
        hw_len: 6,
        proto_len: 4,
        op: ARP_OP_REQUEST,
        sender_mac: [0, 0, 0, 0, 0, 0], // TODO: Get from interface
        sender_ip: *config.ip.as_bytes(),
        target_mac: [0; 6],
        target_ip: *target_ip.as_bytes(),
    };

    // Build Ethernet frame
    let mut frame = [0u8; 42];
    
    // Destination: broadcast
    frame[0..6].copy_from_slice(&[0xFF; 6]);
    
    // Source: our MAC (TODO: get from interface)
    frame[6..12].copy_from_slice(&[0; 6]);
    
    // EtherType: ARP
    frame[12..14].copy_from_slice(&(EtherType::Arp as u16).to_be_bytes());
    
    // ARP packet
    frame[14..42].copy_from_slice(&packet.to_bytes());

    // Send on default interface
    if let Some(idx) = super::default_interface() {
        let _ = super::send_packet(idx, &frame);
    }
}

/// Send ARP reply
fn send_arp_reply(dst_mac: MacAddress, dst_ip: Ipv4Address) {
    let config = super::get_config();
    if !config.is_configured() {
        return;
    }

    let packet = ArpPacket {
        hw_type: ARP_HW_ETHERNET,
        proto_type: EtherType::Ipv4 as u16,
        hw_len: 6,
        proto_len: 4,
        op: ARP_OP_REPLY,
        sender_mac: [0, 0, 0, 0, 0, 0], // TODO: Get from interface
        sender_ip: *config.ip.as_bytes(),
        target_mac: *dst_mac.as_bytes(),
        target_ip: *dst_ip.as_bytes(),
    };

    // Build Ethernet frame
    let mut frame = [0u8; 42];
    
    // Destination
    frame[0..6].copy_from_slice(dst_mac.as_bytes());
    
    // Source: our MAC (TODO)
    frame[6..12].copy_from_slice(&[0; 6]);
    
    // EtherType: ARP
    frame[12..14].copy_from_slice(&(EtherType::Arp as u16).to_be_bytes());
    
    // ARP packet
    frame[14..42].copy_from_slice(&packet.to_bytes());

    // Send on default interface
    if let Some(idx) = super::default_interface() {
        let _ = super::send_packet(idx, &frame);
    }
}

/// Look up MAC address for IP
pub fn lookup(ip: Ipv4Address) -> Option<MacAddress> {
    let cache = ARP_CACHE.lock();
    cache.get(&ip).map(|e| e.mac)
}

/// Resolve IP to MAC (may trigger ARP request)
pub fn resolve(ip: Ipv4Address) -> Option<MacAddress> {
    // Check local network
    let config = super::get_config();
    if !config.is_configured() {
        return None;
    }

    // If not in same subnet, use gateway
    let target_ip = if !ip.in_same_subnet(config.ip, config.netmask) {
        config.gateway
    } else {
        ip
    };

    // Check cache first
    {
        let cache = ARP_CACHE.lock();
        if let Some(entry) = cache.get(&target_ip) {
            if !entry.pending {
                return Some(entry.mac);
            }
        }
    }

    // Need to send ARP request
    send_arp_request(target_ip);
    
    // Return None for now (caller should retry)
    None
}

/// Clean up expired ARP entries
pub fn cleanup_cache() {
    let now = crate::drivers::timer::elapsed_ms();
    let mut cache = ARP_CACHE.lock();
    
    let expired: Vec<_> = cache
        .iter()
        .filter(|(_, e)| !e.pending && now - e.timestamp > ARP_TIMEOUT_MS)
        .map(|(k, _)| *k)
        .collect();
    
    for ip in expired {
        cache.remove(&ip);
    }
}

/// Print ARP cache
pub fn print_cache() {
    let cache = ARP_CACHE.lock();
    
    println!("ARP Cache:");
    println!("{:<20} {:<20} {}", "IP Address", "MAC Address", "Status");
    println!("{}", "-".repeat(60));

    for (ip, entry) in cache.iter() {
        let ip_str = ip.format();
        let ip_str = core::str::from_utf8(&ip_str).unwrap_or("?");
        let mac_str = entry.mac.format();
        let mac_str = core::str::from_utf8(&mac_str).unwrap_or("?");
        
        let status = if entry.pending {
            "PENDING"
        } else {
            "RESOLVED"
        };
        
        println!("{:<20} {:<20} {}", ip_str, mac_str, status);
    }
}
