//! SDIO Host Controller Driver
//!
//! Implementation of the SD Host Controller Interface for Raspberry Pi.
//! Supports SD/SDIO card communication using CMD52 and CMD53 commands.
//!
//! On Raspberry Pi 3/4, the WiFi chip (BCM43438/BCM43455) is connected via SDIO
//! to the Arasan SDHCI controller.

use crate::drivers::DriverError;
use crate::println;
use core::sync::atomic::{fence, Ordering};

// SDIO Register offsets (Arasan SDHCI controller)
const SDHCI_DMA_ADDRESS: usize = 0x00;
const SDHCI_BLOCK_SIZE: usize = 0x04;
const SDHCI_BLOCK_COUNT: usize = 0x06;
const SDHCI_ARGUMENT: usize = 0x08;
const SDHCI_TRANSFER_MODE: usize = 0x0C;
const SDHCI_COMMAND: usize = 0x0E;
const SDHCI_RESPONSE: usize = 0x10; // 0x10-0x1F (4 words)
const SDHCI_BUFFER: usize = 0x20;
const SDHCI_PRESENT_STATE: usize = 0x24;
const SDHCI_HOST_CONTROL: usize = 0x28;
const SDHCI_POWER_CONTROL: usize = 0x29;
const SDHCI_BLOCK_GAP_CONTROL: usize = 0x2A;
const SDHCI_WAKE_UP_CONTROL: usize = 0x2B;
const SDHCI_CLOCK_CONTROL: usize = 0x2C;
const SDHCI_TIMEOUT_CONTROL: usize = 0x2E;
const SDHCI_SOFTWARE_RESET: usize = 0x2F;
const SDHCI_INT_STATUS: usize = 0x30;
const SDHCI_INT_ENABLE: usize = 0x34;
const SDHCI_SIGNAL_ENABLE: usize = 0x38;
const SDHCI_ACMD12_ERR: usize = 0x3C;
const SDHCI_HOST_CONTROL2: usize = 0x3E;
const SDHCI_CAPABILITIES: usize = 0x40;
const SDHCI_CAPABILITIES_1: usize = 0x44;
const SDHCI_MAX_CURRENT: usize = 0x48;
const SDHCI_FORCE_EVENT: usize = 0x50;
const SDHCI_ADMA_ERROR: usize = 0x54;
const SDHCI_ADMA_ADDRESS: usize = 0x58;
const SDHCI_ADMA_ADDRESS_HI: usize = 0x5C;
const SDHCI_PRESET_VALUE: usize = 0x60;
const SDHCI_HOST_VERSION: usize = 0xFE;

// Transfer mode flags
const SDHCI_TRNS_DMA: u16 = 0x0001;
const SDHCI_TRNS_BLK_CNT_EN: u16 = 0x0002;
const SDHCI_TRNS_AUTO_CMD12: u16 = 0x0004;
const SDHCI_TRNS_AUTO_CMD23: u16 = 0x0008;
const SDHCI_TRNS_READ: u16 = 0x0010;
const SDHCI_TRNS_MULTI: u16 = 0x0020;

// Command flags
const SDHCI_CMD_RESP_MASK: u16 = 0x0003;
const SDHCI_CMD_CRC: u16 = 0x0008;
const SDHCI_CMD_INDEX: u16 = 0x0010;
const SDHCI_CMD_DATA: u16 = 0x0020;
const SDHCI_CMD_ABORTCMD: u16 = 0x00C0;
const SDHCI_CMD_RESP_NONE: u16 = 0x0000;
const SDHCI_CMD_RESP_LONG: u16 = 0x0001;
const SDHCI_CMD_RESP_SHORT: u16 = 0x0002;
const SDHCI_CMD_RESP_SHORT_BUSY: u16 = 0x0003;

