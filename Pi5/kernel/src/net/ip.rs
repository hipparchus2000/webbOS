//! IP (Internet Protocol) layer
//!
//! Handles IPv4 packet processing and routing.

use alloc::vec;
use alloc::vec::Vec;
use crate::net::{Ipv4Address, IpProtocol, arp};
use crate::println;

/// IPv4 header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Ipv4Header {
    /// Version (4) and IHL (5)
    pub ver_ihl: u8,
    /// DSCP and ECN
    pub tos: u8,
    /// Total length
    pub total_len: u16,
    /// Identification
    pub id: u16,
    /// Flags and fragment offset
    pub flags_frag: u16,
    /// TTL
    pub ttl: u8,
    /// Protocol
    pub protocol: u8,
    /// Header checksum
    pub checksum: u16,
    /// Source address
    pub src: [u8; 4],
    /// Destination address
    pub dst: [u8; 4],
}

impl Ipv4Header {
    pub fn new(protocol: IpProtocol, src: Ipv4Address, dst: Ipv4Address, payload_len: u16) -> Self {
        let total_len = 20 + payload_len;
        
        Self {
            ver_ihl: 0x45, // Version 4, IHL 5 (20 bytes)
            tos: 0,
            total_len,
            id: 0, // Will be set later
            flags_frag: 0x4000, // Don't fragment
            ttl: 64,
            protocol: protocol as u8,
            checksum: 0, // Will be calculated
            src: *src.as_bytes(),
            dst: *dst.as_bytes(),
        }
    }

    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 20 {
            return None;
        }

        Some(Self {
            ver_ihl: data[0],
            tos: data[1],
            total_len: u16::from_be_bytes([data[2], data[3]]),
            id: u16::from_be_bytes([data[4], data[5]]),
            flags_frag: u16::from_be_bytes([data[6], data[7]]),
            ttl: data[8],
            protocol: data[9],
            checksum: u16::from_be_bytes([data[10], data[11]]),
            src: [data[12], data[13], data[14], data[15]],
            dst: [data[16], data[17], data[18], data[19]],
        })
    }

    pub fn to_bytes(&self) -> [u8; 20] {
        let mut buf = [0u8; 20];
        buf[0] = self.ver_ihl;
        buf[1] = self.tos;
        buf[2..4].copy_from_slice(&self.total_len.to_be_bytes());
        buf[4..6].copy_from_slice(&self.id.to_be_bytes());
        buf[6..8].copy_from_slice(&self.flags_frag.to_be_bytes());
        buf[8] = self.ttl;
        buf[9] = self.protocol;
        buf[10..12].copy_from_slice(&self.checksum.to_be_bytes());
        buf[12..16].copy_from_slice(&self.src);
        buf[16..20].copy_from_slice(&self.dst);
        buf
    }

    /// Get header length in bytes
    pub fn header_len(&self) -> usize {
        ((self.ver_ihl & 0x0F) as usize) * 4
    }

    /// Get payload length
    pub fn payload_len(&self) -> usize {
        (self.total_len as usize).saturating_sub(self.header_len())
    }

    /// Calculate header checksum
    pub fn calculate_checksum(&self) -> u16 {
        let bytes = self.to_bytes();
        let mut sum: u32 = 0;

        // Skip checksum field (bytes 10-11)
        for i in (0..20).step_by(2) {
            if i != 10 {
                sum += u16::from_be_bytes([bytes[i], bytes[i + 1]]) as u32;
            }
        }

        while (sum >> 16) != 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }

        !(sum as u16)
    }

    /// Verify header checksum
    pub fn verify_checksum(&self) -> bool {
        self.checksum == self.calculate_checksum()
    }

    pub fn src_ip(&self) -> Ipv4Address {
        Ipv4Address::new(self.src)
    }

    pub fn dst_ip(&self) -> Ipv4Address {
        Ipv4Address::new(self.dst)
    }
}

/// Process incoming IPv4 packet
pub fn process_ipv4_packet(data: &[u8]) {
    let header = match Ipv4Header::from_bytes(data) {
        Some(h) => h,
        None => return,
    };

    // Verify version
    if (header.ver_ihl >> 4) != 4 {
        return;
    }

    // Verify header length
    let header_len = header.header_len();
    if header_len < 20 || header_len > data.len() {
        return;
    }

    // Verify checksum (optional in many stacks)
    // if !header.verify_checksum() { return; }

    // Verify total length
    let total_len = header.total_len as usize;
    if total_len > data.len() {
        return;
    }

    let payload = &data[header_len..total_len];

    // Dispatch based on protocol
    match IpProtocol::from_u8(header.protocol) {
        Some(IpProtocol::Tcp) => {
            super::tcp::process_tcp_packet(header.src_ip(), header.dst_ip(), payload);
        }
        Some(IpProtocol::Udp) => {
            super::udp::process_udp_packet(header.src_ip(), header.dst_ip(), payload);
        }
        Some(IpProtocol::Icmp) => {
            process_icmp_packet(header.src_ip(), header.dst_ip(), payload);
        }
        None => {
            // Unknown protocol - could send ICMP destination unreachable
        }
    }
}

