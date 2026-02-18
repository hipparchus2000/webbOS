//! SD Host Controller Interface (SDHCI) Driver for Raspberry Pi 5
//!
//! This module implements the SDHOST/SDIO interface for the Raspberry Pi 5
//! using the RP1 I/O controller. Supports SD card block read/write operations.

use crate::error::{VFatError, IoError};
use alloc::format;
use alloc::string::String;
use core::ptr::{read_volatile, write_volatile};

/// SDHCI register offsets (RP1)
#[allow(dead_code)]
mod regs {
    // SDHCI standard registers
    pub const SDMA_SYSTEM_ADDR: usize = 0x00;
    pub const BLOCK_SIZE: usize = 0x04;
    pub const BLOCK_COUNT: usize = 0x06;
    pub const ARGUMENT: usize = 0x08;
    pub const TRANSFER_MODE: usize = 0x0C;
    pub const COMMAND: usize = 0x0E;
    pub const RESPONSE_0: usize = 0x10;
    pub const RESPONSE_1: usize = 0x14;
    pub const RESPONSE_2: usize = 0x18;
    pub const RESPONSE_3: usize = 0x1C;
    pub const BUFFER_DATA_PORT: usize = 0x20;
    pub const PRESENT_STATE: usize = 0x24;
    pub const HOST_CONTROL: usize = 0x28;
    pub const POWER_CONTROL: usize = 0x29;
    pub const BLOCK_GAP_CONTROL: usize = 0x2A;
    pub const WAKEUP_CONTROL: usize = 0x2B;
    pub const CLOCK_CONTROL: usize = 0x2C;
    pub const TIMEOUT_CONTROL: usize = 0x2E;
    pub const SOFTWARE_RESET: usize = 0x2F;
    pub const NORMAL_INT_STATUS: usize = 0x30;
    pub const ERROR_INT_STATUS: usize = 0x32;
    pub const NORMAL_INT_ENABLE: usize = 0x34;
    pub const ERROR_INT_ENABLE: usize = 0x36;
    pub const CAPABILITIES_0: usize = 0x40;
    pub const CAPABILITIES_1: usize = 0x44;
    pub const MAX_CURRENT_0: usize = 0x48;
    pub const MAX_CURRENT_1: usize = 0x4C;
    pub const FORCE_EVENT: usize = 0x50;
    pub const ADMA_ERROR_STATUS: usize = 0x54;
    pub const ADMA_SYSTEM_ADDR: usize = 0x58;
}

/// Transfer mode flags
#[derive(Debug, Clone, Copy)]
#[repr(u16)]
pub enum TransferMode {
    /// DMA enable
    DmaEnable = 0x0001,
    /// Block count enable
    BlockCountEnable = 0x0002,
    /// Auto CMD12 enable
    AutoCmd12Enable = 0x0004,
    /// Transfer direction: read (1) or write (0)
    Read = 0x0010,
    /// Multi-block transfer
    MultiBlock = 0x0020,
}

/// Command register flags
#[derive(Debug, Clone, Copy)]
#[repr(u16)]
pub enum CommandFlags {
    /// Response type: no response
    NoResponse = 0x0000,
    /// Response type: 136-bit response
    Response136 = 0x0001,
    /// Response type: 48-bit response
    Response48 = 0x0002,
    /// Response type: 48-bit response with busy check
    Response48Busy = 0x0003,
    /// Command CRC check enable
    CrcCheck = 0x0008,
    /// Command index check enable
    IndexCheck = 0x0010,
    /// Data present select
    DataPresent = 0x0020,
    /// Command type: abort
    TypeAbort = 0x00C0,
}

