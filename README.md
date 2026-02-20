# 🌐 WebbOS - Multi-Architecture Operating System

A web browser operating system that boots directly into a desktop environment with a full web browser, applications, and user management.

> **Status:** Fully operational on x86_64 and ARM64 (Raspberry Pi)

## 📁 Project Structure

This repository contains WebbOS for multiple architectures:

```
webbOs/
├── PC/          # x86_64 UEFI version (PCs, VMs)
├── Pi/          # ARM64 version (Raspberry Pi 3/4)
└── .gitignore   # Global ignore patterns
```

## 🖥️ PC Version (x86_64)

The original WebbOS for x86_64 PCs with UEFI boot.

**Features:**
- UEFI bootloader with FAT32 disk image
- VESA framebuffer graphics
- PS/2 keyboard and mouse input
- PCI device enumeration
- ATA/NVMe storage drivers
- Intel/AMD network drivers (virtio, e1000)

**Quick Start:**
```powershell
cd PC
python scripts/create-image.py  # First time only
./build.bat                     # Build
./run.bat                       # Run in QEMU
```

[See PC README for details →](PC/README.md)

## 🥧 Pi Version (ARM64/AArch64)

WebbOS ported to Raspberry Pi 3 and 4.

**Features:**
- Bare metal ARM64 boot (kernel8.img)
- VideoCore mailbox framebuffer
- USB HID keyboard and mouse (DWC OTG)
- BCM43438/BCM43455 WiFi (SDIO)
- SD card storage
- Device Tree support

**Quick Start:**
```powershell
cd Pi
python scripts/create-sdcard.py  # Create SD card image
./build.bat                      # Build
./run.bat raspi3b               # Run in QEMU (Pi 3)
```

[See Pi README for details →](Pi/README.md)

## 🔧 Common Build Requirements

**Windows 11 (Primary Platform):**
```powershell
# Install Rust
irm https://win.rustup.rs | iex

# Install QEMU
choco install qemu

# Install nightly toolchain
rustup toolchain install nightly-2025-01-15
rustup component add rust-src --toolchain nightly-2025-01-15
```

**Add architecture targets:**
```powershell
# For PC
rustup target add x86_64-unknown-none x86_64-unknown-uefi --toolchain nightly-2025-01-15

# For Pi
rustup target add aarch64-unknown-none --toolchain nightly-2025-01-15
```

## 🏗️ Architecture Comparison

| Feature | PC (x86_64) | Pi (ARM64) |
|---------|-------------|------------|
| **Boot** | UEFI | Bare metal (0x80000) |
| **Graphics** | VESA BIOS | VideoCore Mailbox |
| **Input** | PS/2 | USB HID (DWC OTG) |
| **Network** | virtio/e1000 | BCM43438/43455 WiFi |
| **Storage** | ATA/NVMe | SD card (SDHCI) |
| **Timer** | APIC/HPET | ARM Generic Timer |
| **Interrupts** | IDT/APIC | GIC/VBAR_EL1 |
| **Image** | webbos.img (FAT32) | webbos-pi.img (SD card) |

## 📝 Documentation

- [PC Build Instructions](PC/docs/BUILD.md)
- [PC Running Guide](PC/docs/RUNNING.md)
- [Pi Porting Notes](Pi/PORTING.md)
- [Pi SD Card Setup](Pi/scripts/README.md)

## 🤝 Contributing

Each architecture folder is self-contained with its own:
- `Cargo.toml` workspace
- `Makefile` and build scripts
- Architecture-specific drivers
- Documentation

## 📜 License

MIT OR Apache-2.0

---

**WebbOS** - Browse the web without the bloat 🚀
