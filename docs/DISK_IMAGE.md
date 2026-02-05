# WebbOS Disk Image Management

This document describes how to create and manage the WebbOS disk image (`webbos.img`), which is a FAT32-formatted bootable disk image.

## Quick Reference

| Task | Command |
|------|---------|
| **Create new image** | `python scripts/create-image.py` |
| **Update bootloader** | `python scripts/update-image.py webbos.img "EFI/BOOT/BOOTX64.EFI" target/x86_64-unknown-uefi/debug/bootloader.efi` |
| **Update kernel** | `python scripts/update-image.py webbos.img kernel.elf target/x86_64-unknown-none/debug/kernel` |
| **Add new file** | `python scripts/add-files-to-image.py webbos.img path/in/image.txt source.txt` |

## Scripts Overview

### 1. `create-image.py` - Create Disk Image from Scratch

Creates a new FAT32 disk image with the proper directory structure for WebbOS.

**Usage:**
```powershell
# Create default 64MB image with current build artifacts
python scripts/create-image.py

# Create larger image
python scripts/create-image.py --size 128

# Create with specific files
python scripts/create-image.py path/to/bootloader.efi path/to/kernel

# Create empty image (for manual population)
python scripts/create-image.py --empty --size 64
```

**Options:**
- `--size SIZE_MB` - Image size in megabytes (default: 64)
- `--output PATH` - Output file path (default: webbos.img)
- `--empty` - Create empty image without bootloader/kernel
- `bootloader` - Path to bootloader.efi (optional)
- `kernel` - Path to kernel binary (optional)

### 2. `update-image.py` - Update Existing Files

Updates files that already exist in the disk image. **This is the primary script for development.**

**Key Features:**
- ✅ Automatically allocates new clusters if file has grown
- ✅ Frees excess clusters if file has shrunk
- ✅ Updates file size in directory entry
- ✅ Shows progress and cluster allocation info

**Usage:**
```powershell
# Update bootloader (after building)
python scripts/update-image.py webbos.img "EFI/BOOT/BOOTX64.EFI" target/x86_64-unknown-uefi/debug/bootloader.efi

# Update kernel (after building)
python scripts/update-image.py webbos.img kernel.elf target/x86_64-unknown-none/debug/kernel

# Both in one command
python scripts/update-image.py webbos.img "EFI/BOOT/BOOTX64.EFI" target/x86_64-unknown-uefi/debug/bootloader.efi
python scripts/update-image.py webbos.img kernel.elf target/x86_64-unknown-none/debug/kernel
```

**Example Output:**
```
Found file: BOOTX64 EFI (current size: 219136 bytes)
  Old size: 219136 bytes (428 clusters)
  New size: 235000 bytes (460 clusters)
  Allocating 32 new cluster(s)...
  Updated successfully!
```

### 3. `add-files-to-image.py` - Add New Files

Adds new files that don't exist yet in the image. Creates parent directories as needed.

**Usage:**
```powershell
# Add a config file
python scripts/add-files-to-image.py webbos.img system/config.txt myconfig.txt

# Add an icon (creates system/icons/ if needed)
python scripts/add-files-to-image.py webbos.img system/icons/browser.png icons/browser.png
```

## Complete Build Workflow

### First Time Setup
```powershell
# 1. Create the disk image
python scripts/create-image.py

# 2. Build bootloader
cargo +nightly-2025-01-15 build -p bootloader --target x86_64-unknown-uefi -Z build-std=core,compiler_builtins,alloc

# 3. Build kernel
cargo +nightly-2025-01-15 build -p kernel --target x86_64-unknown-none -Z build-std=core,compiler_builtins,alloc

# 4. Update disk image
python scripts/update-image.py webbos.img "EFI/BOOT/BOOTX64.EFI" target/x86_64-unknown-uefi/debug/bootloader.efi
python scripts/update-image.py webbos.img kernel.elf target/x86_64-unknown-none/debug/kernel

# 5. Run in QEMU
qemu-system-x86_64 -bios OVMF.fd -drive format=raw,file=webbos.img -m 128M -smp 1 -nographic -serial stdio
```