/// Present state flags
#[derive(Debug, Clone, Copy)]
#[repr(u32)]
pub enum PresentState {
    /// Command inhibit (CMD)
    CmdInhibit = 0x00000001,
    /// Command inhibit (DAT)
    DatInhibit = 0x00000002,
    /// DAT line active
    DatActive = 0x00000004,
    /// Write transfer active
    WriteActive = 0x00000100,
    /// Read transfer active
    ReadActive = 0x00000200,
    /// Buffer write enable
    BufferWriteEnable = 0x00000400,
    /// Buffer read enable
    BufferReadEnable = 0x00000800,
    /// Card inserted
    CardInserted = 0x00010000,
    /// Card state stable
    CardStateStable = 0x00020000,
    /// Card detect pin level
    CardDetectPin = 0x00040000,
    /// Write protect pin level
    WriteProtectPin = 0x00080000,
}

/// Normal interrupt status flags
#[derive(Debug, Clone, Copy)]
#[repr(u16)]
pub enum NormalIntStatus {
    /// Command complete
    CommandComplete = 0x0001,
    /// Transfer complete
    TransferComplete = 0x0002,
    /// Block gap event
    BlockGapEvent = 0x0004,
    /// DMA interrupt
    DmaInterrupt = 0x0008,
    /// Buffer write ready
    BufferWriteReady = 0x0010,
    /// Buffer read ready
    BufferReadReady = 0x0020,
    /// Card insertion
    CardInsertion = 0x0040,
    /// Card removal
    CardRemoval = 0x0080,
    /// Card interrupt
    CardInterrupt = 0x0100,
}

/// Error interrupt status flags
#[derive(Debug, Clone, Copy)]
#[repr(u16)]
pub enum ErrorIntStatus {
    /// Command timeout error
    CmdTimeout = 0x0001,
    /// Command CRC error
    CmdCrcError = 0x0002,
    /// Command end bit error
    CmdEndBitError = 0x0004,
    /// Command index error
    CmdIndexError = 0x0008,
    /// Data timeout error
    DataTimeout = 0x0010,
    /// Data CRC error
    DataCrcError = 0x0020,
    /// Data end bit error
    DataEndBitError = 0x0040,
    /// Auto CMD12 error
    AutoCmd12Error = 0x0080,
}

/// SD card block device
pub struct SdBlockDevice {
    /// Base address of SDHCI registers
    base_addr: usize,
    /// Card capacity in sectors
    capacity_sectors: u64,
    /// Card is high capacity (SDHC/SDXC)
    high_capacity: bool,
    /// Block size (typically 512 bytes)
    block_size: usize,
    /// Maximum retry attempts
    max_retries: u32,
}

/// SD command enumeration
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
#[allow(dead_code)]
enum SdCommand {
    GoIdleState = 0,
    AllSendCid = 2,
    SendRelativeAddr = 3,
    SelectCard = 7,
    SendIfCond = 8,
    SendCsd = 9,
    StopTransmission = 12,
    SendStatus = 13,
    SetBlocklen = 16,
    ReadSingleBlock = 17,
    ReadMultipleBlock = 18,
    WriteSingleBlock = 24,
    WriteMultipleBlock = 25,
    AppCmd = 55,
    ReadOcr = 58,
}

impl SdBlockDevice {
    /// Create a new SD block device instance
    /// 
    /// # Safety
    /// Caller must ensure base_addr is a valid SDHCI register base address
    pub unsafe fn new(base_addr: usize) -> Self {
        Self {
            base_addr,
            capacity_sectors: 0,
            high_capacity: false,
            block_size: 512,
            max_retries: 3,
        }
    }

    /// Initialize the SD card
    pub fn init(&mut self) -> Result<(), VFatError> {
        // Reset the controller
        self.software_reset()?;

        // Check if card is present
        if !self.is_card_present() {
            return Err(VFatError::io(IoError::device_error("No SD card detected")));
        }

        // Initialize the card
        self.card_init()?;

        // Set block size to 512 bytes
        self.set_block_size(512)?;

        // Get card capacity
        self.read_card_capacity()?;

        Ok(())
    }

