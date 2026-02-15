# WebbOS Driver Documentation

This directory contains documentation for webbOS device drivers, specifically for the Raspberry Pi 5 ARM64 port (Week 3 deliverables).

## Overview

The webbOS driver architecture consists of:

1. **Hardware Abstraction Layer (HAL)** - Platform detection and MMIO helpers
2. **GPIO Driver** - General Purpose Input/Output control
3. **UART Driver** - Serial console communication
4. **USB Driver** - USB 3.0/2.0 controller support (research phase)
5. **Ethernet Driver** - Network interface support (research phase)

## Quick Start

### Building the Drivers

```bash
cd projects/webbos
./build-aarch64-drivers.sh
```

### Testing on QEMU

```bash
./test-qemu-aarch64.sh
```

### Testing on Raspberry Pi 5

1. Copy `build/aarch64/kernel8.img` to SD card boot partition
2. Copy `build/aarch64/config.txt` to SD card
3. Insert SD card and power on
4. Connect USB-to-TTL serial cable to GPIO 14/15 (UART0)
5. Open serial terminal at 115200 baud

## Driver Status

| Driver | Status | Notes |
|--------|--------|-------|
| GPIO | Implemented | Full support for Pi 5 RP1 and Pi 4 BCM2711 |
| UART | Implemented | PL011 primary, Mini UART auxiliary |
| USB | Research | XHCI skeleton, needs PCIe enumeration |
| Ethernet | Research | RTL8168 skeleton, needs DMA implementation |

## Documentation

- [GPIO Driver](gpio.md) - GPIO pin control, interrupts, pull-up/down
- [UART Driver](uart.md) - Serial console configuration
- [USB Driver](usb.md) - XHCI/EHCI research notes
- [Ethernet Driver](ethernet.md) - MAC/PHY research notes
- [HAL](hal.md) - Hardware Abstraction Layer

## Architecture

### Platform Support

The drivers support multiple platforms:

- **Raspberry Pi 5** - Uses RP1 I/O controller via PCIe/high-speed bus
- **Raspberry Pi 4** - Uses BCM2711 memory-mapped peripherals
- **QEMU virt** - Uses emulated PL011 and virtio devices

### File Organization

```
kernel/src/
├── hal/
│   └── mod.rs              # Hardware Abstraction Layer
├── arch/aarch64/
│   ├── mod.rs              # ARM64 architecture
│   ├── cpu.rs              # CPU initialization
│   ├── interrupts.rs       # Exception handling
│   ├── paging.rs           # MMU setup
│   └── boot.S              # Boot assembly
└── drivers/
    ├── raspberrypi/
    │   ├── mod.rs          # Pi driver module
    │   ├── gpio/
    │   │   ├── mod.rs      # GPIO driver
    │   │   └── blink_test.rs # LED test
    │   ├── uart/
    │   │   └── mod.rs      # UART driver
    │   ├── usb/
    │   │   └── mod.rs      # USB driver (research)
    │   └── ethernet/
    │       └── mod.rs      # Ethernet driver (research)
    └── tests/
        └── mod.rs          # Driver test suite
```

## API Reference

### GPIO Driver

```rust
use crate::drivers::raspberrypi::gpio;

// Configure pin as output
gpio::set_function(42, gpio::GpioFunction::Output);

// Set pin high/low
gpio::set_level(42, true);
gpio::set_level(42, false);

// Configure pull-up/down
gpio::set_pull(42, gpio::GpioPull::Up);

// Enable interrupt
gpio::enable_interrupt(42, gpio::GpioEdge::Rising);

// Blink LED
gpio::blink_led(42, 10, 200); // pin, count, delay_ms
```

### UART Driver

```rust
use crate::drivers::raspberrypi::uart;

// Send character
uart::putc(b'A');

// Send string
uart::puts("Hello, World!\n");

// Receive character (blocking)
if let Some(c) = uart::getc() {
    // Process character
}

// Configure baud rate
uart::set_baud_rate(115200);
```

## Testing

### Automated Tests

```rust
// Run all driver tests
drivers::run_tests();

// Run specific tests
drivers::tests::test_platform_detection();
drivers::tests::test_gpio_blink();
```

### Manual Tests

Connect to serial console and use commands:
- `info` - Show system information
- `test` - Run test suite
- `storage` - Show storage info

## Hardware Notes

### Raspberry Pi 5 Differences

The Pi 5 introduces significant architectural changes:

1. **RP1 I/O Controller** - Separate chip for GPIO/UART
2. **PCIe XHCI** - USB 3.0 via PCIe instead of internal DWC2
3. **PCIe Ethernet** - External MAC/PHY via PCIe

### GPIO Pinout

| Pin | Function | Notes |
|-----|----------|-------|
| 14 | UART0 TX | Serial console |
| 15 | UART0 RX | Serial console |
| 42 | Activity LED | Boot status |

### Memory Map

| Address | Size | Description |
|---------|------|-------------|
| 0x1F00000000 | 64MB | RP1 Peripheral Base (Pi 5) |
| 0xFE000000 | 16MB | BCM2711 Peripheral Base (Pi 4) |
| 0x80000 | 2MB | Kernel load address |

## Resources

- [ARM Cortex-A76 TRM](https://developer.arm.com/documentation/)
- [Raspberry Pi 5 Documentation](https://www.raspberrypi.com/documentation/)
- [XHCI Specification 1.2](https://www.intel.com/content/www/us/en/io/universal-serial-bus/)
- [USB 3.2 Specification](https://www.usb.org/usb32)

## Contributing

When adding new drivers:

1. Create driver module in `drivers/raspberrypi/`
2. Add initialization to `drivers/raspberrypi/mod.rs`
3. Update platform detection in HAL
4. Add tests to `drivers/tests/`
5. Document in `docs/drivers/`

## License

MIT OR Apache-2.0 (same as webbOS kernel)