// Present state flags
const SDHCI_CMD_INHIBIT: u32 = 0x00000001;
const SDHCI_DATA_INHIBIT: u32 = 0x00000002;
const SDHCI_DOING_WRITE: u32 = 0x00000100;
const SDHCI_DOING_READ: u32 = 0x00000200;
const SDHCI_SPACE_AVAILABLE: u32 = 0x00000400;
const SDHCI_DATA_AVAILABLE: u32 = 0x00000800;
const SDHCI_CARD_PRESENT: u32 = 0x00010000;
const SDHCI_CARD_STATE_STABLE: u32 = 0x00020000;
const SDHCI_CARD_DETECT_PIN_LEVEL: u32 = 0x00040000;
const SDHCI_WRITE_PROTECT: u32 = 0x00080000;

// Interrupt status flags
const SDHCI_INT_RESPONSE: u32 = 0x00000001;
const SDHCI_INT_DATA_END: u32 = 0x00000002;
const SDHCI_INT_BLK_GAP: u32 = 0x00000004;
const SDHCI_INT_DMA_END: u32 = 0x00000008;
const SDHCI_INT_SPACE_AVAIL: u32 = 0x00000010;
const SDHCI_INT_DATA_AVAIL: u32 = 0x00000020;
const SDHCI_INT_CARD_INSERT: u32 = 0x00000040;
const SDHCI_INT_CARD_REMOVE: u32 = 0x00000080;
const SDHCI_INT_CARD_INT: u32 = 0x00000100;
const SDHCI_INT_ERROR: u32 = 0x00008000;
const SDHCI_INT_TIMEOUT: u32 = 0x00010000;
const SDHCI_INT_CRC: u32 = 0x00020000;
const SDHCI_INT_END_BIT: u32 = 0x00040000;
const SDHCI_INT_INDEX: u32 = 0x00080000;
const SDHCI_INT_DATA_TIMEOUT: u32 = 0x00100000;
const SDHCI_INT_DATA_CRC: u32 = 0x00200000;
const SDHCI_INT_DATA_END_BIT: u32 = 0x00400000;
const SDHCI_INT_BUS_POWER: u32 = 0x00800000;
const SDHCI_INT_ACMD12ERR: u32 = 0x01000000;

// Clock control flags
const SDHCI_DIVIDER_SHIFT: u16 = 8;
const SDHCI_DIVIDER_HI_SHIFT: u16 = 6;
const SDHCI_DIV_MASK: u16 = 0xFF;
const SDHCI_DIV_MASK_LEN: u16 = 8;
const SDHCI_DIV_HI_MASK: u16 = 0x300;
const SDHCI_CLOCK_CARD_EN: u16 = 0x0004;
const SDHCI_CLOCK_INT_STABLE: u16 = 0x0002;
const SDHCI_CLOCK_INT_EN: u16 = 0x0001;

// Software reset flags
const SDHCI_RESET_ALL: u8 = 0x01;
const SDHCI_RESET_CMD: u8 = 0x02;
const SDHCI_RESET_DATA: u8 = 0x04;

// Power control flags
const SDHCI_POWER_ON: u8 = 0x01;
const SDHCI_POWER_180: u8 = 0x0A;
const SDHCI_POWER_300: u8 = 0x0C;
const SDHCI_POWER_330: u8 = 0x0E;

// SDIO-specific commands
const SD_CMD_GO_IDLE_STATE: u8 = 0;
const SD_CMD_SEND_RELATIVE_ADDR: u8 = 3;
const SD_CMD_IO_SEND_OP_COND: u8 = 5;
const SD_CMD_SELECT_CARD: u8 = 7;
const SD_CMD_SEND_IF_COND: u8 = 8;
const SD_CMD_SEND_CSD: u8 = 9;
const SD_CMD_STOP_TRANSMISSION: u8 = 12;
const SD_CMD_SET_BLOCKLEN: u8 = 16;
const SD_CMD_READ_SINGLE_BLOCK: u8 = 17;
const SD_CMD_WRITE_SINGLE_BLOCK: u8 = 24;
const SD_CMD_IO_RW_DIRECT: u8 = 52;   // CMD52 - single register access
const SD_CMD_IO_RW_EXTENDED: u8 = 53; // CMD53 - multi-byte/block access

