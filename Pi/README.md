# 🌐 WebbOS for Raspberry Pi (ARM64)

A web browser operating system that boots directly into a desktop environment with a full web browser, applications, and user management. This is the **ARM64/AArch64** port for Raspberry Pi.

> **Status:** Bootloader & Kernel Working | Display on Real Hardware Only

## ✨ Features

- **🖥️ Desktop Environment** - Modern HTML/CSS-based desktop with windows, taskbar, and start menu
- **🎨 Built-in Apps** - Notepad, Paint, File Manager, Task Manager, User Manager, Terminal, Web Browser
- **👤 User Management** - Multi-user support with SHA-256 authentication and sessions
- **🌐 Full Networking** - TCP/IP, HTTP/HTTPS, TLS 1.3, DNS resolver, DHCP
- **💾 File Systems** - EXT2, FAT32 with SD card storage
- **🔒 Security** - SHA-256 password hashing, ChaCha20-Poly1305, X25519 key exchange
- **🎮 Input** - USB HID keyboard and mouse (DWC OTG)
- **🖼️ Graphics** - HDMI via VideoCore mailbox framebuffer (1024x768 @ 32-bit)
- **📡 WiFi** - BCM43438/BCM43455 SDIO WiFi (Pi 3/4)

## ⚠️ Important: QEMU Limitations

**The Pi version requires REAL Raspberry Pi hardware for display output.**

QEMU's `raspi3b` machine does NOT emulate the VideoCore GPU mailbox interface. While the OS boots and runs in QEMU, the display will not be visible.

**For testing with display, use the PC version:** `cd PC && ./run.bat`

## 🚀 Quick Start

### Prerequisites

**Windows 11:**
```powershell
# Install Rust
irm https://win.rustup.rs | iex

# Install QEMU (optional - for testing boot only)
choco install qemu

# Install nightly toolchain with ARM64 support
rustup toolchain install nightly-2025-01-15
rustup component add rust-src --toolchain nightly-2025-01-15
rustup target add aarch64-unknown-none --toolchain nightly-2025-01-15
```

### Build

```powershell
cd Pi
./build.bat
```

This creates:
- `webbos-pi-raw.img` - Combined bootloader+kernel (for QEMU testing)
- `webbos-pi.img` - SD card image (for real Pi hardware)

### Test in QEMU (Boot Only - No Display)

```powershell
./run.bat
```

**Note:** QEMU will boot but display won't work. This is normal - the VideoCore GPU is not emulated.

### Run on Real Raspberry Pi

1. Write `webbos-pi.img` to an SD card:
   - Use Raspberry Pi Imager, Rufus, or BalenaEtcher
   - Or: `dd if=webbos-pi.img of=/dev/sdX bs=4M` (Linux)

2. Insert SD card into Raspberry Pi 3 or 4

