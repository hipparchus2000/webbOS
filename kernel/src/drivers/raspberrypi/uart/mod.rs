//! UART Driver for Raspberry Pi 5
//!
//! Supports both:
//! - PL011 UART (UART0) - Full-featured UART
//! - Mini UART (UART1) - Simpler, used for Bluetooth on Pi 3/4
//!
//! On Raspberry Pi 5, the UART is accessed through the RP1 I/O controller.
//! Standard baud rate is 115200.

use crate::hal::{mmio, platform_info, delay};
use crate::hal::PlatformType;
use crate::println;
use core::fmt::{self, Write};

/// PL011 UART register offsets
pub mod pl011_regs {
    /// Data register
    pub const DR: usize = 0x00;
    /// Receive status / error clear
    pub const RSRECR: usize = 0x04;
    /// Flag register
    pub const FR: usize = 0x18;
    /// IrDA low-power counter
    pub const ILPR: usize = 0x20;
    /// Integer baud rate divisor
    pub const IBRD: usize = 0x24;
    /// Fractional baud rate divisor
    pub const FBRD: usize = 0x28;
    /// Line control register
    pub const LCRH: usize = 0x2C;
    /// Control register
    pub const CR: usize = 0x30;
    /// Interrupt FIFO level select
    pub const IFLS: usize = 0x34;
    /// Interrupt mask set/clear
    pub const IMSC: usize = 0x38;
    /// Raw interrupt status
    pub const RIS: usize = 0x3C;
    /// Masked interrupt status
    pub const MIS: usize = 0x40;
    /// Interrupt clear
    pub const ICR: usize = 0x44;
    /// DMA control
    pub const DMACR: usize = 0x48;
}

/// Mini UART register offsets (BCM2835/6/7 aux peripherals)
pub mod mini_uart_regs {
    /// Aux interrupts
    pub const AUX_IRQ: usize = 0x00;
    /// Aux enables
    pub const AUX_ENABLES: usize = 0x04;
    /// Mini UART I/O data
    pub const MU_IO: usize = 0x40;
    /// Mini UART interrupt enable
    pub const MU_IER: usize = 0x44;
    /// Mini UART interrupt identify
    pub const MU_IIR: usize = 0x48;
    /// Mini UART line control
    pub const MU_LCR: usize = 0x4C;
    /// Mini UART modem control
    pub const MU_MCR: usize = 0x50;
    /// Mini UART line status
    pub const MU_LSR: usize = 0x54;
    /// Mini UART modem status
    pub const MU_MSR: usize = 0x58;
    /// Mini UART scratch
    pub const MU_SCRATCH: usize = 0x5C;
    /// Mini UART extra control
    pub const MU_CNTL: usize = 0x60;
    /// Mini UART extra status
    pub const MU_STAT: usize = 0x64;
    /// Mini UART baud rate
    pub const MU_BAUD: usize = 0x68;
}

/// PL011 FR (Flag Register) bits
pub mod pl011_fr {
    /// Clear to send
    pub const CTS: u32 = 1 << 0;
    /// Data set ready
    pub const DSR: u32 = 1 << 1;
    /// Data carrier detect
    pub const DCD: u32 = 1 << 2;
    /// Busy
    pub const BUSY: u32 = 1 << 3;
    /// Receive FIFO empty
    pub const RXFE: u32 = 1 << 4;
    /// Transmit FIFO full
    pub const TXFF: u32 = 1 << 5;
    /// Receive FIFO full
    pub const RXFF: u32 = 1 << 6;
    /// Transmit FIFO empty
    pub const TXFE: u32 = 1 << 7;
    /// Ring indicator
    pub const RI: u32 = 1 << 8;
}

/// PL011 LCRH (Line Control) bits
pub mod pl011_lcrh {
    /// Send break
    pub const BRK: u32 = 1 << 0;
    /// Parity enable
    pub const PEN: u32 = 1 << 1;
    /// Even parity
    pub const EPS: u32 = 1 << 2;
    /// Stick parity select
    pub const STP2: u32 = 1 << 3;
    /// FIFO enable
    pub const FEN: u32 = 1 << 4;
    /// Word length 5 bits
    pub const WLEN_5: u32 = 0 << 5;
    /// Word length 6 bits
    pub const WLEN_6: u32 = 1 << 5;
    /// Word length 7 bits
    pub const WLEN_7: u32 = 2 << 5;
    /// Word length 8 bits
    pub const WLEN_8: u32 = 3 << 5;
    /// Stick parity
    pub const SPS: u32 = 1 << 7;
}

