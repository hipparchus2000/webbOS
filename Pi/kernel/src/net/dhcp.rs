//! DHCP Client Implementation
//!
//! Implements DHCP (Dynamic Host Configuration Protocol) for automatic
//! IP address configuration on WiFi networks.

use alloc::vec::Vec;
use crate::net::Ipv4Address;
use crate::net::MacAddress;
use crate::println;
use crate::drivers::timer::elapsed_ms;

// DHCP constants
pub const DHCP_CLIENT_PORT: u16 = 68;
pub const DHCP_SERVER_PORT: u16 = 67;

// DHCP message types
pub const DHCP_DISCOVER: u8 = 1;
pub const DHCP_OFFER: u8 = 2;
pub const DHCP_REQUEST: u8 = 3;
pub const DHCP_DECLINE: u8 = 4;
pub const DHCP_ACK: u8 = 5;
pub const DHCP_NAK: u8 = 6;
pub const DHCP_RELEASE: u8 = 7;
pub const DHCP_INFORM: u8 = 8;

// DHCP options
pub const DHCP_OPTION_MESSAGE_TYPE: u8 = 53;
pub const DHCP_OPTION_END: u8 = 255;

// DHCP magic cookie
pub const DHCP_MAGIC_COOKIE: [u8; 4] = [0x63, 0x82, 0x53, 0x63];

// DHCP timeouts (in milliseconds)
pub const DHCP_TIMEOUT_INITIAL: u64 = 4000;  // 4 seconds
pub const DHCP_TIMEOUT_MAX: u64 = 64000;      // 64 seconds
pub const DHCP_RETRY_COUNT: u32 = 4;

/// DHCP message structure (simplified)
#[derive(Debug, Clone)]
pub struct DhcpMessage {
    pub op: u8,
    pub htype: u8,
    pub hlen: u8,
    pub xid: u32,
    pub ciaddr: Ipv4Address,
    pub yiaddr: Ipv4Address,
    pub siaddr: Ipv4Address,
    pub chaddr: [u8; 16],
    pub options: Vec<u8>,
}

impl DhcpMessage {
    /// Create a new DHCP message
    pub fn new(xid: u32, mac: &MacAddress) -> Self {
        let mut chaddr = [0u8; 16];
        chaddr[..6].copy_from_slice(mac.as_bytes());
        
        Self {
            op: 1,  // BOOTREQUEST
            htype: 1,  // Ethernet
            hlen: 6,
            xid,
            ciaddr: Ipv4Address::new([0, 0, 0, 0]),
            yiaddr: Ipv4Address::new([0, 0, 0, 0]),
            siaddr: Ipv4Address::new([0, 0, 0, 0]),
            chaddr,
            options: Vec::new(),
        }
    }
    
    /// Serialize DHCP message to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut result = Vec::with_capacity(300);
        
        result.push(self.op);
        result.push(self.htype);
        result.push(self.hlen);
        result.push(0);  // hops
        result.extend_from_slice(&self.xid.to_be_bytes());
        result.extend_from_slice(&[0u8; 2]);  // secs
        result.extend_from_slice(&[0x80, 0x00]);  // flags (broadcast)
        result.extend_from_slice(self.ciaddr.as_bytes());
        result.extend_from_slice(self.yiaddr.as_bytes());
        result.extend_from_slice(self.siaddr.as_bytes());
        result.extend_from_slice(&[0u8; 4]);  // giaddr
        result.extend_from_slice(&self.chaddr);
        result.extend_from_slice(&[0u8; 64]);  // sname
        result.extend_from_slice(&[0u8; 128]); // file
        
        // Magic cookie
        result.extend_from_slice(&DHCP_MAGIC_COOKIE);
        
        // Options
        result.extend_from_slice(&self.options);
        result.push(DHCP_OPTION_END);
        
        // Pad to minimum size
        while result.len() < 300 {
            result.push(0);
        }
        
        result
    }
    
    /// Set message type
    pub fn set_message_type(&mut self, msg_type: u8) {
        self.options.push(DHCP_OPTION_MESSAGE_TYPE);
        self.options.push(1);
        self.options.push(msg_type);
    }
    
    /// Parse DHCP message from bytes
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 240 {
            return None;
        }
        
        let op = data[0];
        let htype = data[1];
        let hlen = data[2];
        
        // Parse xid
        let xid = ((data[4] as u32) << 24) |
                  ((data[5] as u32) << 16) |
                  ((data[6] as u32) << 8) |
                  (data[7] as u32);
        
        // Parse addresses
        let ciaddr = Ipv4Address::from_bytes(&data[12..16]);
        let yiaddr = Ipv4Address::from_bytes(&data[16..20]);
        let siaddr = Ipv4Address::from_bytes(&data[20..24]);
        
        // Parse chaddr
        let mut chaddr = [0u8; 16];
        chaddr.copy_from_slice(&data[28..44]);
        
        // Parse options (skip magic cookie)
        let mut options = Vec::new();
        if data.len() > 240 {
            let opts_start = 240;
            let mut i = opts_start;
            
            // Check for magic cookie
            if data.len() >= opts_start + 4 && 
               data[opts_start..opts_start+4] == [0x63, 0x82, 0x53, 0x63] {
                i = opts_start + 4;
                
                while i < data.len() && data[i] != DHCP_OPTION_END {
                    let opt_code = data[i];
                    if opt_code == 0 {
                        // Padding
                        i += 1;
                        continue;
                    }
                    
                    if i + 1 >= data.len() {
                        break;
                    }
                    
                    let opt_len = data[i + 1] as usize;
                    options.push(opt_code);
                    options.push(opt_len as u8);
                    
                    if i + 2 + opt_len <= data.len() {
                        options.extend_from_slice(&data[i + 2..i + 2 + opt_len]);
                    }
                    
                    i += 2 + opt_len;
                }
            }
        }
        
        Some(Self {
            op,
            htype,
            hlen,
            xid,
            ciaddr,
            yiaddr,
            siaddr,
            chaddr,
            options,
        })
    }
    
    /// Get message type from options
    pub fn message_type(&self) -> Option<u8> {
        let mut i = 0;
        while i < self.options.len() {
            if i + 1 >= self.options.len() {
                break;
            }
            let code = self.options[i];
            let len = self.options[i + 1] as usize;
            
            if code == DHCP_OPTION_MESSAGE_TYPE && len == 1 && i + 2 < self.options.len() {
                return Some(self.options[i + 2]);
            }
            
            i += 2 + len;
        }
        None
    }
}