    /// Read a single 512-byte sector
    pub fn read_sector(&mut self, sector: u64, buffer: &mut [u8]) -> Result<(), VFatError> {
        if buffer.len() != self.block_size {
            return Err(VFatError::InvalidParameter(
                String::from("Buffer size must match block size (512 bytes)"),
            ));
        }

        let mut retries = self.max_retries;
        loop {
            match self.read_sector_internal(sector, buffer) {
                Ok(()) => return Ok(()),
                Err(e) => {
                    retries -= 1;
                    if retries == 0 {
                        return Err(e);
                    }
                    // Small delay before retry
                    self.delay_us(100);
                }
            }
        }
    }

    /// Write a single 512-byte sector
    pub fn write_sector(&mut self, sector: u64, buffer: &[u8]) -> Result<(), VFatError> {
        if buffer.len() != self.block_size {
            return Err(VFatError::InvalidParameter(
                String::from("Buffer size must match block size (512 bytes)"),
            ));
        }

        let mut retries = self.max_retries;
        loop {
            match self.write_sector_internal(sector, buffer) {
                Ok(()) => return Ok(()),
                Err(e) => {
                    retries -= 1;
                    if retries == 0 {
                        return Err(e);
                    }
                    // Small delay before retry
                    self.delay_us(100);
                }
            }
        }
    }

    /// Read multiple sectors
    pub fn read_sectors(&mut self, start_sector: u64, count: usize, buffer: &mut [u8]) -> Result<(), VFatError> {
        if buffer.len() != count * self.block_size {
            return Err(VFatError::InvalidParameter(
                String::from("Buffer size must match sector count * block size"),
            ));
        }

        for i in 0..count {
            self.read_sector(start_sector + i as u64, &mut buffer[i * self.block_size..(i + 1) * self.block_size])?;
        }

        Ok(())
    }

    /// Write multiple sectors
    pub fn write_sectors(&mut self, start_sector: u64, count: usize, buffer: &[u8]) -> Result<(), VFatError> {
        if buffer.len() != count * self.block_size {
            return Err(VFatError::InvalidParameter(
                String::from("Buffer size must match sector count * block size"),
            ));
        }

        for i in 0..count {
            self.write_sector(start_sector + i as u64, &buffer[i * self.block_size..(i + 1) * self.block_size])?;
        }

        Ok(())
    }

    /// Get card capacity in sectors
    pub fn capacity(&self) -> u64 {
        self.capacity_sectors
    }

    /// Get block size
    pub fn block_size(&self) -> usize {
        self.block_size
    }

    /// Check if card is present
    pub fn is_card_present(&self) -> bool {
        let state = self.read_reg(regs::PRESENT_STATE);
        (state & PresentState::CardInserted as u32) != 0
            && (state & PresentState::CardStateStable as u32) != 0
    }

    /// Check if card is write-protected
    pub fn is_write_protected(&self) -> bool {
        let state = self.read_reg(regs::PRESENT_STATE);
        (state & PresentState::WriteProtectPin as u32) != 0
    }

    // Internal helper methods

    fn read_reg(&self, offset: usize) -> u32 {
        unsafe { read_volatile((self.base_addr + offset) as *const u32) }
    }

    fn write_reg(&self, offset: usize, value: u32) {
        unsafe { write_volatile((self.base_addr + offset) as *mut u32, value) }
    }

    fn read_reg16(&self, offset: usize) -> u16 {
        unsafe { read_volatile((self.base_addr + offset) as *const u16) }
    }

    fn write_reg16(&self, offset: usize, value: u16) {
        unsafe { write_volatile((self.base_addr + offset) as *mut u16, value) }
    }

    fn software_reset(&self) -> Result<(), VFatError> {
        // Reset all
        self.write_reg(regs::SOFTWARE_RESET, 0x01);

        // Wait for reset to complete (bit clears when done)
        let timeout = 10000;
        for _ in 0..timeout {
            if self.read_reg(regs::SOFTWARE_RESET) & 0x01 == 0 {
                return Ok(());
            }
            self.delay_us(10);
        }

        Err(VFatError::io(IoError::timeout()))
    }