/// PL011 CR (Control Register) bits
pub mod pl011_cr {
    /// UART enable
    pub const UARTEN: u32 = 1 << 0;
    /// SIR enable
    pub const SIREN: u32 = 1 << 1;
    /// SIR low power mode
    pub const SIRLPM: u32 = 1 << 2;
    /// Loopback enable
    pub const LBE: u32 = 1 << 7;
    /// Transmit enable
    pub const TXE: u32 = 1 << 8;
    /// Receive enable
    pub const RXE: u32 = 1 << 9;
    /// Data transmit ready
    pub const DTR: u32 = 1 << 10;
    /// Request to send
    pub const RTS: u32 = 1 << 11;
    /// Output 1
    pub const OUT1: u32 = 1 << 12;
    /// Output 2
    pub const OUT2: u32 = 1 << 13;
    /// RTS hardware flow control
    pub const RTSEN: u32 = 1 << 14;
    /// CTS hardware flow control
    pub const CTSEN: u32 = 1 << 15;
}

/// Mini UART LSR (Line Status Register) bits
pub mod mu_lsr {
    /// Data ready
    pub const DR: u32 = 1 << 0;
    /// Overrun error
    pub const OE: u32 = 1 << 1;
    /// Parity error
    pub const PE: u32 = 1 << 2;
    /// Framing error
    pub const FE: u32 = 1 << 3;
    /// Break
    pub const BK: u32 = 1 << 4;
    /// Transmitter empty
    pub const TE: u32 = 1 << 5;
    /// Transmitter idle
    pub const TI: u32 = 1 << 6;
    /// FIFO data error
    pub const FERR: u32 = 1 << 7;
}

/// Mini UART CNTL (Control) bits
pub mod mu_cntl {
    /// Enable receiver
    pub const RXE: u32 = 1 << 0;
    /// Enable transmitter
    pub const TXE: u32 = 1 << 1;
    /// Enable RTS
    pub const RTS: u32 = 1 << 2;
    /// Enable auto flow control CTS
    pub const AUTOFLOW_CTS: u32 = 1 << 3;
    /// Enable auto flow control RTS
    pub const AUTOFLOW_RTS: u32 = 1 << 4;
    /// Receiver trigger level (mask)
    pub const RTRIGGER: u32 = 3 << 5;
}

/// UART types supported
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UartType {
    /// PL011 UART (primary serial console)
    Pl011,
    /// Mini UART (auxiliary)
    MiniUart,
}

/// UART configuration
#[derive(Debug, Clone, Copy)]
pub struct UartConfig {
    /// Baud rate
    pub baud_rate: u32,
    /// Data bits (5, 6, 7, 8)
    pub data_bits: u8,
    /// Stop bits (1, 2)
    pub stop_bits: u8,
    /// Parity (0 = none, 1 = odd, 2 = even)
    pub parity: u8,
    /// Use hardware flow control
    pub flow_control: bool,
}

impl Default for UartConfig {
    fn default() -> Self {
        Self {
            baud_rate: 115200,
            data_bits: 8,
            stop_bits: 1,
            parity: 0,
            flow_control: false,
        }
    }
}

/// UART driver state
pub struct UartDriver {
    /// Base address of UART registers
    base_addr: usize,
    /// UART type
    uart_type: UartType,
    /// Current configuration
    config: UartConfig,
    /// Initialized flag
    initialized: bool,
}

/// Global UART driver instance for console
static mut UART_DRIVER: UartDriver = UartDriver {
    base_addr: 0,
    uart_type: UartType::Pl011,
    config: UartConfig {
        baud_rate: 115200,
        data_bits: 8,
        stop_bits: 1,
        parity: 0,
        flow_control: false,
    },
    initialized: false,
};

