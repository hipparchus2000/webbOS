# UART Driver Documentation

## Overview

The UART driver provides serial communication support for webbOS, enabling console output and debugging. It supports both the PL011 UART (primary) and Mini UART (auxiliary) controllers found on Raspberry Pi.

## Supported UART Controllers

### PL011 UART (UART0)

- **Location**: GPIO 14 (TX) / GPIO 15 (RX)
- **Features**: Full-featured ARM PrimeCell UART
- **FIFO**: 16-byte Tx/Rx FIFOs
- **Baud Rates**: Up to 921600 (limited by clock)

### Mini UART (UART1)

- **Location**: GPIO 14 (TX) / GPIO 15 (RX) - Alt function
- **Features**: Simpler, 8-bit only
- **FIFO**: Smaller buffer
- **Use Case**: Bluetooth on Pi 3/4

## Platform Support

| Platform | Primary UART | Base Address |
|----------|-------------|--------------|
| Raspberry Pi 5 | PL011 via RP1 | 0x1F000D0000 |
| Raspberry Pi 4 | PL011 | 0xFE201000 |
| QEMU virt | PL011 | 0x09000000 |

## API Reference

### Initialization

```rust
// Called automatically during boot
uart::init();

// Print UART info
uart::print_info();
```

### Output Functions

```rust
// Send single character
uart::putc(c: u8);

// Send string (adds \r for \n)
uart::puts(s: &str);

// Flush transmit buffer
uart::flush();
```

### Input Functions

```rust
// Receive character (blocking)
uart::getc() -> Option<u8>;

// Check if data available
uart::has_data() -> bool;

// Try to receive (non-blocking)
uart::try_getc() -> Option<u8>;
```

### Configuration

```rust
// UART configuration structure
pub struct UartConfig {
    pub baud_rate: u32,      // Default: 115200
    pub data_bits: u8,       // Default: 8
    pub stop_bits: u8,       // Default: 1
    pub parity: u8,          // 0=none, 1=odd, 2=even
    pub flow_control: bool,  // Default: false
}

// Get current configuration
uart::get_config() -> UartConfig;

// Configure UART
uart::configure(&UartConfig);

// Change baud rate
uart::set_baud_rate(115200);
```

### Testing

```rust
// Send test pattern
uart::test_pattern();

// Echo test (receive and echo back)
uart::echo_test(count: u32);
```

### fmt::Write Support

```rust
use core::fmt::Write;

// Get UART writer for use with write! macro
let mut writer = uart::writer();
write!(writer, "Value: {}\n", 42).unwrap();
```

## Usage Examples

### Basic Output

```rust
use crate::drivers::raspberrypi::uart;

// Send character
uart::putc(b'A');

// Send string
uart::puts("Hello, World!\r\n");

// Using fmt::Write
use core::fmt::Write;
write!(uart::writer(), "Number: {}\r\n", 42).unwrap();
```

### Input Handling

```rust
// Blocking read
if let Some(c) = uart::getc() {
    uart::putc(c); // Echo back
}

// Non-blocking read
while let Some(c) = uart::try_getc() {
    process_char(c);
}
```

### Configuration

```rust
// Change to 9600 baud
uart::set_baud_rate(9600);

// Full configuration
let config = uart::UartConfig {
    baud_rate: 115200,
    data_bits: 8,
    stop_bits: 1,
    parity: 0,      // None
    flow_control: false,
};
uart::configure(&config);
```

## Register Details

### PL011 Registers

```rust
pub mod pl011_regs {
    pub const DR: usize = 0x00;      // Data Register
    pub const RSRECR: usize = 0x04;  // Receive Status
    pub const FR: usize = 0x18;      // Flag Register
    pub const ILPR: usize = 0x20;    // IrDA Low-Power
    pub const IBRD: usize = 0x24;    // Integer Baud Rate
    pub const FBRD: usize = 0x28;    // Fractional Baud Rate
    pub const LCRH: usize = 0x2C;    // Line Control
    pub const CR: usize = 0x30;      // Control Register
    pub const IFLS: usize = 0x34;    // Interrupt FIFO Level
    pub const IMSC: usize = 0x38;    // Interrupt Mask
    pub const RIS: usize = 0x3C;     // Raw Interrupt Status
    pub const MIS: usize = 0x40;     // Masked Interrupt Status
    pub const ICR: usize = 0x44;     // Interrupt Clear
    pub const DMACR: usize = 0x48;   // DMA Control
}
```

