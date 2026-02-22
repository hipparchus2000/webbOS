# WebbOS Pi Hardware Support

Detailed hardware specifications and driver implementation status for the Raspberry Pi port.

## Memory Map

### Physical Memory Layout

| Region | Address | Size | Purpose |
|--------|---------|------|---------|
| Bootloader | 0x00080000 | 512KB | kernel8.img loaded here |
| Kernel | 0x00100000 | 4MB | Kernel image |
| Stack | 0x00500000 | 128KB | Kernel stack |
| Heap | 0x00600000 | 8MB | Kernel heap |
| MMIO Base | 0x3F000000 | 16MB | Peripheral registers (Pi 3) |
| MMIO Base | 0xFE000000 | 16MB | Peripheral registers (Pi 4) |

### Virtual Memory Layout (Higher Half)

| Region | Address | Description |
|--------|---------|-------------|
| Kernel Text | 0xFFFF000000100000 | Kernel code |
| Kernel Data | 0xFFFF000000200000 | Kernel data/BSS |
| Stack Top | 0xFFFF000000500000 + 128K | Kernel stack |
| Heap | 0xFFFF000004000000 | 8MB heap |
| Framebuffer | 0xFFFF000080000000 | GPU framebuffer (if mapped) |

## Peripheral Base Addresses

### Raspberry Pi 5 (BCM2712) - NOT SUPPORTED

| Peripheral | Base Address | Status |
|------------|--------------|--------|
| Mailbox | 0x10000B880 (estimated) | ❌ Not implemented |
| USB | Unknown | ❌ Not implemented |
| SDIO | Unknown | ❌ Not implemented |
| UART | Unknown | ❌ Not implemented |

**Note:** The Pi 5 uses a completely different memory map and hardware architecture. See [docs/PI5_SUPPORT.md](docs/PI5_SUPPORT.md) for details.

### Raspberry Pi 3 (BCM2837)

| Peripheral | Base Address | Driver | Status |
|------------|--------------|--------|--------|
| Mailbox | 0x3F00B880 | `mailbox/mod.rs` | ✅ Complete |
| USB (DWC OTG) | 0x3F980000 | `usb/dwc_otg.rs` | ✅ Complete |
| SDIO | 0x3F300000 | `sdio/mod.rs` | ✅ Complete |
| UART0 (PL011) | 0x3F201000 | `uart.rs` | ✅ Complete |
| Mini UART | 0x3F215040 | - | Available |
| GPIO | 0x3F200000 | - | Not implemented |

### Raspberry Pi 4 (BCM2711)

| Peripheral | Base Address | Notes |
|------------|--------------|-------|
| Mailbox | 0xFE00B880 | Same interface, different base |
| USB (DWC OTG) | 0xFE980000 | xhci on Pi 4 (not implemented) |
| SDIO | 0xFE300000 | Same as Pi 3 |
| UART0 (PL011) | 0xFE201000 | Same as Pi 3 |

## Drivers

### Mailbox (VideoCore GPU Interface)

**File:** `kernel/src/drivers/mailbox/mod.rs`

The mailbox is the communication channel between the ARM CPU and VideoCore GPU.

**Features:**
- Property interface for GPU communication
- Framebuffer allocation
- Hardware configuration

**Usage:**
```rust
mailbox::init();
let mb = mailbox::mailbox().lock();
mb.call(&mut message);
```

### Framebuffer (Pi Display)

**File:** `kernel/src/drivers/display/pi_framebuffer.rs`

Uses the mailbox to allocate a framebuffer from the GPU.

**Process:**
1. Set physical display size (mailbox tag 0x48003)
2. Set virtual buffer size (mailbox tag 0x48004)
3. Set depth/bpp (mailbox tag 0x48005)
4. Set pixel order RGB (mailbox tag 0x48006)
5. Allocate buffer (mailbox tag 0x40001)
6. Get pitch (mailbox tag 0x40008)

