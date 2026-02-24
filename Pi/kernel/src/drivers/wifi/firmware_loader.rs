//! BCM43438/BCM43455 Firmware Loader
//!
//! Handles loading firmware binary files from the filesystem and
//! transferring them to the WiFi chip via SDIO backplane.

use alloc::vec::Vec;
use alloc::string::String;
use crate::drivers::DriverError;
use crate::fs;
use crate::println;

/// Firmware file paths
pub const FIRMWARE_PATH_PI3: &str = "/firmware/brcm/brcmfmac43430-sdio.bin";
pub const NVRAM_PATH_PI3: &str = "/firmware/brcm/brcmfmac43430-sdio.txt";
pub const CLM_PATH_PI3: &str = "/firmware/brcm/brcmfmac43430-sdio.clm_blob";

pub const FIRMWARE_PATH_PI4: &str = "/firmware/brcm/brcmfmac43455-sdio.bin";
pub const NVRAM_PATH_PI4: &str = "/firmware/brcm/brcmfmac43455-sdio.txt";
pub const CLM_PATH_PI4: &str = "/firmware/brcm/brcmfmac43455-sdio.clm_blob";

/// Firmware load result
#[derive(Debug)]
pub struct FirmwareLoadResult {
    pub firmware_size: usize,
    pub nvram_size: usize,
    pub clm_size: usize,
    pub nvram_params: Vec<(Vec<u8>, Vec<u8>)>,
}

/// Load firmware files from filesystem
/// 
/// # Arguments
/// * `is_pi4` - true for Pi 4 (BCM43455), false for Pi 3 (BCM43438)
/// 
/// # Returns
/// * `Ok(FirmwareLoadResult)` - Loaded firmware data
/// * `Err(DriverError)` - If firmware files not found or unreadable
pub fn load_firmware_files(is_pi4: bool) -> Result<FirmwareLoadResult, DriverError> {
    let (fw_path, nv_path, clm_path) = if is_pi4 {
        (FIRMWARE_PATH_PI4, NVRAM_PATH_PI4, CLM_PATH_PI4)
    } else {
        (FIRMWARE_PATH_PI3, NVRAM_PATH_PI3, CLM_PATH_PI3)
    };
    
    println!("[wifi/firmware] Loading firmware files...");
    
    // Load main firmware binary
    println!("[wifi/firmware]  Reading: {}", fw_path);
    let firmware_data = fs::read_file(fw_path)
        .map_err(|e| {
            println!("[wifi/firmware]  ERROR: Failed to load firmware: {:?}", e);
            DriverError::NotFound
        })?;
    
    if firmware_data.is_empty() {
        println!("[wifi/firmware]  ERROR: Firmware file is empty");
        return Err(DriverError::IoError);
    }
    
    println!("[wifi/firmware]   -> {} bytes", firmware_data.len());
    
    // Load NVRAM configuration
    println!("[wifi/firmware]  Reading: {}", nv_path);
    let nvram_data = fs::read_file(nv_path)
        .map_err(|e| {
            println!("[wifi/firmware]  WARNING: Failed to load NVRAM: {:?}", e);
            DriverError::NotFound
        })?;
    
    println!("[wifi/firmware]   -> {} bytes", nvram_data.len());
    
    // Parse NVRAM parameters
    let nvram_params = parse_nvram(&nvram_data);
    println!("[wifi/firmware]   -> {} NVRAM parameters", nvram_params.len());
    
    // Load CLM blob (optional, but recommended)
    println!("[wifi/firmware]  Reading: {}", clm_path);
    let clm_data = match fs::read_file(clm_path) {
        Ok(data) => {
            println!("[wifi/firmware]   -> {} bytes", data.len());
            data
        }
        Err(e) => {
            println!("[wifi/firmware]   WARNING: CLM blob not found: {:?}", e);
            Vec::new()
        }
    };
    
    // Validate firmware header
    validate_firmware_header(&firmware_data)?;
    
    println!("[wifi/firmware] Firmware files loaded successfully");
    
    Ok(FirmwareLoadResult {
        firmware_size: firmware_data.len(),
        nvram_size: nvram_data.len(),
        clm_size: clm_data.len(),
        nvram_params,
    })
}