    fn card_init(&mut self) -> Result<(), VFatError> {
        // Simplified card initialization
        // In a real implementation, this would follow the SD spec exactly
        
        // Power on
        self.write_reg(regs::POWER_CONTROL, 0x0F); // 3.3V

        // Set clock to initialization frequency (400 KHz)
        self.set_clock(400_000)?;

        // Send CMD0 - Go idle state
        self.send_command(SdCommand::GoIdleState as u8, 0, CommandFlags::NoResponse as u16)?;

        // Send CMD8 - Send interface condition (SD 2.0+)
        let arg = 0x1AA; // VHS=1 (2.7-3.6V), check pattern=0xAA
        let result = self.send_command(SdCommand::SendIfCond as u8, arg, 
            (CommandFlags::Response48 as u16) | (CommandFlags::CrcCheck as u16));

        let sd_v2 = result.is_ok();

        // ACMD41 - Send OCR (operating condition register)
        // This is the initialization loop
        let mut ocr: u32 = 0;
        for _ in 0..1000 {
            // Send CMD55 (APP_CMD) first
            self.send_command(SdCommand::AppCmd as u8, 0, 
                (CommandFlags::Response48 as u16) | (CommandFlags::CrcCheck as u16))?;
            
            // Then ACMD41
            let acmd41_arg: u32 = if sd_v2 { 0x40300000 } else { 0x00300000 };
            let resp = self.send_command(41, acmd41_arg,
                (CommandFlags::Response48 as u16) | (CommandFlags::CrcCheck as u16));
            
            if let Ok(_r) = resp {
                ocr = self.read_reg(regs::RESPONSE_0);
                if ocr & 0x80000000 != 0 {
                    // Card is ready
                    break;
                }
            }
            
            self.delay_us(1000);
        }

        if ocr & 0x80000000 == 0 {
            return Err(VFatError::io(IoError::device_error("SD card initialization failed")));
        }

        // Check for high capacity card
        self.high_capacity = (ocr & 0x40000000) != 0;

        // Send CMD2 - Get CID
        self.send_command(SdCommand::AllSendCid as u8, 0,
            (CommandFlags::Response136 as u16) | (CommandFlags::CrcCheck as u16))?;

        // Send CMD3 - Get RCA (relative card address)
        self.send_command(SdCommand::SendRelativeAddr as u8, 0,
            (CommandFlags::Response48 as u16) | (CommandFlags::CrcCheck as u16))?;

        let rca = self.read_reg(regs::RESPONSE_0) >> 16;

        // Send CMD9 - Get CSD
        self.send_command(SdCommand::SendCsd as u8, rca << 16,
            (CommandFlags::Response136 as u16) | (CommandFlags::CrcCheck as u16))?;

        // Select the card
        self.send_command(SdCommand::SelectCard as u8, rca << 16,
            (CommandFlags::Response48Busy as u16) | (CommandFlags::CrcCheck as u16))?;

        // Set clock to high speed (25 MHz for SD, 50 MHz for high speed)
        self.set_clock(25_000_000)?;

        // Enable 4-bit mode (optional optimization)
        // This would require ACMD6

        Ok(())
    }

    fn read_card_capacity(&mut self) -> Result<(), VFatError> {
        // Parse CSD register to get capacity
        // This is simplified - full implementation would parse CSD v1 and v2
        
        if self.high_capacity {
            // CSD version 2.0 (SDHC/SDXC)
            // Capacity = (C_SIZE + 1) * 512K bytes
            // For now, use a reasonable default
            self.capacity_sectors = 15_625_000; // ~8GB card
        } else {
            // CSD version 1.0 (standard SD)
            self.capacity_sectors = 1_966_080; // ~1GB card
        }

        Ok(())
    }

    fn set_block_size(&self, size: u16) -> Result<(), VFatError> {
        self.send_command(SdCommand::SetBlocklen as u8, size as u32,
            (CommandFlags::Response48 as u16) | (CommandFlags::CrcCheck as u16))?;
        Ok(())
    }

