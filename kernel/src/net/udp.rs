//! UDP (User Datagram Protocol)
//!
//! Simple connectionless transport protocol.

use alloc::collections::BTreeMap;
use alloc::vec;
use alloc::vec::Vec;
use spin::Mutex;
use lazy_static::lazy_static;

use crate::net::{Ipv4Address, Port, IpProtocol, ip};
use crate::println;

/// UDP header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct UdpHeader {
    pub src_port: u16,
    pub dst_port: u16,
    pub length: u16,
    pub checksum: u16,
}

impl UdpHeader {
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 8 {
            return None;
        }

        Some(Self {
            src_port: u16::from_be_bytes([data[0], data[1]]),
            dst_port: u16::from_be_bytes([data[2], data[3]]),
            length: u16::from_be_bytes([data[4], data[5]]),
            checksum: u16::from_be_bytes([data[6], data[7]]),
        })
    }

    pub fn to_bytes(&self) -> [u8; 8] {
        let mut buf = [0u8; 8];
        buf[0..2].copy_from_slice(&self.src_port.to_be_bytes());
        buf[2..4].copy_from_slice(&self.dst_port.to_be_bytes());
        buf[4..6].copy_from_slice(&self.length.to_be_bytes());
        buf[6..8].copy_from_slice(&self.checksum.to_be_bytes());
        buf
    }

    pub fn calculate_checksum(&self, src: Ipv4Address, dst: Ipv4Address, data: &[u8]) -> u16 {
        let header_bytes = self.to_bytes();
        let mut sum: u32 = 0;

        // Pseudo-header
        for chunk in src.as_bytes().chunks(2) {
            sum += u16::from_be_bytes([chunk[0], chunk[1]]) as u32;
        }
        for chunk in dst.as_bytes().chunks(2) {
            sum += u16::from_be_bytes([chunk[0], chunk[1]]) as u32;
        }
        sum += IpProtocol::Udp as u32;
        sum += (8 + data.len()) as u32;

        // UDP header
        for i in (0..8).step_by(2) {
            sum += u16::from_be_bytes([header_bytes[i], header_bytes[i + 1]]) as u32;
        }

        // UDP data
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

        let checksum = !(sum as u16);
        if checksum == 0 {
            0xFFFF
        } else {
            checksum
        }
    }
}

/// UDP socket
pub struct UdpSocket {
    pub local_port: Port,
    pub receive_queue: Vec<(Ipv4Address, Port, Vec<u8>)>,
}

/// UDP socket table
lazy_static! {
    static ref SOCKETS: Mutex<BTreeMap<Port, UdpSocket>> = Mutex::new(BTreeMap::new());
    static ref NEXT_EPHEMERAL_PORT: Mutex<u16> = Mutex::new(33434);
}

/// Get ephemeral port
fn get_ephemeral_port() -> Port {
    let mut port = NEXT_EPHEMERAL_PORT.lock();
    let p = *port;
    *port = if *port >= 65535 { 33434 } else { *port + 1 };
    Port::new(p)
}

/// Process incoming UDP packet
pub fn process_udp_packet(src: Ipv4Address, dst: Ipv4Address, data: &[u8]) {
    let header = match UdpHeader::from_bytes(data) {
        Some(h) => h,
        None => return,
    };

    let payload = &data[8..];
    let dst_port = Port::new(header.dst_port);

    let mut sockets = SOCKETS.lock();
    
    if let Some(socket) = sockets.get_mut(&dst_port) {
        // Store in receive queue
        if socket.receive_queue.len() < 64 {
            socket.receive_queue.push((
                src,
                Port::new(header.src_port),
                payload.to_vec()
            ));
        }
    }
}

/// Bind UDP socket to port
pub fn bind(port: Port) -> Result<(), ()> {
    let mut sockets = SOCKETS.lock();
    
    if sockets.contains_key(&port) {
        return Err(()); // Already bound
    }

    sockets.insert(port, UdpSocket {
        local_port: port,
        receive_queue: Vec::new(),
    });

    Ok(())
}

/// Send UDP packet
pub fn send_to(
    local_port: Port,
    remote_addr: Ipv4Address,
    remote_port: Port,
    data: &[u8]
) -> Result<usize, ()> {
    let config = super::get_config();
    if !config.is_configured() {
        return Err(());
    }

    let header = UdpHeader {
        src_port: local_port.as_u16(),
        dst_port: remote_port.as_u16(),
        length: (8 + data.len()) as u16,
        checksum: 0,
    };

    let mut packet = vec![0u8; 8 + data.len()];
    packet[0..8].copy_from_slice(&header.to_bytes());
    packet[8..].copy_from_slice(data);

    ip::send_ipv4_packet(IpProtocol::Udp, remote_addr, &packet)?;

    Ok(data.len())
}

/// Receive UDP packet
pub fn receive_from(
    local_port: Port,
    buf: &mut [u8]
) -> Option<(Ipv4Address, Port, usize)> {
    let mut sockets = SOCKETS.lock();
    let socket = sockets.get_mut(&local_port)?;

    if let Some((src_addr, src_port, data)) = socket.receive_queue.pop() {
        let len = buf.len().min(data.len());
        buf[..len].copy_from_slice(&data[..len]);
        Some((src_addr, src_port, len))
    } else {
        None
    }
}

/// Close UDP socket
pub fn close(port: Port) {
    SOCKETS.lock().remove(&port);
}

/// Print UDP statistics
pub fn print_stats() {
    let sockets = SOCKETS.lock();

    println!("UDP Sockets: {}", sockets.len());
    
    for (port, socket) in sockets.iter() {
        println!("  Port {}: {} packets queued", port.as_u16(), socket.receive_queue.len());
    }
}
