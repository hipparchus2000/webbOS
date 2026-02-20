//! Socket API
//!
//! BSD-style socket interface for network programming.

use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::string::String;
use spin::Mutex;
use lazy_static::lazy_static;

use crate::net::{Ipv4Address, Port, tcp, udp};
use crate::net;
use crate::println;

/// Socket domain
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketDomain {
    Inet = 2,   // IPv4
    Inet6 = 10, // IPv6
}

/// Socket type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketType {
    Stream = 1, // TCP
    Dgram = 2,  // UDP
}

/// Socket protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketProtocol {
    Default = 0,
    Tcp = 6,
    Udp = 17,
}

/// Socket state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketState {
    Created,
    Bound,
    Listening,
    Connecting,
    Connected,
    Closed,
}

/// Socket structure
pub struct Socket {
    /// Socket file descriptor
    pub fd: usize,
    /// Domain (IPv4/IPv6)
    pub domain: SocketDomain,
    /// Type (Stream/Dgram)
    pub type_: SocketType,
    /// Protocol
    pub protocol: SocketProtocol,
    /// Current state
    pub state: SocketState,
    /// Local address
    pub local_addr: Option<Ipv4Address>,
    /// Local port
    pub local_port: Option<Port>,
    /// Remote address
    pub remote_addr: Option<Ipv4Address>,
    /// Remote port
    pub remote_port: Option<Port>,
    /// TCP connection ID (if stream socket)
    pub tcp_id: Option<tcp::ConnectionId>,
    /// Receive buffer
    pub rx_buffer: Vec<u8>,
    /// Non-blocking mode
    pub non_blocking: bool,
}

impl Socket {
    pub fn new(fd: usize, domain: SocketDomain, type_: SocketType, protocol: SocketProtocol) -> Self {
        Self {
            fd,
            domain,
            type_,
            protocol,
            state: SocketState::Created,
            local_addr: None,
            local_port: None,
            remote_addr: None,
            remote_port: None,
            tcp_id: None,
            rx_buffer: Vec::with_capacity(65536),
            non_blocking: false,
        }
    }
}

/// Socket table
lazy_static! {
    static ref SOCKETS: Mutex<Vec<Option<Box<Socket>>>> = Mutex::new(Vec::new());
    static ref NEXT_FD: Mutex<usize> = Mutex::new(3); // Start after stdin/stdout/stderr
}

/// Create new socket
pub fn socket(domain: SocketDomain, type_: SocketType, protocol: SocketProtocol) -> Result<usize, NetError> {
    let fd = {
        let mut next = NEXT_FD.lock();
        let fd = *next;
        *next += 1;
        fd
    };

    let socket = Socket::new(fd, domain, type_, protocol);

    let mut sockets = SOCKETS.lock();
    
    // Extend vector if needed
    if fd >= sockets.len() {
        sockets.resize_with(fd + 1, || None);
    }
    
    sockets[fd] = Some(Box::new(socket));

    Ok(fd)
}

/// Bind socket to address
pub fn bind(fd: usize, addr: Ipv4Address, port: Port) -> Result<(), NetError> {
    let mut sockets = SOCKETS.lock();
    let socket = sockets.get_mut(fd)
        .and_then(|s| s.as_mut())
        .ok_or(NetError::InvalidSocket)?;

    if socket.state != SocketState::Created {
        return Err(NetError::InvalidState);
    }

    // For UDP sockets
    if socket.type_ == SocketType::Dgram {
        udp::bind(port).map_err(|_| NetError::AddressInUse)?;
    }

    socket.local_addr = Some(addr);
    socket.local_port = Some(port);
    socket.state = SocketState::Bound;

    Ok(())
}

/// Listen for connections
pub fn listen(fd: usize, _backlog: usize) -> Result<(), NetError> {
    let mut sockets = SOCKETS.lock();
    let socket = sockets.get_mut(fd)
        .and_then(|s| s.as_mut())
        .ok_or(NetError::InvalidSocket)?;

    if socket.type_ != SocketType::Stream {
        return Err(NetError::NotSupported);
    }

    if socket.state != SocketState::Bound {
        return Err(NetError::InvalidState);
    }

    // Start listening on TCP port
    tcp::listen(socket.local_port.unwrap()).map_err(|_| NetError::AddressInUse)?;

    socket.state = SocketState::Listening;

    Ok(())
}