/// Initialize the UART driver
/// This sets up the primary serial console (PL011 UART0)
pub fn init() {
    println!("[UART] Initializing UART driver...");
    
    let info = platform_info();
    
    // Determine which UART to use based on platform
    let (uart_type, base_addr) = match info.platform_type {
        PlatformType::RaspberryPi5 => {
            // Pi 5 uses RP1 for UART
            (UartType::Pl011, info.uart0_base)
        }
        PlatformType::RaspberryPi4 => {
            // Pi 4 has PL011 at 0xFE201000
            (UartType::Pl011, info.uart0_base)
        }
        PlatformType::QemuVirt => {
            // QEMU virt uses PL011 at 0x09000000
            (UartType::Pl011, info.uart0_base)
        }
        _ => {
            // Default to PL011
            (UartType::Pl011, info.uart0_base)
        }
    };
    
    let config = UartConfig::default();
    
    unsafe {
        UART_DRIVER = UartDriver {
            base_addr,
            uart_type,
            config,
            initialized: false,
        };
    }
    
    println!("[UART] Base address: 0x{:016X}", base_addr);
    println!("[UART] Type: {:?}", uart_type);
    
    // Initialize the UART
    match uart_type {
        UartType::Pl011 => {
            init_pl011(base_addr, &config);
        }
        UartType::MiniUart => {
            init_mini_uart(base_addr, &config);
        }
    }
    
    unsafe {
        UART_DRIVER.initialized = true;
    }
    
    println!("[UART] Initialization complete at {} baud", config.baud_rate);
}

/// Initialize PL011 UART
fn init_pl011(base: usize, config: &UartConfig) {
    println!("[UART] Initializing PL011 UART...");
    
    unsafe {
        // Disable UART first
        mmio::write32(base + pl011_regs::CR, 0);
        
        // Flush FIFOs
        mmio::write32(base + pl011_regs::LCRH, 0);
        
        // Clear all interrupts
        mmio::write32(base + pl011_regs::ICR, 0x7FF);
        
        // Calculate baud rate divisor
        // UART clock / (16 * baud_rate)
        let info = platform_info();
        let uart_clock = info.uart_clock_hz as u64;
        let baud_rate = config.baud_rate as u64;
        
        let divisor = (uart_clock * 4) / baud_rate; // Multiply by 4 for fractional part
        let ibrd = (divisor >> 6) as u32;
        let fbrd = (divisor & 0x3F) as u32;
        
        println!("[UART] Clock: {} Hz, Divisor: {}/{}", uart_clock, ibrd, fbrd);
        
        mmio::write32(base + pl011_regs::IBRD, ibrd);
        mmio::write32(base + pl011_regs::FBRD, fbrd);
        
        // Configure line control: 8 data bits, 1 stop bit, no parity, FIFO enabled
        let mut lcrh = pl011_lcrh::WLEN_8 | pl011_lcrh::FEN;
        if config.stop_bits == 2 {
            lcrh |= pl011_lcrh::STP2;
        }
        if config.parity != 0 {
            lcrh |= pl011_lcrh::PEN;
            if config.parity == 2 {
                lcrh |= pl011_lcrh::EPS;
            }
        }
        mmio::write32(base + pl011_regs::LCRH, lcrh);
        
        // Set interrupt FIFO levels
        mmio::write32(base + pl011_regs::IFLS, 0); // 1/8 full for RX, 1/8 empty for TX
        
        // Disable all interrupts for now (polling mode)
        mmio::write32(base + pl011_regs::IMSC, 0);
        
        // Enable UART, TX, and RX
        let mut cr = pl011_cr::UARTEN | pl011_cr::TXE | pl011_cr::RXE;
        if config.flow_control {
            cr |= pl011_cr::RTSEN | pl011_cr::CTSEN;
        }
        mmio::write32(base + pl011_regs::CR, cr);
        
        mmio::memory_barrier();
    }
    
    // Small delay for UART to stabilize
    delay::microseconds(100);
}