    fn set_clock(&self, _freq: u32) -> Result<(), VFatError> {
        // Simplified clock setup
        // In a real implementation, this would calculate and set the appropriate divider
        
        // Stop SD clock
        self.write_reg16(regs::CLOCK_CONTROL, 0);
        self.delay_us(10);

        // Set internal clock enable and SD clock enable
        // Assuming base clock is 100MHz
        let divider = 1; // Simplified - should calculate based on freq
        self.write_reg16(regs::CLOCK_CONTROL, 0x0001 | (divider << 8));
        self.delay_us(10);

        // Wait for clock stable
        let timeout = 10000;
        for _ in 0..timeout {
            if self.read_reg16(regs::CLOCK_CONTROL) & 0x0002 != 0 {
                // Enable SD clock
                self.write_reg16(regs::CLOCK_CONTROL, 
                    self.read_reg16(regs::CLOCK_CONTROL) | 0x0004);
                return Ok(());
            }
            self.delay_us(10);
        }

        Err(VFatError::io(IoError::timeout()))
    }

    fn send_command(&self, cmd: u8, arg: u32, flags: u16) -> Result<(), VFatError> {
        // Wait for command line to be free
        let timeout = 100000;
        for _ in 0..timeout {
            if self.read_reg(regs::PRESENT_STATE) & PresentState::CmdInhibit as u32 == 0 {
                break;
            }
            self.delay_us(1);
        }

        // Clear interrupt status
        self.write_reg16(regs::NORMAL_INT_STATUS, 0xFFFF);
        self.write_reg16(regs::ERROR_INT_STATUS, 0xFFFF);

        // Set argument
        self.write_reg(regs::ARGUMENT, arg);

        // Send command
        let cmd_reg = ((cmd as u16) << 8) | flags;
        self.write_reg16(regs::COMMAND, cmd_reg);

        // Wait for command complete
        let timeout = 100000;
        for _ in 0..timeout {
            let status = self.read_reg16(regs::NORMAL_INT_STATUS);
            
            if status & NormalIntStatus::CommandComplete as u16 != 0 {
                // Clear command complete bit
                self.write_reg16(regs::NORMAL_INT_STATUS, NormalIntStatus::CommandComplete as u16);
                return Ok(());
            }

            let error_status = self.read_reg16(regs::ERROR_INT_STATUS);
            if error_status != 0 {
                // Clear error bits
                self.write_reg16(regs::ERROR_INT_STATUS, error_status);
                return Err(VFatError::io(IoError::other(
                    &format!("SD command error: 0x{:04X}", error_status),
                )));
            }

            self.delay_us(1);
        }

        Err(VFatError::io(IoError::timeout()))
    }

