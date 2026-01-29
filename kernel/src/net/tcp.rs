//! TCP (Transmission Control Protocol)
//!
//! Full TCP implementation with connection state management.

use alloc::collections::BTreeMap;
use alloc::vec;
use alloc::vec::Vec;
use spin::Mutex;
use lazy_static::lazy_static;
use core::sync::atomic::{AtomicU32, Ordering};

use crate::net::{Ipv4Address, Port, IpProtocol, ip};
use crate::println;

/// TCP header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct TcpHeader {
    pub src_port: u16,
    pub dst_port: u16,
    pub seq: u32,
    pub ack: u32,
    pub data_offset: u8,
    pub flags: u8,
    pub window: u16,
    pub checksum: u16,
    pub urgent: u16,
}

/// TCP flags
pub const TCP_FLAG_FIN: u8 = 0x01;
pub const TCP_FLAG_SYN: u8 = 0x02;
pub const TCP_FLAG_RST: u8 = 0x04;
pub const TCP_FLAG_PSH: u8 = 0x08;
pub const TCP_FLAG_ACK: u8 = 0x10;
pub const TCP_FLAG_URG: u8 = 0x20;

impl TcpHeader {
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 20 {
            return None;
        }

        Some(Self {
            src_port: u16::from_be_bytes([data[0], data[1]]),
            dst_port: u16::from_be_bytes([data[2], data[3]]),
            seq: u32::from_be_bytes([data[4], data[5], data[6], data[7]]),
            ack: u32::from_be_bytes([data[8], data[9], data[10], data[11]]),
            data_offset: data[12],
            flags: data[13],
            window: u16::from_be_bytes([data[14], data[15]]),
            checksum: u16::from_be_bytes([data[16], data[17]]),
            urgent: u16::from_be_bytes([data[18], data[19]]),
        })
    }

    pub fn to_bytes(&self) -> [u8; 20] {
        let mut buf = [0u8; 20];
        buf[0..2].copy_from_slice(&self.src_port.to_be_bytes());
        buf[2..4].copy_from_slice(&self.dst_port.to_be_bytes());
        buf[4..8].copy_from_slice(&self.seq.to_be_bytes());
        buf[8..12].copy_from_slice(&self.ack.to_be_bytes());
        buf[12] = self.data_offset;
        buf[13] = self.flags;
        buf[14..16].copy_from_slice(&self.window.to_be_bytes());
        buf[16..18].copy_from_slice(&self.checksum.to_be_bytes());
        buf[18..20].copy_from_slice(&self.urgent.to_be_bytes());
        buf
    }

    pub fn header_len(&self) -> usize {
        (((self.data_offset >> 4) & 0x0F) as usize) * 4
    }

    pub fn has_flag(&self, flag: u8) -> bool {
        (self.flags & flag) != 0
    }

    /// Calculate TCP checksum (pseudo-header + header + data)
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
        sum += IpProtocol::Tcp as u32;
        sum += (20 + data.len()) as u32;

        // TCP header
        for i in (0..20).step_by(2) {
            sum += u16::from_be_bytes([header_bytes[i], header_bytes[i + 1]]) as u32;
        }

        // TCP data
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

/// TCP connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcpState {
    Closed,
    Listen,
    SynSent,
    SynReceived,
    Established,
    FinWait1,
    FinWait2,
    CloseWait,
    Closing,
    LastAck,
    TimeWait,
}

/// TCP connection identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConnectionId {
    pub local_addr: Ipv4Address,
    pub local_port: Port,
    pub remote_addr: Ipv4Address,
    pub remote_port: Port,
}

/// TCP connection
pub struct TcpConnection {
    pub id: ConnectionId,
    pub state: TcpState,
    pub seq_num: u32,
    pub ack_num: u32,
    pub recv_window: u16,
    pub send_window: u16,
    /// Receive buffer
    pub rx_buffer: Vec<u8>,
    /// Send buffer
    pub tx_buffer: Vec<u8>,
    /// User waiting on this connection
    pub waiting: bool,
}

impl TcpConnection {
    pub fn new(id: ConnectionId) -> Self {
        static NEXT_SEQ: AtomicU32 = AtomicU32::new(1000);

        Self {
            id,
            state: TcpState::Closed,
            seq_num: NEXT_SEQ.fetch_add(1, Ordering::SeqCst),
            ack_num: 0,
            recv_window: 65535,
            send_window: 65535,
            rx_buffer: Vec::with_capacity(65536),
            tx_buffer: Vec::with_capacity(65536),
            waiting: false,
        }
    }
}

