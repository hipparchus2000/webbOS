//! IOCTL Interface for BCM43438/BCM43455
//!
//! IOCTL (Input/Output Control) commands are used to configure the WiFi chip
#![allow(dead_code)]

//! and retrieve information like scan results, connection status, etc.

use alloc::vec::Vec;
use alloc::string::String;
use crate::println;

// IOCTL command codes (Broadcom definitions)
pub const BRCMF_C_GET_VERSION: u32 = 1;
pub const BRCMF_C_UP: u32 = 2;
pub const BRCMF_C_DOWN: u32 = 3;
pub const BRCMF_C_SET_PROMISC: u32 = 9;
pub const BRCMF_C_GET_RATE: u32 = 12;
pub const BRCMF_C_GET_INFRA: u32 = 19;
pub const BRCMF_C_SET_INFRA: u32 = 20;
pub const BRCMF_C_GET_AUTH: u32 = 21;
pub const BRCMF_C_SET_AUTH: u32 = 22;
pub const BRCMF_C_GET_BSSID: u32 = 23;
pub const BRCMF_C_GET_SSID: u32 = 25;
pub const BRCMF_C_SET_SSID: u32 = 26;
pub const BRCMF_C_GET_CHANNEL: u32 = 29;
pub const BRCMF_C_SET_CHANNEL: u32 = 30;
pub const BRCMF_C_GET_SRL: u32 = 31;
pub const BRCMF_C_GET_LRL: u32 = 33;
pub const BRCMF_C_GET_RADIO: u32 = 37;
pub const BRCMF_C_SET_RADIO: u32 = 38;
pub const BRCMF_C_GET_PHYTYPE: u32 = 39;
pub const BRCMF_C_GET_CURR_RATESET: u32 = 114;
pub const BRCMF_C_GET_AP: u32 = 117;
pub const BRCMF_C_SET_AP: u32 = 118;
pub const BRCMF_C_SET_SCAN_CHANNEL_TIME: u32 = 124;
pub const BRCMF_C_SET_SCAN_UNASSOC_TIME: u32 = 126;
pub const BRCMF_C_SET_SCAN_PASSIVE_TIME: u32 = 127;
pub const BRCMF_C_SCAN: u32 = 129;
pub const BRCMF_C_SCAN_RESULTS: u32 = 130;
pub const BRCMF_C_DISASSOC: u32 = 136;
pub const BRCMF_C_SET_ROAM_TRIGGER: u32 = 137;
pub const BRCMF_C_SET_ROAM_DELTA: u32 = 139;
pub const BRCMF_C_GET_BCNPRD: u32 = 142;
pub const BRCMF_C_SET_BCNPRD: u32 = 143;
pub const BRCMF_C_GET_DTIMPRD: u32 = 144;
pub const BRCMF_C_SET_DTIMPRD: u32 = 145;
pub const BRCMF_C_SET_COUNTRY: u32 = 179;
pub const BRCMF_C_GET_PHYLIST: u32 = 231;
pub const BRCMF_C_GET_BAND: u32 = 242;
pub const BRCMF_C_SET_BAND: u32 = 243;
pub const BRCMF_C_GET_ASSOC_INFO: u32 = 301;
pub const BRCMF_C_GET_ASSOC_LIST: u32 = 302;
pub const BRCMF_C_GET_VAR: u32 = 262;
pub const BRCMF_C_SET_VAR: u32 = 263;

// IOCTL response status
pub const BRCMF_E_STATUS_SUCCESS: u32 = 0;
pub const BRCMF_E_STATUS_FAIL: u32 = 1;
pub const BRCMF_E_STATUS_TIMEOUT: u32 = 2;
pub const BRCMF_E_STATUS_NO_NETWORKS: u32 = 4;
pub const BRCMF_E_STATUS_ABORT: u32 = 6;
pub const BRCMF_E_STATUS_NO_ACK: u32 = 9;
pub const BRCMF_E_STATUS_UNSOLICITED: u32 = 10;
pub const BRCMF_E_STATUS_ATTEMPT: u32 = 11;