    fn read_sector_internal(&self, sector: u64, buffer: &mut [u8]) -> Result<(), VFatError> {
        // Convert sector to byte address for standard capacity cards
        let address = if self.high_capacity { sector } else { sector * 512 };

        // Wait for data line to be free
        let timeout = 100000;
        for _ in 0..timeout {
            if self.read_reg(regs::PRESENT_STATE) & PresentState::DatInhibit as u32 == 0 {
                break;
            }
            self.delay_us(1);
        }

        // Clear interrupt status
        self.write_reg16(regs::NORMAL_INT_STATUS, 0xFFFF);
        self.write_reg16(regs::ERROR_INT_STATUS, 0xFFFF);

        // Set block size and count
        self.write_reg16(regs::BLOCK_SIZE, 512);
        self.write_reg16(regs::BLOCK_COUNT, 1);

        // Set argument
        self.write_reg(regs::ARGUMENT, address as u32);

        // Send READ_SINGLE_BLOCK command (CMD17)
        let transfer_mode = TransferMode::Read as u16;
        let cmd_reg = ((SdCommand::ReadSingleBlock as u16) << 8) 
            | (CommandFlags::Response48 as u16)
            | (CommandFlags::CrcCheck as u16)
            | (CommandFlags::IndexCheck as u16)
            | (CommandFlags::DataPresent as u16);
        
        self.write_reg16(regs::TRANSFER_MODE, transfer_mode);
        self.write_reg16(regs::COMMAND, cmd_reg);

        // Read data from buffer
        let mut words_read = 0;
        let words_to_read = 512 / 4;
        
        while words_read < words_to_read {
            // Wait for buffer read ready or transfer complete
            let timeout = 100000;
            let mut ready = false;
            
            for _ in 0..timeout {
                let status = self.read_reg16(regs::NORMAL_INT_STATUS);
                
                if status & NormalIntStatus::BufferReadReady as u16 != 0 {
                    ready = true;
                    // Clear buffer read ready
                    self.write_reg16(regs::NORMAL_INT_STATUS, NormalIntStatus::BufferReadReady as u16);
                    break;
                }

                if status & NormalIntStatus::TransferComplete as u16 != 0 {
                    // Clear transfer complete
                    self.write_reg16(regs::NORMAL_INT_STATUS, NormalIntStatus::TransferComplete as u16);
                    break;
                }

                let error_status = self.read_reg16(regs::ERROR_INT_STATUS);
                if error_status != 0 {
                    self.write_reg16(regs::ERROR_INT_STATUS, error_status);
                    return Err(VFatError::io(IoError::other(
                        &format!("SD read error: 0x{:04X}", error_status),
                    )));
                }

                self.delay_us(1);
            }

            if !ready && words_read < words_to_read {
                return Err(VFatError::io(IoError::timeout()));
            }

            // Read up to 512 bytes from buffer port
            while words_read < words_to_read && self.read_reg(regs::PRESENT_STATE) & PresentState::BufferReadEnable as u32 != 0 {
                let word = self.read_reg(regs::BUFFER_DATA_PORT);
                let offset = words_read * 4;
                buffer[offset] = (word & 0xFF) as u8;
                buffer[offset + 1] = ((word >> 8) & 0xFF) as u8;
                buffer[offset + 2] = ((word >> 16) & 0xFF) as u8;
                buffer[offset + 3] = ((word >> 24) & 0xFF) as u8;
                words_read += 1;
            }
        }

        // Wait for transfer complete
        let timeout = 100000;
        for _ in 0..timeout {
            let status = self.read_reg16(regs::NORMAL_INT_STATUS);
            if status & NormalIntStatus::TransferComplete as u16 != 0 {
                self.write_reg16(regs::NORMAL_INT_STATUS, NormalIntStatus::TransferComplete as u16);
                break;
            }

            let error_status = self.read_reg16(regs::ERROR_INT_STATUS);
            if error_status != 0 {
                self.write_reg16(regs::ERROR_INT_STATUS, error_status);
                return Err(VFatError::io(IoError::other(
                    &format!("SD read completion error: 0x{:04X}", error_status),
                )));
            }

            self.delay_us(1);
        }

        Ok(())
    }

