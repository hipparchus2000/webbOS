//! BCM43438 / BCM43455 WiFi Driver
//!
//! Driver for the Broadcom/Cypress BCM43438 (Pi 3) and BCM43455 (Pi 4)
//! wireless chips connected via SDIO.
//!
//! The BCM4343x and BCM4345x chips use the FullMAC architecture where
//! much of the 802.11 processing is handled by firmware running on the chip.
//!
//! Firmware files (loaded from /lib/firmware/brcm/):
//! - brcmfmac43430-sdio.bin (Pi 3 firmware binary)
//! - brcmfmac43430-sdio.clm_blob (Pi 3 calibration data)
//! - brcmfmac43430-sdio.txt (Pi 3 NVRAM config)
//! - brcmfmac43455-sdio.bin (Pi 4 firmware binary)
//! - brcmfmac43455-sdio.clm_blob (Pi 4 calibration data)
//! - brcmfmac43455-sdio.txt (Pi 4 NVRAM config)

use crate::drivers::DriverError;
use crate::drivers::sdio::SdioFunction;
use crate::net::{MacAddress, NetworkInterface, NetError};
use crate::net;
use crate::println;
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

// Import submodules


// SDIO Function numbers for BCM4343x
const SDIO_FUNC_BUS: u8 = 0;      // SDIO bus (function 0)
const SDIO_FUNC_BACKPLANE: u8 = 1; // Backplane (core register access)
const SDIO_FUNC_WLAN: u8 = 2;      // WLAN data
const SDIO_FUNC_BT: u8 = 3;        // Bluetooth (if present)

// Backplane addresses
const CHIP_COMMON_BASE: u32 = 0x18000000;
const SDIO_BASE: u32 = 0x18002000;
const WLAN_BASE: u32 = 0x18001000;

// SDIO bus registers (Function 0)
const SDIO_CCCR_IOEN: u32 = 0x02;
const SDIO_CCCR_IORDY: u32 = 0x03;
const SDIO_CCCR_INTEN: u32 = 0x04;
const SDIO_CCCR_BLKSIZE_L: u32 = 0x10;
const SDIO_CCCR_BLKSIZE_H: u32 = 0x11;

// SDIO core registers
const SDIO_INT_STATUS: u32 = 0x20;
const SDIO_INT_HOST_MASK: u32 = 0x24;
const SDIO_FUNCTION_INT_MASK: u32 = 0x34;
const SDIO_FRAME_CTRL: u32 = 0x100;
const SDIO_CHIP_CLOCK_CSR: u32 = 0x1000;

// ChipCommon registers
const CHIP_COMMON_CHIPID: u32 = 0x00;

// SDPCM header constants
const SDPCM_HEADER_LEN: usize = 12;
const BDC_HEADER_LEN: usize = 4;
const SDPCM_FRAME_LEN_MASK: u16 = 0x7FFF;

// Channel types
const SDPCM_CONTROL_CHANNEL: u8 = 0;
const SDPCM_EVENT_CHANNEL: u8 = 1;
const SDPCM_DATA_CHANNEL: u8 = 2;

use crate::drivers::wifi::firmware_download;
use crate::drivers::wifi::sdpcm;
use crate::drivers::wifi::ioctl;
use crate::drivers::wifi::wpa2::{FourWayHandshake, HandshakeState};
use crate::drivers::wifi::eapol;
use crate::net::dhcp_client::{DhcpClientSocket, DhcpEvent};
use crate::net::Ipv4Address;

// Firmware file paths
const FIRMWARE_PATH_PI3: &str = "/lib/firmware/brcm/brcmfmac43430-sdio.bin";
const NVRAM_PATH_PI3: &str = "/lib/firmware/brcm/brcmfmac43430-sdio.txt";
const CLM_PATH_PI3: &str = "/lib/firmware/brcm/brcmfmac43430-sdio.clm_blob";

const FIRMWARE_PATH_PI4: &str = "/lib/firmware/brcm/brcmfmac43455-sdio.bin";
const NVRAM_PATH_PI4: &str = "/lib/firmware/brcm/brcmfmac43455-sdio.txt";
const CLM_PATH_PI4: &str = "/lib/firmware/brcm/brcmfmac43455-sdio.clm_blob";

/// BCM4343x chip IDs
const CHIP_ID_BCM43438: u32 = 0x43430;
const CHIP_ID_BCM43455: u32 = 0x43455;
const CHIP_ID_BCM43456: u32 = 0x43456;

/// Firmware state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FirmwareState {
    Uninitialized = 0,
    Downloading = 1,
    DownloadDone = 2,
    Ready = 3,
}

/// WiFi connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ConnectionState {
    Disconnected = 0,
    Scanning = 1,
    Authenticating = 2,
    Associating = 3,
    Connected = 4,
}