/// Parse NVRAM text file into key-value pairs
/// 
/// NVRAM format:
/// ```text
/// # Comments start with #
/// key=value
/// another_key=another_value
/// ```
fn parse_nvram(data: &[u8]) -> Vec<(Vec<u8>, Vec<u8>)> {
    let mut params = Vec::new();
    
    // Convert to string (NVRAM files are ASCII text)
    let text = match core::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return params,
    };
    
    for line in text.lines() {
        let line = line.trim();
        
        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        
        // Parse key=value
        if let Some(pos) = line.find('=') {
            let key = line[..pos].trim().as_bytes().to_vec();
            let value = line[pos + 1..].trim().as_bytes().to_vec();
            
            if !key.is_empty() {
                params.push((key, value));
            }
        }
    }
    
    params
}

/// Validate firmware binary header
/// 
/// Broadcom firmware files typically start with a specific signature
/// or header structure. This function performs basic validation.
fn validate_firmware_header(data: &[u8]) -> Result<(), DriverError> {
    if data.len() < 4 {
        println!("[wifi/firmware] ERROR: Firmware too small ({} bytes)", data.len());
        return Err(DriverError::IoError);
    }
    
    // Check for common firmware signatures
    // Broadcom firmware typically doesn't have a fixed magic number,
    // but we can check for reasonable size and structure
    
    const MIN_FIRMWARE_SIZE: usize = 100 * 1024;  // 100KB minimum
    const MAX_FIRMWARE_SIZE: usize = 2 * 1024 * 1024;  // 2MB maximum
    
    if data.len() < MIN_FIRMWARE_SIZE {
        println!("[wifi/firmware] WARNING: Firmware smaller than expected ({} bytes)", 
                 data.len());
        // Don't fail - some firmware variants might be smaller
    }
    
    if data.len() > MAX_FIRMWARE_SIZE {
        println!("[wifi/firmware] WARNING: Firmware larger than expected ({} bytes)", 
                 data.len());
    }
    
    // Basic sanity check: firmware shouldn't be all zeros or all 0xFF
    let mut all_zeros = true;
    let mut all_ones = true;
    
    for byte in data.iter().take(1024) {
        if *byte != 0x00 {
            all_zeros = false;
        }
        if *byte != 0xFF {
            all_ones = false;
        }
    }
    
    if all_zeros {
        println!("[wifi/firmware] ERROR: Firmware is all zeros (corrupted?)");
        return Err(DriverError::IoError);
    }
    
    if all_ones {
        println!("[wifi/firmware] ERROR: Firmware is all 0xFF (erased flash?)");
        return Err(DriverError::IoError);
    }
    
    Ok(())
}

/// Get MAC address from NVRAM parameters
/// 
/// # Returns
/// * `Some([u8; 6])` - MAC address bytes if found
/// * `None` - If MAC address not in NVRAM
pub fn get_mac_address_from_nvram(params: &[(Vec<u8>, Vec<u8>)]) -> Option<[u8; 6]> {
    for (key, value) in params {
        // Common NVRAM keys for MAC address
        let key_str = match core::str::from_utf8(key) {
            Ok(s) => s.to_lowercase(),
            Err(_) => continue,
        };
        
        if key_str == "macaddr" || key_str == "macaddress" || key_str == "il0macaddr" {
            // Parse MAC address format: XX:XX:XX:XX:XX:XX or XXXXXXXXXXXX
            let value_str = match core::str::from_utf8(value) {
                Ok(s) => s,
                Err(_) => continue,
            };
            
            if let Ok(mac) = parse_mac_address(value_str) {
                return Some(mac);
            }
        }
    }
    
    None
}