    fn write_sector_internal(&self, sector: u64, buffer: &[u8]) -> Result<(), VFatError> {
        // Check write protection
        if self.is_write_protected() {
            return Err(VFatError::io(IoError::permission_denied(
                "SD card is write-protected",
            )));
        }

        // Convert sector to byte address for standard capacity cards
        let address = if self.high_capacity { sector } else { sector * 512 };

        // Wait for data line to be free
        let timeout = 100000;
        for _ in 0..timeout {
            if self.read_reg(regs::PRESENT_STATE) & PresentState::DatInhibit as u32 == 0 {
                break;
            }
            self.delay_us(1);
        }

        // Clear interrupt status
        self.write_reg16(regs::NORMAL_INT_STATUS, 0xFFFF);
        self.write_reg16(regs::ERROR_INT_STATUS, 0xFFFF);

        // Set block size and count
        self.write_reg16(regs::BLOCK_SIZE, 512);
        self.write_reg16(regs::BLOCK_COUNT, 1);

        // Set argument
        self.write_reg(regs::ARGUMENT, address as u32);

        // Send WRITE_SINGLE_BLOCK command (CMD24)
        let transfer_mode = 0; // Write direction (bit 4 = 0)
        let cmd_reg = ((SdCommand::WriteSingleBlock as u16) << 8)
            | (CommandFlags::Response48 as u16)
            | (CommandFlags::CrcCheck as u16)
            | (CommandFlags::IndexCheck as u16)
            | (CommandFlags::DataPresent as u16);

        self.write_reg16(regs::TRANSFER_MODE, transfer_mode);
        self.write_reg16(regs::COMMAND, cmd_reg);

        // Write data to buffer
        let mut words_written = 0;
        let words_to_write = 512 / 4;

        while words_written < words_to_write {
            // Wait for buffer write ready
            let timeout = 100000;
            let mut ready = false;

            for _ in 0..timeout {
                let status = self.read_reg16(regs::NORMAL_INT_STATUS);

                if status & NormalIntStatus::BufferWriteReady as u16 != 0 {
                    ready = true;
                    // Clear buffer write ready
                    self.write_reg16(regs::NORMAL_INT_STATUS, NormalIntStatus::BufferWriteReady as u16);
                    break;
                }

                if status & NormalIntStatus::TransferComplete as u16 != 0 {
                    // Clear transfer complete
                    self.write_reg16(regs::NORMAL_INT_STATUS, NormalIntStatus::TransferComplete as u16);
                    break;
                }

                let error_status = self.read_reg16(regs::ERROR_INT_STATUS);
                if error_status != 0 {
                    self.write_reg16(regs::ERROR_INT_STATUS, error_status);
                    return Err(VFatError::io(IoError::other(
                        &format!("SD write error: 0x{:04X}", error_status),
                    )));
                }

                self.delay_us(1);
            }

            if !ready && words_written < words_to_write {
                return Err(VFatError::io(IoError::timeout()));
            }

            // Write up to 512 bytes to buffer port
            while words_written < words_to_write && self.read_reg(regs::PRESENT_STATE) & PresentState::BufferWriteEnable as u32 != 0 {
                let offset = words_written * 4;
                let word = buffer[offset] as u32
                    | ((buffer[offset + 1] as u32) << 8)
                    | ((buffer[offset + 2] as u32) << 16)
                    | ((buffer[offset + 3] as u32) << 24);
                self.write_reg(regs::BUFFER_DATA_PORT, word);
                words_written += 1;
            }
        }

        // Wait for transfer complete
        let timeout = 100000;
        for _ in 0..timeout {
            let status = self.read_reg16(regs::NORMAL_INT_STATUS);
            if status & NormalIntStatus::TransferComplete as u16 != 0 {
                self.write_reg16(regs::NORMAL_INT_STATUS, NormalIntStatus::TransferComplete as u16);
                break;
            }

            let error_status = self.read_reg16(regs::ERROR_INT_STATUS);
            if error_status != 0 {
                self.write_reg16(regs::ERROR_INT_STATUS, error_status);
                return Err(VFatError::io(IoError::device_error(&format!("SD write completion error: 0x{:04X}", error_status))));
            }

            self.delay_us(1);
        }

        Ok(())
    }

    fn delay_us(&self, us: u32) {
        // Simple busy-wait delay
        // In a real implementation, this would use a timer
        for _ in 0..us * 10 {
            #[cfg(target_arch = "aarch64")]
            unsafe { core::arch::aarch64::__nop() };
            #[cfg(target_arch = "x86_64")]
            unsafe { core::arch::x86_64::_mm_pause() };
        }
    }
}

unsafe impl Send for SdBlockDevice {}
unsafe impl Sync for SdBlockDevice {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transfer_mode_flags() {
        assert_eq!(TransferMode::DmaEnable as u16, 0x0001);
        assert_eq!(TransferMode::Read as u16, 0x0010);
    }

    #[test]
    fn test_command_flags() {
        assert_eq!(CommandFlags::Response48 as u16, 0x0002);
        assert_eq!(CommandFlags::CrcCheck as u16, 0x0008);
    }
}