/// DHCP client state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DhcpState {
    Idle,
    Selecting,
    Requesting,
    Bound,
    Renewing,
    Rebinding,
}

/// DHCP client configuration
pub struct DhcpClient {
    pub state: DhcpState,
    pub mac_address: MacAddress,
    pub xid: u32,
    pub assigned_ip: Option<Ipv4Address>,
    pub subnet_mask: Option<Ipv4Address>,
    pub gateway: Option<Ipv4Address>,
    pub dns_servers: Vec<Ipv4Address>,
    pub server_id: Option<Ipv4Address>,
    pub lease_time: u32,
    pub lease_start: u64,
    pub last_activity: u64,
    pub retry_count: u32,
    pub timeout: u64,
}

impl DhcpClient {
    /// Create a new DHCP client
    pub fn new(mac: MacAddress) -> Self {
        let xid = ((mac.as_bytes()[2] as u32) << 24) |
                  ((mac.as_bytes()[3] as u32) << 16) |
                  ((mac.as_bytes()[4] as u32) << 8) |
                  (mac.as_bytes()[5] as u32);
        
        Self {
            state: DhcpState::Idle,
            mac_address: mac,
            xid,
            assigned_ip: None,
            subnet_mask: None,
            gateway: None,
            dns_servers: Vec::new(),
            server_id: None,
            lease_time: 0,
            lease_start: 0,
            last_activity: 0,
            retry_count: 0,
            timeout: DHCP_TIMEOUT_INITIAL,
        }
    }
    
    /// Start DHCP discovery process
    pub fn start_discovery(&mut self) -> Vec<u8> {
        println!("[dhcp] Starting DHCP discovery...");
        
        self.state = DhcpState::Selecting;
        self.retry_count = 0;
        self.timeout = DHCP_TIMEOUT_INITIAL;
        self.last_activity = elapsed_ms();
        
        self.create_discover()
    }
    
    /// Create DHCP DISCOVER message
    pub fn create_discover(&self) -> Vec<u8> {
        let mut msg = DhcpMessage::new(self.xid, &self.mac_address);
        msg.set_message_type(DHCP_DISCOVER);
        msg.to_bytes()
    }
    
    /// Create DHCP REQUEST message
    pub fn create_request(&self, server_id: Ipv4Address, requested_ip: Ipv4Address) -> Vec<u8> {
        let mut msg = DhcpMessage::new(self.xid, &self.mac_address);
        msg.set_message_type(DHCP_REQUEST);
        msg.siaddr = server_id;
        msg.to_bytes()
    }
    
    /// Check if timeout occurred and retry needed
    pub fn check_timeout(&mut self) -> Option<Vec<u8>> {
        let elapsed = elapsed_ms() - self.last_activity;
        
        if elapsed < self.timeout {
            return None;
        }
        
        self.retry_count += 1;
        
        if self.retry_count >= DHCP_RETRY_COUNT {
            println!("[dhcp] Max retries reached, giving up");
            self.state = DhcpState::Idle;
            return None;
        }
        
        // Exponential backoff
        self.timeout = (self.timeout * 2).min(DHCP_TIMEOUT_MAX);
        self.last_activity = elapsed_ms();
        
        println!("[dhcp] Timeout, retry {}/{}...", self.retry_count, DHCP_RETRY_COUNT);
        
        match self.state {
            DhcpState::Selecting => Some(self.create_discover()),
            DhcpState::Requesting => {
                if let (Some(server), Some(ip)) = (self.server_id, self.assigned_ip) {
                    Some(self.create_request(server, ip))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
    
    /// Check if lease needs renewal
    pub fn check_renewal(&mut self) -> Option<Vec<u8>> {
        if self.state != DhcpState::Bound {
            return None;
        }
        
        let elapsed = (elapsed_ms() - self.lease_start) / 1000;
        
        if elapsed >= self.lease_time as u64 {
            // Lease expired
            println!("[dhcp] Lease expired, restarting discovery...");
            self.state = DhcpState::Idle;
            return Some(self.start_discovery());
        }
        
        None
    }
    
    /// Get configured IP address
    pub fn ip_address(&self) -> Option<Ipv4Address> {
        self.assigned_ip
    }
    
    /// Check if we have a valid lease
    pub fn is_bound(&self) -> bool {
        self.state == DhcpState::Bound
    }
    
    /// Print configuration
    pub fn print_config(&self) {
        println!("[dhcp] Current configuration:");
        println!("  State: {:?}", self.state);
        if let Some(ip) = self.assigned_ip {
            println!("  IP Address: {:?}", ip);
        }
        if let Some(mask) = self.subnet_mask {
            println!("  Subnet Mask: {:?}", mask);
        }
        if let Some(gw) = self.gateway {
            println!("  Gateway: {:?}", gw);
        }
    }
}