// SDIO argument construction for CMD52
fn make_cmd52_arg(write: bool, function: u8, address: u32, data: u8, raw: bool) -> u32 {
    let mut arg: u32 = 0;
    if write {
        arg |= 0x80000000;
    }
    if raw && write {
        arg |= 0x08000000;
    }
    arg |= ((function & 0x07) as u32) << 28;
    arg |= (address & 0x1FFFF) << 9;
    arg |= (data as u32) & 0xFF;
    arg
}

// SDIO argument construction for CMD53
fn make_cmd53_arg(write: bool, function: u8, block_mode: bool, opcode: bool, address: u32, count: u16) -> u32 {
    let mut arg: u32 = 0;
    if write {
        arg |= 0x80000000;
    }
    if block_mode {
        arg |= 0x08000000;
    }
    if opcode {
        arg |= 0x04000000;
    }
    arg |= ((function & 0x07) as u32) << 28;
    arg |= (address & 0x1FFFF) << 9;
    arg |= (count as u32) & 0x1FF;
    if block_mode && count > 511 {
        arg |= ((count as u32) & 0x7FFF) << 0;
    }
    arg
}

/// SDIO base address for Pi 3 (BCM2837)
const SDIO_BASE_PI3: usize = 0x3F300000;
/// SDIO base address for Pi 4 (BCM2711)
const SDIO_BASE_PI4: usize = 0xFE300000;

/// SDHCI controller state
pub struct SdhciController {
    base_addr: usize,
    is_pi4: bool,
    max_clock: u32,
}

impl SdhciController {
    /// Create a new SDHCI controller instance
    /// 
    /// # Safety
    /// Caller must ensure the base address is valid and mapped
    pub unsafe fn new(pi4: bool) -> Self {
        let base = if pi4 { SDIO_BASE_PI4 } else { SDIO_BASE_PI3 };
        
        Self {
            base_addr: base,
            is_pi4: pi4,
            max_clock: 50_000_000, // 50 MHz default
        }
    }

    /// Read 8-bit register
    #[inline]
    unsafe fn read8(&self, offset: usize) -> u8 {
        core::ptr::read_volatile((self.base_addr + offset) as *const u8)
    }

    /// Read 16-bit register
    #[inline]
    unsafe fn read16(&self, offset: usize) -> u16 {
        core::ptr::read_volatile((self.base_addr + offset) as *const u16)
    }

    /// Read 32-bit register
    #[inline]
    unsafe fn read32(&self, offset: usize) -> u32 {
        core::ptr::read_volatile((self.base_addr + offset) as *const u32)
    }

    /// Write 8-bit register
    #[inline]
    unsafe fn write8(&self, offset: usize, val: u8) {
        core::ptr::write_volatile((self.base_addr + offset) as *mut u8, val);
    }

    /// Write 16-bit register
    #[inline]
    unsafe fn write16(&self, offset: usize, val: u16) {
        core::ptr::write_volatile((self.base_addr + offset) as *mut u16, val);
    }

    /// Write 32-bit register
    #[inline]
    unsafe fn write32(&self, offset: usize, val: u32) {
        core::ptr::write_volatile((self.base_addr + offset) as *mut u32, val);
    }

    /// Wait for command line to be free
    unsafe fn wait_for_cmd(&self, timeout_ms: u32) -> Result<(), DriverError> {
        let mut timeout = timeout_ms * 1000;
        while (self.read32(SDHCI_PRESENT_STATE) & SDHCI_CMD_INHIBIT) != 0 {
            if timeout == 0 {
                return Err(DriverError::Timeout);
            }
            timeout -= 1;
            core::arch::asm!("nop");
        }
        Ok(())
    }