/// SDPCM header (12 bytes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct SdpcmHeader {
    /// Frame length and flags
    frame_len: u16,
    /// Checksum (optional)
    checksum: u16,
    /// Sequence number
    sequence: u16,
    /// Channel and flags
    channel_flags: u8,
    /// Next data offset
    next_offset: u8,
    /// Flow control info
    flow_control: u8,
    /// Version
    version: u8,
    /// Bus data credit
    bus_data_credit: u8,
    /// Reserved
    reserved: [u8; 3],
}

impl SdpcmHeader {
    /// Create a new SDPCM header
    fn new(len: u16, channel: u8, sequence: u16) -> Self {
        Self {
            frame_len: len,
            checksum: 0,
            sequence,
            channel_flags: channel,
            next_offset: 0,
            flow_control: 0,
            version: 0x02, // SDPCM version 2
            bus_data_credit: 0,
            reserved: [0; 3],
        }
    }

    /// Parse header from bytes
    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 12 {
            return None;
        }
        Some(Self {
            frame_len: u16::from_le_bytes([bytes[0], bytes[1]]),
            checksum: u16::from_le_bytes([bytes[2], bytes[3]]),
            sequence: u16::from_le_bytes([bytes[4], bytes[5]]),
            channel_flags: bytes[6],
            next_offset: bytes[7],
            flow_control: bytes[8],
            version: bytes[9],
            bus_data_credit: bytes[10],
            reserved: [bytes[11], bytes[11], bytes[11]], // Simplified
        })
    }
}

/// BDC header (4 bytes minimum)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct BdcHeader {
    /// Flags and priority
    flags: u8,
    /// Priority
    priority: u8,
    /// Flags2
    flags2: u8,
    /// Data offset
    data_offset: u8,
}

impl BdcHeader {
    fn new(priority: u8) -> Self {
        Self {
            flags: 0x20, // BDC version 2
            priority,
            flags2: 0,
            data_offset: 0,
        }
    }
}

/// IOCTL request header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct IoctlRequest {
    /// Command
    cmd: u32,
    /// Transaction ID
    trans_id: u32,
    /// Input buffer length
    input_len: u32,
    /// Output buffer length
    output_len: u32,
    /// Flags
    flags: u16,
    /// Status
    status: u16,
    /// Reserved
    reserved: [u32; 2],
}

impl IoctlRequest {
    fn new(cmd: u32, trans_id: u32, in_len: u32, out_len: u32) -> Self {
        Self {
            cmd,
            trans_id,
            input_len: in_len,
            output_len: out_len,
            flags: 2, // Set
            status: 0,
            reserved: [0; 2],
        }
    }
}

/// Scan result entry
#[derive(Debug, Clone)]
pub struct ScanResult {
    pub bssid: [u8; 6],
    pub ssid: Vec<u8>,
    pub channel: u8,
    pub rssi: i8,
    pub security: SecurityType,
}

/// Security type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SecurityType {
    Open = 0,
    Wep = 1,
    Wpa = 2,
    Wpa2 = 3,
    Wpa3 = 4,
    Unknown = 255,
}

/// BCM43438/BCM43455 WiFi device
pub struct Bcm43438Device {
    chip_id: u32,
    is_pi4: bool,
    mac_address: MacAddress,
    firmware_state: Mutex<FirmwareState>,
    connection_state: Mutex<ConnectionState>,
    sequence_number: Mutex<u16>,
    ioctl_trans_id: Mutex<u32>,
    link_up: AtomicBool,
    rx_buffer: Mutex<Vec<u8>>,
    tx_buffer: Mutex<Vec<u8>>,
    /// Current SSID
    current_ssid: Mutex<Vec<u8>>,
    /// Firmware data (if loaded)
    firmware_data: Mutex<Option<Vec<u8>>>,
    /// NVRAM data
    nvram_data: Mutex<Option<Vec<u8>>>,
    /// WPA2 handshake state
    wpa2_handshake: Mutex<Option<FourWayHandshake>>,
    /// DHCP client for IP configuration
    dhcp_client: Mutex<Option<DhcpClientSocket>>,
    /// Current IP configuration
    ip_address: Mutex<Option<Ipv4Address>>,
    subnet_mask: Mutex<Option<Ipv4Address>>,
    gateway: Mutex<Option<Ipv4Address>>,
}

impl Bcm43438Device {
    /// Create a new BCM43438/BCM43455 device instance
    pub fn new(is_pi4: bool) -> Result<Self, DriverError> {
        println!("[bcm43438] Creating WiFi device (Pi 4: {})", is_pi4);
        
        Ok(Self {
            chip_id: 0,
            is_pi4,
            mac_address: MacAddress::new([0x00; 6]),
            firmware_state: Mutex::new(FirmwareState::Uninitialized),
            connection_state: Mutex::new(ConnectionState::Disconnected),
            sequence_number: Mutex::new(0),
            ioctl_trans_id: Mutex::new(0),
            link_up: AtomicBool::new(false),
            rx_buffer: Mutex::new(Vec::with_capacity(2048)),
            tx_buffer: Mutex::new(Vec::with_capacity(2048)),
            current_ssid: Mutex::new(Vec::new()),
            firmware_data: Mutex::new(None),
            nvram_data: Mutex::new(None),
            wpa2_handshake: Mutex::new(None),
            dhcp_client: Mutex::new(None),
            ip_address: Mutex::new(None),
            subnet_mask: Mutex::new(None),
            gateway: Mutex::new(None),
        })
    }