/// Initialize Mini UART
fn init_mini_uart(base: usize, config: &UartConfig) {
    println!("[UART] Initializing Mini UART...");
    
    unsafe {
        // Note: Mini UART requires AUX block to be enabled first
        // The base address for mini UART includes the AUX offset
        let aux_base = base & !0xFFFF; // Align to 64KB boundary
        
        // Enable Mini UART
        let enables = mmio::read32(aux_base + mini_uart_regs::AUX_ENABLES);
        mmio::write32(aux_base + mini_uart_regs::AUX_ENABLES, enables | 1);
        
        // Disable TX/RX and auto flow control
        mmio::write32(base + mini_uart_regs::MU_CNTL, 0);
        
        // Disable interrupts
        mmio::write32(base + mini_uart_regs::MU_IER, 0);
        
        // Enable 8-bit mode
        mmio::write32(base + mini_uart_regs::MU_LCR, 3);
        
        // Set RTS high
        mmio::write32(base + mini_uart_regs::MU_MCR, 0);
        
        // Clear FIFOs
        mmio::write32(base + mini_uart_regs::MU_IIR, 0xC6);
        
        // Calculate baud rate
        let info = platform_info();
        let uart_clock = info.uart_clock_hz;
        let baud_reg = (uart_clock / (8 * config.baud_rate)) - 1;
        
        mmio::write32(base + mini_uart_regs::MU_BAUD, baud_reg);
        
        // Enable TX and RX
        mmio::write32(base + mini_uart_regs::MU_CNTL, mu_cntl::RXE | mu_cntl::TXE);
        
        mmio::memory_barrier();
    }
    
    delay::microseconds(100);
}

/// Get the UART driver instance
fn driver() -> &'static mut UartDriver {
    unsafe { &mut UART_DRIVER }
}

/// Send a single character (blocking)
pub fn putc(c: u8) {
    let drv = driver();
    if !drv.initialized {
        return;
    }
    
    match drv.uart_type {
        UartType::Pl011 => {
            pl011_putc(drv.base_addr, c);
        }
        UartType::MiniUart => {
            mini_uart_putc(drv.base_addr, c);
        }
    }
}

/// Send a single character via PL011
fn pl011_putc(base: usize, c: u8) {
    unsafe {
        // Wait for transmit FIFO to have space
        while mmio::read32(base + pl011_regs::FR) & pl011_fr::TXFF != 0 {
            core::arch::asm!("nop", options(nomem, nostack));
        }
        
        // Write the character
        mmio::write32(base + pl011_regs::DR, c as u32);
    }
}

/// Send a single character via Mini UART
fn mini_uart_putc(base: usize, c: u8) {
    unsafe {
        // Wait for transmitter to be empty
        while mmio::read32(base + mini_uart_regs::MU_LSR) & mu_lsr::TE == 0 {
            core::arch::asm!("nop", options(nomem, nostack));
        }
        
        // Write the character
        mmio::write32(base + mini_uart_regs::MU_IO, c as u32);
    }
}

/// Send a string
pub fn puts(s: &str) {
    for c in s.bytes() {
        if c == b'\n' {
            putc(b'\r');
        }
        putc(c);
    }
}

/// Receive a single character (blocking)
/// Returns None if UART is not initialized
pub fn getc() -> Option<u8> {
    let drv = driver();
    if !drv.initialized {
        return None;
    }
    
    match drv.uart_type {
        UartType::Pl011 => {
            pl011_getc(drv.base_addr)
        }
        UartType::MiniUart => {
            mini_uart_getc(drv.base_addr)
        }
    }
}

/// Receive a single character via PL011 (blocking)
fn pl011_getc(base: usize) -> Option<u8> {
    unsafe {
        // Wait for receive FIFO to have data
        while mmio::read32(base + pl011_regs::FR) & pl011_fr::RXFE != 0 {
            core::arch::asm!("nop", options(nomem, nostack));
        }
        
        // Read the character
        let data = mmio::read32(base + pl011_regs::DR);
        Some((data & 0xFF) as u8)
    }
}

/// Receive a single character via Mini UART (blocking)
fn mini_uart_getc(base: usize) -> Option<u8> {
    unsafe {
        // Wait for data ready
        while mmio::read32(base + mini_uart_regs::MU_LSR) & mu_lsr::DR == 0 {
            core::arch::asm!("nop", options(nomem, nostack));
        }
        
        // Read the character
        let data = mmio::read32(base + mini_uart_regs::MU_IO);
        Some((data & 0xFF) as u8)
    }
}

