# WebbOS Scripts

This directory contains helper scripts for building and running WebbOS.

## Quick Start

### First Time Setup (One-time only)

**Option A: Batch File (Recommended for Windows)**
```batch
:: Right-click on setup-wsl-and-run.bat and select "Run as administrator"
scripts\setup-wsl-and-run.bat
```

**Option B: PowerShell (Recommended)**
```powershell
# Right-click on setup-wsl-and-run.ps1 and select "Run with PowerShell"
# Or from elevated PowerShell:
.\scripts\setup-wsl-and-run.ps1
```

This will:
1. Install WSL (Windows Subsystem for Linux) if not present
2. Install Ubuntu distribution
3. Install required tools (mtools)
4. Build the kernel and bootloader
5. Create a bootable disk image
6. Download OVMF firmware
7. Launch WebbOS in QEMU

### Subsequent Runs

Once setup is complete, you can simply run:

```powershell
.\scripts\run-qemu.ps1
```

Or if you prefer batch:
```batch
scripts\run-qemu.ps1
```

## Available Scripts

### setup-wsl-and-run.bat
**Purpose:** Complete first-time setup including WSL installation  
**Requires:** Administrator privileges  
**Usage:** Right-click → "Run as administrator"

### setup-wsl-and-run.ps1
**Purpose:** PowerShell version of setup with better feedback  
**Requires:** Administrator privileges  
**Usage:** Right-click → "Run with PowerShell" or from elevated PowerShell

### run-qemu.ps1
**Purpose:** Run WebbOS after initial setup  
**Requires:** WSL with Ubuntu and mtools  
**Parameters:**
- `-Network` - Enable network with port forwarding (8080 → 80)
- `-Debug` - Enable GDB server on port 1234
- `-Release` - Build and run release mode
- `-Rebuild` - Force rebuild of kernel and disk image
- `-NoGraphic` - Run without graphics (serial only)

**Examples:**
```powershell
# Basic run
.\scripts\run-qemu.ps1

# With network
.\scripts\run-qemu.ps1 -Network

# Debug mode (waits for GDB connection)
.\scripts\run-qemu.ps1 -Debug

# Force rebuild
.\scripts\run-qemu.ps1 -Rebuild

# Release mode
.\scripts\run-qemu.ps1 -Release
```

### create-image.ps1
**Purpose:** Create bootable disk image  
**Usage:** Usually called by run-qemu.ps1, but can be run standalone

### make-fat32-image.ps1
**Purpose:** Attempt to create FAT32 image using PowerShell (experimental)  
**Note:** The WSL-based image creation is more reliable

## Prerequisites

1. **Windows 10 version 2004+ or Windows 11** (for WSL2)
2. **Administrator access** (for WSL setup)
3. **Rust** with nightly toolchain
4. **QEMU** for x86_64

### Installing Prerequisites Manually

**Rust:**
```powershell
irm https://win.rustup.rs | iex
rustup toolchain install nightly-2025-01-15
rustup component add rust-src --toolchain nightly-2025-01-15
```

**QEMU:**
```powershell
# Using chocolatey
choco install qemu

# Or download from https://www.qemu.org/download/#windows
```

## Troubleshooting

### "WSL is not installed"
Run the setup script as Administrator:
```powershell
.\scripts\setup-wsl-and-run.ps1
```

### "mtools not found"
Install mtools in WSL:
```powershell
wsl -d Ubuntu -e sudo apt update
wsl -d Ubuntu -e sudo apt install mtools
```

### "QEMU not found"
Install QEMU or add it to your PATH:
```powershell
# Check location
where.exe qemu-system-x86_64

# Or download from https://www.qemu.org/download/#windows
```

### "OVMF.fd not found"
The script will download this automatically. If it fails, download manually:
- URL: https://github.com/retrage/edk2-nightly/raw/master/bin/RELEASEX64_OVMF.fd
- Save as: `OVMF.fd` in the webbOs directory

### "Disk image creation failed"
Make sure WSL Ubuntu is properly set up:
```powershell
wsl --list --verbose
wsl -d Ubuntu
# In Ubuntu:
sudo apt update && sudo apt install mtools
exit
```

## Manual Steps (if scripts fail)

If the scripts don't work, you can do it manually:

### 1. Install WSL
```powershell
wsl --install -d Ubuntu
# Restart computer when prompted, then set up Ubuntu username/password
```

### 2. Install mtools in WSL
```bash
sudo apt update
sudo apt install mtools
```

### 3. Build WebbOS
```powershell
cargo +nightly-2025-01-15 build -p kernel --target x86_64-unknown-none -Z build-std=core,compiler_builtins,alloc
cargo +nightly-2025-01-15 build -p bootloader --target x86_64-unknown-uefi -Z build-std=core,compiler_builtins,alloc
```

### 4. Create Disk Image (in WSL)
```bash
cd /mnt/c/Users/$USERNAME/src/webbOs
dd if=/dev/zero of=webbos.img bs=1M count=64
mkfs.fat -F 32 webbos.img
mmd -i webbos.img ::/EFI
mmd -i webbos.img ::/EFI/BOOT
mcopy -i webbos.img target/x86_64-unknown-uefi/debug/bootloader.efi ::/EFI/BOOT/BOOTX64.EFI
mcopy -i webbos.img target/x86_64-unknown-none/debug/kernel ::/kernel
```

### 5. Run
```powershell
qemu-system-x86_64 -bios OVMF.fd -drive format=raw,file=webbos.img,if=virtio -vga std -m 512M -serial stdio
```

## Default Login Credentials

Once WebbOS boots:
- **Username:** `admin`
- **Password:** `admin`

Or:
- **Username:** `user`
- **Password:** `user`

## Support

For issues, check the main [README.md](../README.md) or [RUNNING.md](../docs/RUNNING.md).
