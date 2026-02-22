# Running WebbOS on Raspberry Pi

This guide explains how to build and run WebbOS Pi on both QEMU (for testing) and real Raspberry Pi hardware.

## ⚠️ Important Notice

**QEMU does NOT support the Raspberry Pi's VideoCore GPU mailbox interface.** This means:
- ✅ The OS boots and runs in QEMU
- ❌ No display output in QEMU (VideoCore not emulated)
- ❌ No WiFi in QEMU (SDIO not emulated)

**For a working display, you must run on real Raspberry Pi hardware.**

## Prerequisites

### Required Tools
- **Rust** (nightly toolchain: `nightly-2025-01-15`)
- **QEMU** (optional - for testing boot only)
- **Python 3** (for image creation)

### Installation

#### Windows 11 (PowerShell) - Primary Platform
```powershell
# Install Rust
irm https://win.rustup.rs | iex

# Install QEMU (optional - for testing)
choco install qemu

# Install nightly toolchain with ARM64 support
rustup toolchain install nightly-2025-01-15
rustup component add rust-src --toolchain nightly-2025-01-15
rustup target add aarch64-unknown-none --toolchain nightly-2025-01-15

# Verify Python
python --version
```

#### Linux (Ubuntu/Debian)
```bash
# Install dependencies
sudo apt update
sudo apt install qemu-system-arm python3

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install nightly toolchain
rustup toolchain install nightly-2025-01-15
rustup component add rust-src --toolchain nightly-2025-01-15
rustup target add aarch64-unknown-none --toolchain nightly-2025-01-15
```

#### macOS
```bash
# Install dependencies
brew install qemu python3

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install nightly toolchain
rustup toolchain install nightly-2025-01-15
rustup component add rust-src --toolchain nightly-2025-01-15
rustup target add aarch64-unknown-none --toolchain nightly-2025-01-15
```

## Building WebbOS Pi

### Quick Build (Windows)

```powershell
# Build everything and create images
./build.bat
```

This creates:
- `webbos-pi-raw.img` - Combined bootloader+kernel (for QEMU)
- `webbos-pi.img` - SD card image (for real Pi)

### Manual Build

```bash
# Build bootloader
cargo +nightly-2025-01-15 build -p bootloader --target aarch64-unknown-none -Z build-std=core,compiler_builtins,alloc --release

# Build kernel
cargo +nightly-2025-01-15 build -p kernel --target aarch64-unknown-none -Z build-std=core,compiler_builtins,alloc --release

# Create raw image (for QEMU)
python make-raw-image.py target/aarch64-unknown-none/release/bootloader target/aarch64-unknown-none/release/kernel webbos-pi-raw.img

# Create SD card image (for real Pi)
python scripts/create-sdcard.py target/aarch64-unknown-none/release/bootloader -o webbos-pi.img
```

## Running on QEMU (Testing Only)

**Note:** QEMU cannot show the display, but you can verify the OS boots.

### Windows
```powershell
./run.bat
```

### Manual QEMU Command
```bash
qemu-system-aarch64 \
    -M raspi3b \
    -cpu cortex-a53 \
    -m 1G \
    -kernel webbos-pi-raw.img \
    -display sdl \
    -device usb-kbd \
    -device usb-mouse \
    -snapshot \
    -no-reboot
```

### Expected Behavior
- QEMU window opens
- Window may show black screen (no GPU emulation)
- OS is running - no crash = success
- Close window or press Ctrl+C to stop

## Running on Real Raspberry Pi

### What You Need
- Raspberry Pi 3 or 4
- MicroSD card (4GB or larger)
- HDMI display
- USB keyboard (required for login)
- USB mouse (optional)
- Power supply

### Prepare SD Card

#### Windows
1. Insert SD card
2. Use Raspberry Pi Imager or Rufus to write `webbos-pi.img`
3. Or use Win32DiskImager

#### Linux/macOS
```bash
# Find your SD card device (e.g., /dev/sdb, /dev/mmcblk0)
lsblk

# Write image (replace /dev/sdX with your device)
sudo dd if=webbos-pi.img of=/dev/sdX bs=4M status=progress

# Or use Etcher (GUI tool)
```

### First Boot

1. Insert prepared SD card into Pi
2. Connect HDMI display
3. Connect USB keyboard
4. Connect power
5. WebbOS will boot to login screen

### Default Login

| Username | Password | Type |
|----------|----------|------|
| `admin` | `admin` | Administrator |
| `user` | `user` | Standard User |

## Troubleshooting

### QEMU Shows Black Screen
**This is normal.** QEMU doesn't emulate the VideoCore GPU. The OS is still running - it just can't display. Use real Pi hardware for display.

### Build Errors
```powershell
# Clean and rebuild
cargo clean
cargo +nightly-2025-01-15 build -p bootloader --target aarch64-unknown-none -Z build-std=core,compiler_builtins,alloc --release
cargo +nightly-2025-01-15 build -p kernel --target aarch64-unknown-none -Z build-std=core,compiler_builtins,alloc --release
```

### Pi Won't Boot
- Check SD card is properly written
- Try different SD card
- Check power supply (needs 2.5A for Pi 3, 3A for Pi 4)
- Ensure HDMI cable is connected before power

### No WiFi
WiFi requires firmware files on the SD card. These are not included:
- `brcmfmac43430-sdio.bin` (Pi 3)
- `brcmfmac43430-sdio.txt` (Pi 3)
- `brcmfmac43455-sdio.bin` (Pi 4)
- `brcmfmac43455-sdio.txt` (Pi 4)

Copy these from Raspberry Pi OS firmware to `/lib/firmware/brcm/` on the SD card.

## For Display Testing Use PC Version

If you need to test the desktop without real Pi hardware:

```powershell
cd PC
./build.bat
./run.bat
```

The PC version uses VESA framebuffer which QEMU emulates properly.

## Hardware Differences

| Feature | PC (x86_64) | Pi (ARM64) |
|---------|-------------|------------|
| Boot | UEFI | Bare metal |
| Display | VESA BIOS | Mailbox/VideoCore |
| Input | PS/2 | USB HID |
| Storage | ATA/NVMe | SD card |
| Network | PCI Ethernet/WiFi | SDIO WiFi |
| QEMU Display | ✅ Works | ❌ Not supported |

## See Also

- [HARDWARE.md](../HARDWARE.md) - Detailed hardware specifications
- [BUILD.md](BUILD.md) - Detailed build instructions
- [PORTING.md](../PORTING.md) - ARM64 porting notes
