# Raspberry Pi 5 Support Status

## Current Status: ❌ NOT SUPPORTED (Requires Port)

WebbOS currently supports **Raspberry Pi 3B/3B+** and **Raspberry Pi 4B** only. The Raspberry Pi 5 requires additional work to support.

## Why Pi 5 is Different

The Raspberry Pi 5 uses the **BCM2712** SoC, which differs significantly from previous generations:

| Feature | Pi 3 (BCM2837) | Pi 4 (BCM2711) | Pi 5 (BCM2712) |
|---------|----------------|----------------|----------------|
| **Peripheral Base** | 0x3F000000 | 0xFE000000 | 0x100000000 |
| **Mailbox** | 0x3F00B880 | 0xFE00B880 | 0x10000B880? |
| **SDIO** | 0x3F300000 | 0xFE300000 | Unknown |
| **USB** | 0x3F980000 | 0xFE980000 | Unknown |
| **GPIO** | 0x3F200000 | 0xFE200000 | Unknown |
| **CPU** | Cortex-A53 | Cortex-A72 | Cortex-A76 |
| **Boot Process** | Standard | Standard | Modified |

## Required Changes for Pi 5 Support

### 1. Peripheral Base Addresses
All hardware register addresses need to be updated:

```rust
// Current code in kernel/src/drivers/mailbox/mod.rs
pub const MAILBOX_BASE_PI3: usize = 0x3F00B880;
pub const MAILBOX_BASE_PI4: usize = 0xFE00B880;
// TODO: Add Pi 5 address
pub const MAILBOX_BASE_PI5: usize = 0x10000B880; // Verify this!
```

### 2. CPU Detection
Update the Pi detection logic:

```rust
// Current code uses a simple boolean
let pi4 = false; // TODO: Detect from device tree

// Need to detect Pi 5 as well
enum PiModel {
    Pi3,
    Pi4,
    Pi5,
}
```

### 3. Memory Map
The Pi 5 has a different memory layout:

- **Pi 3/4**: Peripherals at 0x3F000000 or 0xFE000000
- **Pi 5**: Peripherals likely at 0x100000000 (requires 64-bit addressing)

### 4. Interrupt Controller
Pi 5 uses a different interrupt controller (GICv3 vs legacy):

```rust
// Current: Simple IRQ handling
// Pi 5 requires GICv3 support for interrupts
```

### 5. GPU/Display
The Pi 5 uses a different VideoCore GPU:

- Mailbox interface may be different
- Framebuffer allocation method may differ
- Pixel format or stride might vary

### 6. Power Management
Pi 5 has new power management features:

- Different reset/poweroff sequence
- PMIC (Power Management IC) control
- Fan control (if implementing thermal management)

### 7. Boot Sequence
The Pi 5 boot process differs:

- Starts in a different exception level
- May require different bootloader
- Device tree blob (DTB) format changes

## What Would Work vs. What Wouldn't

### Likely to Work with Address Changes Only
- Basic kernel boot
- Serial console (UART)
- GPIO (if addresses updated)

### Requires Significant Changes
- USB controller (different IP block)
- Ethernet (different MAC/PHY)
- Display/framebuffer (VideoCore changes)
- PCIe (new on Pi 5)
- SD card interface

### Completely New Features to Support
- PCIe support (x1 lane)
- Fan control
- Power button
- Dedicated fan header
- New power management IC

## How to Add Pi 5 Support

### Step 1: Gather Documentation
1. Download BCM2712 peripheral specification (if available)
2. Study Raspberry Pi 5 device tree source (DTS)
3. Review Linux kernel changes for Pi 5

### Step 2: Update Hardware Abstraction
```rust
// kernel/src/drivers/pi_model.rs
pub enum PiModel {
    Pi3,
    Pi4,
    Pi5,
}

pub fn detect_model() -> PiModel {
    // Read processor ID or device tree
    // MIDR_EL1 register or similar
}

pub fn get_peripheral_base(model: PiModel) -> usize {
    match model {
        PiModel::Pi3 => 0x3F000000,
        PiModel::Pi4 => 0xFE000000,
        PiModel::Pi5 => 0x100000000, // Verify!
    }
}
```

### Step 3: Update All Driver Base Addresses
Modify each driver to use runtime-detected addresses:

- `mailbox/mod.rs` - VideoCore communication
- `sdio/mod.rs` - SD card controller
- `usb/dwc_otg.rs` - USB controller
- `console/serial.rs` - UART
- `wifi/` - SDIO/WiFi

### Step 4: Test and Debug
1. Start with serial console only
2. Verify mailbox communication
3. Test framebuffer/display
4. Add USB support
5. Add network support

## Alternative: Use Pi 3 or Pi 4

If you want to run WebbOS today, use a supported board:

### Recommended: Raspberry Pi 4B
- Best performance
- Full hardware support in WebbOS
- Widely available
- More RAM options (1GB/2GB/4GB/8GB)

### Budget Option: Raspberry Pi 3B/3B+
- Fully supported
- Lower power consumption
- Slower but functional
- May have thermal issues without heatsink

## Community Contribution

Pi 5 support would be a welcome contribution! To help:

1. **Hardware Investigation**:
   - Dump device tree from Pi 5 running Linux
   - Document peripheral base addresses
   - Test mailbox interface

2. **Code Changes**:
   - Implement Pi model detection
   - Update all peripheral addresses
   - Add GICv3 interrupt support

3. **Testing**:
   - Verify each subsystem works
   - Document any limitations

## Resources for Pi 5 Development

- [Raspberry Pi 5 Product Brief](https://www.raspberrypi.com/products/raspberry-pi-5/)
- [BCM2712 Datasheet](https://datasheets.raspberrypi.com/bcm2712/bcm2712-peripherals.pdf) (if available)
- [Linux Kernel Pi 5 Changes](https://github.com/raspberrypi/linux)
- [Device Tree Source](https://github.com/raspberrypi/linux/arch/arm64/boot/dts/broadcom/bcm2712.dts)

## Summary

| Question | Answer |
|----------|--------|
| Does WebbOS work on Pi 5? | **No** - requires porting effort |
| How much work to support Pi 5? | **Moderate** - address changes + some driver updates |
| Should I buy a Pi 5 for WebbOS? | **No** - use Pi 3 or Pi 4 instead |
| Will Pi 5 be supported in future? | **Possible** - depends on community interest |

## Quick Reference: Pi 5 Peripheral Addresses

**WARNING: These are ESTIMATES based on patterns. VERIFY before use!**

| Peripheral | Estimated Address | Status |
|------------|-------------------|--------|
| GPIO | 0x10000200000 | UNVERIFIED |
| Mailbox | 0x10000B880 | UNVERIFIED |
| SDHCI/SDIO | 0x1000300000 | UNVERIFIED |
| USB | 0x1000980000 | UNVERIFIED |
| UART0 | 0x10000201000 | UNVERIFIED |
| SPI0 | 0x10000204000 | UNVERIFIED |

**Note**: The 0x100 prefix indicates the 47-bit address space used by BCM2712.
