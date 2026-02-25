//! SDPCM Protocol Implementation
//!
//! SDPCM (SDIO Protocol for Control and Management) is Broadcom's protocol
#![allow(dead_code)]

//! for communicating with FullMAC WiFi chips over SDIO.

use alloc::vec::Vec;
use crate::drivers::DriverError;
use crate::println;

// SDPCM header constants
pub const SDPCM_HEADER_LEN: usize = 12;
pub const BDC_HEADER_LEN: usize = 4;
pub const SDPCM_FRAME_LEN_MASK: u16 = 0x7FFF;

// SDPCM channels
pub const SDPCM_CONTROL_CHANNEL: u8 = 0;
pub const SDPCM_EVENT_CHANNEL: u8 = 1;
pub const SDPCM_DATA_CHANNEL: u8 = 2;

// SDPCM event types (Broadcom wireless events)
pub const BRCMF_E_SET_SSID: u16 = 0;        // SSID set
pub const BRCMF_E_JOIN: u16 = 1;            // Join attempt
pub const BRCMF_E_START: u16 = 2;           // BSS started
pub const BRCMF_E_AUTH: u16 = 3;            // Authentication
pub const BRCMF_E_AUTH_IND: u16 = 4;        // Authentication indication
pub const BRCMF_E_DEAUTH: u16 = 5;          // Deauthentication
pub const BRCMF_E_DEAUTH_IND: u16 = 6;      // Deauthentication indication
pub const BRCMF_E_ASSOC: u16 = 7;           // Association
pub const BRCMF_E_ASSOC_IND: u16 = 8;       // Association indication
pub const BRCMF_E_REASSOC: u16 = 9;         // Reassociation
pub const BRCMF_E_REASSOC_IND: u16 = 10;    // Reassociation indication
pub const BRCMF_E_DISASSOC: u16 = 11;       // Disassociation
pub const BRCMF_E_DISASSOC_IND: u16 = 12;   // Disassociation indication
pub const BRCMF_E_LINK: u16 = 16;           // Link up/down
pub const BRCMF_E_MIC_ERROR: u16 = 17;      // MIC error
pub const BRCMF_E_ROAM: u16 = 18;           // Roaming
pub const BRCMF_E_IF: u16 = 54;             // Interface event
pub const BRCMF_E_PSM_WATCHDOG: u16 = 71;   // PSM watchdog

/// SDPCM header (12 bytes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct SdpcmHeader {
    /// Frame length and flags (2 bytes)
    pub frame_len: u16,
    /// Checksum (2 bytes)
    pub checksum: u16,
    /// Sequence number (2 bytes)
    pub sequence: u16,
    /// Channel and flags (1 byte)
    pub channel_flags: u8,
    /// Next data offset (1 byte)
    pub next_offset: u8,
    /// Flow control info (1 byte)
    pub flow_control: u8,
    /// Version (1 byte)
    pub version: u8,
    /// Bus data credit (1 byte)
    pub bus_data_credit: u8,
    /// Reserved (3 bytes)
    pub reserved: [u8; 3],
}

impl SdpcmHeader {
    /// Create a new SDPCM header
    pub fn new(frame_len: u16, channel: u8, sequence: u16) -> Self {
        Self {
            frame_len: frame_len & SDPCM_FRAME_LEN_MASK,
            checksum: 0,
            sequence,
            channel_flags: channel & 0x0F,
            next_offset: 0,
            flow_control: 0,
            version: 0,
            bus_data_credit: 0,
            reserved: [0; 3],
        }
    }
    
    /// Parse SDPCM header from bytes
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < SDPCM_HEADER_LEN {
            return None;
        }
        
