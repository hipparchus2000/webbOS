#![allow(dead_code)]

//! DHCP Client with UDP Socket Integration

use crate::net::{Ipv4Address, MacAddress, Port, udp};
use crate::net::dhcp::{DhcpClient, DhcpMessage, DhcpState, DHCP_CLIENT_PORT, DHCP_SERVER_PORT, DHCP_MAGIC_COOKIE, DHCP_OPTION_MESSAGE_TYPE, DHCP_OPTION_END};
use crate::drivers::timer::elapsed_ms;
use crate::println;
use spin::Mutex;
use lazy_static::lazy_static;

/// DHCP client with UDP socket
pub struct DhcpClientSocket {
    pub client: DhcpClient,
    pub local_port: Port,
    pub socket_bound: bool,
    pub last_tx_time: u64,
    pub tx_interval: u64,
    pub max_retries: u32,
    pub retry_count: u32,
}

impl DhcpClientSocket {
    pub fn new(mac: MacAddress) -> Self {
        Self {
            client: DhcpClient::new(mac),
            local_port: Port::new(DHCP_CLIENT_PORT),
            socket_bound: false,
            last_tx_time: 0,
            tx_interval: 4000,
            max_retries: 4,
            retry_count: 0,
        }
    }
    
    pub fn init(&mut self) -> Result<(), ()> {
        udp::bind(self.local_port)?;
        self.socket_bound = true;
        println!("[dhcp_client] Bound to port {}", DHCP_CLIENT_PORT);
        Ok(())
    }
    
    pub fn start(&mut self) -> Result<(), ()> {
        if !self.socket_bound {
            self.init()?;
        }
        let discover = self.client.start_discovery();
        self.send_broadcast(&discover)?;
        self.last_tx_time = elapsed_ms();
        self.retry_count = 0;
        println!("[dhcp_client] DHCP discovery started");
        Ok(())
    }
    
    pub fn poll(&mut self) -> Result<DhcpEvent, ()> {
        if !self.socket_bound {
            return Err(());
        }
        
        let mut buf = [0u8; 1024];
        match udp::recv_from(self.local_port, &mut buf) {
            Ok((src_ip, src_port, len)) => {
                if src_port.as_u16() == DHCP_SERVER_PORT {
                    if let Some(event) = self.process_response(&buf[..len], src_ip) {
                        return Ok(event);
                    }
                }
            }
            Err(()) => {}
        }
        
        let elapsed = elapsed_ms() - self.last_tx_time;
        if elapsed >= self.tx_interval {
            self.handle_timeout()?;
        }
        
        if let Some(_renew) = self.client.check_renewal() {
            let _ = self.send_renewal();
        }
        
        Ok(DhcpEvent::None)
    }
    
    fn process_response(&mut self, data: &[u8], src_ip: Ipv4Address) -> Option<DhcpEvent> {
        let msg = DhcpMessage::from_bytes(data)?;
        if msg.xid != self.client.xid {
            return None;
        }
        if data.len() < 240 || &data[236..240] != &DHCP_MAGIC_COOKIE {
            return None;
        }
        
        let msg_type = self.get_message_type(data)?;
        match msg_type {
            2 => self.handle_offer(msg, src_ip),
            5 => self.handle_ack(msg),
            6 => self.handle_nak(),
            _ => None,
        }
    }
    
    fn handle_offer(&mut self, msg: DhcpMessage, server_ip: Ipv4Address) -> Option<DhcpEvent> {
        println!("[dhcp_client] Offer received: {:?}", msg.yiaddr);
        self.client.server_id = Some(server_ip);
        self.client.assigned_ip = Some(msg.yiaddr);
        
        if let (Some(server), Some(ip)) = (self.client.server_id, self.client.assigned_ip) {
            let request = self.client.create_request(server, ip);
            let _ = self.send_broadcast(&request);
            self.client.state = DhcpState::Requesting;
            self.last_tx_time = elapsed_ms();
            println!("[dhcp_client] Request sent for {:?}", ip);
        }
        Some(DhcpEvent::OfferReceived)
    }
    
    fn handle_ack(&mut self, msg: DhcpMessage) -> Option<DhcpEvent> {
        println!("[dhcp_client] ACK received: {:?}", msg.yiaddr);
        self.client.assigned_ip = Some(msg.yiaddr);
        self.extract_options(&msg);
        self.client.state = DhcpState::Bound;
        self.client.lease_start = elapsed_ms();
        println!("[dhcp_client] Lease acquired");
        Some(DhcpEvent::Bound)
    }
    
    fn handle_nak(&mut self) -> Option<DhcpEvent> {
        println!("[dhcp_client] NAK received");
        self.client.assigned_ip = None;
        self.client.state = DhcpState::Idle;
        let _ = self.start();
        Some(DhcpEvent::NakReceived)
    }
    
