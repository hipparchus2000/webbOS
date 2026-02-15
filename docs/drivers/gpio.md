# GPIO Driver Documentation

## Overview

The GPIO driver provides control over General Purpose Input/Output pins on the Raspberry Pi 5 and Pi 4. It supports pin configuration, level control, interrupts, and pull-up/down resistors.

## Architecture

### Raspberry Pi 5 (RP1 I/O Controller)

The Pi 5 uses the RP1 I/O controller which is accessed differently than previous models:

- **Base Address**: `0x1F00000000` (via PCIe/high-speed bus)
- **GPIO Offset**: `0xD0000`
- **Access Method**: Memory-mapped or PCIe configuration

### Raspberry Pi 4 (BCM2711)

Traditional memory-mapped GPIO:

- **Base Address**: `0xFE000000`
- **GPIO Offset**: `0x200000`
- **Access Method**: Direct MMIO

## API Reference

### Pin Functions

```rust
pub enum GpioFunction {
    Input = 0,      // Digital input
    Output = 1,     // Digital output
    Alt0 = 4,       // Alternative function 0
    Alt1 = 5,       // Alternative function 1
    Alt2 = 6,       // Alternative function 2
    Alt3 = 7,       // Alternative function 3
    Alt4 = 3,       // Alternative function 4
    Alt5 = 2,       // Alternative function 5
}
```

### Pull-up/down Configuration

```rust
pub enum GpioPull {
    None = 0,       // No pull resistor
    Up = 1,         // Pull-up resistor
    Down = 2,       // Pull-down resistor
}
```

### Interrupt Edges

```rust
pub enum GpioEdge {
    None = 0,       // No interrupt
    Rising = 1,     // Rising edge trigger
    Falling = 2,    // Falling edge trigger
    Both = 3,       // Both edges trigger
}
```

## Functions

### Initialization

```rust
// Called automatically during driver initialization
gpio::init();
```

### Pin Configuration

```rust
// Set pin function
gpio::set_function(pin: u8, function: GpioFunction);

// Set as input/output (convenience functions)
gpio::set_direction(pin: u8, is_output: bool);
```

### Level Control

```rust
// Set pin high/low
gpio::set_level(pin: u8, high: bool);

// Read pin level
gpio::get_level(pin: u8) -> bool;

// Toggle pin
gpio::toggle(pin: u8);
```

### Pull Configuration

```rust
// Configure pull-up/down resistor
gpio::set_pull(pin: u8, pull: GpioPull);
```

### Interrupts

```rust
// Enable interrupt on pin
gpio::enable_interrupt(pin: u8, edge: GpioEdge);

// Disable interrupt
gpio::disable_interrupt(pin: u8);
```

### Testing

```rust
// Blink LED on pin
gpio::blink_led(pin: u8, count: u32, delay_ms: u32);

// Test activity LED (GPIO 42)
gpio::test_activity_led();

// Print driver info
gpio::print_info();
```

## Usage Examples

### LED Control

```rust
use crate::drivers::raspberrypi::gpio;

// Configure GPIO 42 (activity LED) as output
gpio::set_function(42, gpio::GpioFunction::Output);
gpio::set_pull(42, gpio::GpioPull::None);

// Turn LED on
gpio::set_level(42, true);

// Turn LED off
gpio::set_level(42, false);

// Blink LED 10 times
gpio::blink_led(42, 10, 200);
```

### Input Reading

```rust
// Configure GPIO 20 as input with pull-up
gpio::set_function(20, gpio::GpioFunction::Input);
gpio::set_pull(20, gpio::GpioPull::Up);

// Read input level
let pressed = !gpio::get_level(20); // Active low
```

### Interrupt Handling

```rust
// Configure GPIO 21 for rising edge interrupt
gpio::set_function(21, gpio::GpioFunction::Input);
gpio::set_pull(21, gpio::GpioPull::Up);
gpio::enable_interrupt(21, gpio::GpioEdge::Rising);

// Interrupt will be handled by the system interrupt controller
```

## Pin Mappings

### Common GPIO Pins

| Pin | Function | Typical Use |
|-----|----------|-------------|
| 2 | SDA1 | I2C data |
| 3 | SCL1 | I2C clock |
| 14 | TXD0 | UART0 transmit |
| 15 | RXD0 | UART0 receive |
| 18 | PCM_CLK | Audio clock |
| 40 | - | User GPIO |
| 41 | - | User GPIO |
| 42 | ACT_LED | Activity LED |
| 43 | - | User GPIO |
| 44 | - | User GPIO |
| 45 | - | User GPIO |

## Implementation Details

### RP1 Registers (Pi 5)

```rust
pub mod rp1_regs {
    pub const GIO_STATUS: usize = 0x00;
    pub const GIO_RAW_INT: usize = 0x04;
    pub const GIO_INT_EN: usize = 0x08;
    pub const GIO_INT_DIS: usize = 0x0C;
    pub const GIO_INT_TYPE: usize = 0x10;
    pub const GIO_INT_POLARITY: usize = 0x14;
    pub const GIO_DATA: usize = 0x18;
    pub const GIO_DIR: usize = 0x1C;
    pub const GIO_MODE: usize = 0x20;
    pub const GIO_PULL_EN: usize = 0x24;
    pub const GIO_PULL_DIR: usize = 0x28;
    pub const GIO_DRIVE: usize = 0x2C;
    pub const GIO_SET: usize = 0x30;
    pub const GIO_CLR: usize = 0x34;
}
```

### BCM2711 Registers (Pi 4)

```rust
pub mod bcm2711_regs {
    pub const GPFSEL0: usize = 0x00;  // Function select 0-9
    pub const GPFSEL1: usize = 0x04;  // Function select 10-19
    pub const GPFSEL2: usize = 0x08;  // Function select 20-29
    pub const GPFSEL3: usize = 0x0C;  // Function select 30-39
    pub const GPFSEL4: usize = 0x10;  // Function select 40-49
    pub const GPFSEL5: usize = 0x14;  // Function select 50-53
    pub const GPSET0: usize = 0x1C;   // Set pins 0-31
    pub const GPSET1: usize = 0x20;   // Set pins 32-53
    pub const GPCLR0: usize = 0x28;   // Clear pins 0-31
    pub const GPCLR1: usize = 0x2C;   // Clear pins 32-53
    pub const GPLEV0: usize = 0x34;   // Level pins 0-31
    pub const GPLEV1: usize = 0x38;   // Level pins 32-53
}
```

## Testing

### Automated Tests

```rust
// Run GPIO tests
gpio::blink_led(42, 10, 200);

// Read/write test
gpio::set_function(42, gpio::GpioFunction::Output);
for i in 0..10 {
    gpio::toggle(42);
    delay::milliseconds(100);
}
```

### Manual Verification

1. Connect LED to GPIO 42 (with appropriate resistor)
2. Boot webbOS
3. Run: `test` command in kernel console
4. Observe LED blinking pattern

## Troubleshooting

### GPIO Not Responding

- Verify platform detection: Check `info` command output
- Check pin number: Must be 0-53
- Verify pin function: Set to Output before writing

### Interrupts Not Firing

- Check interrupt controller initialization
- Verify edge configuration
- Ensure GPIO IRQ routing in GIC

## See Also

- [HAL Documentation](hal.md) - Platform detection
- [UART Documentation](uart.md) - Alternative functions
- [Testing Guide](../testing.md) - Comprehensive testing