/// IOCTL request header
#[derive(Debug, Clone)]
pub struct IoctlRequest {
    /// Command code
    pub cmd: u32,
    /// Interface index
    pub ifidx: u32,
    /// Transaction ID
    pub trans_id: u32,
    /// Input data length
    pub in_len: u32,
    /// Output data length
    pub out_len: u32,
    /// Data (variable length)
    pub data: Vec<u8>,
}

impl IoctlRequest {
    /// Create a new IOCTL request
    pub fn new(cmd: u32, ifidx: u32, trans_id: u32) -> Self {
        Self {
            cmd,
            ifidx,
            trans_id,
            in_len: 0,
            out_len: 0,
            data: Vec::new(),
        }
    }
    
    /// Set input data
    pub fn with_input(mut self, data: &[u8]) -> Self {
        self.data = data.to_vec();
        self.in_len = data.len() as u32;
        self
    }
    
    /// Set expected output length
    pub fn with_output_len(mut self, len: u32) -> Self {
        self.out_len = len;
        self
    }
    
    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut result = Vec::with_capacity(20 + self.data.len());
        
        result.extend_from_slice(&self.cmd.to_le_bytes());
        result.extend_from_slice(&self.ifidx.to_le_bytes());
        result.extend_from_slice(&self.trans_id.to_le_bytes());
        result.extend_from_slice(&self.in_len.to_le_bytes());
        result.extend_from_slice(&self.out_len.to_le_bytes());
        result.extend_from_slice(&self.data);
        
        result
    }
}

/// IOCTL response
#[derive(Debug, Clone)]
pub struct IoctlResponse {
    /// Command code
    pub cmd: u32,
    /// Transaction ID
    pub trans_id: u32,
    /// Status code
    pub status: u32,
    /// Response data
    pub data: Vec<u8>,
}

impl IoctlResponse {
    /// Parse response from bytes
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 12 {
            return None;
        }
        
        let cmd = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let trans_id = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        let status = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
        
        let resp_data = if data.len() > 12 {
            data[12..].to_vec()
        } else {
            Vec::new()
        };
        
        Some(Self {
            cmd,
            trans_id,
            status,
            data: resp_data,
        })
    }
    
    /// Check if response indicates success
    pub fn is_success(&self) -> bool {
        self.status == BRCMF_E_STATUS_SUCCESS
    }
    
    /// Get status as string
    pub fn status_str(&self) -> &'static str {
        match self.status {
            BRCMF_E_STATUS_SUCCESS => "SUCCESS",
            BRCMF_E_STATUS_FAIL => "FAIL",
            BRCMF_E_STATUS_TIMEOUT => "TIMEOUT",
            BRCMF_E_STATUS_NO_NETWORKS => "NO_NETWORKS",
            BRCMF_E_STATUS_ABORT => "ABORT",
            BRCMF_E_STATUS_NO_ACK => "NO_ACK",
            BRCMF_E_STATUS_UNSOLICITED => "UNSOLICITED",
            BRCMF_E_STATUS_ATTEMPT => "ATTEMPT",
            _ => "UNKNOWN",
        }
    }
}

/// Scan parameters
#[derive(Debug, Clone, Copy)]
pub struct ScanParams {
    pub version: u32,
    pub action: u16,
    pub sync_id: u16,
    pub ssid_len: u32,
    pub ssid: [u8; 32],
    pub bssid: [u8; 6],
    pub bss_type: u8,
    pub scan_type: u8,
    pub nprobes: i32,
    pub active_time: i32,
    pub passive_time: i32,
    pub home_time: i32,
    pub channel_num: u16,
    pub channel_list: [u16; 1],  // Variable length
}

impl ScanParams {
    /// Create default scan parameters
    pub fn default() -> Self {
        Self {
            version: 1,
            action: 1,  // SCAN_ACTION_START
            sync_id: 0,
            ssid_len: 0,
            ssid: [0; 32],
            bssid: [0xFF; 6],  // Broadcast
            bss_type: 2,  // BSS_INFRASTRUCTURE
            scan_type: 1,  // SCAN_TYPE_ACTIVE
            nprobes: -1,
            active_time: -1,
            passive_time: -1,
            home_time: -1,
            channel_num: 0,
            channel_list: [0],
        }
    }
    
