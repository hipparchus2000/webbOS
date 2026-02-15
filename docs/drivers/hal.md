# Hardware Abstraction Layer (HAL) Documentation

## Overview

The Hardware Abstraction Layer (HAL) provides platform detection and hardware-specific abstractions for webbOS. It allows the kernel to run on different platforms (Raspberry Pi 5, Pi 4, QEMU) without modification.

## Features

- **Platform Detection**: Automatic detection of hardware platform
- **Device Tree Parsing**: Read hardware configuration from DTB
- **MMIO Helpers**: Safe memory-mapped I/O operations
- **Delay Functions**: Timing for hardware initialization
- **CPU Information**: Architecture feature detection

## Supported Platforms

| Platform | Type | CPU | Peripherals |
|----------|------|-----|-------------|
| Raspberry Pi 5 | `RaspberryPi5` | Cortex-A76 | RP1 I/O |
| Raspberry Pi 4 | `RaspberryPi4` | Cortex-A72 | BCM2711 |
| QEMU virt | `QemuVirt` | Cortex-A72 (emulated) | virtio |
| Generic | `GenericArm64` | Unknown | Unknown |

## Platform Detection

### Detection Methods

1. **Device Tree** (preferred): Parse DTB passed by bootloader
2. **CPU Features**: Read MIDR_EL1 register
3. **Fallback**: Assume QEMU for testing

### Usage

```rust
use crate::hal;

// Initialize HAL (called during boot)
hal::init();

// Get platform information
let info = hal::platform_info();
println!("Platform: {:?}", info.platform_type);

// Check specific platform
if hal::is_raspberry_pi5() {
    println!("Running on Raspberry Pi 5!");
}
```

## Platform Information

### Structure

```rust
pub struct PlatformInfo {
    pub platform_type: PlatformType,     // Platform enum
    pub cpu_freq_hz: u64,                // CPU clock frequency
    pub uart_clock_hz: u64,              // UART base clock
    pub gpio_base: usize,                // GPIO MMIO base
    pub uart0_base: usize,               // UART0 MMIO base
    pub mini_uart_base: usize,           // Mini UART MMIO base
    pub gic_base: usize,                 // GIC MMIO base
    pub peripheral_base: usize,          // Peripherals base
    pub timer_base: usize,               // Timer MMIO base
    pub usb_xhci_base: usize,            // USB XHCI MMIO base
    pub ethernet_base: usize,            // Ethernet MMIO base
}
```

### Platform-Specific Values

#### Raspberry Pi 5

```rust
PlatformInfo {
    platform_type: PlatformType::RaspberryPi5,
    cpu_freq_hz: 2_400_000_000,      // 2.4 GHz
    uart_clock_hz: 48_000_000,       // 48 MHz
    gpio_base: 0x1F000D0000,         // RP1 GPIO
    uart0_base: 0x1F000D0000,        // RP1 UART
    mini_uart_base: 0x1F000D1000,
    gic_base: 0x107FFF0000,          // GIC-400
    peripheral_base: 0x1F00000000,   // RP1 base
    timer_base: 0x107E00B000,
    usb_xhci_base: 0x1F01000000,     // Via PCIe
    ethernet_base: 0x1F02000000,     // Via PCIe
}
```

#### Raspberry Pi 4

```rust
PlatformInfo {
    platform_type: PlatformType::RaspberryPi4,
    cpu_freq_hz: 1_500_000_000,      // 1.5 GHz
    uart_clock_hz: 48_000_000,
    gpio_base: 0xFE200000,           // BCM2711 GPIO
    uart0_base: 0xFE201000,          // PL011
    mini_uart_base: 0xFE215000,
    gic_base: 0xFF840000,
    peripheral_base: 0xFE000000,
    timer_base: 0xFE003000,
    usb_xhci_base: 0,                // Via VL805
    ethernet_base: 0xFE580000,
}
```

#### QEMU virt

```rust
PlatformInfo {
    platform_type: PlatformType::QemuVirt,
    cpu_freq_hz: 1_000_000_000,      // 1 GHz
    uart_clock_hz: 0,                // Not applicable
    gpio_base: 0x09000000,
    uart0_base: 0x09000000,          // PL011
    mini_uart_base: 0,               // Not present
    gic_base: 0x08000000,
    peripheral_base: 0x08000000,
    timer_base: 0x09000000,
    usb_xhci_base: 0,                // virtio
    ethernet_base: 0,                // virtio
}
```

## MMIO Operations

### Safe Memory-Mapped I/O

```rust
use crate::hal::mmio;

// Read 32-bit value
let value = unsafe { mmio::read32(addr) };

// Write 32-bit value
unsafe { mmio::write32(addr, value) };

// Read 64-bit value
let value = unsafe { mmio::read64(addr) };

// Write 64-bit value
unsafe { mmio::write64(addr, value) };

// Memory barrier (ensure write completion)
mmio::memory_barrier();

// Individual barriers
mmio::dsb();  // Data synchronization
mmio::isb();  // Instruction synchronization
```

### Usage Guidelines

