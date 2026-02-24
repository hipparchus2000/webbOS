# Alternative Ways to Run WebbOS

This document describes alternative methods to build and run WebbOS on different platforms.

> **Note:** The primary development workflow uses Python scripts on Windows 11. See `docs/DISK_IMAGE.md` and `docs/RUNNING.md` for the main instructions.

## Method 1: Native Windows (Primary - Recommended)

This is the main development platform. No WSL required!

```powershell
# 1. Create disk image (first time only)
python scripts/create-image.py

# 2. Build
cargo +nightly-2025-01-15 build -p bootloader --target x86_64-unknown-uefi -Z build-std=core,compiler_builtins,alloc
cargo +nightly-2025-01-15 build -p kernel --target x86_64-unknown-none -Z build-std=core,compiler_builtins,alloc

# 3. Update disk image
python scripts/update-image.py webbos.img "EFI/BOOT/BOOTX64.EFI" target/x86_64-unknown-uefi/debug/bootloader.efi
python scripts/update-image.py webbos.img kernel.elf target/x86_64-unknown-none/debug/kernel

# 4. Run
qemu-system-x86_64 -bios OVMF.fd -drive format=raw,file=webbos.img -m 128M -smp 1 -nographic -serial stdio
```

Or use the helper script:
```powershell
.\scripts\run-qemu.ps1 -Rebuild
```

## Method 2: Linux (Ubuntu/Debian)

### Using Python Scripts (Same as Windows)

The Python scripts work on Linux too:

```bash
# Create disk image
python3 create-image.py

# Build
cargo +nightly-2025-01-15 build -p bootloader --target x86_64-unknown-uefi -Z build-std=core,compiler_builtins,alloc
cargo +nightly-2025-01-15 build -p kernel --target x86_64-unknown-none -Z build-std=core,compiler_builtins,alloc

# Update disk image
python3 update-image.py webbos.img "EFI/BOOT/BOOTX64.EFI" target/x86_64-unknown-uefi/debug/bootloader.efi
python3 update-image.py webbos.img kernel.elf target/x86_64-unknown-none/debug/kernel

# Run
qemu-system-x86_64 -bios OVMF.fd -drive format=raw,file=webbos.img -m 128M -smp 1 -nographic -serial stdio
```

### Using mtools (Traditional Linux approach)

If you prefer traditional Linux tools:

```bash
# Install tools
sudo apt update
sudo apt install mtools qemu-system-x86

# Create disk image
dd if=/dev/zero of=webbos.img bs=1M count=64
mkfs.fat -F 32 webbos.img

# Create directory structure and copy files
mmd -i webbos.img ::/EFI
mmd -i webbos.img ::/EFI/BOOT
mcopy -i webbos.img target/x86_64-unknown-uefi/debug/bootloader.efi ::/EFI/BOOT/BOOTX64.EFI
mcopy -i webbos.img target/x86_64-unknown-none/debug/kernel ::/kernel

# Run
qemu-system-x86_64 -bios OVMF.fd -drive format=raw,file=webbos.img -m 128M -smp 1 -nographic -serial stdio
```

## Method 3: macOS

### Using Python Scripts

```bash
# Create disk image
python3 create-image.py

# Build (same commands as Linux)
cargo +nightly-2025-01-15 build -p bootloader --target x86_64-unknown-uefi -Z build-std=core,compiler_builtins,alloc
cargo +nightly-2025-01-15 build -p kernel --target x86_64-unknown-none -Z build-std=core,compiler_builtins,alloc

# Update disk image
python3 update-image.py webbos.img "EFI/BOOT/BOOTX64.EFI" target/x86_64-unknown-uefi/debug/bootloader.efi
python3 update-image.py webbos.img kernel.elf target/x86_64-unknown-none/debug/kernel

# Install QEMU if needed
brew install qemu

# Run
qemu-system-x86_64 -bios OVMF.fd -drive format=raw,file=webbos.img -m 128M -smp 1 -nographic -serial stdio
```

## Method 4: Windows with WSL (Legacy)