    /// Wait for data line to be free
    unsafe fn wait_for_data(&self, timeout_ms: u32) -> Result<(), DriverError> {
        let mut timeout = timeout_ms * 1000;
        while (self.read32(SDHCI_PRESENT_STATE) & SDHCI_DATA_INHIBIT) != 0 {
            if timeout == 0 {
                return Err(DriverError::Timeout);
            }
            timeout -= 1;
            core::arch::asm!("nop");
        }
        Ok(())
    }

    /// Reset the controller
    pub fn reset(&self) -> Result<(), DriverError> {
        unsafe {
            // Reset all
            self.write8(SDHCI_SOFTWARE_RESET, SDHCI_RESET_ALL);
            
            let mut timeout = 10000;
            while self.read8(SDHCI_SOFTWARE_RESET) != 0 {
                if timeout == 0 {
                    return Err(DriverError::Timeout);
                }
                timeout -= 1;
            }
        }
        Ok(())
    }

    /// Set the clock frequency
    pub fn set_clock(&mut self, freq: u32) -> Result<(), DriverError> {
        unsafe {
            // Disable clock first
            self.write16(SDHCI_CLOCK_CONTROL, 0);
            
            // Calculate divider (base clock is typically 50MHz on Pi)
            let base_clock = 50_000_000u32;
            let mut divider = (base_clock + freq - 1) / freq;
            
            if divider > 1 {
                divider = if divider % 2 == 0 { divider - 1 } else { divider };
            } else {
                divider = 0;
            }
            
            let div_hi = ((divider >> SDHCI_DIVIDER_HI_SHIFT) as u16) << SDHCI_DIVIDER_SHIFT;
            let div_lo = ((divider & SDHCI_DIV_MASK as u32) as u16) << SDHCI_DIVIDER_SHIFT;
            
            // Enable internal clock
            let clk = div_lo | SDHCI_CLOCK_INT_EN | div_hi;
            self.write16(SDHCI_CLOCK_CONTROL, clk);
            
            // Wait for clock to stabilize
            let mut timeout = 10000;
            while (self.read16(SDHCI_CLOCK_CONTROL) & SDHCI_CLOCK_INT_STABLE) == 0 {
                if timeout == 0 {
                    return Err(DriverError::Timeout);
                }
                timeout -= 1;
            }
            
            // Enable card clock
            self.write16(SDHCI_CLOCK_CONTROL, clk | SDHCI_CLOCK_CARD_EN);
        }
        
        Ok(())
    }

    /// Set bus power
    pub fn set_power(&self, voltage: u8) -> Result<(), DriverError> {
        unsafe {
            self.write8(SDHCI_POWER_CONTROL, voltage | SDHCI_POWER_ON);
            
            // Wait for power to stabilize
            let mut timeout = 10000;
            while (self.read8(SDHCI_POWER_CONTROL) & SDHCI_POWER_ON) == 0 {
                if timeout == 0 {
                    return Err(DriverError::Timeout);
                }
                timeout -= 1;
            }
        }
        Ok(())
    }

    /// Check if card is present
    pub fn card_present(&self) -> bool {
        unsafe {
            (self.read32(SDHCI_PRESENT_STATE) & SDHCI_CARD_PRESENT) != 0
        }
    }

    /// Wait for interrupt status
    unsafe fn wait_for_int(&self, mask: u32, timeout_ms: u32) -> Result<u32, DriverError> {
        let mut timeout = timeout_ms * 1000;
        let mut status;
        
        loop {
            status = self.read32(SDHCI_INT_STATUS);
            if (status & mask) != 0 {
                break;
            }
            if (status & SDHCI_INT_ERROR) != 0 {
                return Err(DriverError::IoError);
            }
            if timeout == 0 {
                return Err(DriverError::Timeout);
            }
            timeout -= 1;
        }
        
        // Clear the interrupts
        self.write32(SDHCI_INT_STATUS, status & mask);
        
        Ok(status)
    }

