//! EAPOL (EAP over LAN) Processing for WPA2
//!
//! Handles EAPOL frame processing for WPA2 4-way handshake.
//! Integrates with SDIO data path to capture and send EAPOL frames.

use alloc::vec::Vec;
use crate::drivers::wifi::wpa2::{FourWayHandshake, HandshakeState, EAPOL_KEY, EAPOL_KEY_TYPE_RSN};
use crate::drivers::wifi::sdpcm::{SDPCM_DATA_CHANNEL, SdpcmPacketBuilder};
use crate::drivers::wifi::bcm43438::Bcm43438Device;
use crate::net::MacAddress;
use crate::println;

// EAPOL Ethernet type
pub const ETHERNET_TYPE_EAPOL: u16 = 0x888E;

// EAPOL version
pub const EAPOL_VERSION_1: u8 = 1;
pub const EAPOL_VERSION_2: u8 = 2;

// EAPOL packet types
pub const EAPOL_PACKET_TYPE_EAP: u8 = 0;
pub const EAPOL_PACKET_TYPE_START: u8 = 1;
pub const EAPOL_PACKET_TYPE_LOGOFF: u8 = 2;
pub const EAPOL_PACKET_TYPE_KEY: u8 = 3;
pub const EAPOL_PACKET_TYPE_ENCAPSULATED_ASF_ALERT: u8 = 4;

/// EAPOL frame header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct EapolHeader {
    pub version: u8,
    pub packet_type: u8,
    pub body_length: u16,
}

impl EapolHeader {
    /// Create a new EAPOL header
    pub fn new(packet_type: u8, body_length: u16) -> Self {
        Self {
            version: EAPOL_VERSION_1,
            packet_type,
            body_length: body_length.to_be_bytes()[0] as u16 | 
                        ((body_length.to_be_bytes()[1] as u16) << 8),
        }
    }
    
    /// Parse from bytes
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 4 {
            return None;
        }
        
        Some(Self {
            version: data[0],
            packet_type: data[1],
            body_length: u16::from_be_bytes([data[2], data[3]]),
        })
    }
    
    /// Serialize to bytes
    pub fn to_bytes(&self) -> [u8; 4] {
        [
            self.version,
            self.packet_type,
            (self.body_length >> 8) as u8,
            (self.body_length & 0xFF) as u8,
        ]
    }
}

/// Ethernet header for EAPOL frames
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct EthernetHeader {
    pub dst_mac: [u8; 6],
    pub src_mac: [u8; 6],
    pub ethertype: u16,
}

impl EthernetHeader {
    /// Create Ethernet header for EAPOL
    pub fn new_eapol(dst_mac: [u8; 6], src_mac: [u8; 6]) -> Self {
        Self {
            dst_mac,
            src_mac,
            ethertype: ETHERNET_TYPE_EAPOL.to_be(),
        }
    }
    
    /// Parse from bytes
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 14 {
            return None;
        }
        
        let mut dst_mac = [0u8; 6];
        let mut src_mac = [0u8; 6];
        dst_mac.copy_from_slice(&data[0..6]);
        src_mac.copy_from_slice(&data[6..12]);
        
        Some(Self {
            dst_mac,
            src_mac,
            ethertype: u16::from_be_bytes([data[12], data[13]]),
        })
    }
    
    /// Serialize to bytes
    pub fn to_bytes(&self) -> [u8; 14] {
        let mut buf = [0u8; 14];
        buf[0..6].copy_from_slice(&self.dst_mac);
        buf[6..12].copy_from_slice(&self.src_mac);
        buf[12..14].copy_from_slice(&self.ethertype.to_be_bytes());
        buf
    }
    
    /// Check if this is an EAPOL frame
    pub fn is_eapol(&self) -> bool {
        self.ethertype.to_be() == ETHERNET_TYPE_EAPOL
    }
}

/// EAPOL processor for WPA2 handshake
pub struct EapolProcessor {
    /// Our MAC address
    pub sta_mac: [u8; 6],
    /// AP MAC address
    pub ap_mac: [u8; 6],
    /// Current handshake state
    pub handshake: Option<FourWayHandshake>,
    /// Pending EAPOL frame to send
    pub pending_tx: Option<Vec<u8>>,
    /// EAPOL frame received callback
    pub rx_callback: Option<fn(&[u8])>,
}

