//! SDIO-over-SPI Fallback Driver
//!
//! Alternative SPI-based SDIO implementation for compatibility with
//! systems that don't have native SDIO support or as a fallback.
//!
//! This implements the SPI protocol subset of SDIO for the BCM43438/BCM43455
//! chips. While slower than native SDIO, it provides wider compatibility.
//!
//! SPI mode uses:
//! - MOSI (GPIO10 on Pi) - Data to card
//! - MISO (GPIO9 on Pi) - Data from card
//! - SCLK (GPIO11 on Pi) - Clock
//! - CS (GPIO8 on Pi) - Chip select

use crate::drivers::DriverError;
use crate::println;
use core::sync::atomic::{fence, Ordering};

// BCM2835/2836/2837/2711 GPIO registers
const GPIO_BASE_PI3: usize = 0x3F200000;
const GPIO_BASE_PI4: usize = 0xFE200000;

// GPIO register offsets
const GPIO_GPFSEL0: usize = 0x00;  // Function select
const GPIO_GPFSEL1: usize = 0x04;
const GPIO_GPFSEL2: usize = 0x08;
const GPIO_GPSET0: usize = 0x1C;   // Pin output set
const GPIO_GPCLR0: usize = 0x28;   // Pin output clear
const GPIO_GPLEV0: usize = 0x34;   // Pin level
const GPIO_GPPUD: usize = 0x94;    // Pull up/down (legacy)
const GPIO_GPPUDCLK0: usize = 0x98;
const GPIO_GPPUPPDN0: usize = 0xE4; // Pull up/down (new)

// SPI0 registers
const SPI_BASE_OFFSET: usize = 0x204000; // GPIO_BASE + 0x204000
const SPI_CS: usize = 0x00;    // Control and status
const SPI_FIFO: usize = 0x04;  // TX/RX FIFO
const SPI_CLK: usize = 0x08;   // Clock divider
const SPI_DLEN: usize = 0x0C;  // Data length
const SPI_LTOH: usize = 0x10;  // LoSSI output hold
const SPI_DC: usize = 0x14;    // DMA DREQ controls

// SPI CS register bits
const SPI_CS_LEN_LONG: u32 = 0x02000000;
const SPI_CS_DMA_LEN: u32 = 0x01000000;
const SPI_CS_CSPOL2: u32 = 0x00800000;
const SPI_CS_CSPOL1: u32 = 0x00400000;
const SPI_CS_CSPOL0: u32 = 0x00200000;
const SPI_CS_RXF: u32 = 0x00100000;
const SPI_CS_RXR: u32 = 0x00080000;
const SPI_CS_TXD: u32 = 0x00040000;
const SPI_CS_RXD: u32 = 0x00020000;
const SPI_CS_DONE: u32 = 0x00010000;
const SPI_CS_LEN: u32 = 0x00002000;
const SPI_CS_REN: u32 = 0x00001000;
const SPI_CS_ADCS: u32 = 0x00000800;
const SPI_CS_INTR: u32 = 0x00000400;
const SPI_CS_INTD: u32 = 0x00000200;
const SPI_CS_DMAEN: u32 = 0x00000100;
const SPI_CS_TA: u32 = 0x00000080;
const SPI_CS_CSPOL: u32 = 0x00000040;
const SPI_CS_CLEAR_RX: u32 = 0x00000020;
const SPI_CS_CLEAR_TX: u32 = 0x00000010;
const SPI_CS_CPOL: u32 = 0x00000008;
const SPI_CS_CPHA: u32 = 0x00000004;
const SPI_CS_CS1: u32 = 0x00000002;
const SPI_CS_CS0: u32 = 0x00000001;

// SPI GPIO pins (GPIO numbers)
const SPI_MOSI_PIN: u8 = 10;
const SPI_MISO_PIN: u8 = 9;
const SPI_SCLK_PIN: u8 = 11;
const SPI_CS0_PIN: u8 = 8;
const SPI_CS1_PIN: u8 = 7;