    /// Set SSID for directed scan
    pub fn with_ssid(mut self, ssid: &[u8]) -> Self {
        self.ssid_len = ssid.len() as u32;
        self.ssid[..ssid.len()].copy_from_slice(ssid);
        self
    }
    
    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut result = Vec::with_capacity(core::mem::size_of::<Self>());
        
        result.extend_from_slice(&self.version.to_le_bytes());
        result.extend_from_slice(&self.action.to_le_bytes());
        result.extend_from_slice(&self.sync_id.to_le_bytes());
        result.extend_from_slice(&self.ssid_len.to_le_bytes());
        result.extend_from_slice(&self.ssid);
        result.extend_from_slice(&self.bssid);
        result.push(self.bss_type);
        result.push(self.scan_type);
        result.extend_from_slice(&self.nprobes.to_le_bytes());
        result.extend_from_slice(&self.active_time.to_le_bytes());
        result.extend_from_slice(&self.passive_time.to_le_bytes());
        result.extend_from_slice(&self.home_time.to_le_bytes());
        result.extend_from_slice(&self.channel_num.to_le_bytes());
        // channel_list is u16, convert to bytes
        for ch in self.channel_list.iter() {
            result.extend_from_slice(&ch.to_le_bytes());
        }
        
        result
    }
}

/// Scan result entry
#[derive(Debug, Clone)]
pub struct ScanResult {
    /// SSID length
    pub ssid_len: u32,
    /// SSID bytes
    pub ssid: [u8; 32],
    /// BSSID
    pub bssid: [u8; 6],
    /// Channel
    pub channel: u16,
    /// RSSI (signal strength)
    pub rssi: i16,
    /// SNR
    pub snr: i16,
    /// Noise
    pub noise: i16,
    /// Beacon period
    pub beacon_period: u16,
    /// Capability
    pub capability: u16,
    /// Security type
    pub security: u32,
}

impl ScanResult {
    /// Parse scan result from bytes
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 64 {
            return None;
        }
        
        let ssid_len = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        
        let mut ssid = [0u8; 32];
        ssid.copy_from_slice(&data[4..36]);
        
        let mut bssid = [0u8; 6];
        bssid.copy_from_slice(&data[36..42]);
        
        let channel = u16::from_le_bytes([data[44], data[45]]);
        let rssi = i16::from_le_bytes([data[46], data[47]]);
        let snr = i16::from_le_bytes([data[48], data[49]]);
        let noise = i16::from_le_bytes([data[50], data[51]]);
        let beacon_period = u16::from_le_bytes([data[52], data[53]]);
        let capability = u16::from_le_bytes([data[54], data[55]]);
        
        Some(Self {
            ssid_len,
            ssid,
            bssid,
            channel,
            rssi,
            snr,
            noise,
            beacon_period,
            capability,
            security: 0,
        })
    }
    
    /// Get SSID as string
    pub fn ssid_string(&self) -> String {
        if self.ssid_len > 32 {
            return String::new();
        }
        String::from_utf8_lossy(&self.ssid[..self.ssid_len as usize]).into_owned()
    }
    
    /// Format BSSID as MAC address string
    pub fn bssid_string(&self) -> [u8; 17] {
        let mut result = [0u8; 17];
        for i in 0..6 {
            let byte = self.bssid[i];
            result[i * 3] = hex_nibble(byte >> 4);
            result[i * 3 + 1] = hex_nibble(byte & 0xF);
            if i < 5 {
                result[i * 3 + 2] = b':';
            }
        }
        result
    }
    
    /// Check if network is open (no security)
    pub fn is_open(&self) -> bool {
        (self.capability & 0x0010) == 0  // No privacy bit
    }
    
    /// Check if network uses WPA/WPA2
    pub fn is_wpa(&self) -> bool {
        (self.capability & 0x0010) != 0  // Privacy bit set
    }
    
    /// Get signal quality (0-100)
    pub fn signal_quality(&self) -> u8 {
        // RSSI typically ranges from -100 dBm (weak) to -30 dBm (strong)
        if self.rssi >= -30 {
            100
        } else if self.rssi <= -100 {
            0
        } else {
            ((self.rssi as i16 + 100) * 100 / 70) as u8
        }
    }
}