/// Accept connection
pub fn accept(fd: usize) -> Result<usize, NetError> {
    let local_port = {
        let mut sockets = SOCKETS.lock();
        let socket = sockets.get_mut(fd)
            .and_then(|s| s.as_mut())
            .ok_or(NetError::InvalidSocket)?;

        if socket.state != SocketState::Listening {
            return Err(NetError::InvalidState);
        }

        socket.local_port.unwrap()
    };

    // Try to accept
    let conn_id = tcp::accept(local_port).ok_or(NetError::WouldBlock)?;

    // Create new socket for connection
    let new_fd = {
        let mut next = NEXT_FD.lock();
        let fd = *next;
        *next += 1;
        fd
    };

    let mut new_socket = Socket::new(new_fd, SocketDomain::Inet, SocketType::Stream, SocketProtocol::Tcp);
    new_socket.state = SocketState::Connected;
    new_socket.local_addr = Some(conn_id.local_addr);
    new_socket.local_port = Some(conn_id.local_port);
    new_socket.remote_addr = Some(conn_id.remote_addr);
    new_socket.remote_port = Some(conn_id.remote_port);
    new_socket.tcp_id = Some(conn_id);

    let mut sockets = SOCKETS.lock();
    if new_fd >= sockets.len() {
        sockets.resize_with(new_fd + 1, || None);
    }
    sockets[new_fd] = Some(Box::new(new_socket));

    Ok(new_fd)
}

/// Connect to remote host
pub fn connect(fd: usize, addr: Ipv4Address, port: Port) -> Result<(), NetError> {
    let mut sockets = SOCKETS.lock();
    let socket = sockets.get_mut(fd)
        .and_then(|s| s.as_mut())
        .ok_or(NetError::InvalidSocket)?;

    match socket.type_ {
        SocketType::Stream => {
            // TCP connect
            let conn_id = tcp::connect(addr, port).map_err(|_| NetError::ConnectionRefused)?;
            socket.tcp_id = Some(conn_id);
            socket.state = SocketState::Connecting;
            socket.remote_addr = Some(addr);
            socket.remote_port = Some(port);
            
            // Get local port from connection
            socket.local_port = Some(conn_id.local_port);
        }
        SocketType::Dgram => {
            // UDP - just store remote address
            socket.remote_addr = Some(addr);
            socket.remote_port = Some(port);
            socket.state = SocketState::Connected;
        }
    }

    Ok(())
}

/// Send data
pub fn send(fd: usize, data: &[u8], _flags: i32) -> Result<usize, NetError> {
    let mut sockets = SOCKETS.lock();
    let socket = sockets.get_mut(fd)
        .and_then(|s| s.as_mut())
        .ok_or(NetError::InvalidSocket)?;

    match socket.type_ {
        SocketType::Stream => {
            let conn_id = socket.tcp_id.ok_or(NetError::NotConnected)?;
            tcp::send(conn_id, data).map_err(|_| NetError::ConnectionReset)
        }
        SocketType::Dgram => {
            let local_port = socket.local_port.ok_or(NetError::NotBound)?;
            let remote_addr = socket.remote_addr.ok_or(NetError::NotConnected)?;
            let remote_port = socket.remote_port.ok_or(NetError::NotConnected)?;
            
            udp::send_to(local_port, remote_addr, remote_port, data)
                .map_err(|_| NetError::NetworkError)
        }
    }
}

/// Receive data
pub fn recv(fd: usize, buf: &mut [u8], _flags: i32) -> Result<usize, NetError> {
    let mut sockets = SOCKETS.lock();
    let socket = sockets.get_mut(fd)
        .and_then(|s| s.as_mut())
        .ok_or(NetError::InvalidSocket)?;

    match socket.type_ {
        SocketType::Stream => {
            let conn_id = socket.tcp_id.ok_or(NetError::NotConnected)?;
            tcp::receive(conn_id, buf).map_err(|_| NetError::ConnectionReset)
        }
        SocketType::Dgram => {
            let local_port = socket.local_port.ok_or(NetError::NotBound)?;
            
            match udp::receive_from(local_port, buf) {
                Some((_, _, len)) => Ok(len),
                None => Err(NetError::WouldBlock),
            }
        }
    }
}

/// Send to specific address (UDP)
pub fn sendto(fd: usize, data: &[u8], _flags: i32, addr: Ipv4Address, port: Port) -> Result<usize, NetError> {
    let mut sockets = SOCKETS.lock();
    let socket = sockets.get_mut(fd)
        .and_then(|s| s.as_mut())
        .ok_or(NetError::InvalidSocket)?;

    if socket.type_ != SocketType::Dgram {
        return Err(NetError::NotSupported);
    }

    let local_port = socket.local_port.ok_or(NetError::NotBound)?;

    udp::send_to(local_port, addr, port, data)
        .map_err(|_| NetError::NetworkError)
}

/// Receive from address (UDP)
pub fn recvfrom(fd: usize, buf: &mut [u8], _flags: i32) -> Result<(usize, Ipv4Address, Port), NetError> {
    let mut sockets = SOCKETS.lock();
    let socket = sockets.get_mut(fd)
        .and_then(|s| s.as_mut())
        .ok_or(NetError::InvalidSocket)?;

    if socket.type_ != SocketType::Dgram {
        return Err(NetError::NotSupported);
    }

    let local_port = socket.local_port.ok_or(NetError::NotBound)?;

    match udp::receive_from(local_port, buf) {
        Some((addr, port, len)) => Ok((len, addr, port)),
        None => Err(NetError::WouldBlock),
    }
}