If you prefer using WSL (Windows Subsystem for Linux):

```powershell
# In WSL Ubuntu
wsl -d Ubuntu

# Install tools
sudo apt update
sudo apt install mtools

# Navigate to project
cd /mnt/c/Users/$USERNAME/src/webbOs

# Create disk image using Linux tools
dd if=/dev/zero of=webbos.img bs=1M count=64
mkfs.fat -F 32 webbos.img
mmd -i webbos.img ::/EFI
mmd -i webbos.img ::/EFI/BOOT
mcopy -i webbos.img target/x86_64-unknown-uefi/debug/bootloader.efi ::/EFI/BOOT/BOOTX64.EFI
mcopy -i webbos.img target/x86_64-unknown-none/debug/kernel ::/kernel

exit

# Run from Windows PowerShell
qemu-system-x86_64 -bios OVMF.fd -drive format=raw,file=webbos.img -m 128M -smp 1 -nographic -serial stdio
```

## Method 5: Docker

If you have Docker installed:

```bash
# Run build in container
docker run --rm -v "${PWD}:/webbos" -w /webbos rust:nightly \
    cargo +nightly-2025-01-15 build -p kernel --target x86_64-unknown-none -Z build-std=core,compiler_builtins,alloc

# Create disk image
docker run --rm -v "${PWD}:/webbos" -w /webbos ubuntu:22.04 bash -c "
    apt update && apt install -y mtools python3
    python3 create-image.py
"

# Run with QEMU (on host)
qemu-system-x86_64 -bios OVMF.fd -drive format=raw,file=webbos.img -m 128M -smp 1 -nographic -serial stdio
```

## Method 6: Boot from USB on Real Hardware

**⚠️ Warning: This will erase your USB drive!**

### On Linux:
```bash
# Find your USB device (be careful!)
lsblk

# Copy image to USB (replace sdX with your device)
sudo dd if=webbos.img of=/dev/sdX bs=4M status=progress
sync
```

### On Windows:
```powershell
# Use Rufus or similar tool to write webbos.img to USB
# Or use PowerShell (be very careful with drive letter!)
# WARNING: This will destroy data on the target drive!
```

Then boot from the USB on any PC with UEFI.

## Method 7: Cloud VM

You can run WebbOS on cloud providers that support custom images:

1. Upload `webbos.img` as a custom image
2. Create a VM from that image
3. Connect via serial console

Supported providers: AWS, GCP, Azure (with custom image import)

## Troubleshooting

### "python/python3 not found"
Windows: Usually pre-installed. If not, install from Microsoft Store.
Linux: `sudo apt install python3`
macOS: `brew install python3`

### "QEMU not found"
Install QEMU:
- Windows: `choco install qemu` or download from qemu.org
- Linux: `sudo apt install qemu-system-x86`
- macOS: `brew install qemu`

### "OVMF.fd not found"
Download from: https://github.com/retrage/edk2-nightly/raw/master/bin/RELEASEX64_OVMF.fd

### "Kernel panic" or crash
The kernel expects certain UEFI structures. Make sure you're using the full disk image creation process, not direct kernel loading.

### "File not found" in update-image.py
The file doesn't exist in the image yet. Create a new image:
```bash
python scripts/create-image.py
```

## Quick Reference

| Method | Requirements | Difficulty | Works On |
|--------|--------------|------------|----------|
| Python Scripts | Python 3 | Easy | Windows/Linux/macOS |
| mtools (Linux) | mtools | Easy | Linux |
| WSL | Windows 10/11 + WSL | Medium | Windows |
| Docker | Docker Desktop | Medium | Windows/Mac/Linux |
| Real Hardware | USB drive | Hard | Physical PC |
| Cloud VM | Cloud account | Hard | Cloud providers |

## Recommendation

**For Windows development:** Use Method 1 (Python scripts) - this is the primary development workflow and requires no WSL.

**For Linux development:** Use Method 2 (Python scripts or mtools) - both work well.

**For macOS development:** Use Method 3 (Python scripts) - requires QEMU installation.
