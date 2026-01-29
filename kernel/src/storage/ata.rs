//! ATA/IDE Driver
//!
//! Supports legacy ATA/IDE hard disk controllers.

use core::arch::asm;
use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::string::String;
use spin::Mutex;

use crate::storage::{BlockDevice, StorageError};
use crate::println;

/// ATA I/O ports (primary channel)
const PRIMARY_DATA: u16 = 0x1F0;
const PRIMARY_ERROR: u16 = 0x1F1;
const PRIMARY_SECTOR_COUNT: u16 = 0x1F2;
const PRIMARY_LBA_LOW: u16 = 0x1F3;
const PRIMARY_LBA_MID: u16 = 0x1F4;
const PRIMARY_LBA_HIGH: u16 = 0x1F5;
const PRIMARY_DRIVE: u16 = 0x1F6;
const PRIMARY_STATUS: u16 = 0x1F7;
const PRIMARY_COMMAND: u16 = 0x1F7;
const PRIMARY_CONTROL: u16 = 0x3F6;

/// ATA commands
const CMD_READ_SECTORS: u8 = 0x20;
const CMD_WRITE_SECTORS: u8 = 0x30;
const CMD_IDENTIFY: u8 = 0xEC;
const CMD_FLUSH_CACHE: u8 = 0xE7;

/// ATA status bits
const STATUS_BSY: u8 = 0x80;  // Busy
const STATUS_DRDY: u8 = 0x40; // Drive ready
const STATUS_DRQ: u8 = 0x08;  // Data request
const STATUS_ERR: u8 = 0x01;  // Error

/// ATA drive structure
pub struct AtaDrive {
    base_port: u16,
    control_port: u16,
    is_master: bool,
    model: [u8; 40],
    serial: [u8; 20],
    sector_count: u64,
    lba48: bool,
}

impl AtaDrive {
    /// Create new ATA drive instance
    pub fn new(base_port: u16, control_port: u16, is_master: bool) -> Self {
        Self {
            base_port,
            control_port,
            is_master,
            model: [0; 40],
            serial: [0; 20],
            sector_count: 0,
            lba48: false,
        }
    }

    /// Initialize and identify drive
    pub fn init(&mut self) -> Result<(), StorageError> {
        // Select drive
        let drive_sel = if self.is_master { 0xA0 } else { 0xB0 };
        unsafe {
            write_port(self.base_port + 6, drive_sel);
        }
        
        // Small delay
        wait_400ns(self.control_port);

        // Send IDENTIFY command
        unsafe {
            write_port(self.base_port + 7, CMD_IDENTIFY);
        }

        // Wait for response
        let status = self.wait_status();
        if status & STATUS_ERR != 0 {
            return Err(StorageError::NotFound);
        }

        // Check if drive exists (ATA or ATAPI)
        let mid = unsafe { read_port(self.base_port + 4) };
        let high = unsafe { read_port(self.base_port + 5) };
        
        if mid != 0 || high != 0 {
            // ATAPI or SATA drive - skip for now
            return Err(StorageError::NotFound);
        }

        // Read identification data
        let mut id_buffer = [0u16; 256];
        for i in 0..256 {
            id_buffer[i] = unsafe { read_port_word(self.base_port) };
        }

        // Parse identification data
        self.parse_identify(&id_buffer);

        Ok(())
    }

    /// Parse IDENTIFY data
    fn parse_identify(&mut self, data: &[u16; 256]) {
        // Model name (words 27-46, byte-swapped)
        for i in 0..20 {
            let word = data[27 + i];
            self.model[i * 2] = (word >> 8) as u8;
            self.model[i * 2 + 1] = (word & 0xFF) as u8;
        }

        // Serial number (words 10-19, byte-swapped)
        for i in 0..10 {
            let word = data[10 + i];
            self.serial[i * 2] = (word >> 8) as u8;
            self.serial[i * 2 + 1] = (word & 0xFF) as u8;
        }

        // Check LBA48 support (word 83, bit 10)
        self.lba48 = (data[83] & 0x0400) != 0;

        // Get sector count
        if self.lba48 {
            // LBA48 total sectors (words 100-103)
            self.sector_count = 
                (data[100] as u64) |
                ((data[101] as u64) << 16) |
                ((data[102] as u64) << 32) |
                ((data[103] as u64) << 48);
        } else {
            // LBA28 total sectors (words 60-61)
            self.sector_count = 
                (data[60] as u64) | ((data[61] as u64) << 16);
        }
    }