/// TCP socket table
lazy_static! {
    static ref CONNECTIONS: Mutex<BTreeMap<ConnectionId, TcpConnection>> = Mutex::new(BTreeMap::new());
    static ref LISTENING_SOCKETS: Mutex<BTreeMap<Port, ConnectionId>> = Mutex::new(BTreeMap::new());
    static ref NEXT_EPHEMERAL_PORT: Mutex<u16> = Mutex::new(49152);
}

/// Get ephemeral port
fn get_ephemeral_port() -> Port {
    let mut port = NEXT_EPHEMERAL_PORT.lock();
    let p = *port;
    *port = if *port >= 65535 { 49152 } else { *port + 1 };
    Port::new(p)
}

/// Process incoming TCP packet
pub fn process_tcp_packet(src: Ipv4Address, dst: Ipv4Address, data: &[u8]) {
    let header = match TcpHeader::from_bytes(data) {
        Some(h) => h,
        None => return,
    };

    let header_len = header.header_len();
    if header_len > data.len() {
        return;
    }

    let payload = &data[header_len..];

    // Build connection ID
    let id = ConnectionId {
        local_addr: dst,
        local_port: Port::new(header.dst_port),
        remote_addr: src,
        remote_port: Port::new(header.src_port),
    };

    // Check for existing connection
    let mut connections = CONNECTIONS.lock();

    if let Some(conn) = connections.get_mut(&id) {
        // Handle based on state
        handle_packet(conn, &header, payload);
    } else {
        // Check for listening socket
        let listening = LISTENING_SOCKETS.lock();
        
        if let Some(_) = listening.get(&Port::new(header.dst_port)) {
            // New connection attempt
            if header.has_flag(TCP_FLAG_SYN) {
                drop(listening);
                drop(connections);
                handle_syn(dst, src, header, payload);
            }
        } else {
            // No such connection - send RST
            send_rst(dst, src, header.dst_port, header.src_port, header.ack);
        }
    }
}

/// Handle packet for established connection
fn handle_packet(conn: &mut TcpConnection, header: &TcpHeader, payload: &[u8]) {
    // Update ACK number
    if header.seq == conn.ack_num {
        conn.ack_num = header.seq.wrapping_add(payload.len() as u32);
        
        if header.has_flag(TCP_FLAG_SYN) {
            conn.ack_num = conn.ack_num.wrapping_add(1);
        }
        if header.has_flag(TCP_FLAG_FIN) {
            conn.ack_num = conn.ack_num.wrapping_add(1);
        }

        // Copy payload to receive buffer
        if !payload.is_empty() && conn.rx_buffer.len() + payload.len() <= conn.rx_buffer.capacity() {
            conn.rx_buffer.extend_from_slice(payload);
        }
    }

    // Update send window
    conn.send_window = header.window;

    match conn.state {
        TcpState::SynSent => {
            if header.has_flag(TCP_FLAG_SYN) && header.has_flag(TCP_FLAG_ACK) {
                conn.state = TcpState::Established;
                conn.ack_num = header.seq.wrapping_add(1);
                
                // Send ACK
                send_ack(conn);
            }
        }
        TcpState::SynReceived => {
            if header.has_flag(TCP_FLAG_ACK) {
                conn.state = TcpState::Established;
            }
        }
        TcpState::Established => {
            if header.has_flag(TCP_FLAG_FIN) {
                conn.state = TcpState::CloseWait;
                
                // Send FIN-ACK
                send_fin_ack(conn);
                conn.seq_num = conn.seq_num.wrapping_add(1);
                conn.state = TcpState::LastAck;
            } else if !payload.is_empty() || header.has_flag(TCP_FLAG_ACK) {
                // Send ACK for received data
                send_ack(conn);
            }
        }
        TcpState::FinWait1 => {
            if header.has_flag(TCP_FLAG_FIN) && header.has_flag(TCP_FLAG_ACK) {
                conn.state = TcpState::TimeWait;
            } else if header.has_flag(TCP_FLAG_ACK) {
                conn.state = TcpState::FinWait2;
            }
        }
        TcpState::FinWait2 => {
            if header.has_flag(TCP_FLAG_FIN) {
                conn.ack_num = conn.ack_num.wrapping_add(1);
                send_ack(conn);
                conn.state = TcpState::TimeWait;
            }
        }
        TcpState::LastAck => {
            if header.has_flag(TCP_FLAG_ACK) {
                conn.state = TcpState::Closed;
            }
        }
        _ => {}
    }
}