// SDIO-over-SPI protocol constants
const SDIO_SPI_START_BLOCK: u8 = 0xFE;
const SDIO_SPI_START_BLOCK_WRITE: u8 = 0xFC;
const SDIO_SPI_STOP_TRAN: u8 = 0xFD;
const SDIO_SPI_DATA_ACCEPTED: u8 = 0x05;
const SDIO_SPI_WRITE_CRC_ERROR: u8 = 0x0B;
const SDIO_SPI_WRITE_ERROR: u8 = 0x0D;

// SDIO commands over SPI
const CMD0_GO_IDLE: u8 = 0;
const CMD8_SEND_IF_COND: u8 = 8;
const CMD55_APP_CMD: u8 = 55;
const CMD58_READ_OCR: u8 = 58;
const ACMD41_SD_SEND_OP_COND: u8 = 41;
const CMD52_IO_RW_DIRECT: u8 = 52;
const CMD53_IO_RW_EXTENDED: u8 = 53;

/// SDIO response types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SdioResponse {
    None,
    R1(u8),       // 1-byte status
    R2([u8; 17]), // 136-bit CID/CSD
    R3(u32),      // OCR register
    R5([u8; 2]),  // SDIO response
    R7(u32),      // Interface condition
}

/// SDIO SPI controller
pub struct SdioSpiController {
    gpio_base: usize,
    spi_base: usize,
    cs_pin: u8,
    clock_divider: u16,
}

impl SdioSpiController {
    /// Create a new SDIO SPI controller
    /// 
    /// # Safety
    /// Caller must ensure GPIO/SPI addresses are valid and mapped
    pub unsafe fn new(pi4: bool, use_cs1: bool) -> Self {
        let gpio_base = if pi4 { GPIO_BASE_PI4 } else { GPIO_BASE_PI3 };
        let spi_base = gpio_base + SPI_BASE_OFFSET;
        let cs_pin = if use_cs1 { SPI_CS1_PIN } else { SPI_CS0_PIN };
        
        Self {
            gpio_base,
            spi_base,
            cs_pin,
            clock_divider: 256, // ~1MHz default for initialization
        }
    }

    /// Read GPIO register
    #[inline]
    unsafe fn gpio_read(&self, offset: usize) -> u32 {
        core::ptr::read_volatile((self.gpio_base + offset) as *const u32)
    }

    /// Write GPIO register
    #[inline]
    unsafe fn gpio_write(&self, offset: usize, val: u32) {
        core::ptr::write_volatile((self.gpio_base + offset) as *mut u32, val);
    }

    /// Read SPI register
    #[inline]
    unsafe fn spi_read(&self, offset: usize) -> u32 {
        core::ptr::read_volatile((self.spi_base + offset) as *const u32)
    }

    /// Write SPI register
    #[inline]
    unsafe fn spi_write(&self, offset: usize, val: u32) {
        core::ptr::write_volatile((self.spi_base + offset) as *mut u32, val);
    }

    /// Set GPIO pin function
    unsafe fn set_gpio_function(&self, pin: u8, function: u8) {
        let reg = (pin / 10) as usize * 4;
        let shift = (pin % 10) * 3;
        
        let mut val = self.gpio_read(reg);
        val &= !(0x7 << shift);
        val |= ((function & 0x7) as u32) << shift;
        self.gpio_write(reg, val);
    }

    /// Set GPIO pin pull-up/down
    unsafe fn set_gpio_pull(&self, pin: u8, pull: u8) {
        // Use new register for Pi 4, legacy for Pi 3
        let reg = (pin / 16) as usize * 4 + GPIO_GPPUPPDN0;
        let shift = (pin % 16) * 2;
        
        let mut val = self.gpio_read(reg);
        val &= !(0x3 << shift);
        val |= ((pull & 0x3) as u32) << shift;
        self.gpio_write(reg, val);
    }

    /// Set GPIO pin high
    unsafe fn gpio_set(&self, pin: u8) {
        self.gpio_write(GPIO_GPSET0, 1 << pin);
    }