    /// Wait for status, return final status
    fn wait_status(&self) -> u8 {
        let mut status;
        loop {
            status = unsafe { read_port(self.base_port + 7) };
            if status & STATUS_BSY == 0 {
                break;
            }
        }
        status
    }

    /// Wait for DRQ (data ready)
    fn wait_drq(&self) -> Result<(), StorageError> {
        let timeout = 100000;
        for _ in 0..timeout {
            let status = unsafe { read_port(self.base_port + 7) };
            if status & STATUS_ERR != 0 {
                return Err(StorageError::IoError);
            }
            if status & STATUS_DRQ != 0 {
                return Ok(());
            }
        }
        Err(StorageError::Timeout)
    }

    /// Read sectors using LBA28
    fn read_sectors_lba28(&self, lba: u64, count: u8, buf: &mut [u8]) -> Result<(), StorageError> {
        if count == 0 {
            return Err(StorageError::InvalidArgument);
        }

        let sector_count = count; // 0 means 256 sectors in ATA

        // Select drive and LBA
        let drive_sel = if self.is_master { 0xE0 } else { 0xF0 };
        unsafe {
            write_port(self.base_port + 6, drive_sel | ((lba >> 24) & 0x0F) as u8);
            write_port(self.base_port + 2, sector_count);
            write_port(self.base_port + 3, (lba & 0xFF) as u8);
            write_port(self.base_port + 4, ((lba >> 8) & 0xFF) as u8);
            write_port(self.base_port + 5, ((lba >> 16) & 0xFF) as u8);
        }

        // Send read command
        unsafe {
            write_port(self.base_port + 7, CMD_READ_SECTORS);
        }

        // Read data
        let mut offset = 0;
        for _ in 0..count {
            self.wait_drq()?;
            
            unsafe {
                for _ in 0..256 {
                    let word = read_port_word(self.base_port);
                    buf[offset] = (word & 0xFF) as u8;
                    buf[offset + 1] = (word >> 8) as u8;
                    offset += 2;
                }
            }
        }

        Ok(())
    }

    /// Write sectors using LBA28
    fn write_sectors_lba28(&self, lba: u64, count: u8, buf: &[u8]) -> Result<(), StorageError> {
        if count == 0 {
            return Err(StorageError::InvalidArgument);
        }

        let sector_count = count; // 0 means 256 sectors in ATA

        // Select drive and LBA
        let drive_sel = if self.is_master { 0xE0 } else { 0xF0 };
        unsafe {
            write_port(self.base_port + 6, drive_sel | ((lba >> 24) & 0x0F) as u8);
            write_port(self.base_port + 2, sector_count);
            write_port(self.base_port + 3, (lba & 0xFF) as u8);
            write_port(self.base_port + 4, ((lba >> 8) & 0xFF) as u8);
            write_port(self.base_port + 5, ((lba >> 16) & 0xFF) as u8);
        }

        // Send write command
        unsafe {
            write_port(self.base_port + 7, CMD_WRITE_SECTORS);
        }

        // Write data
        let mut offset = 0;
        for _ in 0..count {
            self.wait_drq()?;
            
            unsafe {
                for _ in 0..256 {
                    let word = (buf[offset] as u16) | ((buf[offset + 1] as u16) << 8);
                    write_port_word(self.base_port, word);
                    offset += 2;
                }
            }
        }

        // Flush cache
        self.flush()
    }
}

impl BlockDevice for AtaDrive {
    fn name(&self) -> &str {
        if self.is_master {
            "ata0"
        } else {
            "ata1"
        }
    }