/// Handle incoming SYN (new connection)
fn handle_syn(dst: Ipv4Address, src: Ipv4Address, header: TcpHeader, _payload: &[u8]) {
    let local_port = Port::new(header.dst_port);
    let remote_port = Port::new(header.src_port);

    let id = ConnectionId {
        local_addr: dst,
        local_port,
        remote_addr: src,
        remote_port,
    };

    let mut conn = TcpConnection::new(id);
    conn.state = TcpState::SynReceived;
    conn.ack_num = header.seq.wrapping_add(1);

    // Send SYN-ACK
    let mut reply = TcpHeader {
        src_port: local_port.as_u16(),
        dst_port: remote_port.as_u16(),
        seq: conn.seq_num,
        ack: conn.ack_num,
        data_offset: 0x50, // 20 bytes header
        flags: TCP_FLAG_SYN | TCP_FLAG_ACK,
        window: conn.recv_window,
        checksum: 0,
        urgent: 0,
    };

    reply.checksum = reply.calculate_checksum(dst, src, &[]);

    let mut packet = vec![0u8; 20];
    packet.copy_from_slice(&reply.to_bytes());

    let _ = ip::send_ipv4_packet(IpProtocol::Tcp, src, &packet);

    conn.seq_num = conn.seq_num.wrapping_add(1);

    // Store connection
    CONNECTIONS.lock().insert(id, conn);
}

/// Send ACK
fn send_ack(conn: &mut TcpConnection) {
    let mut header = TcpHeader {
        src_port: conn.id.local_port.as_u16(),
        dst_port: conn.id.remote_port.as_u16(),
        seq: conn.seq_num,
        ack: conn.ack_num,
        data_offset: 0x50,
        flags: TCP_FLAG_ACK,
        window: conn.recv_window,
        checksum: 0,
        urgent: 0,
    };

    header.checksum = header.calculate_checksum(
        conn.id.local_addr,
        conn.id.remote_addr,
        &[]
    );

    let mut packet = vec![0u8; 20];
    packet.copy_from_slice(&header.to_bytes());

    let _ = ip::send_ipv4_packet(IpProtocol::Tcp, conn.id.remote_addr, &packet);
}

/// Send FIN-ACK
fn send_fin_ack(conn: &mut TcpConnection) {
    let mut header = TcpHeader {
        src_port: conn.id.local_port.as_u16(),
        dst_port: conn.id.remote_port.as_u16(),
        seq: conn.seq_num,
        ack: conn.ack_num,
        data_offset: 0x50,
        flags: TCP_FLAG_FIN | TCP_FLAG_ACK,
        window: conn.recv_window,
        checksum: 0,
        urgent: 0,
    };

    header.checksum = header.calculate_checksum(
        conn.id.local_addr,
        conn.id.remote_addr,
        &[]
    );

    let mut packet = vec![0u8; 20];
    packet.copy_from_slice(&header.to_bytes());

    let _ = ip::send_ipv4_packet(IpProtocol::Tcp, conn.id.remote_addr, &packet);
}

/// Send RST
fn send_rst(src: Ipv4Address, dst: Ipv4Address, src_port: u16, dst_port: u16, ack: u32) {
    let mut header = TcpHeader {
        src_port,
        dst_port,
        seq: 0,
        ack: ack.wrapping_add(1),
        data_offset: 0x50,
        flags: TCP_FLAG_RST | TCP_FLAG_ACK,
        window: 0,
        checksum: 0,
        urgent: 0,
    };

    header.checksum = header.calculate_checksum(src, dst, &[]);

    let mut packet = vec![0u8; 20];
    packet.copy_from_slice(&header.to_bytes());

    let _ = ip::send_ipv4_packet(IpProtocol::Tcp, dst, &packet);
}