1. **Always use `read_volatile`/`write_volatile`**: Prevents compiler optimization
2. **Use barriers after writes**: Ensure hardware sees the update
3. **Check alignment**: Unaligned access causes faults on ARM64
4. **Cache considerations**: MMIO is typically device memory (uncached)

## Delay Functions

### Busy-Wait Delays

```rust
use crate::hal::delay;

// Microseconds delay
delay::microseconds(100);  // 100 µs

// Milliseconds delay
delay::milliseconds(10);   // 10 ms

// Seconds delay
delay::seconds(1);         // 1 second
```

### Implementation

Delays are implemented using busy-wait loops calibrated to CPU frequency:

```rust
pub fn microseconds(us: u32) {
    let freq_hz = platform_info().cpu_freq_hz;
    let iterations = (us as u64 * freq_hz) / 4_000_000;
    
    for _ in 0..iterations {
        unsafe { asm!("nop"); }
    }
}
```

### Accuracy

- **Accuracy**: ~10% at microsecond level
- **Jitter**: Affected by interrupts and cache
- **Use case**: Hardware initialization, not precise timing

## Device Tree

### Structure

```rust
#[repr(C)]
pub struct DeviceTreeHeader {
    pub magic: u32,           // 0xD00DFEED (big-endian)
    pub totalsize: u32,       // Total DTB size
    pub off_dt_struct: u32,   // Offset to structure block
    pub off_dt_strings: u32,  // Offset to strings block
    pub off_mem_rsvmap: u32,  // Offset to memory map
    pub version: u32,         // DTB version (17 = current)
    pub last_comp_version: u32, // Last compatible version
    pub boot_cpuid_phys: u32, // Boot CPU ID
    pub size_dt_strings: u32, // Strings block size
    pub size_dt_struct: u32,  // Structure block size
}
```

### Detection

The HAL searches for DTB at common locations:

1. `0x1000` - Common bootloader location
2. `0x10000` - Alternative location
3. `0x80000` - May overlap with kernel

### Parsing

```rust
fn parse_device_tree(dt_addr: usize) -> Option<PlatformInfo> {
    // Read model property
    let model = read_property(dt_addr, "model");
    
    match model {
        "Raspberry Pi 5" => Some(PlatformInfo::raspberry_pi5()),
        "Raspberry Pi 4" => Some(PlatformInfo::raspberry_pi4()),
        _ => None,
    }
}
```

## CPU Detection

### MIDR_EL1 Register

```
MIDR_EL1 Layout:
  [31:24] Implementer - 0x41 = ARM, 0x42 = Broadcom
  [23:20] Variant - Revision variant
  [19:16] Architecture - 0xF = ARMv8
  [15:4]  Part Number - Core identifier
  [3:0]   Revision - Core revision
```

### Part Numbers

| Part Number | Core | Used In |
|-------------|------|---------|
| 0xD03 | Cortex-A53 | Pi 3 |
| 0xD08 | Cortex-A72 | Pi 4 |
| 0xD0B | Cortex-A76 | Pi 5 |
| 0xD0C | Neoverse N1 | Server |

### Reading MIDR

```rust
pub fn cpu_info() -> (u32, u32) {
    let midr: u64;
    unsafe {
        asm!("mrs {}, midr_el1", out(reg) midr);
    }
    
    let implementer = ((midr >> 24) & 0xFF) as u32;
    let partnum = ((midr >> 4) & 0xFFF) as u32;
    
    (implementer, partnum)
}
```

## API Reference

### Initialization

```rust
// Initialize HAL (must be called early in boot)
hal::init();
```

### Platform Information

```rust
// Get platform info
let info = hal::platform_info();

// Check platform type
hal::is_raspberry_pi5() -> bool;
hal::is_qemu() -> bool;
```

### Printing

```rust
// Print platform information
hal::print_info();
```

## Testing

### Platform Detection Test

```rust
fn test_platform_detection() {
    hal::init();
    
    let info = hal::platform_info();
    println!("Platform: {:?}", info.platform_type);
    println!("CPU: {} MHz", info.cpu_freq_hz / 1_000_000);
    println!("Peripheral base: 0x{:016X}", info.peripheral_base);
    
    assert!(info.cpu_freq_hz > 0);
}
```

### MMIO Test

```rust
fn test_mmio() {
    let test_addr = 0x09000000; // QEMU UART
    
    // Write and read back
    unsafe {
        hal::mmio::write32(test_addr, 0x12345678);
        let value = hal::mmio::read32(test_addr);
        assert_eq!(value, 0x12345678);
    }
}
```

## Architecture

```
hal::init()
    ├── detect_platform()
    │   ├── detect_from_device_tree()
    │   ├── detect_from_cpu()
    │   └── PlatformInfo::qemu_virt() [fallback]
    ├── Store PLATFORM_INFO
    └── init_platform_drivers()
```

## Future Enhancements

- [ ] Complete device tree parsing
- [ ] ACPI support for server platforms
- [ ] PSCI (Power State Coordination Interface)
- [ ] Clock framework integration
- [ ] Pin muxing framework

## See Also

- [GPIO Documentation](gpio.md) - GPIO driver
- [UART Documentation](uart.md) - UART driver
- [ARM ARMv8-A Reference](https://developer.arm.com/documentation) - Architecture details