    /// Initialize the WiFi chip
    pub fn init(&mut self) -> Result<(), DriverError> {
        println!("[bcm43438] Initializing BCM43438/BCM43455 WiFi...");
        
        // Enable SDIO functions
        self.enable_sdio_functions()?;
        
        // Read chip ID
        self.chip_id = self.read_chip_id()?;
        println!("[bcm43438] Chip ID: {:06X}", self.chip_id);
        
        // Verify chip ID
        match self.chip_id {
            CHIP_ID_BCM43438 | CHIP_ID_BCM43455 | CHIP_ID_BCM43456 => {},
            _ => {
                println!("[bcm43438] Warning: Unknown chip ID {:06X}", self.chip_id);
            }
        }
        
        // Reset the chip
        self.reset_chip()?;
        
        // Load firmware (stubbed - would load from SD card)
        println!("[bcm43438] Loading firmware...");
        self.load_firmware()?;
        
        // Initialize core
        self.init_core()?;
        
        // Get MAC address
        self.mac_address = self.read_mac_address()?;
        let mac_str = self.mac_address.format();
        let mac_str = unsafe { core::str::from_utf8_unchecked(&mac_str) };
        println!("[bcm43438] MAC Address: {}", mac_str);
        
        // Set firmware state to ready
        *self.firmware_state.lock() = FirmwareState::Ready;
        
        println!("[bcm43438] WiFi initialization complete");
        Ok(())
    }

    /// Enable SDIO functions
    fn enable_sdio_functions(&self) -> Result<(), DriverError> {
        match crate::drivers::sdio::with_controller(|controller| -> Result<(), DriverError> {
            // Enable function 1 (backplane)
            controller.write_byte(SDIO_FUNC_BUS, SDIO_CCCR_IOEN, 0x02)?;
            
            // Wait for function to be ready
            let mut timeout = 1000;
            loop {
                let ready = controller.read_byte(SDIO_FUNC_BUS, SDIO_CCCR_IORDY)?;
                if (ready & 0x02) != 0 {
                    break;
                }
                timeout -= 1;
                if timeout == 0 {
                    return Err(DriverError::Timeout);
                }
            }
            
            // Enable function 2 (WLAN)
            controller.write_byte(SDIO_FUNC_BUS, SDIO_CCCR_IOEN, 0x06)?;
            
            // Wait for functions to be ready
            let mut timeout = 1000;
            loop {
                let ready = controller.read_byte(SDIO_FUNC_BUS, SDIO_CCCR_IORDY)?;
                if (ready & 0x06) == 0x06 {
                    break;
                }
                timeout -= 1;
                if timeout == 0 {
                    return Err(DriverError::Timeout);
                }
            }
            
            // Enable interrupts
            controller.write_byte(SDIO_FUNC_BUS, SDIO_CCCR_INTEN, 0x07)?;
            
            // Set block size to 512 bytes for function 1
            controller.write_byte(SDIO_FUNC_BUS, 0x10 + 1 * 2, 0x00)?; // Low byte
            controller.write_byte(SDIO_FUNC_BUS, 0x11 + 1 * 2, 0x02)?; // High byte (512 >> 8)
            
            // Set block size to 512 bytes for function 2
            controller.write_byte(SDIO_FUNC_BUS, 0x10 + 2 * 2, 0x00)?;
            controller.write_byte(SDIO_FUNC_BUS, 0x11 + 2 * 2, 0x02)?;
            
            println!("[bcm43438] SDIO functions enabled");
            Ok(())
        }) {
            Some(result) => result,
            None => Err(DriverError::NotFound),
        }
    }

    /// Read chip ID from backplane
    fn read_chip_id(&self) -> Result<u32, DriverError> {
        let val = self.backplane_read(CHIP_COMMON_BASE + CHIP_COMMON_CHIPID)?;
        // Chip ID is in upper 16 bits, with revision in lower bits
        let chip_id = ((val >> 16) & 0xFFFF) as u32;
        let chip_rev = (val & 0xFFFF) as u32;
        Ok((chip_id << 4) | (chip_rev & 0xF))
    }