### Flag Register Bits

```rust
pub mod pl011_fr {
    pub const CTS: u32 = 1 << 0;   // Clear to Send
    pub const DSR: u32 = 1 << 1;   // Data Set Ready
    pub const DCD: u32 = 1 << 2;   // Data Carrier Detect
    pub const BUSY: u32 = 1 << 3;  // UART Busy
    pub const RXFE: u32 = 1 << 4;  // Receive FIFO Empty
    pub const TXFF: u32 = 1 << 5;  // Transmit FIFO Full
    pub const RXFF: u32 = 1 << 6;  // Receive FIFO Full
    pub const TXFE: u32 = 1 << 7;  // Transmit FIFO Empty
    pub const RI: u32 = 1 << 8;    // Ring Indicator
}
```

### Control Register Bits

```rust
pub mod pl011_cr {
    pub const UARTEN: u32 = 1 << 0;  // UART Enable
    pub const SIREN: u32 = 1 << 1;   // SIR Enable
    pub const SIRLPM: u32 = 1 << 2;  // SIR Low Power
    pub const LBE: u32 = 1 << 7;     // Loopback Enable
    pub const TXE: u32 = 1 << 8;     // Transmit Enable
    pub const RXE: u32 = 1 << 9;     // Receive Enable
    pub const DTR: u32 = 1 << 10;    // Data Transmit Ready
    pub const RTS: u32 = 1 << 11;    // Request to Send
    pub const RTSEN: u32 = 1 << 14;  // RTS Hardware Flow
    pub const CTSEN: u32 = 1 << 15;  // CTS Hardware Flow
}
```

## Baud Rate Calculation

### PL011 Formula

```
Divisor = UART_Clock / (16 * Baud_Rate)
IBRD = Integer part of Divisor
FBRD = Fractional part * 64 + 0.5
```

### Example: 115200 baud @ 48MHz

```
Divisor = 48,000,000 / (16 * 115200) = 26.042
IBRD = 26
FBRD = 0.042 * 64 = 2.688 ≈ 3
Actual baud = 48,000,000 / (16 * (26 + 3/64)) ≈ 115177 (0.02% error)
```

## Hardware Connection

### USB-to-TTL Cable

```
USB-TTL Cable     Pi GPIO
------------      -------
GND      ------>  GND  (Pin 6)
TX       ------>  RXD0 (Pin 10, GPIO 15)
RX       ------>  TXD0 (Pin 8, GPIO 14)
```

**Important**: Do NOT connect 5V from USB-TTL to Pi!

### Terminal Settings

- **Baud Rate**: 115200
- **Data Bits**: 8
- **Stop Bits**: 1
- **Parity**: None
- **Flow Control**: None

## Testing

### Automatic Tests

```rust
// Run UART tests
uart::test_pattern();
uart::echo_test(100);
```

### Manual Tests

1. Connect USB-to-TTL cable
2. Open terminal at 115200 baud
3. Power on Pi
4. Type characters - should see echo
5. Run: `test` command for diagnostics

## Troubleshooting

### No Serial Output

1. **Check connections**: Verify TX/RX not swapped
2. **Check baud rate**: Verify 115200 in terminal
3. **Check GPIO config**: Pins 14/15 in ALT0 function
4. **Check cable**: Try different USB-TTL adapter

### Garbled Output

1. **Baud rate mismatch**: Verify both sides use 115200
2. **Clock issue**: Check UART clock frequency setting
3. **Electrical noise**: Use shorter cables, add ferrite bead

### No Input

1. **Flow control**: Ensure disabled in terminal
2. **Pin function**: GPIO 15 must be ALT0 (RX)
3. **Voltage levels**: Ensure 3.3V logic levels

## See Also

- [GPIO Documentation](gpio.md) - Pin configuration
- [HAL Documentation](hal.md) - Platform detection
- [Testing Guide](../testing.md) - Serial console testing