/// Connect to remote host
pub fn connect(remote_addr: Ipv4Address, remote_port: Port) -> Result<ConnectionId, ()> {
    let config = super::get_config();
    if !config.is_configured() {
        return Err(());
    }

    let local_port = get_ephemeral_port();

    let id = ConnectionId {
        local_addr: config.ip,
        local_port,
        remote_addr,
        remote_port,
    };

    let mut conn = TcpConnection::new(id);
    conn.state = TcpState::SynSent;

    // Send SYN
    let mut header = TcpHeader {
        src_port: local_port.as_u16(),
        dst_port: remote_port.as_u16(),
        seq: conn.seq_num,
        ack: 0,
        data_offset: 0x50,
        flags: TCP_FLAG_SYN,
        window: conn.recv_window,
        checksum: 0,
        urgent: 0,
    };

    header.checksum = header.calculate_checksum(config.ip, remote_addr, &[]);

    let mut packet = vec![0u8; 20];
    packet.copy_from_slice(&header.to_bytes());

    ip::send_ipv4_packet(IpProtocol::Tcp, remote_addr, &packet)?;

    conn.seq_num = conn.seq_num.wrapping_add(1);

    CONNECTIONS.lock().insert(id, conn);

    Ok(id)
}

/// Listen on port
pub fn listen(port: Port) -> Result<(), ()> {
    LISTENING_SOCKETS.lock().insert(port, ConnectionId {
        local_addr: Ipv4Address::unspecified(),
        local_port: port,
        remote_addr: Ipv4Address::unspecified(),
        remote_port: Port::new(0),
    });
    Ok(())
}

/// Accept connection
pub fn accept(port: Port) -> Option<ConnectionId> {
    let connections = CONNECTIONS.lock();
    
    for (id, conn) in connections.iter() {
        if id.local_port == port && conn.state == TcpState::Established {
            return Some(*id);
        }
    }
    
    None
}

/// Send data on connection
pub fn send(id: ConnectionId, data: &[u8]) -> Result<usize, ()> {
    let mut connections = CONNECTIONS.lock();
    let conn = connections.get_mut(&id).ok_or(())?;

    if conn.state != TcpState::Established {
        return Err(());
    }

    // Send data
    let mut header = TcpHeader {
        src_port: id.local_port.as_u16(),
        dst_port: id.remote_port.as_u16(),
        seq: conn.seq_num,
        ack: conn.ack_num,
        data_offset: 0x50,
        flags: TCP_FLAG_ACK | TCP_FLAG_PSH,
        window: conn.recv_window,
        checksum: 0,
        urgent: 0,
    };

    header.checksum = header.calculate_checksum(id.local_addr, id.remote_addr, data);

    let mut packet = vec![0u8; 20 + data.len()];
    packet[0..20].copy_from_slice(&header.to_bytes());
    packet[20..].copy_from_slice(data);

    ip::send_ipv4_packet(IpProtocol::Tcp, id.remote_addr, &packet)?;

    conn.seq_num = conn.seq_num.wrapping_add(data.len() as u32);

    Ok(data.len())
}

/// Receive data from connection
pub fn receive(id: ConnectionId, buf: &mut [u8]) -> Result<usize, ()> {
    let mut connections = CONNECTIONS.lock();
    let conn = connections.get_mut(&id).ok_or(())?;

    let len = buf.len().min(conn.rx_buffer.len());
    if len == 0 {
        return Ok(0);
    }

    buf[..len].copy_from_slice(&conn.rx_buffer[..len]);
    conn.rx_buffer.drain(..len);

    Ok(len)
}

/// Close connection
pub fn close(id: ConnectionId) -> Result<(), ()> {
    let mut connections = CONNECTIONS.lock();
    let conn = connections.get_mut(&id).ok_or(())?;

    match conn.state {
        TcpState::Established => {
            send_fin_ack(conn);
            conn.seq_num = conn.seq_num.wrapping_add(1);
            conn.state = TcpState::FinWait1;
            Ok(())
        }
        TcpState::CloseWait => {
            send_fin_ack(conn);
            conn.seq_num = conn.seq_num.wrapping_add(1);
            conn.state = TcpState::LastAck;
            Ok(())
        }
        _ => Err(()),
    }
}

/// Print TCP statistics
pub fn print_stats() {
    let connections = CONNECTIONS.lock();
    let listening = LISTENING_SOCKETS.lock();

    println!("TCP Connections: {}", connections.len());
    println!("Listening Ports: {}", listening.len());
}
