# 🌐 WebbOS - Multi-Architecture Operating System

A web browser operating system that boots directly into a desktop environment with a full web browser, applications, and user management.

> **Status:** Fully operational on x86_64 and ARM64 (Raspberry Pi 3/4/5)

## 📁 Project Structure

This repository contains WebbOS for multiple architectures:

```
webbOs/
├── PC/          # x86_64 UEFI version (PCs, VMs)
├── Pi/          # ARM64 version (Raspberry Pi 3/4)
├── Pi5/         # ARM64 version (Raspberry Pi 5)
└── .gitignore   # Global ignore patterns
```

## 🖥️ PC Version (x86_64)

The original WebbOS for x86_64 PCs with UEFI boot.

**Features:**
- UEFI bootloader with FAT32 disk image
- VESA framebuffer graphics with dirty rectangle tracking
- PS/2 keyboard and mouse input
- PCI device enumeration
- ATA/NVMe storage drivers
- Intel/AMD network drivers (virtio, e1000)
- Advanced DHCP client with lease renewal
- Full browser with DOM API and JavaScript bindings
- Process scheduler with kernel thread support

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
- BCM43438 WiFi with WPA2-PSK support
- SD card storage
- Device Tree support
- Full preemptive process scheduler
- Hardware entropy for cryptographic operations

**Quick Start:**
```powershell
cd Pi
python scripts/create-sdcard.py  # Create SD card image
./build.bat                      # Build
./run.bat raspi3b               # Run in QEMU (Pi 3)
```

[See Pi README for details →](Pi/README.md)

## 🥧 Pi5 Version (ARM64/AArch64)

WebbOS for Raspberry Pi 5.

**Features:**
- Bare metal ARM64 boot for BCM2712
- VideoCore mailbox framebuffer
- SD card storage (SDHCI)
- Full process scheduler and context switching
- SHA1/PBKDF2 cryptography for future WiFi support
- Synchronization with Pi improvements

**Quick Start:**
```powershell
cd Pi5
./build.bat                      # Build
# Copy to SD card for hardware testing
```

[See Pi5 README for details →](Pi5/README.md)

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

# For Pi/Pi5
rustup target add aarch64-unknown-none --toolchain nightly-2025-01-15
```

## 🏗️ Architecture Comparison

| Feature | PC (x86_64) | Pi (ARM64) | Pi5 (ARM64) |
|---------|-------------|------------|-------------|
| **Boot** | UEFI | Bare metal (0x80000) | Bare metal |
| **Graphics** | VESA BIOS | VideoCore Mailbox | VideoCore Mailbox |
| **Input** | PS/2 | USB HID (DWC OTG) | USB HID |
| **Network** | virtio/e1000 | BCM43438 WiFi | (Pending) |
| **Storage** | ATA/NVMe | SD card (SDHCI) | SD card (SDHCI) |
| **Timer** | APIC/HPET | ARM Generic Timer | ARM Generic Timer |
| **Interrupts** | IDT/APIC | GIC/VBAR_EL1 | GIC/VBAR_EL1 |
| **Image** | webbos.img (FAT32) | webbos-pi.img | webbos-pi5.img |
| **Build Warnings** | ~474 | ~500 | ~1067 |

Legend: ✅ Working, 🚧 In Progress, ❌ Not Available

## 🔒 Security Features

| Feature | PC | Pi | Pi5 |
|---------|----|----|-----|
| TCP ISN (RFC 6528) | ✅ | ✅ | ✅ |
| Filesystem Bounds Checking | ✅ | ✅ | ✅ |
| WPA2-PSK | N/A | ✅ | 🚧 |
| PBKDF2 Password Hashing | ✅ | ✅ | ✅ |
| Static Mut Safety | ✅ | ✅ | ✅ |
| Hardware Entropy | ✅ (RDTSC) | ✅ (CNTPCT) | ✅ (CNTPCT) |

## 📝 Documentation

- [PC Build Instructions](PC/docs/BUILD.md)
- [PC Running Guide](PC/docs/RUNNING.md)
- [Pi Porting Notes](Pi/PORTING.md)
- [Pi SD Card Setup](Pi/scripts/README.md)
- [Port Comparison](PORT_COMPARISON.md)
- [Security Audit Report](SECURITY_AUDIT_REPORT.md)
- [Development Tasks](TASKS.md)

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