    /// Reset the chip
    fn reset_chip(&self) -> Result<(), DriverError> {
        println!("[bcm43438] Resetting chip...");
        
        // Write to SDPCM frame control to initiate reset
        // This is a simplified reset sequence
        self.backplane_write(SDIO_BASE + SDIO_FRAME_CTRL, 0x01)?;
        
        // Wait for reset to complete
        for _ in 0..1000 {
            let val = self.backplane_read(SDIO_BASE + SDIO_FRAME_CTRL)?;
            if (val & 0x01) == 0 {
                break;
            }
        }
        
        // Initialize SDPCM
        self.backplane_write(SDIO_BASE + SDIO_INT_HOST_MASK, 0xFFFFFFFF)?;
        
        println!("[bcm43438] Chip reset complete");
        Ok(())
    }

    /// Load firmware into the chip
    fn load_firmware(&mut self) -> Result<(), DriverError> {
        *self.firmware_state.lock() = FirmwareState::Downloading;
        
        println!("[bcm43438] Loading firmware...");
        
        // Use firmware loader to load files from SD card
        let fw_result = crate::drivers::wifi::firmware_loader::load_firmware_files(self.is_pi4)?;
        
        println!("[bcm43438] Firmware files loaded:");
        println!("  - Firmware binary: {} bytes", fw_result.firmware_size);
        println!("  - NVRAM config: {} bytes ({} params)", 
                 fw_result.nvram_size, fw_result.nvram_params.len());
        
        // Get MAC address from NVRAM if available
        if let Some(mac) = crate::drivers::wifi::firmware_loader::get_mac_address_from_nvram(
            &fw_result.nvram_params) {
            println!("[bcm43438] MAC address from NVRAM: {:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
                     mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]);
            // Store MAC address - will be used instead of generated one
            // Note: MAC is actually set after firmware is fully loaded (see init())
        }
        
        // Load firmware binary data (stored for download)
        let fw_path = if self.is_pi4 {
            crate::drivers::wifi::firmware_loader::FIRMWARE_PATH_PI4
        } else {
            crate::drivers::wifi::firmware_loader::FIRMWARE_PATH_PI3
        };
        
        let firmware_binary = crate::fs::read_file(fw_path)
            .map_err(|_| DriverError::NotFound)?;
        
        // Build NVRAM binary
        let nvram_binary = crate::drivers::wifi::firmware_loader::build_nvram_binary(
            &fw_result.nvram_params);
        
        // Download firmware to chip (using new protocol)
        println!("[bcm43438] Downloading firmware to chip RAM...");
        if let Err(e) = crate::drivers::wifi::firmware_download::full_firmware_download(&firmware_binary, &nvram_binary) {
            println!("[bcm43438] WARNING: Firmware download failed: {:?}", e);
            println!("[bcm43438] Continuing with stubbed initialization...");
        }
        