/// Close socket
pub fn close(fd: usize) -> Result<(), NetError> {
    let mut sockets = SOCKETS.lock();
    
    if let Some(Some(socket)) = sockets.get_mut(fd) {
        if socket.type_ == SocketType::Stream {
            if let Some(conn_id) = socket.tcp_id {
                let _ = tcp::close(conn_id);
            }
        } else if socket.type_ == SocketType::Dgram {
            if let Some(port) = socket.local_port {
                udp::close(port);
            }
        }
        
        socket.state = SocketState::Closed;
    }

    if fd < sockets.len() {
        sockets[fd] = None;
    }

    Ok(())
}

/// Get socket by fd
pub fn get_socket(fd: usize) -> Option<Box<Socket>> {
    SOCKETS.lock().get(fd).and_then(|opt| opt.as_ref().map(|s| alloc::boxed::Box::new(Socket {
        fd: s.fd,
        domain: s.domain,
        type_: s.type_,
        protocol: s.protocol,
        state: s.state,
        local_addr: s.local_addr,
        local_port: s.local_port,
        remote_addr: s.remote_addr,
        remote_port: s.remote_port,
        tcp_id: s.tcp_id,
        rx_buffer: alloc::vec::Vec::new(),
        non_blocking: s.non_blocking,
    })))
}

/// Print socket list
pub fn print_sockets() {
    let sockets = SOCKETS.lock();

    println!("Open Sockets:");
    println!("{:<6} {:<8} {:<10} {:<15} {:<15} {}",
        "FD", "Type", "State", "Local", "Remote", "TCP ID");
    println!("{}", "-".repeat(70));

    for opt in sockets.iter() {
        if let Some(socket) = opt {
            let type_str = match socket.type_ {
                SocketType::Stream => "TCP",
                SocketType::Dgram => "UDP",
            };

            let state_str = match socket.state {
                SocketState::Created => "CREATED",
                SocketState::Bound => "BOUND",
                SocketState::Listening => "LISTEN",
                SocketState::Connecting => "CONNECTING",
                SocketState::Connected => "CONNECTED",
                SocketState::Closed => "CLOSED",
            };

            let local = if let Some(port) = socket.local_port {
                let addr_str = socket.local_addr.map(|a| {
                    let s = a.format();
                    let mut buf = [0u8; 16];
                    buf.copy_from_slice(&s[..16.min(s.len())]);
                    buf
                }).unwrap_or([b'*', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
                let addr = core::str::from_utf8(&addr_str).unwrap_or("*").trim_end_matches('\0');
                let mut buf = [0u8; 32];
                let s = format_socket_addr(addr, port.as_u16(), &mut buf);
                let s = core::str::from_utf8(s).unwrap_or("?");
                alloc::string::String::from(s)
            } else {
                alloc::string::String::from("-")
            };

            let remote = if let Some(port) = socket.remote_port {
                let addr_str = socket.remote_addr.map(|a| {
                    let s = a.format();
                    let mut buf = [0u8; 16];
                    buf.copy_from_slice(&s[..16.min(s.len())]);
                    buf
                }).unwrap_or([b'*', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
                let addr = core::str::from_utf8(&addr_str).unwrap_or("*").trim_end_matches('\0');
                let mut buf = [0u8; 32];
                let s = format_socket_addr(addr, port.as_u16(), &mut buf);
                let s = core::str::from_utf8(s).unwrap_or("?");
                alloc::string::String::from(s)
            } else {
                alloc::string::String::from("-")
            };

            println!("{:<6} {:<8} {:<10} {:<15} {:<15} {:?}",
                socket.fd, type_str, state_str, local, remote,
                socket.tcp_id.is_some());
        }
    }
}

fn format_socket_addr<'a>(addr: &str, port: u16, buf: &'a mut [u8]) -> &'a [u8] {
    let mut pos = 0;
    for c in addr.bytes() {
        if pos < buf.len() {
            buf[pos] = c;
            pos += 1;
        }
    }
    if pos < buf.len() {
        buf[pos] = b':';
        pos += 1;
    }
    
    // Format port number
    let port_str = format_u16(port);
    for c in port_str.iter().copied() {
        if pos < buf.len() && c != 0 {
            buf[pos] = c;
            pos += 1;
        }
    }
    
    &buf[..pos]
}

fn format_u16(n: u16) -> [u8; 5] {
    let mut buf = [0u8; 5];
    let mut n = n;
    let mut pos = 5;
    
    if n == 0 {
        return [b'0', 0, 0, 0, 0];
    }
    
    while n > 0 && pos > 0 {
        pos -= 1;
        buf[pos] = b'0' + (n % 10) as u8;
        n /= 10;
    }
    
    // Rotate to beginning
    let mut result = [0u8; 5];
    for i in pos..5 {
        result[i - pos] = buf[i];
    }
    result
}

/// Network error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetError {
    Success = 0,
    InvalidSocket = 1,
    InvalidState = 2,
    AddressInUse = 3,
    NotBound = 4,
    NotConnected = 5,
    ConnectionRefused = 6,
    ConnectionReset = 7,
    WouldBlock = 8,
    NotSupported = 9,
    NetworkError = 10,
}