    fn block_size(&self) -> usize {
        512
    }

    fn block_count(&self) -> u64 {
        self.sector_count
    }

    fn read_blocks(&self, start: u64, count: usize, buf: &mut [u8]) -> Result<(), StorageError> {
        if count == 0 {
            return Ok(());
        }

        if count > 256 {
            // Split into multiple reads
            let mut offset = 0;
            let mut remaining = count;
            let mut current_lba = start;

            while remaining > 0 {
                let to_read = remaining.min(256);
                self.read_blocks(current_lba, to_read, &mut buf[offset..offset + to_read * 512])?;
                offset += to_read * 512;
                remaining -= to_read;
                current_lba += to_read as u64;
            }
            return Ok(());
        }

        self.read_sectors_lba28(start, count as u8, buf)
    }

    fn write_blocks(&self, start: u64, count: usize, buf: &[u8]) -> Result<(), StorageError> {
        if count == 0 {
            return Ok(());
        }

        if count > 256 {
            // Split into multiple writes
            let mut offset = 0;
            let mut remaining = count;
            let mut current_lba = start;

            while remaining > 0 {
                let to_write = remaining.min(256);
                self.write_blocks(current_lba, to_write, &buf[offset..offset + to_write * 512])?;
                offset += to_write * 512;
                remaining -= to_write;
                current_lba += to_write as u64;
            }
            return Ok(());
        }

        self.write_sectors_lba28(start, count as u8, buf)
    }

    fn flush(&self) -> Result<(), StorageError> {
        unsafe {
            write_port(self.base_port + 7, CMD_FLUSH_CACHE);
        }
        
        let timeout = 100000;
        for _ in 0..timeout {
            let status = unsafe { read_port(self.base_port + 7) };
            if status & STATUS_BSY == 0 {
                return Ok(());
            }
        }
        
        Err(StorageError::Timeout)
    }
}

/// Initialize ATA drives
pub fn init() {
    println!("[ata] Probing for ATA drives...");

    // Try primary master
    let mut drive0 = AtaDrive::new(PRIMARY_DATA, PRIMARY_CONTROL, true);
    if drive0.init().is_ok() {
        let model = core::str::from_utf8(&drive0.model).unwrap_or("Unknown").trim();
        let serial = core::str::from_utf8(&drive0.serial).unwrap_or("Unknown").trim();
        println!("[ata] Found drive: {} ({})", model, serial);
        
        crate::storage::register_device(Box::new(drive0));
    }

    // Try primary slave
    let mut drive1 = AtaDrive::new(PRIMARY_DATA, PRIMARY_CONTROL, false);
    if drive1.init().is_ok() {
        let model = core::str::from_utf8(&drive1.model).unwrap_or("Unknown").trim();
        let serial = core::str::from_utf8(&drive1.serial).unwrap_or("Unknown").trim();
        println!("[ata] Found drive: {} ({})", model, serial);
        
        crate::storage::register_device(Box::new(drive1));
    }
}

/// Read byte from I/O port
unsafe fn read_port(port: u16) -> u8 {
    let val: u8;
    asm!(
        "in al, dx",
        in("dx") port,
        out("al") val,
        options(nomem, nostack)
    );
    val
}

/// Read word from I/O port
unsafe fn read_port_word(port: u16) -> u16 {
    let val: u16;
    asm!(
        "in ax, dx",
        in("dx") port,
        out("ax") val,
        options(nomem, nostack)
    );
    val
}

/// Write byte to I/O port
unsafe fn write_port(port: u16, val: u8) {
    asm!(
        "out dx, al",
        in("dx") port,
        in("al") val,
        options(nomem, nostack)
    );
}

/// Write word to I/O port
unsafe fn write_port_word(port: u16, val: u16) {
    asm!(
        "out dx, ax",
        in("dx") port,
        in("ax") val,
        options(nomem, nostack)
    );
}

/// Wait ~400ns
fn wait_400ns(control_port: u16) {
    unsafe {
        for _ in 0..4 {
            read_port(control_port);
        }
    }
}