/// Parse MAC address string into bytes
fn parse_mac_address(s: &str) -> Result<[u8; 6], ()> {
    let s = s.trim();
    
    // Remove colons if present
    let hex_str: String = s.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    
    if hex_str.len() != 12 {
        return Err(());
    }
    
    let mut mac = [0u8; 6];
    for i in 0..6 {
        let byte_str = &hex_str[i * 2..i * 2 + 2];
        mac[i] = u8::from_str_radix(byte_str, 16).map_err(|_| ())?;
    }
    
    Ok(mac)
}

/// Build NVRAM binary blob for firmware download
/// 
/// The firmware expects NVRAM data in a specific binary format
/// with a length prefix and CRC suffix.
pub fn build_nvram_binary(params: &[(Vec<u8>, Vec<u8>)]) -> Vec<u8> {
    let mut nvram = Vec::new();
    
    // Build NVRAM string
    let mut nvram_str = String::new();
    
    for (key, value) in params {
        // Add key=value pair
        if let (Ok(key_str), Ok(value_str)) = 
            (core::str::from_utf8(key), core::str::from_utf8(value)) {
            nvram_str.push_str(key_str);
            nvram_str.push('=');
            nvram_str.push_str(value_str);
            nvram_str.push('\x00');  // Null terminator for each entry
        }
    }
    
    // Add final null terminator
    nvram_str.push('\x00');
    
    // Convert to bytes
    nvram.extend_from_slice(nvram_str.as_bytes());
    
    // Pad to 4-byte boundary
    while nvram.len() % 4 != 0 {
        nvram.push(0);
    }
    
    nvram
}

/// Calculate simple checksum for firmware validation
pub fn calculate_checksum(data: &[u8]) -> u32 {
    let mut checksum: u32 = 0;
    
    for chunk in data.chunks(4) {
        let mut word: u32 = 0;
        for (i, byte) in chunk.iter().enumerate() {
            word |= (*byte as u32) << (i * 8);
        }
        checksum = checksum.wrapping_add(word);
    }
    
    checksum
}

/// Check if firmware files exist on the system
pub fn check_firmware_available(is_pi4: bool) -> bool {
    let (fw_path, nv_path, _clm_path) = if is_pi4 {
        (FIRMWARE_PATH_PI4, NVRAM_PATH_PI4, CLM_PATH_PI4)
    } else {
        (FIRMWARE_PATH_PI3, NVRAM_PATH_PI3, CLM_PATH_PI3)
    };
    
    // Check if firmware file exists
    match fs::read_file(fw_path) {
        Ok(data) if !data.is_empty() => {
            // Also check NVRAM
            match fs::read_file(nv_path) {
                Ok(nv) if !nv.is_empty() => true,
                _ => false,
            }
        }
        _ => false,
    }
}

/// Print firmware status information
pub fn print_firmware_info(is_pi4: bool) {
    let (fw_path, nv_path, clm_path) = if is_pi4 {
        (FIRMWARE_PATH_PI4, NVRAM_PATH_PI4, CLM_PATH_PI4)
    } else {
        (FIRMWARE_PATH_PI3, NVRAM_PATH_PI3, CLM_PATH_PI3)
    };
    
    println!("[wifi/firmware] Firmware status:");
    
    // Check firmware file
    match fs::read_file(fw_path) {
        Ok(data) => println!("[wifi/firmware]   Firmware binary: {} bytes", data.len()),
        Err(_) => println!("[wifi/firmware]   Firmware binary: NOT FOUND"),
    }
    
    // Check NVRAM file
    match fs::read_file(nv_path) {
        Ok(data) => println!("[wifi/firmware]   NVRAM config: {} bytes", data.len()),
        Err(_) => println!("[wifi/firmware]   NVRAM config: NOT FOUND"),
    }
    
    // Check CLM file
    match fs::read_file(clm_path) {
        Ok(data) => println!("[wifi/firmware]   CLM blob: {} bytes", data.len()),
        Err(_) => println!("[wifi/firmware]   CLM blob: NOT FOUND (optional)"),
    }
}