        Some(Self {
            frame_len: u16::from_le_bytes([data[0], data[1]]),
            checksum: u16::from_le_bytes([data[2], data[3]]),
            sequence: u16::from_le_bytes([data[4], data[5]]),
            channel_flags: data[6],
            next_offset: data[7],
            flow_control: data[8],
            version: data[9],
            bus_data_credit: data[10],
            reserved: [data[11], 0, 0],  // Only 1 byte in header
        })
    }
    
    /// Serialize header to bytes
    pub fn to_bytes(&self) -> [u8; SDPCM_HEADER_LEN] {
        [
            self.frame_len as u8,
            (self.frame_len >> 8) as u8,
            self.checksum as u8,
            (self.checksum >> 8) as u8,
            self.sequence as u8,
            (self.sequence >> 8) as u8,
            self.channel_flags,
            self.next_offset,
            self.flow_control,
            self.version,
            self.bus_data_credit,
            self.reserved[0],
        ]
    }
    
    /// Get channel from header
    pub fn channel(&self) -> u8 {
        self.channel_flags & 0x0F
    }
    
    /// Check if this is a control channel packet
    pub fn is_control(&self) -> bool {
        self.channel() == SDPCM_CONTROL_CHANNEL
    }
    
    /// Check if this is an event channel packet
    pub fn is_event(&self) -> bool {
        self.channel() == SDPCM_EVENT_CHANNEL
    }
    
    /// Check if this is a data channel packet
    pub fn is_data(&self) -> bool {
        self.channel() == SDPCM_DATA_CHANNEL
    }
}

/// BDC (Broadcom Data Control) header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct BdcHeader {
    /// Flags (1 byte)
    pub flags: u8,
    /// Priority (1 byte)
    pub priority: u8,
    /// Flags2 + data offset (1 byte)
    pub flags2_offset: u8,
    /// Sequence number (1 byte)
    pub sequence: u8,
}

impl BdcHeader {
    /// Create a new BDC header
    pub fn new(priority: u8, sequence: u8) -> Self {
        Self {
            flags: 0,
            priority,
            flags2_offset: 0x02,  // BDC version 2
            sequence,
        }
    }
    
    /// Parse BDC header from bytes
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < BDC_HEADER_LEN {
            return None;
        }
        
        Some(Self {
            flags: data[0],
            priority: data[1],
            flags2_offset: data[2],
            sequence: data[3],
        })
    }
    
    /// Get data offset (in 4-byte words)
    pub fn data_offset(&self) -> usize {
        ((self.flags2_offset >> 4) & 0x0F) as usize * 4
    }
}

/// SDPCM event packet
#[derive(Debug, Clone)]
pub struct SdpcmEvent {
    /// Event type (BRCMF_E_*)
    pub event_type: u16,
    /// Event status
    pub status: u32,
    /// Event reason code
    pub reason: u32,
    /// BSSID (MAC address)
    pub bssid: [u8; 6],
    /// Additional data
    pub data: Vec<u8>,
}

impl SdpcmEvent {
    /// Parse event from SDPCM payload
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 16 {
            return None;
        }
        
        let event_type = u16::from_le_bytes([data[0], data[1]]);
        let status = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        let reason = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
        
        let mut bssid = [0u8; 6];
        if data.len() >= 18 {
            bssid.copy_from_slice(&data[12..18]);
        }
        
        let event_data = if data.len() > 18 {
            data[18..].to_vec()
        } else {
            Vec::new()
        };
        
        Some(Self {
            event_type,
            status,
            reason,
            bssid,
            data: event_data,
        })
    }
    
    /// Get event type as string
    pub fn event_type_str(&self) -> &'static str {
        match self.event_type {
            BRCMF_E_SET_SSID => "SET_SSID",
            BRCMF_E_JOIN => "JOIN",
            BRCMF_E_START => "START",
            BRCMF_E_AUTH => "AUTH",
            BRCMF_E_AUTH_IND => "AUTH_IND",
            BRCMF_E_DEAUTH => "DEAUTH",
            BRCMF_E_DEAUTH_IND => "DEAUTH_IND",
            BRCMF_E_ASSOC => "ASSOC",
            BRCMF_E_ASSOC_IND => "ASSOC_IND",
            BRCMF_E_REASSOC => "REASSOC",
            BRCMF_E_REASSOC_IND => "REASSOC_IND",
            BRCMF_E_DISASSOC => "DISASSOC",
            BRCMF_E_DISASSOC_IND => "DISASSOC_IND",
            BRCMF_E_LINK => "LINK",
            BRCMF_E_MIC_ERROR => "MIC_ERROR",
            BRCMF_E_ROAM => "ROAM",
            BRCMF_E_IF => "IF",
            BRCMF_E_PSM_WATCHDOG => "PSM_WATCHDOG",
            _ => "UNKNOWN",
        }
    }
}