### Daily Development
```powershell
# Quick build and test (PowerShell one-liner)
cargo +nightly-2025-01-15 build -p bootloader --target x86_64-unknown-uefi -Z build-std=core,compiler_builtins,alloc; cargo +nightly-2025-01-15 build -p kernel --target x86_64-unknown-none -Z build-std=core,compiler_builtins,alloc; python update-image.py webbos.img "EFI/BOOT/BOOTX64.EFI" target/x86_64-unknown-uefi/debug/bootloader.efi; python update-image.py webbos.img kernel.elf target/x86_64-unknown-none/debug/kernel; qemu-system-x86_64 -bios OVMF.fd -drive format=raw,file=webbos.img -m 128M -smp 1 -nographic -serial stdio
```

## Disk Image Structure

The FAT32 disk image contains:

```
webbos.img (FAT32)
├── EFI/
│   └── BOOT/
│       └── BOOTX64.EFI    ← UEFI bootloader
├── KERNEL.ELF             ← Kernel binary
└── [other files...]
```

### FAT32 Parameters

- **Bytes per sector:** 512
- **Sectors per cluster:** 1 (512 bytes per cluster)
- **Reserved sectors:** 32
- **Number of FATs:** 2
- **Root cluster:** 2

## Troubleshooting

### "File not found" Error

**Problem:** The file you're trying to update doesn't exist in the image.

**Solution:**
- For new files, use `add-files-to-image.py`
- To recreate the image from scratch, use `create-image.py`

### "Not enough free clusters" Error

**Problem:** The disk image is full.

**Solution:**
```powershell
# Create a larger image
python create-image.py --size 128

# Or clean up unused files first
```

### "Invalid boot sector signature" Error

**Problem:** The disk image is corrupted or not a FAT32 image.

**Solution:**
```powershell
# Recreate the image
python create-image.py --output webbos.img.new
# Copy any needed files from old image, then replace
```

### Kernel Loads but Crashes

**Problem:** The kernel file in the image might be truncated (old `update-image.py` bug).

**Check:**
```powershell
# Compare sizes
(Get-Item target/x86_64-unknown-none/debug/kernel).Length
# Should match the size in the image

# Use the fixed update-image.py which handles file growth
python scripts/update-image.py webbos.img kernel.elf target/x86_64-unknown-none/debug/kernel
```

## Technical Details

### Why These Scripts?

On Windows, we don't have access to standard Unix tools like `mtools` without WSL. These Python scripts provide:

1. **Native Windows support** - No WSL required
2. **Direct FAT32 manipulation** - Read/write raw filesystem structures
3. **Safety** - Backup FAT copies are maintained
4. **Flexibility** - Handle file growth/shrinking automatically

### Script Differences

| Feature | `scripts/create-image.py` | `scripts/update-image.py` | `scripts/add-files-to-image.py` |
|---------|------------------|-------------------|------------------------|
| Creates new image | ✅ | ❌ | ❌ |
| Updates existing files | ❌ | ✅ | ❌ |
| Adds new files | ✅ (bootloader/kernel) | ❌ | ✅ |
| Handles file growth | N/A | ✅ | ✅ |
| Creates directories | ✅ | ❌ | ✅ |

### FAT32 Implementation Notes

The scripts implement a minimal but functional FAT32 writer:

- **8.3 filenames only** - Long filenames (LFN) not supported
- **No timestamps** - File times are set to 0
- **Simple allocation** - First-fit cluster allocation
- **No fragmentation handling** - Assumes contiguous allocation is possible

For WebbOS use case (bootloader + kernel + some config files), these limitations are acceptable.

## Alternative: Using WSL/mtools

If you have WSL installed, you can use traditional Linux tools:

```bash
# In WSL
sudo apt install mtools

# Create FAT32 image
dd if=/dev/zero of=webbos.img bs=1M count=64
mformat -i webbos.img -F ::

# Create directories
mmd -i webbos.img ::/EFI
mmd -i webbos.img ::/EFI/BOOT

# Copy files
mcopy -i webbos.img target/x86_64-unknown-uefi/debug/bootloader.efi ::/EFI/BOOT/BOOTX64.EFI
mcopy -i webbos.img target/x86_64-unknown-none/debug/kernel ::/kernel.elf
```

However, the Python scripts are recommended for Windows development as they don't require WSL.