impl EapolProcessor {
    /// Create new EAPOL processor
    pub fn new(sta_mac: [u8; 6], ap_mac: [u8; 6]) -> Self {
        Self {
            sta_mac,
            ap_mac,
            handshake: None,
            pending_tx: None,
            rx_callback: None,
        }
    }
    
    /// Set WPA2 handshake handler
    pub fn set_handshake(&mut self, handshake: FourWayHandshake) {
        self.handshake = Some(handshake);
    }
    
    /// Process received EAPOL frame
    /// 
    /// Returns true if frame was processed, false otherwise
    pub fn process_rx_frame(&mut self, frame: &[u8]) -> bool {
        // Parse Ethernet header
        let eth_header = match EthernetHeader::from_bytes(frame) {
            Some(h) => h,
            None => return false,
        };
        
        // Check if this is an EAPOL frame
        if !eth_header.is_eapol() {
            return false;
        }
        
        // Check if frame is for us (or broadcast)
        if !self.is_our_frame(&eth_header.dst_mac) {
            return false;
        }
        
        println!("[eapol] Received EAPOL frame from {:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
                 eth_header.src_mac[0], eth_header.src_mac[1], eth_header.src_mac[2],
                 eth_header.src_mac[3], eth_header.src_mac[4], eth_header.src_mac[5]);
        
        // Parse EAPOL header
        let eapol_header = match EapolHeader::from_bytes(&frame[14..]) {
            Some(h) => h,
            None => {
                println!("[eapol] Failed to parse EAPOL header");
                return false;
            }
        };
        
        match eapol_header.packet_type {
            EAPOL_PACKET_TYPE_KEY => {
                // EAPOL-Key frame (WPA2 handshake)
                self.process_eapol_key(&frame[14..])
            }
            EAPOL_PACKET_TYPE_START => {
                println!("[eapol] EAPOL-Start received");
                true
            }
            EAPOL_PACKET_TYPE_LOGOFF => {
                println!("[eapol] EAPOL-Logoff received");
                true
            }
            _ => {
                println!("[eapol] Unknown EAPOL packet type: {}", eapol_header.packet_type);
                false
            }
        }
    }
    
    /// Process EAPOL-Key frame
    fn process_eapol_key(&mut self, data: &[u8]) -> bool {
        println!("[eapol] Processing EAPOL-Key frame");
        
        // Parse EAPOL header
        let eapol_header = match EapolHeader::from_bytes(data) {
            Some(h) => h,
            None => return false,
        };
        
        let body_length = eapol_header.body_length as usize;
        if data.len() < 4 + body_length {
            println!("[eapol] EAPOL body truncated");
            return false;
        }
        
        let eapol_body = &data[4..4 + body_length];
        
        // Check if we have an active handshake
        if let Some(ref mut handshake) = self.handshake {
            // Process the key frame through the handshake
            let response_opt = handshake.process_message(eapol_body);
            let is_complete = handshake.is_complete();
            
            // Build EAPOL response frame (after mutable borrow ends)
            if let Some(response) = response_opt {
                let response_frame = self.build_eapol_key_frame(&response);
                self.pending_tx = Some(response_frame);
                println!("[eapol] Generated EAPOL-Key response");
            }
            
            // Check if handshake is complete
            if is_complete {
                println!("[eapol] WPA2 handshake completed successfully!");
            }
            
            true
        } else {
            println!("[eapol] No active handshake, ignoring EAPOL-Key");
            false
        }
    }
    
    /// Build EAPOL-Key frame for transmission
    fn build_eapol_key_frame(&self, key_data: &[u8]) -> Vec<u8> {
        let mut frame = Vec::with_capacity(14 + 4 + key_data.len());
        
        // Ethernet header
        let eth_header = EthernetHeader::new_eapol(self.ap_mac, self.sta_mac);
        frame.extend_from_slice(&eth_header.to_bytes());
        
        // EAPOL header
        let eapol_header = EapolHeader::new(EAPOL_PACKET_TYPE_KEY, key_data.len() as u16);
        frame.extend_from_slice(&eapol_header.to_bytes());
        
        // EAPOL-Key data
        frame.extend_from_slice(key_data);
        
        frame
    }
    