    /// Set GPIO pin low
    unsafe fn gpio_clear(&self, pin: u8) {
        self.gpio_write(GPIO_GPCLR0, 1 << pin);
    }

    /// Read GPIO pin level
    unsafe fn gpio_level(&self, pin: u8) -> bool {
        (self.gpio_read(GPIO_GPLEV0) & (1 << pin)) != 0
    }

    /// Configure SPI pins
    unsafe fn configure_pins(&self) {
        // Configure MOSI (GPIO10) as ALT0
        self.set_gpio_function(SPI_MOSI_PIN, 0);
        
        // Configure MISO (GPIO9) as ALT0
        self.set_gpio_function(SPI_MISO_PIN, 0);
        
        // Configure SCLK (GPIO11) as ALT0
        self.set_gpio_function(SPI_SCLK_PIN, 0);
        
        // Configure CS pin as ALT0 (SPI) initially
        self.set_gpio_function(self.cs_pin, 0);
        
        // Disable pull-up/down on all SPI pins
        self.set_gpio_pull(SPI_MOSI_PIN, 0);
        self.set_gpio_pull(SPI_MISO_PIN, 0);
        self.set_gpio_pull(SPI_SCLK_PIN, 0);
        self.set_gpio_pull(self.cs_pin, 0);
    }

    /// Initialize the SPI controller
    pub fn init(&mut self) -> Result<(), DriverError> {
        println!("[sdio_spi] Initializing SPI controller...");
        
        unsafe {
            // Configure pins
            self.configure_pins();
            
            // Clear FIFOs
            self.spi_write(SPI_CS, SPI_CS_CLEAR_RX | SPI_CS_CLEAR_TX);
            
            // Set clock divider
            self.set_clock_divider(256)?;
            
            // Configure mode (CPOL=0, CPHA=0 for SDIO)
            let mut cs = self.spi_read(SPI_CS);
            cs &= !(SPI_CS_CPOL | SPI_CS_CPHA);
            self.spi_write(SPI_CS, cs);
            
            // Deassert CS
            self.spi_write(SPI_CS, cs & !SPI_CS_TA);
        }
        
        println!("[sdio_spi] SPI controller initialized");
        Ok(())
    }

    /// Set SPI clock divider
    pub fn set_clock_divider(&mut self, divider: u16) -> Result<(), DriverError> {
        if divider == 0 || (divider & 1) != 0 {
            return Err(DriverError::Unsupported);
        }
        
        self.clock_divider = divider;
        
        unsafe {
            self.spi_write(SPI_CLK, divider as u32);
        }
        
        Ok(())
    }

    /// Set SPI clock frequency
    pub fn set_clock(&mut self, freq_hz: u32) -> Result<(), DriverError> {
        // Base clock is 250MHz on Pi
        let base_clock = 250_000_000u32;
        let divider = ((base_clock + freq_hz - 1) / freq_hz) as u16;
        
        // Ensure even number
        let divider = if divider % 2 == 0 { divider } else { divider + 1 };
        
        // Clamp to valid range
        let divider = divider.clamp(2, 65534);
        
        self.set_clock_divider(divider)
    }

    /// Assert chip select
    unsafe fn cs_assert(&self) {
        let cs_bits = if self.cs_pin == SPI_CS1_PIN { SPI_CS_CS1 } else { SPI_CS_CS0 };
        self.spi_write(SPI_CS, SPI_CS_TA | cs_bits);
    }

    /// Deassert chip select
    unsafe fn cs_deassert(&self) {
        self.spi_write(SPI_CS, 0);
    }

    /// Wait for TX FIFO to have space
    unsafe fn wait_tx_ready(&self, timeout_us: u32) -> Result<(), DriverError> {
        let mut timeout = timeout_us;
        while (self.spi_read(SPI_CS) & SPI_CS_TXD) == 0 {
            if timeout == 0 {
                return Err(DriverError::Timeout);
            }
            timeout -= 1;
            core::arch::asm!("nop");
        }
        Ok(())
    }