    /// Send a command
    unsafe fn send_command(&self, cmd: u8, arg: u32, flags: u16) -> Result<(), DriverError> {
        self.wait_for_cmd(1000)?;
        
        if (flags & SDHCI_CMD_DATA) != 0 {
            self.wait_for_data(1000)?;
        }
        
        // Clear interrupt status
        self.write32(SDHCI_INT_STATUS, !0);
        
        // Set argument
        self.write32(SDHCI_ARGUMENT, arg);
        
        // Send command
        let cmd_val = ((cmd as u16) << 8) | flags;
        self.write16(SDHCI_COMMAND, cmd_val);
        
        // Wait for command complete
        self.wait_for_int(SDHCI_INT_RESPONSE, 1000)?;
        
        Ok(())
    }

    /// Get response from last command
    unsafe fn get_response(&self) -> [u32; 4] {
        [
            self.read32(SDHCI_RESPONSE),
            self.read32(SDHCI_RESPONSE + 4),
            self.read32(SDHCI_RESPONSE + 8),
            self.read32(SDHCI_RESPONSE + 12),
        ]
    }

    /// Initialize the controller
    pub fn init(&mut self) -> Result<(), DriverError> {
        println!("[sdio] Initializing SDHCI controller at {:08X}", self.base_addr);
        
        // Reset controller
        self.reset()?;
        
        // Read capabilities
        let caps = unsafe { self.read32(SDHCI_CAPABILITIES) };
        let max_block_len = 1 << (((caps >> 16) & 0x3) + 9);
        println!("[sdio] Max block length: {}", max_block_len);
        
        // Set power (3.3V)
        self.set_power(SDHCI_POWER_330)?;
        
        // Set initial clock (400KHz for initialization)
        self.set_clock(400_000)?;
        
        // Enable interrupts
        unsafe {
            self.write32(SDHCI_INT_ENABLE, !0);
            self.write32(SDHCI_SIGNAL_ENABLE, !0);
        }
        
        println!("[sdio] SDHCI controller initialized");
        Ok(())
    }

    /// Go idle state (CMD0)
    pub fn go_idle(&self) -> Result<(), DriverError> {
        unsafe {
            self.send_command(
                SD_CMD_GO_IDLE_STATE,
                0,
                SDHCI_CMD_RESP_NONE
            )
        }
    }

    /// Send interface condition (CMD8)
    pub fn send_if_cond(&self) -> Result<u32, DriverError> {
        unsafe {
            self.send_command(
                SD_CMD_SEND_IF_COND,
                0x1AA, // VHS=1, check pattern=0xAA
                SDHCI_CMD_RESP_SHORT | SDHCI_CMD_CRC | SDHCI_CMD_INDEX
            )?;
            
            let resp = self.get_response();
            Ok(resp[0])
        }
    }

    /// IO send operation condition (CMD5) - SDIO specific
    pub fn io_send_op_cond(&self, ocr: u32) -> Result<u32, DriverError> {
        unsafe {
            self.send_command(
                SD_CMD_IO_SEND_OP_COND,
                ocr,
                SDHCI_CMD_RESP_SHORT | SDHCI_CMD_CRC
            )?;
            
            let resp = self.get_response();
            Ok(resp[0])
        }
    }

    /// Send relative address (CMD3)
    pub fn send_relative_addr(&self) -> Result<u32, DriverError> {
        unsafe {
            self.send_command(
                SD_CMD_SEND_RELATIVE_ADDR,
                0,
                SDHCI_CMD_RESP_SHORT | SDHCI_CMD_CRC | SDHCI_CMD_INDEX
            )?;
            
            let resp = self.get_response();
            Ok(resp[0])
        }
    }

    /// Select card (CMD7)
    pub fn select_card(&self, rca: u32) -> Result<(), DriverError> {
        unsafe {
            self.send_command(
                SD_CMD_SELECT_CARD,
                rca << 16,
                SDHCI_CMD_RESP_SHORT_BUSY | SDHCI_CMD_CRC | SDHCI_CMD_INDEX
            )
        }
    }

