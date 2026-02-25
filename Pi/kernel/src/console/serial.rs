//! Serial port driver (PL011 UART for Raspberry Pi)
//!
//! The PL011 UART is a memory-mapped serial controller on the Raspberry Pi.
//! Base addresses:
//! - Pi 3 (BCM2837): 0x3F201000
//! - Pi 4 (BCM2711): 0xFE201000

use core::fmt;
use core::ptr::{read_volatile, write_volatile};

/// PL011 UART base address for Pi 3
const UART_BASE_PI3: usize = 0x3F201000;
/// PL011 UART base address for Pi 4
#[allow(dead_code)]
const UART_BASE_PI4: usize = 0xFE201000;

// PL011 register offsets (from base)
const REG_DR: usize = 0x00;      // Data Register
const REG_FR: usize = 0x18;      // Flag Register
const REG_IBRD: usize = 0x24;    // Integer Baud Rate Divisor
const REG_FBRD: usize = 0x28;    // Fractional Baud Rate Divisor
const REG_LCRH: usize = 0x2C;    // Line Control Register
const REG_CR: usize = 0x30;      // Control Register
#[allow(dead_code)]
const REG_IMSC: usize = 0x38;    // Interrupt Mask Set/Clear
const REG_ICR: usize = 0x44;     // Interrupt Clear Register

// Flag register bits
const FR_TXFF: u32 = 0x20;       // Transmit FIFO full
const FR_RXFE: u32 = 0x10;       // Receive FIFO empty
#[allow(dead_code)]
const FR_BUSY: u32 = 0x08;       // UART busy

/// Serial port
pub struct SerialPort {
    base: usize,
}

impl SerialPort {
    /// Create and initialize a serial port
    /// 
    /// For Pi, this uses the mini UART (UART1) or PL011 (UART0)
    /// We'll use PL011 which is the primary UART on Pi
    pub fn new(_port: u16) -> Self {
        // Use Pi 3 base address by default
        // In a real implementation, we'd detect the Pi model
        let base = UART_BASE_PI3;
        
        unsafe {
            // Disable UART
            write_volatile((base + REG_CR) as *mut u32, 0);
            
            // Clear pending interrupts
            write_volatile((base + REG_ICR) as *mut u32, 0x7FF);
            
            // Set baud rate to 115200 (assuming 3MHz UART clock)
            // IBRD = 3,000,000 / (16 * 115200) = 1.627 ~ 1
            // FBRD = 0.627 * 64 = 40
            write_volatile((base + REG_IBRD) as *mut u32, 1);
            write_volatile((base + REG_FBRD) as *mut u32, 40);
            
            // 8 bits, no parity, 1 stop bit (8N1), enable FIFOs
            write_volatile((base + REG_LCRH) as *mut u32, 0x70);
            
            // Enable UART, TX, and RX
            write_volatile((base + REG_CR) as *mut u32, 0x301);
        }
        
        Self { base }
    }

    /// Check if transmit FIFO is full
    fn is_tx_full(&self) -> bool {
        unsafe {
            (read_volatile((self.base + REG_FR) as *const u32) & FR_TXFF) != 0
        }
    }

    /// Check if receive FIFO is empty
    #[allow(dead_code)]
    fn is_rx_empty(&self) -> bool {
        unsafe {
            (read_volatile((self.base + REG_FR) as *const u32) & FR_RXFE) != 0
        }
    }

    /// Write a byte to the serial port
    pub fn write_byte(&mut self, byte: u8) {
        unsafe {
            // Wait for transmit FIFO to have space
            while self.is_tx_full() {}
            
            // Write byte (add to data register)
            write_volatile((self.base + REG_DR) as *mut u32, byte as u32);
        }
    }

    /// Read a byte from the serial port
    #[allow(dead_code)]
    pub fn read_byte(&mut self) -> Option<u8> {
        unsafe {
            if self.is_rx_empty() {
                None
            } else {
                Some(read_volatile((self.base + REG_DR) as *const u32) as u8)
            }
        }
    }

    /// Write a string to the serial port
    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            // Convert newline to CRLF for terminal compatibility
            if byte == b'\n' {
                self.write_byte(b'\r');
            }
            self.write_byte(byte);
        }
    }
}

impl fmt::Write for SerialPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

/// Try to receive a byte from the serial port
pub fn try_receive() -> Option<u8> {
    // Use the default UART base
    let base = UART_BASE_PI3;
    
    unsafe {
        // Check if receive FIFO has data
        let fr = read_volatile((base + REG_FR) as *const u32);
        if (fr & FR_RXFE) != 0 {
            None
        } else {
            Some(read_volatile((base + REG_DR) as *const u32) as u8)
        }
    }
}

// Legacy constants for x86 compatibility (not used on ARM)
/// COM1 base port (x86 legacy)
pub const COM1: u16 = 0x3F8;
/// COM2 base port (x86 legacy)
#[allow(dead_code)]
pub const COM2: u16 = 0x2F8;
/// COM3 base port (x86 legacy)
#[allow(dead_code)]
pub const COM3: u16 = 0x3E8;
/// COM4 base port (x86 legacy)
#[allow(dead_code)]
pub const COM4: u16 = 0x2E8;