**Functions:**
- `init(width, height, bpp)` - Allocate and configure
- `set_pixel(x, y, color)` - Draw pixel
- `fill_rect(x, y, w, h, color)` - Draw rectangle
- `draw_text(text, x, y, color, scale)` - Draw text

### USB DWC OTG (DWC_otg)

**File:** `kernel/src/drivers/usb/dwc_otg.rs`

Synopsys DesignWare Core USB OTG controller.

**Features:**
- Host mode operation
- 8 channels for USB devices
- Control, bulk, and interrupt transfers
- Device enumeration

**Base:** 0x3F980000 (Pi 3), 0xFE980000 (Pi 4)

### USB HID (Keyboard/Mouse)

**File:** `kernel/src/drivers/usb/hid.rs`

HID boot protocol driver for keyboards and mice.

**Features:**
- Keyboard boot protocol (8-byte reports)
- Mouse boot protocol (3-byte reports)
- Keycode to ASCII translation
- Event queue system

### SDIO (Arasan SDHCI)

**File:** `kernel/src/drivers/sdio/mod.rs`

Secure Digital Host Controller Interface for WiFi.

**Features:**
- CMD52 (single register read/write)
- CMD53 (multi-byte/block transfer)
- Card initialization
- Function enable/disable

**Base:** 0x3F300000 (Pi 3), 0xFE300000 (Pi 4)

### WiFi (BCM43438/BCM43455)

**File:** `kernel/src/drivers/wifi/bcm43438.rs`

Broadcom SDIO WiFi driver.

**Features:**
- SDPCM protocol
- IOCTL interface
- Scan and connect
- Network interface integration

**Requirements:**
- Firmware files in `/lib/firmware/brcm/`:
  - `brcmfmac43430-sdio.bin` (Pi 3)
  - `brcmfmac43430-sdio.txt` (Pi 3)
  - `brcmfmac43455-sdio.bin` (Pi 4)
  - `brcmfmac43455-sdio.txt` (Pi 4)

## Boot Process

1. **GPU Loads Bootloader**
   - GPU reads `kernel8.img` from SD card
   - Loaded at physical address 0x80000
   - Execution starts in EL2 (Hypervisor mode)

2. **Bootloader Initialization**
   - Switch from EL2 to EL1
   - Parse device tree (DTB) from x0 register
   - Set up initial page tables (identity + higher half)
   - Load kernel ELF from 0x100000
   - Enable MMU
   - Jump to kernel at 0xFFFF000000100000

3. **Kernel Initialization**
   - Initialize console
   - Initialize memory management
   - Initialize drivers (mailbox, USB, etc.)
   - Initialize network stack
   - Initialize graphics (via mailbox)
   - Initialize desktop
   - Show login screen

## Known Limitations

### QEMU
- **VideoCore GPU not emulated** - No display output
- **SDIO not emulated** - No WiFi
- **USB partially works** - May have issues

### Real Hardware Requirements
- Raspberry Pi 3 or 4 (**NOT Pi 5** - see [PI5_SUPPORT.md](docs/PI5_SUPPORT.md))
- HDMI display
- USB keyboard (required for login)
- SD card (4GB minimum)
- WiFi requires firmware files on SD card

## GPIO Pinout (Reference)

Not implemented in WebbOS, but available for future expansion:

| Pin | Function | Notes |
|-----|----------|-------|
| 2,4 | 5V Power | - |
| 6,9,14,20,25,30,34,39 | Ground | - |
| 1,17 | 3.3V | - |
| 8,10 | UART0 | PL011 console |
| 14,15 | UART1 | Mini UART |

## References

- [BCM2835 ARM Peripherals](https://www.raspberrypi.org/documentation/hardware/raspberrypi/bcm2835/BCM2835-ARM-Peripherals.pdf)
- [BCM2711 ARM Peripherals](https://datasheets.raspberrypi.com/bcm2711/bcm2711-peripherals.pdf)
- [DWC OTG Programming Guide](https://www.intel.com/content/www/us/en/io/universal-serial-bus/dwc-otg-driver.html)
- [SD Host Controller Spec](https://www.sdcard.org/downloads/pls/)