    /// Wait for RX FIFO to have data
    unsafe fn wait_rx_ready(&self, timeout_us: u32) -> Result<(), DriverError> {
        let mut timeout = timeout_us;
        while (self.spi_read(SPI_CS) & SPI_CS_RXD) == 0 {
            if timeout == 0 {
                return Err(DriverError::Timeout);
            }
            timeout -= 1;
            core::arch::asm!("nop");
        }
        Ok(())
    }

    /// Wait for transfer to complete
    unsafe fn wait_done(&self, timeout_us: u32) -> Result<(), DriverError> {
        let mut timeout = timeout_us;
        while (self.spi_read(SPI_CS) & SPI_CS_DONE) == 0 {
            if timeout == 0 {
                return Err(DriverError::Timeout);
            }
            timeout -= 1;
            core::arch::asm!("nop");
        }
        Ok(())
    }

    /// Transfer a single byte
    pub fn transfer_byte(&self, byte: u8) -> Result<u8, DriverError> {
        unsafe {
            self.wait_tx_ready(10000)?;
            self.spi_write(SPI_FIFO, byte as u32);
            self.wait_rx_ready(10000)?;
            Ok(self.spi_read(SPI_FIFO) as u8)
        }
    }

    /// Transfer multiple bytes
    pub fn transfer(&self, tx_data: &[u8], mut rx_data: Option<&mut [u8]>) -> Result<(), DriverError> {
        unsafe {
            self.cs_assert();
            
            // Send all bytes
            for (i, &byte) in tx_data.iter().enumerate() {
                self.wait_tx_ready(10000)?;
                self.spi_write(SPI_FIFO, byte as u32);
                
                // Read back if requested
                if let Some(ref mut rx) = rx_data {
                    self.wait_rx_ready(10000)?;
                    rx[i] = self.spi_read(SPI_FIFO) as u8;
                }
            }
            
            // Wait for completion
            self.wait_done(10000)?;
            self.cs_deassert();
        }
        
        Ok(())
    }

    /// Send a command over SPI
    fn send_command(&self, cmd: u8, arg: u32, crc: u8) -> Result<(), DriverError> {
        let cmd_packet: [u8; 6] = [
            0x40 | cmd,  // Command byte with start bit
            (arg >> 24) as u8,
            (arg >> 16) as u8,
            (arg >> 8) as u8,
            arg as u8,
            crc,
        ];
        
        self.transfer(&cmd_packet, None)?;
        Ok(())
    }

    /// Wait for and read response R1
    fn read_r1(&self, timeout_ms: u32) -> Result<u8, DriverError> {
        let mut timeout = timeout_ms * 1000;
        
        loop {
            let byte = self.transfer_byte(0xFF)?;
            if (byte & 0x80) == 0 {
                return Ok(byte);
            }
            
            timeout -= 1;
            if timeout == 0 {
                return Err(DriverError::Timeout);
            }
        }
    }

    /// Wait for and read response R3/R7 (OCR/Interface condition)
    fn read_r3_r7(&self, timeout_ms: u32) -> Result<u32, DriverError> {
        let r1 = self.read_r1(timeout_ms)?;
        
        // Read 4 bytes of response
        let mut resp = [0u8; 4];
        self.transfer(&[0xFF; 4], Some(&mut resp))?;
        
        // Extra clock cycles
        let _ = self.transfer_byte(0xFF)?;
        
        Ok(u32::from_be_bytes(resp))
    }

    /// Wait for data start token
    fn wait_data_start(&self, timeout_ms: u32) -> Result<(), DriverError> {
        let mut timeout = timeout_ms * 1000;
        
        loop {
            let byte = self.transfer_byte(0xFF)?;
            if byte == SDIO_SPI_START_BLOCK {
                return Ok(());
            }
            if byte != 0xFF {
                return Err(DriverError::IoError);
            }
            
            timeout -= 1;
            if timeout == 0 {
                return Err(DriverError::Timeout);
            }
        }
    }

