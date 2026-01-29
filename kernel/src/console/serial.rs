//! Serial port driver (UART 16550)

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
            // Disable interrupts
            Self::outb(port + 1, 0x00);

            // Enable DLAB (set baud rate divisor)
            Self::outb(port + 3, 0x80);

            // Set divisor to 3 (38400 baud)
            Self::outb(port + 0, 0x03);
            Self::outb(port + 1, 0x00);

            // 8 bits, no parity, one stop bit
            Self::outb(port + 3, 0x03);

            // Enable FIFO, clear them, with 14-byte threshold
            Self::outb(port + 2, 0xC7);

            // IRQs enabled, RTS/DSR set
            Self::outb(port + 4, 0x0B);

            // Enable interrupts
            Self::outb(port + 1, 0x01);
        }

        Self { port }
    }

    /// Output byte to port
    unsafe fn outb(port: u16, val: u8) {
        core::arch::asm!(
            "out dx, al",
            in("dx") port,
            in("al") val,
            options(nomem, nostack)
        );
    }

    /// Input byte from port
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

/// Try to receive a byte from COM1
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