    fn extract_options(&mut self, msg: &DhcpMessage) {
        // Extract subnet mask
        if let Some(mask) = self.get_option_u32(&msg.options, 1) {
            self.client.subnet_mask = Some(Ipv4Address::new(mask.to_be_bytes()));
        }
        // Extract router
        if let Some(router) = self.get_option_u32(&msg.options, 3) {
            self.client.gateway = Some(Ipv4Address::new(router.to_be_bytes()));
        }
        // Extract DNS
        if let Some(dns) = self.get_option_u32(&msg.options, 6) {
            self.client.dns_servers.push(Ipv4Address::new(dns.to_be_bytes()));
        }
        // Extract lease time
        if let Some(lease) = self.get_option_u32(&msg.options, 51) {
            self.client.lease_time = lease;
        }
    }
    
    fn get_option_u32(&self, options: &[u8], code: u8) -> Option<u32> {
        let mut i = 0;
        while i < options.len() {
            if options[i] == code {
                let len = options[i + 1] as usize;
                if len == 4 {
                    return Some(u32::from_be_bytes([
                        options[i + 2], options[i + 3],
                        options[i + 4], options[i + 5]
                    ]));
                }
            }
            if options[i] == 255 {
                break;
            }
            if options[i] == 0 {
                i += 1;
                continue;
            }
            let len = options[i + 1] as usize;
            i += 2 + len;
        }
        None
    }
    
    fn get_message_type(&self, data: &[u8]) -> Option<u8> {
        let options = &data[240..];
        let mut i = 0;
        while i < options.len() {
            if options[i] == DHCP_OPTION_MESSAGE_TYPE {
                let len = options[i + 1];
                if len == 1 {
                    return Some(options[i + 2]);
                }
            }
            if options[i] == DHCP_OPTION_END {
                break;
            }
            if options[i] == 0 {
                i += 1;
                continue;
            }
            let len = options[i + 1] as usize;
            i += 2 + len;
        }
        None
    }
    
    fn handle_timeout(&mut self) -> Result<(), ()> {
        self.retry_count += 1;
        if self.retry_count >= self.max_retries {
            self.client.state = DhcpState::Idle;
            return Err(());
        }
        self.tx_interval = (self.tx_interval * 2).min(64000);
        self.last_tx_time = elapsed_ms();
        
        match self.client.state {
            DhcpState::Selecting => {
                let discover = self.client.create_discover();
                self.send_broadcast(&discover)?;
            }
            DhcpState::Requesting => {
                if let (Some(server), Some(ip)) = (self.client.server_id, self.client.assigned_ip) {
                    let request = self.client.create_request(server, ip);
                    self.send_broadcast(&request)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    
    fn send_broadcast(&self, data: &[u8]) -> Result<(), ()> {
        let broadcast_ip = Ipv4Address::new([255, 255, 255, 255]);
        match udp::send_to(self.local_port, broadcast_ip, Port::new(DHCP_SERVER_PORT), data) {
            Ok(_) => Ok(()),
            Err(_) => Err(())
        }
    }
    
    fn send_renewal(&self) -> Result<(), ()> {
        if let (Some(server), Some(ip)) = (self.client.server_id, self.client.assigned_ip) {
            let request = self.client.create_request(server, ip);
            udp::send_to(self.local_port, server, Port::new(DHCP_SERVER_PORT), &request)?;
        }
        Ok(())
    }
    
    pub fn get_config(&self) -> Option<(Ipv4Address, Ipv4Address, Ipv4Address)> {
        if let (Some(ip), Some(mask), Some(gw)) = 
            (self.client.assigned_ip, self.client.subnet_mask, self.client.gateway) {
            Some((ip, mask, gw))
        } else {
            None
        }
    }
    
    pub fn is_bound(&self) -> bool {
        self.client.is_bound()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DhcpEvent {
    None,
    OfferReceived,
    Bound,
    NakReceived,
}

lazy_static! {
    static ref DHCP_CLIENT: Mutex<Option<DhcpClientSocket>> = Mutex::new(None);
}

pub fn init(mac: MacAddress) {
    let client = DhcpClientSocket::new(mac);
    *DHCP_CLIENT.lock() = Some(client);
    println!("[dhcp_client] Initialized");
}

pub fn start() -> Result<(), ()> {
    if let Some(ref mut client) = *DHCP_CLIENT.lock() {
        client.start()
    } else {
        Err(())
    }
}

pub fn poll() -> Result<DhcpEvent, ()> {
    if let Some(ref mut client) = *DHCP_CLIENT.lock() {
        client.poll()
    } else {
        Err(())
    }
}

pub fn get_config() -> Option<(Ipv4Address, Ipv4Address, Ipv4Address)> {
    if let Some(ref client) = *DHCP_CLIENT.lock() {
        client.get_config()
    } else {
        None
    }
}

pub fn is_bound() -> bool {
    if let Some(ref client) = *DHCP_CLIENT.lock() {
        client.is_bound()
    } else {
        false
    }
}
