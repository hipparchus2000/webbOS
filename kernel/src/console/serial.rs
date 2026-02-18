//! Serial port driver (UART 16550)
//!
//! NOTE: This is x86_64-specific. ARM64 uses a different UART implementation.

#![cfg_attr(target_arch = "aarch64", allow(dead_code))]

use core::fmt;

/// COM1 base port
pub const COM1: u16 = 0x3F8;
/// COM2 base port
pub const COM2: u16 = 0x2F8;
/// COM3 base port
pub const COM3: u16 = 0x3E8;
/// COM4 base port
pub const COM4: u16 = 0x2E8;

/// Serial port
pub struct SerialPort {
    port: u16,
}

impl SerialPort {
    /// Create and initialize a serial port
    pub fn new(port: u16) -> Self {
        unsafe {
            // Debug: indicate serial init is starting
            early_write_string("[SERIAL] Initializing port...\n");
            
            // Disable interrupts
            Self::outb(port + 1, 0x00);
            early_write_string("[SERIAL] Interrupts disabled\n");

            // Enable DLAB (set baud rate divisor)
            Self::outb(port + 3, 0x80);
            early_write_string("[SERIAL] DLAB enabled\n");

            // Set divisor to 3 (38400 baud)
            Self::outb(port + 0, 0x03);
            Self::outb(port + 1, 0x00);
            early_write_string("[SERIAL] Baud rate set\n");

            // 8 bits, no parity, one stop bit
            Self::outb(port + 3, 0x03);
            early_write_string("[SERIAL] Line config set\n");

            // Enable FIFO, clear them, with 14-byte threshold
            Self::outb(port + 2, 0xC7);
            early_write_string("[SERIAL] FIFO enabled\n");

            // IRQs enabled, RTS/DSR set
            Self::outb(port + 4, 0x0B);
            early_write_string("[SERIAL] MCR set\n");

            // Enable interrupts
            Self::outb(port + 1, 0x01);
            early_write_string("[SERIAL] Interrupts enabled, init complete\n");
        }

        Self { port }
    }

    /// Output byte to port (x86_64 only)
    #[cfg(target_arch = "x86_64")]
    unsafe fn outb(port: u16, val: u8) {
        core::arch::asm!(
            "out dx, al",
            in("dx") port,
            in("al") val,
            options(nomem, nostack)
        );
    }

    /// Input byte from port (x86_64 only)
    #[cfg(target_arch = "x86_64")]
    unsafe fn inb(port: u16) -> u8 {
        let val: u8;
        core::arch::asm!(
            "in al, dx",
            in("dx") port,
            out("al") val,
            options(nomem, nostack)
        );
        val
    }
    
    /// Output byte to port stub (aarch64)
    #[cfg(target_arch = "aarch64")]
    unsafe fn outb(_port: u16, _val: u8) {}

    /// Input byte from port stub (aarch64)
    #[cfg(target_arch = "aarch64")]
    unsafe fn inb(_port: u16) -> u8 { 0 }

    /// Check if transmit buffer is empty
    fn is_transmit_empty(&self) -> bool {
        unsafe { (Self::inb(self.port + 5) & 0x20) != 0 }
    }

    /// Check if data is available to read
    fn data_available(&self) -> bool {
        unsafe { (Self::inb(self.port + 5) & 0x01) != 0 }
    }

    /// Write a byte to the serial port
    pub fn write_byte(&mut self, byte: u8) {
        unsafe {
            // Wait for transmit buffer to be empty
            while !self.is_transmit_empty() {}
            Self::outb(self.port, byte);
        }
    }

    /// Read a byte from the serial port
    pub fn read_byte(&mut self) -> Option<u8> {
        unsafe {
            if self.data_available() {
                Some(Self::inb(self.port))
            } else {
                None
            }
        }
    }

    /// Write a string to the serial port
    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
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

/// Try to receive a byte from COM1 (x86_64 only)
#[cfg(target_arch = "x86_64")]
pub fn try_receive() -> Option<u8> {
    // Simple implementation - just check COM1
    unsafe {
        let port = COM1;
        
        // Check if data available
        let status: u8;
        core::arch::asm!(
            "in al, dx",
            in("dx") port + 5,
            out("al") status,
            options(nomem, nostack)
        );
        
        if (status & 0x01) != 0 {
            let val: u8;
            core::arch::asm!(
                "in al, dx",
                in("dx") port,
                out("al") val,
                options(nomem, nostack)
            );
            Some(val)
        } else {
            None
        }
    }
}

/// Try to receive a byte from COM1 stub (aarch64)
#[cfg(target_arch = "aarch64")]
pub fn try_receive() -> Option<u8> {
    None
}

/// Early raw serial output - bypasses all initialization
/// This can be called before ANY Rust code runs to debug boot issues
/// 
/// # Safety
/// This is only safe on x86_64 with a valid serial port at COM1
#[cfg(target_arch = "x86_64")]
pub unsafe fn early_write_byte(byte: u8) {
    // Wait for transmit buffer to be empty (bit 5 of LSR)
    loop {
        let status: u8;
        core::arch::asm!(
            "in al, dx",
            in("dx") COM1 + 5,  // Line Status Register
            out("al") status,
            options(nomem, nostack)
        );
        if (status & 0x20) != 0 {
            break;
        }
    }
    
    // Write byte to data port
    core::arch::asm!(
        "out dx, al",
        in("dx") COM1,
        in("al") byte,
        options(nomem, nostack)
    );
}

/// Early raw serial output for aarch64 (stub)
#[cfg(target_arch = "aarch64")]
pub unsafe fn early_write_byte(_byte: u8) {}

/// Write a string using early raw serial output
/// 
/// # Safety
/// This bypasses all synchronization and initialization
#[cfg(target_arch = "x86_64")]
pub unsafe fn early_write_string(s: &str) {
    for byte in s.bytes() {
        early_write_byte(byte);
        // Also send carriage return after newlines for proper display
        if byte == b'\n' {
            early_write_byte(b'\r');
        }
    }
}

/// Early write string stub for aarch64
#[cfg(target_arch = "aarch64")]
pub unsafe fn early_write_string(_s: &str) {}