3. Power on - WebbOS will boot to desktop

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────┐
│  Desktop Environment (7 Applications)                   │
│  ├── File Manager, Notepad, Paint                      │
│  ├── Task Manager, User Manager                        │
│  ├── Terminal, WebbBrowser                             │
├─────────────────────────────────────────────────────────┤
│  Browser Engine                                         │
│  ├── HTML/CSS/JS Parsers                               │
│  ├── WebAssembly Parser                                │
│  ├── Layout & Rendering Engine                         │
├─────────────────────────────────────────────────────────┤
│  System Services                                        │
│  ├── User Management (SHA-256, Sessions)               │
│  ├── Graphics (Pi Mailbox Framebuffer)                 │
│  ├── Input (USB HID via DWC OTG)                       │
├─────────────────────────────────────────────────────────┤
│  Network Stack                                          │
│  ├── HTTP/HTTPS Client                                 │
│  ├── TLS 1.3 (ChaCha20-Poly1305, X25519)              │
│  ├── TCP/IP, DNS, DHCP                                 │
│  └── WiFi (BCM43438/43455 via SDIO)                    │
├─────────────────────────────────────────────────────────┤
│  Kernel Core                                            │
│  ├── Memory Management (ARM64 MMU)                     │
│  ├── Process Scheduler                                 │
│  ├── VFS (EXT2, FAT32), SD Storage                     │
│  └── Exception Handling (VBAR_EL1)                     │
├─────────────────────────────────────────────────────────┤
│  Bare Metal Bootloader                                  │
│  ├── ARM64 Entry at 0x80000                            │
│  ├── MMU Setup (4KB pages)                             │
│  └── Higher-Half Kernel Mapping                        │
└─────────────────────────────────────────────────────────┘
```

## 🔧 Hardware Support

### ✅ Fully Implemented

| Component | Driver | Hardware Address | Status |
|-----------|--------|------------------|--------|
| **CPU** | ARM64 Cortex-A53/A72 | - | ✅ Working |
| **MMU** | 4-level page tables | - | ✅ Working |
| **Timer** | ARM Generic Timer | `CNTPCT_EL0` | ✅ Working |
| **Mailbox** | VideoCore GPU | `0x3F00B880` (Pi3) | ✅ Working |
| **Framebuffer** | HDMI Output | GPU allocated | ✅ Working |
| **USB Host** | DWC OTG | `0x3F980000` (Pi3) | ✅ Implemented |
| **USB HID** | Keyboard/Mouse | USB bus | ✅ Implemented |
| **SDIO** | Arasan SDHCI | `0x3F300000` (Pi3) | ✅ Implemented |
| **WiFi** | BCM43438/43455 | SDIO bus | ⚠️ Needs firmware |

### ❌ Not Available on Pi

| Component | Reason |
|-----------|--------|
| PCI/PCIe | Pi doesn't have PCI bus |
| SATA/NVMe | No PCI = no SATA/NVMe |
| VESA BIOS | x86 only |
| PS/2 | Pi uses USB |

### ⚠️ QEMU Emulation Status

| Feature | QEMU Support | Notes |
|---------|--------------|-------|
| CPU | ✅ Yes | Full ARM64 emulation |
| MMU | ✅ Yes | Page tables work |
| Timer | ✅ Yes | Generic timer |
| Mailbox | ❌ No | No VideoCore GPU |
| Framebuffer | ❌ No | No display output |
| USB | ⚠️ Partial | May work partially |
| SDIO | ❌ No | No WiFi emulation |

## 📊 Implementation Status

| Component | Status |
|-----------|--------|
| Bare Metal Bootloader | ✅ Complete |
| ARM64 Kernel Core | ✅ Complete |
| ARM64 MMU | ✅ Complete |
| Memory Management | ✅ Complete |
| Process Scheduler | ✅ Complete |
| VFS (EXT2/FAT32) | ✅ Complete |
| Network Stack | ✅ Complete |
| TLS 1.3 | ✅ Complete |
| HTTP Client | ✅ Complete |
| Desktop Environment | ✅ Complete |
| User Management | ✅ Complete |
| Mailbox Interface | ✅ Complete |
| Pi Framebuffer | ✅ Complete |
| USB DWC OTG | ✅ Complete |
| USB HID | ✅ Complete |
| SDIO Controller | ✅ Complete |
| WiFi Driver | ⚠️ Firmware needed |

## 🛠️ Development

### Platform

Developed and tested on **Windows 11** using cross-compilation for ARM64.

### Build Commands

```powershell
# Build bootloader
cargo +nightly-2025-01-15 build -p bootloader --target aarch64-unknown-none -Z build-std=core,compiler_builtins,alloc

# Build kernel
cargo +nightly-2025-01-15 build -p kernel --target aarch64-unknown-none -Z build-std=core,compiler_builtins,alloc

# Create raw image (for QEMU)
python make-raw-image.py target/aarch64-unknown-none/release/bootloader target/aarch64-unknown-none/release/kernel webbos-pi-raw.img

# Create SD card image (for real Pi)
python scripts/create-sdcard.py target/aarch64-unknown-none/release/bootloader -o webbos-pi.img
```

## 📚 Documentation

- [Build Instructions](docs/BUILD.md) - Detailed build process
- [Running Guide](docs/RUNNING.md) - How to run (QEMU vs real hardware)
- [Hardware Details](HARDWARE.md) - Pi-specific hardware information
- [Porting Notes](PORTING.md) - ARM64 porting details

## 📊 Specifications

| Component | Specification |
|-----------|---------------|
| **Architecture** | ARM64 (AArch64) |
| **Boot** | Bare metal (kernel8.img) |
| **Kernel Base** | 0xFFFF000000100000 |
| **Physical Load** | 0x100000 |
| **Heap** | 8MB |
| **Resolution** | 1024x768 (32-bit color) |
| **Memory** | 1GB minimum |
| **Storage** | SD card (FAT32) |
| **Network** | WiFi (BCM43438/43455) |

## 🤝 Contributing

Contributions welcome! This is a complex bare-metal ARM64 project.

## 📝 License

MIT License - see LICENSE file.

---

**WebbOS for Raspberry Pi** - A web browser OS for ARM64. 🌐✨

Built with ❤️ and Rust.