    /// Check if frame is for us (or broadcast)
    fn is_our_frame(&self, dst_mac: &[u8; 6]) -> bool {
        // Check for broadcast
        if dst_mac == &[0xFF; 6] {
            return true;
        }
        
        // Check if it's our MAC
        dst_mac == &self.sta_mac
    }
    
    /// Get pending TX frame (if any)
    pub fn get_pending_tx(&mut self) -> Option<Vec<u8>> {
        self.pending_tx.take()
    }
    
    /// Check if we have a pending TX frame
    pub fn has_pending_tx(&self) -> bool {
        self.pending_tx.is_some()
    }
    
    /// Start EAPOL handshake (send EAPOL-Start)
    pub fn send_eapol_start(&mut self) -> Vec<u8> {
        println!("[eapol] Sending EAPOL-Start");
        
        let mut frame = Vec::with_capacity(14 + 4);
        
        // Ethernet header
        let eth_header = EthernetHeader::new_eapol(self.ap_mac, self.sta_mac);
        frame.extend_from_slice(&eth_header.to_bytes());
        
        // EAPOL header (no body for EAPOL-Start)
        let eapol_header = EapolHeader::new(EAPOL_PACKET_TYPE_START, 0);
        frame.extend_from_slice(&eapol_header.to_bytes());
        
        frame
    }
    
    /// Check if handshake is complete
    pub fn is_handshake_complete(&self) -> bool {
        self.handshake.as_ref().map_or(false, |h| h.is_complete())
    }
    
    /// Get temporal key (if handshake complete)
    pub fn get_temporal_key(&self) -> Option<&[u8]> {
        self.handshake.as_ref().and_then(|h| {
            if h.is_complete() {
                Some(h.get_temporal_key())
            } else {
                None
            }
        })
    }
}

/// Global EAPOL processor instance
use spin::Mutex;
use lazy_static::lazy_static;

lazy_static! {
    static ref EAPOL_PROCESSOR: Mutex<Option<EapolProcessor>> = Mutex::new(None);
}

/// Initialize EAPOL processor
pub fn init(sta_mac: [u8; 6], ap_mac: [u8; 6]) {
    let processor = EapolProcessor::new(sta_mac, ap_mac);
    *EAPOL_PROCESSOR.lock() = Some(processor);
    println!("[eapol] EAPOL processor initialized");
}

/// Get EAPOL processor
pub fn processor() -> Option<spin::MutexGuard<'static, Option<EapolProcessor>>> {
    Some(EAPOL_PROCESSOR.lock())
}

/// Process received frame (check if it's EAPOL)
/// 
/// Returns true if frame was EAPOL and processed
pub fn process_rx_frame(frame: &[u8]) -> bool {
    if let Some(ref mut processor) = *EAPOL_PROCESSOR.lock() {
        processor.process_rx_frame(frame)
    } else {
        false
    }
}

/// Check if there's a pending EAPOL frame to send
pub fn has_pending_tx() -> bool {
    if let Some(ref processor) = *EAPOL_PROCESSOR.lock() {
        processor.has_pending_tx()
    } else {
        false
    }
}

/// Get pending TX frame
pub fn get_pending_tx() -> Option<Vec<u8>> {
    if let Some(ref mut processor) = *EAPOL_PROCESSOR.lock() {
        processor.get_pending_tx()
    } else {
        None
    }
}

/// Set WPA2 handshake
pub fn set_handshake(handshake: FourWayHandshake) {
    if let Some(ref mut processor) = *EAPOL_PROCESSOR.lock() {
        processor.set_handshake(handshake);
    }
}

/// Check if WPA2 handshake is complete
pub fn is_handshake_complete() -> bool {
    if let Some(ref processor) = *EAPOL_PROCESSOR.lock() {
        processor.is_handshake_complete()
    } else {
        false
    }
}

/// Send EAPOL-Start
pub fn send_eapol_start() -> Option<Vec<u8>> {
    if let Some(ref mut processor) = *EAPOL_PROCESSOR.lock() {
        Some(processor.send_eapol_start())
    } else {
        None
    }
}

/// EAPOL receive callback type
pub type EapolRxCallback = fn(&[u8]);

/// Set receive callback
pub fn set_rx_callback(callback: EapolRxCallback) {
    if let Some(ref mut processor) = *EAPOL_PROCESSOR.lock() {
        processor.rx_callback = Some(callback);
    }
}
