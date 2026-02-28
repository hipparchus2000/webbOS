#![allow(dead_code)]

//! DHCP Client with UDP Socket Integration

use crate::net::{Ipv4Address, MacAddress, Port, udp};
use crate::net::dhcp::{DhcpClient, DhcpMessage, DhcpState, DHCP_CLIENT_PORT, DHCP_SERVER_PORT, DHCP_MAGIC_COOKIE, DHCP_OPTION_MESSAGE_TYPE, DHCP_OPTION_END};
use crate::drivers::timer::elapsed_ms;
use crate::println;

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

/// DHCP client events
#[derive(Debug)]
pub enum DhcpEvent {
    None,
    Bound(Ipv4Address, Ipv4Address, Ipv4Address), // ip, netmask, gateway
    OfferReceived,
    NakReceived,
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
        match udp::receive_from(self.local_port, &mut buf) {
            Some((src_ip, src_port, len)) => {
                if src_port.as_u16() == DHCP_SERVER_PORT {
                    if let Some(event) = self.process_response(&buf[..len], src_ip) {
                        return Ok(event);
                    }
                }
            }
            None => {}
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
    
    pub fn get_config(&self) -> Option<(Ipv4Address, Ipv4Address, Ipv4Address)> {
        self.client.get_config()
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
    
    fn get_message_type(&self, data: &[u8]) -> Option<u8> {
        let mut offset = 240;
        while offset < data.len() {
            let opt = data[offset];
            if opt == 0xFF {
                break;
            }
            if opt == DHCP_OPTION_MESSAGE_TYPE {
                return Some(data[offset + 2]);
            }
            let len = data[offset + 1] as usize;
            offset += 2 + len;
        }
        None
    }
    
    fn handle_offer(&mut self, _msg: DhcpMessage, server_ip: Ipv4Address) -> Option<DhcpEvent> {
        self.client.server_id = Some(server_ip);
        let request = self.client.send_request();
        let _ = self.send_broadcast(&request);
        println!("[dhcp_client] DHCP offer received, sending request");
        Some(DhcpEvent::OfferReceived)
    }
    
    fn handle_ack(&mut self, msg: DhcpMessage) -> Option<DhcpEvent> {
        self.client.handle_ack(&msg)?;
        println!("[dhcp_client] DHCP bound, IP: {:?}", self.client.ip_address());
        Some(DhcpEvent::Bound(
            self.client.assigned_ip?,
            self.client.subnet_mask?,
            self.client.gateway?,
        ))
    }
    
    fn handle_nak(&mut self) -> Option<DhcpEvent> {
        self.client.state = DhcpState::Idle;
        println!("[dhcp_client] DHCP NAK received");
        Some(DhcpEvent::NakReceived)
    }
    
    fn handle_timeout(&mut self) -> Result<(), ()> {
        self.retry_count += 1;
        if self.retry_count > self.max_retries {
            return Err(());
        }
        
        match self.client.state {
            DhcpState::Selecting => {
                let discover = self.client.start_discovery();
                self.send_broadcast(&discover)?;
                self.last_tx_time = elapsed_ms();
            }
            _ => {}
        }
        
        Ok(())
    }
    
    fn send_broadcast(&self, data: &[u8]) -> Result<(), ()> {
        let _ = udp::send_to(self.local_port, Ipv4Address::broadcast(), Port::new(DHCP_SERVER_PORT), data);
        Ok(())
    }
    
    fn send_renewal(&mut self) -> Result<(), ()> {
        let renew = self.client.send_renewal();
        self.send_broadcast(&renew)?;
        self.last_tx_time = elapsed_ms();
        Ok(())
    }
}