        *self.firmware_state.lock() = FirmwareState::DownloadDone;
        Ok(())
    }

    /// Initialize chip core after firmware load
    fn init_core(&self) -> Result<(), DriverError> {
        println!("[bcm43438] Initializing chip core...");
        
        // Set up host interrupt mask
        self.backplane_write(SDIO_BASE + SDIO_INT_HOST_MASK, 0x00000000)?;
        
        // Enable function interrupts
        self.backplane_write(SDIO_BASE + SDIO_FUNCTION_INT_MASK, 0x00000007)?;
        
        // Clear interrupt status
        self.backplane_write(SDIO_BASE + SDIO_INT_STATUS, 0xFFFFFFFF)?;
        
        // Wait for firmware to be ready
        // The firmware signals ready by setting certain backplane registers
        for _ in 0..10000 {
            let status = self.backplane_read(SDIO_BASE + SDIO_INT_STATUS)?;
            if status != 0 {
                // Firmware is running
                break;
            }
        }
        
        println!("[bcm43438] Core initialization complete");
        Ok(())
    }

    /// Read MAC address from chip
    fn read_mac_address(&self) -> Result<MacAddress, DriverError> {
        // MAC address would be read from NVRAM or chip OTP
        // For now, generate a locally administered address
        // In a full implementation, this would read from chip
        
        // Generate a MAC address based on chip ID
        let mac = [
            0xB8, 0x27, 0xEB, // Raspberry Pi Foundation OUI
            ((self.chip_id >> 16) & 0xFF) as u8,
            ((self.chip_id >> 8) & 0xFF) as u8,
            (self.chip_id & 0xFF) as u8,
        ];
        
        Ok(MacAddress::new(mac))
    }

    /// Read from backplane via function 1
    fn backplane_read(&self, address: u32) -> Result<u32, DriverError> {
        let func1 = SdioFunction::new(SDIO_FUNC_BACKPLANE);
        
        // Set window address (simplified - assumes address is in current window)
        let addr_low = (address & 0xFFFF) as u16;
        let addr_high = ((address >> 16) & 0x7F) as u8;
        
        // Write address to address registers
        func1.write_byte(0x1000F, addr_high)?;
        func1.write(0x1000C, &addr_low.to_le_bytes())?;
        
        // Read 4 bytes from data register
        let mut buf = [0u8; 4];
        func1.read(0x10000, &mut buf)?;
        
        Ok(u32::from_le_bytes(buf))
    }

    /// Write to backplane via function 1
    fn backplane_write(&self, address: u32, data: u32) -> Result<(), DriverError> {
        let func1 = SdioFunction::new(SDIO_FUNC_BACKPLANE);
        
        // Set window address
        let addr_low = (address & 0xFFFF) as u16;
        let addr_high = ((address >> 16) & 0x7F) as u8;
        
        func1.write_byte(0x1000F, addr_high)?;
        func1.write(0x1000C, &addr_low.to_le_bytes())?;
        
        // Write 4 bytes to data register
        func1.write(0x10000, &data.to_le_bytes())?;
        
        Ok(())
    }

    /// Send SDPCM packet
    fn send_sdpcm_packet(&self, channel: u8, data: &[u8]) -> Result<(), DriverError> {
        let mut tx_buf = self.tx_buffer.lock();
        tx_buf.clear();
        
        // Get sequence number
        let seq = {
            let mut seq_lock = self.sequence_number.lock();
            let s = *seq_lock;
            *seq_lock = seq_lock.wrapping_add(1);
            s
        };
        
        // Build SDPCM header
        let frame_len = (SDPCM_HEADER_LEN + data.len()) as u16;
        let header = SdpcmHeader::new(frame_len, channel, seq);
        
        // Serialize header
        tx_buf.extend_from_slice(&header.frame_len.to_le_bytes());
        tx_buf.extend_from_slice(&header.checksum.to_le_bytes());
        tx_buf.extend_from_slice(&header.sequence.to_le_bytes());
        tx_buf.push(header.channel_flags);
        tx_buf.push(header.next_offset);
        tx_buf.push(header.flow_control);
        tx_buf.push(header.version);
        tx_buf.push(header.bus_data_credit);
        tx_buf.extend_from_slice(&header.reserved);
        
        // Add data
        tx_buf.extend_from_slice(data);
        
        // Pad to 4-byte boundary
        while tx_buf.len() % 4 != 0 {
            tx_buf.push(0);
        }
        
        // Send via SDIO function 2
        let func2 = SdioFunction::new(SDIO_FUNC_WLAN);
        func2.write(0x10000, &tx_buf)?;
        
        Ok(())
    }

    /// Send IOCTL to firmware
    fn send_ioctl(&self, cmd: u32, input: &[u8], output_len: u32) -> Result<Vec<u8>, DriverError> {
        let mut ioctl_buf = Vec::new();
        
        // Get transaction ID
        let trans_id = {
            let mut id_lock = self.ioctl_trans_id.lock();
            let id = *id_lock;
            *id_lock = id_lock.wrapping_add(1);
            id
        };
        
        // Build IOCTL request
        let req = IoctlRequest::new(cmd, trans_id, input.len() as u32, output_len);
        
        // Serialize request
        ioctl_buf.extend_from_slice(&req.cmd.to_le_bytes());
        ioctl_buf.extend_from_slice(&req.trans_id.to_le_bytes());
        ioctl_buf.extend_from_slice(&req.input_len.to_le_bytes());
        ioctl_buf.extend_from_slice(&req.output_len.to_le_bytes());
        ioctl_buf.extend_from_slice(&req.flags.to_le_bytes());
        ioctl_buf.extend_from_slice(&req.status.to_le_bytes());
        ioctl_buf.extend_from_slice(&req.reserved[0].to_le_bytes());
        ioctl_buf.extend_from_slice(&req.reserved[1].to_le_bytes());
        
        // Add input data
        ioctl_buf.extend_from_slice(input);
        
        // Send as control packet
        self.send_sdpcm_packet(SDPCM_CONTROL_CHANNEL, &ioctl_buf)?;
        
        // In a real implementation, we would wait for the response
        // For now, return an empty response
        Ok(Vec::new())
    }

    /// Scan for available networks
    pub fn scan(&self) -> Result<Vec<ScanResult>, DriverError> {
        println!("[bcm43438] Starting WiFi scan...");
        
        let mut state = self.connection_state.lock();
        *state = ConnectionState::Scanning;
        drop(state);
        
        // Send scan IOCTL
        // This would trigger a firmware scan and collect results
        // For now, return an empty list
        
        // Trigger passive scan
        let scan_params = [0u8; 16]; // Simplified scan parameters
        let _ = self.send_ioctl(ioctl::BRCMF_C_SCAN, &scan_params, 256)?;
        
        // Wait for scan to complete
        // In a real implementation, we would poll for results or wait for events
        
        *self.connection_state.lock() = ConnectionState::Disconnected;
        
        println!("[bcm43438] Scan complete");
        Ok(Vec::new())
    }

    /// Connect to a WiFi network with WPA2 authentication
    pub fn connect(&self, ssid: &[u8], password: Option<&[u8]>) -> Result<(), DriverError> {
        println!("[bcm43438] Connecting to WiFi network...");
        
        *self.connection_state.lock() = ConnectionState::Authenticating;
        
        // Set infrastructure mode
        let infra = 1u32.to_le_bytes(); // Infrastructure mode
        self.send_ioctl(ioctl::BRCMF_C_SET_INFRA, &infra, 4)?;
        
        // Set authentication mode
        let auth = if password.is_some() { 4u32 } else { 0u32 }; // WPA2_PSK or OPEN
        self.send_ioctl(ioctl::BRCMF_C_SET_AUTH, &auth.to_le_bytes(), 4)?;
        
        // Set SSID
        let mut ssid_req = Vec::with_capacity(36);
        ssid_req.extend_from_slice(&(ssid.len() as u32).to_le_bytes());
        ssid_req.extend_from_slice(&[0u8; 4]); // Padding
        ssid_req.extend_from_slice(ssid);
        ssid_req.resize(36, 0);
        
        self.send_ioctl(ioctl::BRCMF_C_SET_SSID, &ssid_req, 4)?;
        
        // Store current SSID
        *self.current_ssid.lock() = ssid.to_vec();
        
        // Initialize WPA2 handshake if password provided
        if let Some(pass) = password {
            println!("[bcm43438] Initializing WPA2 handshake...");
            
            // AP MAC address (would be obtained from scan results or beacon)
            // For now, use broadcast address as placeholder
            let ap_mac = [0xFFu8; 6];
            let sta_mac = *self.mac_address.as_bytes();
            
            let handshake = FourWayHandshake::new(ap_mac, sta_mac, pass, ssid);
            
            // Initialize EAPOL processor first
            eapol::init(sta_mac, ap_mac);
            eapol::set_handshake(handshake.clone());
            
            // Store handshake in driver state
            *self.wpa2_handshake.lock() = Some(handshake);
            
            println!("[bcm43438] EAPOL processor initialized");
        }
        
        *self.connection_state.lock() = ConnectionState::Associating;
        
        println!("[bcm43438] Connection initiated");
        Ok(())
    }
    
    /// Start DHCP to obtain IP address
    pub fn start_dhcp(&self) -> Result<(), DriverError> {
        println!("[bcm43438] Starting DHCP client...");
        
        let mut dhcp = DhcpClientSocket::new(self.mac_address);
        
        // Initialize and start DHCP
        match dhcp.start() {
            Ok(()) => {
                println!("[bcm43438] DHCP discover sent");
                *self.dhcp_client.lock() = Some(dhcp);
                Ok(())
            }
            Err(_) => {
                println!("[bcm43438] Failed to start DHCP");
                Err(DriverError::InitFailed)
            }
        }
    }
    
    /// Poll DHCP client for IP configuration
    pub fn poll_dhcp(&self) -> Result<(), DriverError> {
        if let Some(ref mut dhcp) = *self.dhcp_client.lock() {
            match dhcp.poll() {
                Ok(event) => {
                    match event {
                        DhcpEvent::Bound => {
                            println!("[bcm43438] DHCP bound, IP acquired!");
                            if let Some((ip, mask, gw)) = dhcp.get_config() {
                                *self.ip_address.lock() = Some(ip);
                                *self.subnet_mask.lock() = Some(mask);
                                *self.gateway.lock() = Some(gw);
                                println!("[bcm43438] IP Config: {:?}/{:?} GW:{:?}", ip, mask, gw);
                            }
                        }
                        DhcpEvent::OfferReceived => {
                            println!("[bcm43438] DHCP offer received");
                        }
                        DhcpEvent::NakReceived => {
                            println!("[bcm43438] DHCP NAK received");
                        }
                        _ => {}
                    }
                }
                Err(_) => {
                    // Error polling DHCP
                }
            }
        }
        
        Ok(())
    }
    
    /// Poll for EAPOL frames (call periodically)
    pub fn poll_eapol(&self) -> Result<(), DriverError> {
        // Check for pending EAPOL frames to send
        if eapol::has_pending_tx() {
            if let Some(frame) = eapol::get_pending_tx() {
                println!("[bcm43438] Sending EAPOL frame ({} bytes)", frame.len());
                // Send via data channel (would use send_ethernet_frame)
                let _ = self.send_ethernet_frame(&frame);
            }
        }
        
        Ok(())
    }
    
    /// Get current IP configuration
    pub fn get_ip_config(&self) -> Option<(Ipv4Address, Ipv4Address, Ipv4Address)> {
        if let (Some(ip), Some(mask), Some(gw)) = 
            (*self.ip_address.lock(), *self.subnet_mask.lock(), *self.gateway.lock()) {
            Some((ip, mask, gw))
        } else {
            None
        }
    }

    /// Disconnect from current network
    pub fn disconnect(&self) -> Result<(), DriverError> {
        println!("[bcm43438] Disconnecting...");
        
        self.send_ioctl(ioctl::BRCMF_C_DISASSOC, &[], 4)?;
        
        *self.connection_state.lock() = ConnectionState::Disconnected;
        self.link_up.store(false, Ordering::SeqCst);
        self.current_ssid.lock().clear();
        
        println!("[bcm43438] Disconnected");
        Ok(())
    }

    /// Get current connection state
    pub fn connection_state(&self) -> ConnectionState {
        *self.connection_state.lock()
    }

    /// Check if connected to a network
    pub fn is_connected(&self) -> bool {
        matches!(self.connection_state(), ConnectionState::Connected)
    }

    /// Process received packets
    pub fn process_rx(&self) -> Result<(), DriverError> {
        let mut rx_buf = self.rx_buffer.lock();
        rx_buf.resize(2048, 0);
        
        // Read from SDIO function 2
        let func2 = SdioFunction::new(SDIO_FUNC_WLAN);
        func2.read(0x10000, &mut rx_buf)?;
        
        // Parse SDPCM header
        if let Some(header) = SdpcmHeader::from_bytes(&rx_buf) {
            let channel = header.channel_flags & 0x0F;
            let data_offset = header.next_offset as usize;
            let data_len = (header.frame_len & SDPCM_FRAME_LEN_MASK) as usize;
            
            if data_len > SDPCM_HEADER_LEN && data_len <= rx_buf.len() {
                match channel {
                    SDPCM_CONTROL_CHANNEL => {
                        // IOCTL response
                        self.process_ioctl_response(&rx_buf[SDPCM_HEADER_LEN..data_len]);
                    }
                    SDPCM_EVENT_CHANNEL => {
                        // Event packet
                        self.process_event(&rx_buf[SDPCM_HEADER_LEN..data_len]);
                    }
                    SDPCM_DATA_CHANNEL => {
                        // Data packet
                        self.process_data(&rx_buf[SDPCM_HEADER_LEN + BDC_HEADER_LEN..data_len]);
                    }
                    _ => {}
                }
            }
        }
        
        Ok(())
    }

    /// Process IOCTL response
    fn process_ioctl_response(&self, data: &[u8]) {
        if data.len() < 24 {
            return;
        }
        
        let status = u16::from_le_bytes([data[16], data[17]]);
        if status == ioctl::BRCMF_E_STATUS_SUCCESS as u16 {
            // IOCTL succeeded
        } else {
            println!("[bcm43438] IOCTL failed with status {}", status);
        }
    }

    /// Process event packet
    fn process_event(&self, data: &[u8]) {
        if data.len() < 16 {
            return;
        }
        
        let event_type = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        
        match event_type {
            0 => { // SET_SSID
                let status = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
                if status == ioctl::BRCMF_E_STATUS_SUCCESS {
                    println!("[bcm43438] Connected to network");
                    *self.connection_state.lock() = ConnectionState::Connected;
                    self.link_up.store(true, Ordering::SeqCst);
                    
                    // Start DHCP to get IP address
                    if let Err(e) = self.start_dhcp() {
                        println!("[bcm43438] Failed to start DHCP: {:?}", e);
                    }
                }
            }
            1 => { // JOIN
                println!("[bcm43438] Join event received");
            }
            2 => { // START
                println!("[bcm43438] Start event received");
            }
            3 => { // AUTH
                println!("[bcm43438] Authentication event received");
            }
            6 => { // DISASSOC
                println!("[bcm43438] Disassociated from network");
                *self.connection_state.lock() = ConnectionState::Disconnected;
                self.link_up.store(false, Ordering::SeqCst);
                *self.wpa2_handshake.lock() = None;
                *self.dhcp_client.lock() = None;
            }
            16 => { // LINK
                let status = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
                if status == 0 {
                    println!("[bcm43438] Link UP");
                } else {
                    println!("[bcm43438] Link DOWN");
                    self.link_up.store(false, Ordering::SeqCst);
                }
            }
            _ => {}
        }
    }
    
    /// Process WPA2 EAPOL key frame
    fn process_eapol_key(&self, data: &[u8]) -> Option<Vec<u8>> {
        if let Some(ref mut handshake) = *self.wpa2_handshake.lock() {
            if let Some(response) = handshake.process_message(data) {
                // Check if handshake is complete
                if handshake.is_complete() {
                    println!("[bcm43438] WPA2 handshake complete, installing keys...");
                    // Install temporal key to chip
                    let _tk = handshake.get_temporal_key();
                    // Would send key to chip via IOCTL
                }
                return Some(response);
            }
        }
        None
    }

    /// Process data packet
    fn process_data(&self, data: &[u8]) {
        if data.is_empty() {
            return;
        }
        
        // Check if this is an EAPOL frame (WPA2 handshake)
        if eapol::process_rx_frame(data) {
            println!("[bcm43438] EAPOL frame processed");
            return;
        }
        
        // Regular data packet - pass to network stack
        // The data should be an Ethernet frame starting after BDC header
        if data.len() > 14 {
            // Extract source MAC from Ethernet header
            let src_mac = MacAddress::new([data[6], data[7], data[8], data[9], data[10], data[11]]);
            
            // Check ethertype
            let ethertype = u16::from_be_bytes([data[12], data[13]]);
            
            match ethertype {
                0x0800 => { // IPv4
                    // Pass to IP stack
                    crate::net::ip::process_ip_packet(&data[14..]);
                }
                0x0806 => { // ARP
                    crate::net::arp::process_arp_packet(src_mac, &data[14..]);
                }
                _ => {
                    // Unknown ethertype
                }
            }
        }
    }

    /// Send an Ethernet frame via WiFi
    fn send_ethernet_frame(&self, data: &[u8]) -> Result<usize, DriverError> {
        if data.len() > 1500 {
            return Err(DriverError::Unsupported);
        }
        
        let mut frame = Vec::with_capacity(BDC_HEADER_LEN + data.len());
        
        // Add BDC header
        let bdc = BdcHeader::new(0); // Priority 0
        frame.push(bdc.flags);
        frame.push(bdc.priority);
        frame.push(bdc.flags2);
        frame.push(bdc.data_offset);
        
        // Add Ethernet frame
        frame.extend_from_slice(data);
        
        // Send via data channel
        self.send_sdpcm_packet(SDPCM_DATA_CHANNEL, &frame)?;
        
        Ok(data.len())
    }

    /// Load firmware file from SD card
    /// 
    /// This would be called during initialization to load the firmware binary
    pub fn load_firmware_from_file(&self, path: &str) -> Result<(), DriverError> {
        println!("[bcm43438] Loading firmware from: {}", path);
        
        // In a full implementation, this would:
        // 1. Open the file from SD card
        // 2. Read the entire firmware binary
        // 3. Store it for transfer to the chip
        // 4. Verify the firmware checksum
        
        // For now, just log that we would load from the path
        println!("[bcm43438] Firmware loading from '{}' stubbed", path);
        
        Ok(())
    }

    /// Get firmware state
    pub fn firmware_state(&self) -> FirmwareState {
        *self.firmware_state.lock()
    }

    /// Get current SSID
    pub fn current_ssid(&self) -> Vec<u8> {
        self.current_ssid.lock().clone()
    }
}

