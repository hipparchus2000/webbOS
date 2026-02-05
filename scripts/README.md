# WebbOS Scripts

This directory contains helper scripts for running WebbOS.

> **Note:** These scripts are optional. The primary build and run workflow uses Python scripts in the root directory and direct QEMU commands. See `docs/DISK_IMAGE.md` and `docs/RUNNING.md` for the main documentation.

## Available Scripts

### `run-qemu.ps1`

Runs WebbOS in QEMU with various options.

**Usage:**
```powershell
.\scripts\run-qemu.ps1 [-Network] [-Debug] [-Release] [-Rebuild] [-NoGraphic]
```

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

# Serial only (no GUI window)
.\scripts\run-qemu.ps1 -NoGraphic
```

## Primary Workflow (Python Scripts)

The main build and test workflow uses Python scripts that don't require WSL:

```powershell
# 1. Create disk image (if needed)
python scripts/create-image.py

# 2. Build bootloader
cargo +nightly-2025-01-15 build -p bootloader --target x86_64-unknown-uefi -Z build-std=core,compiler_builtins,alloc

# 3. Build kernel
cargo +nightly-2025-01-15 build -p kernel --target x86_64-unknown-none -Z build-std=core,compiler_builtins,alloc

# 4. Update disk image
python scripts/update-image.py webbos.img "EFI/BOOT/BOOTX64.EFI" target/x86_64-unknown-uefi/debug/bootloader.efi
python scripts/update-image.py webbos.img kernel.elf target/x86_64-unknown-none/debug/kernel

# 5. Run
qemu-system-x86_64 -bios OVMF.fd -drive format=raw,file=webbos.img -m 128M -smp 1 -nographic -serial stdio
```

Or as a one-liner:
```powershell
cargo +nightly-2025-01-15 build -p bootloader --target x86_64-unknown-uefi -Z build-std=core,compiler_builtins,alloc; cargo +nightly-2025-01-15 build -p kernel --target x86_64-unknown-none -Z build-std=core,compiler_builtins,alloc; python scripts/update-image.py webbos.img "EFI/BOOT/BOOTX64.EFI" target/x86_64-unknown-uefi/debug/bootloader.efi; python scripts/update-image.py webbos.img kernel.elf target/x86_64-unknown-none/debug/kernel; qemu-system-x86_64 -bios OVMF.fd -drive format=raw,file=webbos.img -m 128M -smp 1 -nographic -serial stdio
```

## Prerequisites

1. **Windows 10/11** (primary development platform)
2. **Rust** with nightly toolchain
3. **QEMU** for x86_64
4. **Python 3** (for disk image management)

### Installing Prerequisites

**Rust:**
```powershell
irm https://win.rustup.rs | iex
rustup toolchain install nightly-2025-01-15
rustup component add rust-src --toolchain nightly-2025-01-15
rustup target add x86_64-unknown-none x86_64-unknown-uefi --toolchain nightly-2025-01-15
```

**QEMU:**
```powershell
# Using chocolatey
choco install qemu

# Or download from https://www.qemu.org/download/#windows
```

**Python 3:**
Usually pre-installed on Windows 11. Verify with:
```powershell
python --version
```

## Disk Image Management

See `docs/DISK_IMAGE.md` for complete documentation on:
- Creating disk images with `create-image.py`
- Updating files with `update-image.py`
- Adding new files with `add-files-to-image.py`

## Tools

The `tools/` directory contains utility scripts:

- **`verify-image.py`** - Verify and inspect FAT32 disk image contents
  ```powershell
  python tools/verify-image.py webbos.img
  ```

## Troubleshooting

### "QEMU not found"
Install QEMU or add it to your PATH:
```powershell
# Check location
where.exe qemu-system-x86_64

# Or download from https://www.qemu.org/download/#windows
```

### "OVMF.fd not found"
The `OVMF.fd` file is included in the repository. If missing, download from:
- URL: https://github.com/retrage/edk2-nightly/raw/master/bin/RELEASEX64_OVMF.fd
- Save as: `OVMF.fd` in the webbOs directory

### "cargo not found"
```powershell
# Ensure Rust is installed and in PATH
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
# Or restart your terminal
```

### "target not found"
```powershell
# Install the target
rustup target add x86_64-unknown-none --toolchain nightly-2025-01-15
rustup target add x86_64-unknown-uefi --toolchain nightly-2025-01-15
```

### Kernel crashes immediately after boot
Check that the entry point in `bootloader/src/main.rs` matches the actual kernel entry point:
```powershell
python -c "import struct; f=open('target/x86_64-unknown-none/debug/kernel','rb'); f.seek(0x18); print(f'Entry: {struct.unpack('<Q', f.read(8))[0]:#x}')"
```

Update in `bootloader/src/main.rs`:
```rust
const KERNEL_ENTRY_PHYS: u64 = 0xXXXXXX; // Use the printed address
```

### QEMU "cannot set up guest memory"
Kill existing QEMU processes:
```powershell
taskkill /F /IM qemu-system-x86_64.exe
```

## Default Login Credentials

Once WebbOS boots:
- **Username:** `admin`
- **Password:** `admin`

Or:
- **Username:** `user`
- **Password:** `user`

## Documentation

- `docs/DISK_IMAGE.md` - Disk image management
- `docs/RUNNING.md` - Running WebbOS
- `docs/BUILD.md` - Build instructions
- `docs/ARCHITECTURE.md` - System architecture