/// Check if a character is available to read
pub fn has_data() -> bool {
    let drv = driver();
    if !drv.initialized {
        return false;
    }
    
    match drv.uart_type {
        UartType::Pl011 => {
            unsafe {
                mmio::read32(drv.base_addr + pl011_regs::FR) & pl011_fr::RXFE == 0
            }
        }
        UartType::MiniUart => {
            unsafe {
                mmio::read32(drv.base_addr + mini_uart_regs::MU_LSR) & mu_lsr::DR != 0
            }
        }
    }
}

/// Try to receive a character (non-blocking)
/// Returns Some(byte) if data available, None otherwise
pub fn try_getc() -> Option<u8> {
    if has_data() {
        getc()
    } else {
        None
    }
}

/// Flush the transmit buffer
pub fn flush() {
    let drv = driver();
    if !drv.initialized {
        return;
    }
    
    match drv.uart_type {
        UartType::Pl011 => {
            unsafe {
                // Wait for transmit to complete
                while mmio::read32(drv.base_addr + pl011_regs::FR) & pl011_fr::BUSY != 0 {
                    core::arch::asm!("nop", options(nomem, nostack));
                }
            }
        }
        UartType::MiniUart => {
            unsafe {
                // Wait for transmitter idle
                while mmio::read32(drv.base_addr + mini_uart_regs::MU_LSR) & mu_lsr::TI == 0 {
                    core::arch::asm!("nop", options(nomem, nostack));
                }
            }
        }
    }
}

/// Configure the UART with new settings
pub fn configure(config: &UartConfig) {
    let drv = driver();
    if !drv.initialized {
        return;
    }
    
    // Re-initialize with new configuration
    match drv.uart_type {
        UartType::Pl011 => {
            init_pl011(drv.base_addr, config);
        }
        UartType::MiniUart => {
            init_mini_uart(drv.base_addr, config);
        }
    }
    
    drv.config = *config;
}

/// Change baud rate
pub fn set_baud_rate(baud_rate: u32) {
    let drv = driver();
    let mut config = drv.config;
    config.baud_rate = baud_rate;
    configure(&config);
}

/// Get current configuration
pub fn get_config() -> UartConfig {
    driver().config
}

/// Print UART driver information
pub fn print_info() {
    let drv = driver();
    
    println!("UART Driver Information:");
    println!("  Initialized: {}", drv.initialized);
    println!("  Type: {:?}", drv.uart_type);
    println!("  Base Address: 0x{:016X}", drv.base_addr);
    println!("  Baud Rate: {}", drv.config.baud_rate);
    println!("  Data Bits: {}", drv.config.data_bits);
    println!("  Stop Bits: {}", drv.config.stop_bits);
    println!("  Parity: {}", if drv.config.parity == 0 { "None" } else if drv.config.parity == 1 { "Odd" } else { "Even" });
}

/// UART writer for use with fmt::Write trait
pub struct UartWriter;

impl Write for UartWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        puts(s);
        Ok(())
    }
}

/// Get a UART writer for use with write! macro
pub fn writer() -> UartWriter {
    UartWriter
}

/// Test the UART by sending a test pattern
pub fn test_pattern() {
    puts("\r\nUART Test Pattern:\r\n");
    puts("ABCDEFGHIJKLMNOPQRSTUVWXYZ\r\n");
    puts("abcdefghijklmnopqrstuvwxyz\r\n");
    puts("0123456789\r\n");
    puts("!@#$%^&*()_+-=[]{}|;':\",./<>?\r\n");
    puts("UART test complete.\r\n");
}

/// Simple echo test - echoes back characters received
pub fn echo_test(count: u32) {
    puts("\r\nUART Echo Test - Type characters:\r\n");
    
    for i in 0..count {
        if let Some(c) = getc() {
            // Echo the character back
            putc(c);
            
            // Add newline if Enter pressed
            if c == b'\r' {
                putc(b'\n');
                puts("Echo: ");
            }
            
            if i % 10 == 9 {
                puts("\r\n");
            }
        }
    }
    
    puts("\r\nEcho test complete.\r\n");
}