    /// CMD52 - IO Read/Write Direct
    /// 
    /// Read or write a single byte to/from an SDIO function
    pub fn cmd52(&self, write: bool, function: u8, address: u32, data: u8) -> Result<u8, DriverError> {
        let arg = make_cmd52_arg(write, function, address, data, false);
        
        unsafe {
            self.send_command(
                SD_CMD_IO_RW_DIRECT,
                arg,
                SDHCI_CMD_RESP_SHORT | SDHCI_CMD_CRC | SDHCI_CMD_INDEX
            )?;
            
            let resp = self.get_response();
            // Response contains data in bits 15:8
            Ok(((resp[0] >> 8) & 0xFF) as u8)
        }
    }

    /// CMD53 - IO Read/Write Extended
    /// 
    /// Read or write multiple bytes/blocks to/from an SDIO function
    pub fn cmd53(
        &self,
        write: bool,
        function: u8,
        address: u32,
        block_mode: bool,
        increment: bool,
        count: u16,
        buffer: *mut u8
    ) -> Result<(), DriverError> {
        let arg = make_cmd53_arg(write, function, block_mode, increment, address, count);
        
        let block_size = if block_mode { 512 } else { 1 };
        let byte_count = if block_mode { count as usize * 512 } else { count as usize };
        
        unsafe {
            // Set block size and count
            self.write16(SDHCI_BLOCK_SIZE, block_size as u16);
            self.write16(SDHCI_BLOCK_COUNT, count);
            
            // Set DMA address if using DMA
            let phys_addr = crate::mm::virt_to_phys_u64(buffer as u64);
            self.write32(SDHCI_DMA_ADDRESS, phys_addr as u32);
            
            // Set transfer mode
            let mut mode: u16 = SDHCI_TRNS_BLK_CNT_EN;
            if write {
                mode |= SDHCI_TRNS_DMA;
            } else {
                mode |= SDHCI_TRNS_DMA | SDHCI_TRNS_READ;
            }
            if count > 1 {
                mode |= SDHCI_TRNS_MULTI;
            }
            self.write16(SDHCI_TRANSFER_MODE, mode);
            
            // Send command
            self.send_command(
                SD_CMD_IO_RW_EXTENDED,
                arg,
                SDHCI_CMD_RESP_SHORT | SDHCI_CMD_CRC | SDHCI_CMD_INDEX | SDHCI_CMD_DATA
            )?;
            
            // Wait for transfer complete
            self.wait_for_int(SDHCI_INT_DATA_END | SDHCI_INT_DMA_END, 5000)?;
            
            Ok(())
        }
    }

    /// Read a byte from an SDIO function
    pub fn read_byte(&self, function: u8, address: u32) -> Result<u8, DriverError> {
        self.cmd52(false, function, address, 0)
    }

    /// Write a byte to an SDIO function
    pub fn write_byte(&self, function: u8, address: u32, data: u8) -> Result<(), DriverError> {
        self.cmd52(true, function, address, data)?;
        Ok(())
    }

    /// Read multiple bytes from an SDIO function
    pub fn read_bytes(&self, function: u8, address: u32, buffer: &mut [u8]) -> Result<(), DriverError> {
        let count = buffer.len();
        
        if count <= 512 {
            // Use byte mode for small transfers
            self.cmd53(false, function, address, false, true, count as u16, buffer.as_mut_ptr())
        } else {
            // Use block mode for larger transfers
            let blocks = (count + 511) / 512;
            self.cmd53(false, function, address, true, true, blocks as u16, buffer.as_mut_ptr())
        }
    }

    /// Write multiple bytes to an SDIO function
    pub fn write_bytes(&self, function: u8, address: u32, buffer: &[u8]) -> Result<(), DriverError> {
        let count = buffer.len();
        
        if count <= 512 {
            // Use byte mode for small transfers
            self.cmd53(true, function, address, false, true, count as u16, buffer.as_ptr() as *mut u8)
        } else {
            // Use block mode for larger transfers
            let blocks = (count + 511) / 512;
            self.cmd53(true, function, address, true, true, blocks as u16, buffer.as_ptr() as *mut u8)
        }
    }