    /// Initialize SD/SDIO card in SPI mode
    pub fn init_card(&mut self) -> Result<(), DriverError> {
        println!("[sdio_spi] Initializing card in SPI mode...");
        
        // Send 80 clock cycles with CS deasserted
        unsafe { self.cs_deassert(); }
        for _ in 0..10 {
            let _ = self.transfer_byte(0xFF);
        }
        
        // CMD0 - Go idle state
        println!("[sdio_spi] Sending CMD0...");
        self.send_command(CMD0_GO_IDLE, 0, 0x95)?;
        let r1 = self.read_r1(1000)?;
        if r1 != 0x01 {
            return Err(DriverError::InitFailed);
        }
        
        // CMD8 - Send interface condition (SDHC/SDXC)
        println!("[sdio_spi] Sending CMD8...");
        self.send_command(CMD8_SEND_IF_COND, 0x1AA, 0x87)?;
        let _ = self.read_r3_r7(1000)?; // Ignore response for now
        
        // ACMD41 - Send operation condition
        println!("[sdio_spi] Sending ACMD41...");
        let mut ocr = 0u32;
        for _ in 0..100 {
            // CMD55 first
            self.send_command(CMD55_APP_CMD, 0, 0)?;
            let _ = self.read_r1(1000)?;
            
            // Then ACMD41
            self.send_command(ACMD41_SD_SEND_OP_COND, 0x40000000, 0)?;
            ocr = self.read_r3_r7(1000)?;
            
            if (ocr & 0x80000000) != 0 {
                // Card ready
                break;
            }
        }
        
        if (ocr & 0x80000000) == 0 {
            return Err(DriverError::Timeout);
        }
        
        println!("[sdio_spi] Card initialized, OCR: {:08X}", ocr);
        Ok(())
    }

    /// CMD52 - IO Read/Write Direct (SPI mode)
    pub fn cmd52(&self, write: bool, function: u8, address: u32, data: u8) -> Result<u8, DriverError> {
        let mut arg: u32 = 0;
        if write {
            arg |= 0x80000000;
        }
        arg |= ((function & 0x07) as u32) << 28;
        arg |= (address & 0x1FFFF) << 9;
        arg |= (data as u32) & 0xFF;
        
        self.send_command(CMD52_IO_RW_DIRECT, arg, 0)?;
        
        let r1 = self.read_r1(1000)?;
        if r1 != 0 {
            return Err(DriverError::IoError);
        }
        
        // Read R5 response (2 bytes)
        let mut resp = [0u8; 2];
        self.transfer(&[0xFF; 2], Some(&mut resp))?;
        
        // Response data is in second byte
        Ok(resp[1])
    }

    /// CMD53 - IO Read/Write Extended (SPI mode)
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
        let mut arg: u32 = 0;
        if write {
            arg |= 0x80000000;
        }
        if block_mode {
            arg |= 0x08000000;
        }
        if increment {
            arg |= 0x04000000;
        }
        arg |= ((function & 0x07) as u32) << 28;
        arg |= (address & 0x1FFFF) << 9;
        arg |= (count as u32) & 0x1FF;
        
        self.send_command(CMD53_IO_RW_EXTENDED, arg, 0)?;
        
        let r1 = self.read_r1(1000)?;
        if r1 != 0 {
            return Err(DriverError::IoError);
        }
        
        let len = if block_mode {
            count as usize * 512
        } else {
            count as usize
        };
        
        unsafe {
            if write {
                // Wait for data ready token
                self.wait_data_start(1000)?;
                
                // Send data
                let data_slice = core::slice::from_raw_parts(buffer, len);
                self.transfer(data_slice, None)?;
                
                // Send CRC (dummy)
                let _ = self.transfer_byte(0xFF);
                let _ = self.transfer_byte(0xFF);
                
                // Check data response
                let response = self.transfer_byte(0xFF)?;
                if (response & 0x1F) != SDIO_SPI_DATA_ACCEPTED {
                    return Err(DriverError::IoError);
                }
                
                // Wait while card is busy
                while self.transfer_byte(0xFF)? == 0 {}
            } else {
                // Wait for start token
                self.wait_data_start(1000)?;
                
                // Read data
                let data_slice = core::slice::from_raw_parts_mut(buffer, len);
                for i in 0..len {
                    data_slice[i] = self.transfer_byte(0xFF)?;
                }
                
                // Read and discard CRC
                let _ = self.transfer_byte(0xFF);
                let _ = self.transfer_byte(0xFF);
            }
        }
        
