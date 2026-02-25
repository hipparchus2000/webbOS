//! WiFi Drivers
#![allow(dead_code)]
//!
//! Wireless network device drivers for WebbOS.
//!
//! Currently supported:
//! - BCM43438 (Raspberry Pi 3) via SDIO
//! - BCM43455 (Raspberry Pi 4) via SDIO
//! - SDIO-over-SPI fallback mode

pub mod bcm43438;
pub mod sdio_spi;
pub mod firmware_loader;
pub mod firmware_download;
pub mod sdpcm;
pub mod ioctl;
pub mod wpa2;
pub mod eapol;

use crate::println;

/// Initialize WiFi subsystem
pub fn init(pi4: bool) {
    println!("[drivers/wifi] Initializing WiFi subsystem...");
    
    // Try native SDIO first (preferred for Pi 3/4)
    crate::drivers::sdio::init(pi4);
    
    // If SDIO is available and a card is detected, initialize BCM43438/BCM43455
    if crate::drivers::sdio::is_initialized() {
        println!("[drivers/wifi] SDIO host initialized, probing for WiFi...");
        
        // Try to probe SDIO card
        if crate::drivers::sdio::probe_card().is_ok() {
            // Initialize BCM43438/BCM43455 driver
            bcm43438::init(pi4);
        } else {
            println!("[drivers/wifi] No SDIO card detected");
        }
    } else {
        println!("[drivers/wifi] SDIO host not available, trying SPI fallback...");
        
        // Try SDIO-over-SPI fallback
        sdio_spi::init(pi4);
        
        // If SPI mode is available, initialize WiFi over SPI
        if sdio_spi::is_available() {
            // Note: In SPI mode, we'd need a modified BCM43438 driver
            // that uses the SPI interface instead of native SDIO
            println!("[drivers/wifi] SPI fallback initialized (WiFi driver not yet implemented)");
        }
    }
    
    println!("[drivers/wifi] WiFi subsystem initialization complete");
}

/// WiFi driver error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WiFiError {
    /// Success
    Success = 0,
    /// Device not found
    NotFound = 1,
    /// Initialization failed
    InitFailed = 2,
    /// Firmware load failed
    FirmwareError = 3,
    /// SDIO communication error
    SdioError = 4,
    /// Timeout
    Timeout = 5,
    /// Invalid state
    InvalidState = 6,
    /// Scan failed
    ScanFailed = 7,
    /// Connection failed
    ConnectFailed = 8,
    /// Authentication failed
    AuthFailed = 9,
    /// Unknown error
    Unknown = 255,
}

/// WiFi security types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SecurityType {
    /// Open network (no security)
    Open = 0,
    /// WEP security (legacy, not recommended)
    Wep = 1,
    /// WPA security
    Wpa = 2,
    /// WPA2 security (most common)
    Wpa2 = 3,
    /// WPA3 security (latest)
    Wpa3 = 4,
    /// Enterprise/WPA-EAP
    Enterprise = 5,
    /// Unknown security type
    Unknown = 255,
}

impl SecurityType {
    /// Parse from string (for configuration files)
    pub fn from_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "OPEN" | "NONE" => Self::Open,
            "WEP" => Self::Wep,
            "WPA" => Self::Wpa,
            "WPA2" => Self::Wpa2,
            "WPA3" => Self::Wpa3,
            "ENTERPRISE" | "WPA-EAP" => Self::Enterprise,
            _ => Self::Unknown,
        }
    }
    
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Open => "Open",
            Self::Wep => "WEP",
            Self::Wpa => "WPA",
            Self::Wpa2 => "WPA2",
            Self::Wpa3 => "WPA3",
            Self::Enterprise => "Enterprise",
            Self::Unknown => "Unknown",
        }
    }
    
    /// Check if security requires a password
    pub fn requires_password(&self) -> bool {
        matches!(self, Self::Wep | Self::Wpa | Self::Wpa2 | Self::Wpa3)
    }
}