/// SDPCM packet builder
pub struct SdpcmPacketBuilder {
    buffer: Vec<u8>,
    sequence: u16,
}

impl SdpcmPacketBuilder {
    /// Create a new packet builder
    pub fn new(sequence: u16) -> Self {
        Self {
            buffer: Vec::with_capacity(512),
            sequence,
        }
    }
    
    /// Add SDPCM header
    pub fn add_sdpcm_header(&mut self, channel: u8, data_len: usize) {
        let frame_len = (SDPCM_HEADER_LEN + data_len) as u16;
        let header = SdpcmHeader::new(frame_len, channel, self.sequence);
        self.buffer.extend_from_slice(&header.to_bytes());
    }
    
    /// Add BDC header
    pub fn add_bdc_header(&mut self, priority: u8) {
        let header = BdcHeader::new(priority, 0);
        self.buffer.push(header.flags);
        self.buffer.push(header.priority);
        self.buffer.push(header.flags2_offset);
        self.buffer.push(header.sequence);
    }
    
    /// Add raw data
    pub fn add_data(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data);
    }
    
    /// Pad to 4-byte boundary
    pub fn pad(&mut self) {
        while self.buffer.len() % 4 != 0 {
            self.buffer.push(0);
        }
    }
    
    /// Get the built packet
    pub fn finish(self) -> Vec<u8> {
        self.buffer
    }
    
    /// Increment sequence number
    pub fn next_sequence(&mut self) {
        self.sequence = self.sequence.wrapping_add(1);
    }
}

/// Parse received SDPCM packet
pub fn parse_sdpcm_packet(data: &[u8]) -> Result<(SdpcmHeader, Vec<u8>), DriverError> {
    if data.len() < SDPCM_HEADER_LEN {
        return Err(DriverError::IoError);
    }
    
    let header = SdpcmHeader::from_bytes(data)
        .ok_or(DriverError::IoError)?;
    
    let payload_len = (header.frame_len & SDPCM_FRAME_LEN_MASK) as usize;
    
    if payload_len > data.len() {
        return Err(DriverError::IoError);
    }
    
    let payload = data[SDPCM_HEADER_LEN..payload_len].to_vec();
    
    Ok((header, payload))
}

/// Handle SDPCM event
pub fn handle_sdpcm_event(event: &SdpcmEvent) {
    println!("[sdpcm] Event: {} (status={}, reason={})",
             event.event_type_str(), event.status, event.reason);
    
    match event.event_type {
        BRCMF_E_LINK => {
            if event.status == 0 {
                println!("[sdpcm] Link UP");
            } else {
                println!("[sdpcm] Link DOWN");
            }
        }
        BRCMF_E_ASSOC | BRCMF_E_REASSOC => {
            println!("[sdpcm] Associated with network");
        }
        BRCMF_E_DISASSOC | BRCMF_E_DISASSOC_IND => {
            println!("[sdpcm] Disassociated from network");
        }
        BRCMF_E_AUTH => {
            println!("[sdpcm] Authentication complete");
        }
        BRCMF_E_DEAUTH => {
            println!("[sdpcm] Deauthenticated");
        }
        BRCMF_E_ROAM => {
            println!("[sdpcm] Roaming...");
        }
        _ => {
            // Unknown event - just log it
        }
    }
}

/// Event queue for async event handling
use spin::Mutex;
use lazy_static::lazy_static;

lazy_static! {
    static ref EVENT_QUEUE: Mutex<Vec<SdpcmEvent>> = Mutex::new(Vec::new());
}

/// Queue an event for processing
pub fn queue_event(event: SdpcmEvent) {
    EVENT_QUEUE.lock().push(event);
}

/// Process all queued events
pub fn process_events() {
    let mut queue = EVENT_QUEUE.lock();
    
    for event in queue.drain(..) {
        handle_sdpcm_event(&event);
    }
}

/// Check if there are pending events
pub fn has_pending_events() -> bool {
    !EVENT_QUEUE.lock().is_empty()
}

/// Event polling function - call periodically
pub fn poll_events() {
    if has_pending_events() {
        process_events();
    }
}