// SAFETY: Bcm43438Device is thread-safe through interior mutability
unsafe impl Send for Bcm43438Device {}
unsafe impl Sync for Bcm43438Device {}

impl NetworkInterface for Bcm43438Device {
    fn name(&self) -> &str {
        if self.is_pi4 {
            "wlan0 (BCM43455)"
        } else {
            "wlan0 (BCM43438)"
        }
    }

    fn mac_address(&self) -> MacAddress {
        self.mac_address
    }

    fn mtu(&self) -> usize {
        1500
    }

    fn send(&self, data: &[u8]) -> Result<usize, NetError> {
        // Wrap Ethernet frame in WiFi encapsulation
        match self.send_ethernet_frame(data) {
            Ok(len) => Ok(len),
            Err(DriverError::Timeout) => Err(NetError::Timeout),
            Err(_) => Err(NetError::NoDevice),
        }
    }

    fn receive(&self, buf: &mut [u8]) -> Result<usize, NetError> {
        // Process any pending RX packets
        if let Err(_) = self.process_rx() {
            return Err(NetError::NoBuffer);
        }
        
        // In a real implementation, we would dequeue from an RX queue
        // For now, return no data available
        Err(NetError::NoBuffer)
    }

    fn is_link_up(&self) -> bool {
        self.link_up.load(Ordering::SeqCst)
    }
}

/// Global WiFi device instance
use spin::Mutex;
use lazy_static::lazy_static;
lazy_static! {
    static ref WIFI_DEVICE: Mutex<Option<Bcm43438Device>> = Mutex::new(None);
}

/// Initialize BCM43438/BCM43455 WiFi driver
pub fn init(pi4: bool) {
    println!("[bcm43438] Initializing BCM43438/BCM43455 driver...");
    
    // Create device
    match Bcm43438Device::new(pi4) {
        Ok(mut device) => {
            // Initialize the device
            if let Err(e) = device.init() {
                println!("[bcm43438] Failed to initialize device: {:?}", e);
                return;
            }
            
            // Register with network stack
            let device_box: Box<dyn NetworkInterface> = Box::new(device);
            net::register_interface(device_box);
            
            println!("[bcm43438] WiFi driver initialized");
        }
        Err(e) => {
            println!("[bcm43438] Failed to create device: {:?}", e);
        }
    }
}

/// With the global WiFi device
pub fn with_device<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut Bcm43438Device) -> R,
{
    let mut guard = WIFI_DEVICE.lock();
    guard.as_mut().map(f)
}

/// Check if WiFi is available
pub fn is_available() -> bool {
    WIFI_DEVICE.lock().is_some()
}