/// Parse scan results from IOCTL response
pub fn parse_scan_results(data: &[u8]) -> Vec<ScanResult> {
    let mut results = Vec::new();
    let mut offset = 0;
    
    // Skip header if present
    if data.len() >= 4 {
        let count = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        println!("[ioctl] Scan results count: {}", count);
        offset = 4;
    }
    
    // Parse each result entry
    while offset + 64 <= data.len() {
        if let Some(result) = ScanResult::from_bytes(&data[offset..]) {
            results.push(result);
            offset += 64;  // Move to next entry
        } else {
            break;
        }
    }
    
    results
}

/// SSID configuration for connection
#[repr(C, packed)]
pub struct SsidConfig {
    pub len: u32,
    pub ssid: [u8; 32],
}

impl SsidConfig {
    /// Create SSID configuration
    pub fn new(ssid: &[u8]) -> Self {
        let mut config = Self {
            len: ssid.len() as u32,
            ssid: [0; 32],
        };
        config.ssid[..ssid.len()].copy_from_slice(ssid);
        config
    }
    
    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut result = Vec::with_capacity(36);
        result.extend_from_slice(&self.len.to_le_bytes());
        result.extend_from_slice(&self.ssid);
        result
    }
}

/// Get IOCTL command name
pub fn ioctl_name(cmd: u32) -> &'static str {
    match cmd {
        BRCMF_C_GET_VERSION => "GET_VERSION",
        BRCMF_C_UP => "UP",
        BRCMF_C_DOWN => "DOWN",
        BRCMF_C_GET_RATE => "GET_RATE",
        BRCMF_C_GET_INFRA => "GET_INFRA",
        BRCMF_C_SET_INFRA => "SET_INFRA",
        BRCMF_C_GET_AUTH => "GET_AUTH",
        BRCMF_C_SET_AUTH => "SET_AUTH",
        BRCMF_C_GET_BSSID => "GET_BSSID",
        BRCMF_C_GET_SSID => "GET_SSID",
        BRCMF_C_SET_SSID => "SET_SSID",
        BRCMF_C_GET_CHANNEL => "GET_CHANNEL",
        BRCMF_C_SET_CHANNEL => "SET_CHANNEL",
        BRCMF_C_SCAN => "SCAN",
        BRCMF_C_SCAN_RESULTS => "SCAN_RESULTS",
        BRCMF_C_DISASSOC => "DISASSOC",
        BRCMF_C_SET_COUNTRY => "SET_COUNTRY",
        BRCMF_C_GET_VAR => "GET_VAR",
        BRCMF_C_SET_VAR => "SET_VAR",
        _ => "UNKNOWN",
    }
}

fn hex_nibble(n: u8) -> u8 {
    if n < 10 {
        b'0' + n
    } else {
        b'A' + (n - 10)
    }
}

/// Print scan results nicely
pub fn print_scan_results(results: &[ScanResult]) {
    println!("┌────┬──────────────────────────────┬───────────┬───────┬────────────┐");
    println!("│ ## │ SSID                         │ BSSID     │ Ch    │ RSSI       │");
    println!("├────┼──────────────────────────────┼───────────┼───────┼────────────┤");
    
    for (i, result) in results.iter().enumerate() {
        let ssid = result.ssid_string();
        let bssid = result.bssid_string();
        let security = if result.is_open() { "Open" } else { "WPA" };
        
        // Get last part of BSSID (last 2 bytes)
        let bssid_short = unsafe { 
            core::str::from_utf8_unchecked(&bssid[9..17]) 
        };
        
        // Copy values to avoid packed field issues
        let channel = result.channel;
        let rssi = result.rssi;
        
        println!("│ {:2} │ {:28} │ {} │ {:3} │ {:4} dBm {} │",
                 i,
                 if ssid.len() > 28 { &ssid[..28] } else { &ssid },
                 bssid_short,
                 channel,
                 rssi,
                 security);
    }
    
    if results.is_empty() {
        println!("│    No networks found                                          │");
    }
    
    println!("└────┴──────────────────────────────┴───────────┴───────┴────────────┘");
}