/// Send IPv4 packet
pub fn send_ipv4_packet(
    protocol: IpProtocol,
    dst: Ipv4Address,
    payload: &[u8]
) -> Result<usize, ()> {
    let config = super::get_config();
    if !config.is_configured() {
        return Err(());
    }

    // Create header
    let mut header = Ipv4Header::new(protocol, config.ip, dst, payload.len() as u16);
    
    // Calculate and set checksum
    header.checksum = header.calculate_checksum();

    // Build complete packet
    let packet_len = 20 + payload.len();
    if packet_len > 1500 {
        return Err(()); // Too large
    }

    let mut packet = vec![0u8; packet_len];
    packet[0..20].copy_from_slice(&header.to_bytes());
    packet[20..].copy_from_slice(payload);

    // Resolve destination MAC
    let dst_mac = match arp::resolve(dst) {
        Some(mac) => mac,
        None => return Err(()), // Could queue and retry
    };

    // Build Ethernet frame
    let mut frame = vec![0u8; 14 + packet_len];
    frame[0..6].copy_from_slice(dst_mac.as_bytes());
    frame[6..12].copy_from_slice(&[0; 6]); // TODO: Our MAC
    frame[12..14].copy_from_slice(&(super::EtherType::Ipv4 as u16).to_be_bytes());
    frame[14..].copy_from_slice(&packet);

    // Send
    if let Some(idx) = super::default_interface() {
        match super::send_packet(idx, &frame) {
            Ok(n) => Ok(n.saturating_sub(14)),
            Err(_) => Err(()),
        }
    } else {
        Err(())
    }
}

/// ICMP types
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum IcmpType {
    EchoReply = 0,
    EchoRequest = 8,
    DestinationUnreachable = 3,
    TimeExceeded = 11,
}

/// ICMP header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct IcmpHeader {
    type_: u8,
    code: u8,
    checksum: u16,
    id: u16,
    seq: u16,
}

impl IcmpHeader {
    fn to_bytes(&self) -> [u8; 8] {
        let mut buf = [0u8; 8];
        buf[0] = self.type_;
        buf[1] = self.code;
        buf[2..4].copy_from_slice(&self.checksum.to_be_bytes());
        buf[4..6].copy_from_slice(&self.id.to_be_bytes());
        buf[6..8].copy_from_slice(&self.seq.to_be_bytes());
        buf
    }

    fn calculate_checksum(&self, data: &[u8]) -> u16 {
        let header_bytes = self.to_bytes();
        let mut sum: u32 = 0;

        for i in (0..8).step_by(2) {
            sum += u16::from_be_bytes([header_bytes[i], header_bytes[i + 1]]) as u32;
        }

        for i in (0..data.len()).step_by(2) {
            if i + 1 < data.len() {
                sum += u16::from_be_bytes([data[i], data[i + 1]]) as u32;
            } else {
                sum += (data[i] as u32) << 8;
            }
        }

        while (sum >> 16) != 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }

        !(sum as u16)
    }
}

/// Process ICMP packet
fn process_icmp_packet(src: Ipv4Address, dst: Ipv4Address, data: &[u8]) {
    if data.len() < 8 {
        return;
    }

    let type_ = data[0];
    let code = data[1];
    // let checksum = u16::from_be_bytes([data[2], data[3]]);
    let id = u16::from_be_bytes([data[4], data[5]]);
    let seq = u16::from_be_bytes([data[6], data[7]]);

    match type_ {
        8 => {
            // Echo request - send reply
            send_icmp_echo_reply(src, id, seq, &data[8..]);
        }
        _ => {
            // Other types not handled
        }
    }
}

/// Send ICMP echo reply (ping response)
fn send_icmp_echo_reply(dst: Ipv4Address, id: u16, seq: u16, data: &[u8]) {
    let mut header = IcmpHeader {
        type_: 0, // Echo reply
        code: 0,
        checksum: 0,
        id,
        seq,
    };

    header.checksum = header.calculate_checksum(data);

    let mut packet = vec![0u8; 8 + data.len()];
    packet[0..8].copy_from_slice(&header.to_bytes());
    packet[8..].copy_from_slice(data);

    let _ = send_ipv4_packet(IpProtocol::Icmp, dst, &packet);
}

/// Send ping request
pub fn ping(dst: Ipv4Address) -> Result<(), ()> {
    let data = b"WebbOS";
    
    let mut header = IcmpHeader {
        type_: 8, // Echo request
        code: 0,
        checksum: 0,
        id: 1,
        seq: 1,
    };

    header.checksum = header.calculate_checksum(data);

    let mut packet = vec![0u8; 8 + data.len()];
    packet[0..8].copy_from_slice(&header.to_bytes());
    packet[8..].copy_from_slice(data);

    send_ipv4_packet(IpProtocol::Icmp, dst, &packet)
        .map(|_| ())
}

/// Packet counter for identification
static mut PACKET_ID: u16 = 0;

/// Get next packet ID
pub fn next_packet_id() -> u16 {
    unsafe {
        PACKET_ID = PACKET_ID.wrapping_add(1);
        PACKET_ID
    }
}