    /// Set bus width (4-bit mode)
    pub fn set_bus_width(&self, width: u8) -> Result<(), DriverError> {
        let host_control = unsafe { self.read8(SDHCI_HOST_CONTROL) };
        let new_control = match width {
            4 => (host_control & !0x06) | 0x02, // 4-bit mode
            8 => (host_control & !0x06) | 0x04, // 8-bit mode (not supported on Pi SDIO)
            _ => host_control & !0x06,          // 1-bit mode
        };
        unsafe { self.write8(SDHCI_HOST_CONTROL, new_control) };
        Ok(())
    }

    /// Set high speed mode
    pub fn set_high_speed(&self, enable: bool) -> Result<(), DriverError> {
        unsafe {
            let mut control = self.read8(SDHCI_HOST_CONTROL);
            if enable {
                control |= 0x04;
            } else {
                control &= !0x04;
            }
            self.write8(SDHCI_HOST_CONTROL, control);
        }
        Ok(())
    }
    
    /// Set block length (CMD16)
    pub fn set_blocklen(&self, blocklen: u32) -> Result<(), DriverError> {
        unsafe {
            self.send_command(
                SD_CMD_SET_BLOCKLEN,
                blocklen,
                SDHCI_CMD_RESP_SHORT | SDHCI_CMD_CRC | SDHCI_CMD_INDEX
            )
        }
    }
    
    /// Read single block (CMD17)
    pub fn read_single_block(&self, block_addr: u32, buffer: &mut [u8]) -> Result<(), DriverError> {
        unsafe {
            // Set block size
            self.write16(SDHCI_BLOCK_SIZE, 512);
            self.write16(SDHCI_BLOCK_COUNT, 1);
            
            // Set DMA address
            let phys_addr = crate::mm::virt_to_phys_u64(buffer.as_mut_ptr() as u64);
            self.write32(SDHCI_DMA_ADDRESS, phys_addr as u32);
            
            // Set transfer mode - read, single block
            let mode = SDHCI_TRNS_BLK_CNT_EN | SDHCI_TRNS_DMA | SDHCI_TRNS_READ;
            self.write16(SDHCI_TRANSFER_MODE, mode);
            
            // Send CMD17 - READ_SINGLE_BLOCK
            self.send_command(
                SD_CMD_READ_SINGLE_BLOCK,
                block_addr,
                SDHCI_CMD_RESP_SHORT | SDHCI_CMD_CRC | SDHCI_CMD_INDEX | SDHCI_CMD_DATA
            )?;
            
            // Wait for transfer complete
            self.wait_for_int(SDHCI_INT_DATA_END | SDHCI_INT_DMA_END, 5000)?;
            
            Ok(())
        }
    }
    
    /// Read multiple blocks (CMD18)
    pub fn read_multiple_blocks(&self, block_addr: u32, count: u16, buffer: &mut [u8]) -> Result<(), DriverError> {
        unsafe {
            // Set block size and count
            self.write16(SDHCI_BLOCK_SIZE, 512);
            self.write16(SDHCI_BLOCK_COUNT, count);
            
            // Set DMA address
            let phys_addr = crate::mm::virt_to_phys_u64(buffer.as_mut_ptr() as u64);
            self.write32(SDHCI_DMA_ADDRESS, phys_addr as u32);
            
            // Set transfer mode - read, multiple blocks
            let mode = SDHCI_TRNS_BLK_CNT_EN | SDHCI_TRNS_DMA | SDHCI_TRNS_READ | SDHCI_TRNS_MULTI;
            self.write16(SDHCI_TRANSFER_MODE, mode);
            
            // Send CMD18 - READ_MULTIPLE_BLOCK
            // Note: CMD18 constant not defined, using value 18 directly
            self.send_command(
                18, // CMD18
                block_addr,
                SDHCI_CMD_RESP_SHORT | SDHCI_CMD_CRC | SDHCI_CMD_INDEX | SDHCI_CMD_DATA
            )?;
            
            // Wait for transfer complete
            self.wait_for_int(SDHCI_INT_DATA_END | SDHCI_INT_DMA_END, 5000)?;
            
            Ok(())
        }
    }
}