        Ok(())
    }

    /// Read a byte from SDIO function
    pub fn read_byte(&self, function: u8, address: u32) -> Result<u8, DriverError> {
        self.cmd52(false, function, address, 0)
    }

    /// Write a byte to SDIO function
    pub fn write_byte(&self, function: u8, address: u32, data: u8) -> Result<(), DriverError> {
        self.cmd52(true, function, address, data)?;
        Ok(())
    }
}

// SAFETY: SdioSpiController is thread-safe when properly synchronized
unsafe impl Send for SdioSpiController {}
unsafe impl Sync for SdioSpiController {}

/// SDIO-over-SPI device wrapper
pub struct SdioSpiDevice {
    controller: SdioSpiController,
}

impl SdioSpiDevice {
    /// Create a new SDIO-over-SPI device
    pub fn new(pi4: bool) -> Result<Self, DriverError> {
        let controller = unsafe { SdioSpiController::new(pi4, false) };
        
        Ok(Self { controller })
    }

    /// Initialize the device
    pub fn init(&mut self) -> Result<(), DriverError> {
        self.controller.init()?;
        self.controller.init_card()?;
        Ok(())
    }

    /// Read from SDIO function
    pub fn read(&self, function: u8, address: u32, buffer: &mut [u8]) -> Result<(), DriverError> {
        let count = buffer.len();
        
        if count == 1 {
            buffer[0] = self.controller.read_byte(function, address)?;
            Ok(())
        } else {
            self.controller.cmd53(false, function, address, false, true, count as u16, buffer.as_mut_ptr())
        }
    }

    /// Write to SDIO function
    pub fn write(&self, function: u8, address: u32, buffer: &[u8]) -> Result<(), DriverError> {
        let count = buffer.len();
        
        if count == 1 {
            self.controller.write_byte(function, address, buffer[0])
        } else {
            self.controller.cmd53(true, function, address, false, true, count as u16, buffer.as_ptr() as *mut u8)
        }
    }
}

// SAFETY: SdioSpiDevice is thread-safe through interior mutability
unsafe impl Send for SdioSpiDevice {}
unsafe impl Sync for SdioSpiDevice {}

/// Global SDIO SPI controller instance
static mut SDIO_SPI_CONTROLLER: Option<SdioSpiController> = None;

/// Initialize SDIO-over-SPI subsystem
pub fn init(pi4: bool) {
    println!("[sdio_spi] Initializing SDIO-over-SPI fallback...");
    
    unsafe {
        let mut controller = SdioSpiController::new(pi4, false);
        
        if let Err(e) = controller.init() {
            println!("[sdio_spi] Failed to initialize controller: {:?}", e);
            return;
        }
        
        SDIO_SPI_CONTROLLER = Some(controller);
    }
    
    println!("[sdio_spi] SDIO-over-SPI initialized");
}

/// Check if SDIO SPI is available
pub fn is_available() -> bool {
    unsafe { SDIO_SPI_CONTROLLER.is_some() }
}

/// Get the global SDIO SPI controller
pub fn controller() -> Option<&'static mut SdioSpiController> {
    unsafe { SDIO_SPI_CONTROLLER.as_mut() }
}

/// SDIO I/O function abstraction for SPI mode
pub struct SdioSpiFunction {
    function_number: u8,
}

impl SdioSpiFunction {
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
        controller.cmd53(false, self.function_number, address, false, true, buffer.len() as u16, buffer.as_mut_ptr())
    }

    /// Write multiple bytes to this function
    pub fn write(&self, address: u32, buffer: &[u8]) -> Result<(), DriverError> {
        let controller = controller().ok_or(DriverError::NotFound)?;
        controller.cmd53(true, self.function_number, address, false, true, buffer.len() as u16, buffer.as_ptr() as *mut u8)
    }
}
