//! PL011 UART driver for Raspberry Pi
//!
//! The Pi 3 and Pi 4 use the PL011 UART for serial communication.
//! Base address: 0xFE201000 (Pi 4) or 0x3F201000 (Pi 3)

use core::ptr::{read_volatile, write_volatile};

/// UART base address
/// Pi 3: 0x3F201000 (PL011), Pi 4: 0xFE201000 (PL011)
/// Mini UART (alternative): 0x3F215040 (Pi 3) / 0xFE215040 (Pi 4)
#[cfg(feature = "pi4")]
const UART_BASE: usize = 0xFE201000;
#[cfg(not(feature = "pi4"))]
const UART_BASE: usize = 0x3F201000;

// Try mini UART instead for QEMU compatibility
// const UART_BASE: usize = 0x3F215040;

/// UART register offsets
const UART_DR: usize = 0x00;     // Data Register
const UART_FR: usize = 0x18;     // Flag Register
const UART_IBRD: usize = 0x24;   // Integer Baud Rate Divisor
const UART_FBRD: usize = 0x28;   // Fractional Baud Rate Divisor
const UART_LCRH: usize = 0x2C;   // Line Control Register
const UART_CR: usize = 0x30;     // Control Register
const UART_ICR: usize = 0x44;    // Interrupt Clear Register

/// Flag register bits
const FR_TXFF: u32 = 1 << 5;     // Transmit FIFO full
const FR_RXFE: u32 = 1 << 4;     // Receive FIFO empty
const FR_BUSY: u32 = 1 << 3;     // UART busy

/// Initialize the UART
pub fn init() {
    unsafe {
        // Disable UART
        write_reg(UART_CR, 0);

        // Clear pending interrupts
        write_reg(UART_ICR, 0x7FF);

        // Set baud rate to 115200 (assuming 48MHz UART clock)
        // Divisor = 48000000 / (16 * 115200) = 26.041667
        // IBRD = 26, FBRD = 0.041667 * 64 + 0.5 = 3
        write_reg(UART_IBRD, 26);
        write_reg(UART_FBRD, 3);

        // Set line control: 8 bits, no parity, 1 stop bit, FIFOs enabled
        write_reg(UART_LCRH, 0x70);

        // Enable UART, TX, and RX
        write_reg(UART_CR, 0x301);
    }
}

/// Write a character to UART
pub fn putc(c: u8) {
    unsafe {
        // Wait for transmit FIFO to have space
        while read_reg(UART_FR) & FR_TXFF != 0 {
            core::arch::asm!("nop");
        }
        write_reg(UART_DR, c as u32);
    }
}

/// Write a string to UART
pub fn puts(s: &str) {
    for c in s.bytes() {
        if c == b'\n' {
            putc(b'\r');
        }
        putc(c);
    }
}

/// Read a character from UART (non-blocking)
pub fn getc() -> Option<u8> {
    unsafe {
        if read_reg(UART_FR) & FR_RXFE != 0 {
            None
        } else {
            Some(read_reg(UART_DR) as u8)
        }
    }
}

/// Print a hexadecimal number
pub fn puthex(mut n: u64) {
    if n == 0 {
        putc(b'0');
        return;
    }

    // Find highest nibble
    let mut shift: u32 = 60;
    while shift > 0 && (n >> shift) & 0xF == 0 {
        shift -= 4;
    }

    // Print nibbles
    while shift <= 60 {
        let digit = ((n >> shift) & 0xF) as u8;
        let c = if digit < 10 {
            b'0' + digit
        } else {
            b'A' + digit - 10
        };
        putc(c);
        shift = shift.saturating_sub(4);
    }
}

/// Print a decimal number
pub fn putdec(mut n: u64) {
    if n == 0 {
        putc(b'0');
        return;
    }

    // Find highest power of 10
    let mut divisor = 1u64;
    while n / divisor >= 10 {
        divisor *= 10;
    }

    // Print digits
    while divisor > 0 {
        let digit = (n / divisor) as u8;
        putc(b'0' + digit);
        n %= divisor;
        divisor /= 10;
    }
}

/// Read UART register
unsafe fn read_reg(offset: usize) -> u32 {
    read_volatile((UART_BASE + offset) as *const u32)
}

/// Write UART register
unsafe fn write_reg(offset: usize, value: u32) {
    write_volatile((UART_BASE + offset) as *mut u32, value);
}