/// WiFi network scan result
#[derive(Debug, Clone)]
pub struct ScanResult {
    /// BSSID (MAC address of AP)
    pub bssid: [u8; 6],
    /// SSID (network name)
    pub ssid: alloc::vec::Vec<u8>,
    /// Channel number
    pub channel: u8,
    /// Signal strength (RSSI in dBm)
    pub rssi: i8,
    /// Security type
    pub security: SecurityType,
}

impl ScanResult {
    /// Get SSID as string (may contain non-UTF8 bytes)
    pub fn ssid_string(&self) -> alloc::string::String {
        alloc::string::String::from_utf8_lossy(&self.ssid).into_owned()
    }
    
    /// Format BSSID as MAC address string
    pub fn bssid_string(&self) -> [u8; 17] {
        let mut buf = [0u8; 17];
        for i in 0..6 {
            let byte = self.bssid[i];
            buf[i * 3] = hex_nibble(byte >> 4);
            buf[i * 3 + 1] = hex_nibble(byte & 0xF);
            if i < 5 {
                buf[i * 3 + 2] = b':';
            }
        }
        buf
    }
    
    /// Get signal quality (0-100, higher is better)
    pub fn signal_quality(&self) -> u8 {
        // RSSI typically ranges from -100 dBm (weak) to -30 dBm (strong)
        // Map this to 0-100 scale
        if self.rssi >= -30 {
            100
        } else if self.rssi <= -100 {
            0
        } else {
            ((self.rssi as i16 + 100) * 100 / 70) as u8
        }
    }
}

fn hex_nibble(n: u8) -> u8 {
    if n < 10 {
        b'0' + n
    } else {
        b'A' + (n - 10)
    }
}

/// WiFi connection configuration
#[derive(Debug, Clone)]
pub struct WiFiConfig {
    /// SSID (network name)
    pub ssid: alloc::vec::Vec<u8>,
    /// Password (empty for open networks)
    pub password: alloc::vec::Vec<u8>,
    /// Security type
    pub security: SecurityType,
    /// Use static IP (if false, use DHCP)
    pub use_static_ip: bool,
    /// Static IP address
    pub static_ip: Option<crate::net::Ipv4Address>,
    /// Static netmask
    pub static_netmask: Option<crate::net::Ipv4Address>,
    /// Static gateway
    pub static_gateway: Option<crate::net::Ipv4Address>,
    /// Static DNS
    pub static_dns: Option<crate::net::Ipv4Address>,
}

impl WiFiConfig {
    /// Create a new WiFi configuration for an open network
    pub fn open_network(ssid: &[u8]) -> Self {
        Self {
            ssid: ssid.to_vec(),
            password: alloc::vec::Vec::new(),
            security: SecurityType::Open,
            use_static_ip: false,
            static_ip: None,
            static_netmask: None,
            static_gateway: None,
            static_dns: None,
        }
    }
    
    /// Create a new WiFi configuration with password
    pub fn with_password(ssid: &[u8], password: &[u8], security: SecurityType) -> Self {
        Self {
            ssid: ssid.to_vec(),
            password: password.to_vec(),
            security,
            use_static_ip: false,
            static_ip: None,
            static_netmask: None,
            static_gateway: None,
            static_dns: None,
        }
    }
}

/// WiFi manager trait for high-level operations
pub trait WiFiManager: Send + Sync {
    /// Scan for available networks
    fn scan(&self) -> Result<alloc::vec::Vec<ScanResult>, WiFiError>;
    
    /// Connect to a network
    fn connect(&self, config: &WiFiConfig) -> Result<(), WiFiError>;
    
    /// Disconnect from current network
    fn disconnect(&self) -> Result<(), WiFiError>;
    
    /// Get current connection status
    fn is_connected(&self) -> bool;
    
    /// Get current SSID (if connected)
    fn current_ssid(&self) -> Option<alloc::vec::Vec<u8>>;
    
    /// Get signal strength (if connected)
    fn signal_strength(&self) -> Option<i8>;
}

/// WiFi power save modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PowerSaveMode {
    /// No power saving (always on)
    Off = 0,
    /// CAM (Continuously Aware Mode)
    Cam = 1,
    /// PS-Poll (legacy power save)
    PsPoll = 2,
    /// Fast PSP (power save polling)
    FastPsp = 3,
}