// SAFETY: SDHCI controller is thread-safe when properly synchronized
unsafe impl Send for SdhciController {}
unsafe impl Sync for SdhciController {}

/// Global SDIO controller instance
static mut SDHCI_CONTROLLER: Option<SdhciController> = None;

/// Initialize the SDIO subsystem
pub fn init(pi4: bool) {
    println!("[sdio] Initializing SDIO subsystem...");
    
    unsafe {
        SDHCI_CONTROLLER = Some(SdhciController::new(pi4));
        
        if let Some(ref mut controller) = SDHCI_CONTROLLER {
            if let Err(e) = controller.init() {
                println!("[sdio] Failed to initialize controller: {:?}", e);
                SDHCI_CONTROLLER = None;
                return;
            }
        }
    }
    
    println!("[sdio] SDIO subsystem initialized");
}

/// Get the global SDHCI controller
pub fn controller() -> Option<&'static mut SdhciController> {
    unsafe { SDHCI_CONTROLLER.as_mut() }
}

/// Check if SDIO is initialized
pub fn is_initialized() -> bool {
    unsafe { SDHCI_CONTROLLER.is_some() }
}

/// Probe for SDIO card
pub fn probe_card() -> Result<(), DriverError> {
    let controller = controller().ok_or(DriverError::NotFound)?;
    
    // Check if card is present
    if !controller.card_present() {
        return Err(DriverError::NotFound);
    }
    
    println!("[sdio] Card detected");
    
    // Go idle
    controller.go_idle()?;
    
    // Send interface condition (for SD cards, may fail on pure SDIO)
    let _ = controller.send_if_cond();
    
    // Send IO op condition (CMD5) for SDIO
    let mut ocr = 0u32;
    loop {
        let response = controller.io_send_op_cond(ocr)?;
        if (response & 0x80000000) != 0 {
            // Card ready
            println!("[sdio] Card ready, OCR: {:08X}", response);
            break;
        }
        ocr = response;
    }
    
    // Get relative address
    let rca = controller.send_relative_addr()?;
    println!("[sdio] RCA: {:08X}", rca);
    
    // Select card
    controller.select_card(rca >> 16)?;
    
    // Set to high speed (50MHz)
    controller.set_clock(50_000_000)?;
    controller.set_high_speed(true)?;
    
    println!("[sdio] Card initialized");
    Ok(())
}

/// SDIO I/O function abstraction
pub struct SdioFunction {
    function_number: u8,
}

impl SdioFunction {
    /// Create new SDIO function handle
    pub const fn new(function: u8) -> Self {
        Self {
            function_number: function,
        }
    }

    /// Read a byte from this function
    pub fn read_byte(&self, address: u32) -> Result<u8, DriverError> {
        let controller = controller().ok_or(DriverError::NotFound)?;
        controller.read_byte(self.function_number, address)
    }

    /// Write a byte to this function
    pub fn write_byte(&self, address: u32, data: u8) -> Result<(), DriverError> {
        let controller = controller().ok_or(DriverError::NotFound)?;
        controller.write_byte(self.function_number, address, data)
    }

    /// Read multiple bytes from this function
    pub fn read(&self, address: u32, buffer: &mut [u8]) -> Result<(), DriverError> {
        let controller = controller().ok_or(DriverError::NotFound)?;
        controller.read_bytes(self.function_number, address, buffer)
    }

    /// Write multiple bytes to this function
    pub fn write(&self, address: u32, buffer: &[u8]) -> Result<(), DriverError> {
        let controller = controller().ok_or(DriverError::NotFound)?;
        controller.write_bytes(self.function_number, address, buffer)
    }
}
