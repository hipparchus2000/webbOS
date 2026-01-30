# Build Instructions for WebbOS

## Overview

WebbOS requires a cross-compilation toolchain to build the kernel and bootloader. This document provides platform-specific build instructions.

> **Note:** This project was developed and tested on Windows 11. The build process uses native Windows tools (PowerShell, Python) rather than WSL.

## Prerequisites

### Required Tools

1. **Rust nightly toolchain** (specified in `rust-toolchain.toml`):
   ```powershell
   rustup install nightly-2025-01-15
   rustup component add rust-src --toolchain nightly-2025-01-15
   rustup target add x86_64-unknown-none x86_64-unknown-uefi --toolchain nightly-2025-01-15
   ```

2. **QEMU** for testing:
   - Windows: `choco install qemu` or download from https://www.qemu.org/download/#windows
   - macOS: `brew install qemu`
   - Linux: `sudo apt-get install qemu-system-x86`

3. **Python 3** (for disk image updates on Windows):
   - Windows: Usually pre-installed or from Microsoft Store
   - Used by `update-image.py` script

## Windows 11 Toolchain (Primary Development Platform)

This is the toolchain used for active development:

### 1. Build the Kernel

```powershell
cargo +nightly-2025-01-15 build -p kernel --target x86_64-unknown-none -Z build-std=core,compiler_builtins,alloc
```

**Output:** `target/x86_64-unknown-none/debug/kernel`

### 2. Build the Bootloader

```powershell
cargo +nightly-2025-01-15 build -p bootloader --target x86_64-unknown-uefi -Z build-std=core,compiler_builtins,alloc
```

**Output:** `target/x86_64-unknown-uefi/debug/bootloader.efi`

### 3. Update Disk Image

The disk image (`webbos.img`) is a FAT32 filesystem. Use the Python script to update files:

```powershell
# Update bootloader
python update-image.py webbos.img "EFI/BOOT/BOOTX64.EFI" target/x86_64-unknown-uefi/debug/bootloader.efi

# Update kernel
python update-image.py webbos.img kernel.elf target/x86_64-unknown-none/debug/kernel
```

> **Note:** The `update-image.py` script locates files by name in the FAT32 image and overwrites them. No WSL or `mtools` required.

### 4. Run in QEMU

```powershell
qemu-system-x86_64 -bios OVMF.fd -drive format=raw,file=webbos.img -m 128M -smp 1 -nographic -serial stdio
```

### Complete Build Script

```powershell
# Build everything
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"

cargo +nightly-2025-01-15 build -p bootloader --target x86_64-unknown-uefi -Z build-std=core,compiler_builtins,alloc
cargo +nightly-2025-01-15 build -p kernel --target x86_64-unknown-none -Z build-std=core,compiler_builtins,alloc

# Update disk image
python update-image.py webbos.img "EFI/BOOT/BOOTX64.EFI" target/x86_64-unknown-uefi/debug/bootloader.efi
python update-image.py webbos.img kernel.elf target/x86_64-unknown-none/debug/kernel

# Run
qemu-system-x86_64 -bios OVMF.fd -drive format=raw,file=webbos.img -m 128M -smp 1 -nographic -serial stdio
```

## Alternative Platforms

### Linux (Ubuntu/Debian)

```bash
# Install build dependencies
sudo apt-get update
sudo apt-get install -y build-essential lld qemu-system-x86 mtools

# Build
cargo +nightly-2025-01-15 build -p bootloader --target x86_64-unknown-uefi -Z build-std=core,compiler_builtins,alloc
cargo +nightly-2025-01-15 build -p kernel --target x86_64-unknown-none -Z build-std=core,compiler_builtins,alloc

# Create/update disk image with mtools
mcopy -o -i webbos.img target/x86_64-unknown-uefi/debug/bootloader.efi ::/EFI/BOOT/BOOTX64.EFI
mcopy -o -i webbos.img target/x86_64-unknown-none/debug/kernel ::/kernel.elf

# Run
qemu-system-x86_64 -bios OVMF.fd -drive format=raw,file=webbos.img -m 128M -smp 1 -nographic -serial stdio
```

### macOS

```bash
# Install dependencies
brew install llvm qemu mtools

# Build
cargo +nightly-2025-01-15 build -p bootloader --target x86_64-unknown-uefi -Z build-std=core,compiler_builtins,alloc
cargo +nightly-2025-01-15 build -p kernel --target x86_64-unknown-none -Z build-std=core,compiler_builtins,alloc

# Use Python script or mtools for disk image
python update-image.py webbos.img kernel.elf target/x86_64-unknown-none/debug/kernel

# Run
qemu-system-x86_64 -bios OVMF.fd -drive format=raw,file=webbos.img -m 128M -smp 1 -nographic -serial stdio
```

## Build Output

After a successful build, you should have:

- `target/x86_64-unknown-uefi/debug/bootloader.efi` - UEFI bootloader
- `target/x86_64-unknown-none/debug/kernel` - Kernel binary
- `webbos.img` - Bootable disk image (updated in place)

## Build Configuration

### Rust Toolchain

Specified in `rust-toolchain.toml`:
```toml
[toolchain]
channel = "nightly-2025-01-15"
components = ["rust-src"]
```

### Cargo Configuration

Located in `.cargo/config.toml`:
```toml
[unstable]
build-std = ["core", "compiler_builtins", "alloc"]
```

### Kernel Entry Point

The kernel entry point changes with each build. The bootloader reads the ELF header to get the correct address. Current entry point can be checked with:

```powershell
python -c "import struct; f=open('target/x86_64-unknown-none/debug/kernel','rb'); f.seek(0x18); print(f'Entry: {struct.unpack('<Q', f.read(8))[0]:#x}')"
```

## Troubleshooting

### "cargo not found"

```powershell
# Ensure Rust is installed and in PATH
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
# Or restart your terminal
```

### "target not found"

```bash
# Install the target
rustup target add x86_64-unknown-none --toolchain nightly-2025-01-15
rustup target add x86_64-unknown-uefi --toolchain nightly-2025-01-15
```

### "rust-src component not found"

```bash
rustup component add rust-src --toolchain nightly-2025-01-15
```

### "cannot find -lgcc"

When using GNU toolchain, you may need to install the appropriate target libraries. The build uses `compiler_builtins` instead.

### Kernel crashes immediately after boot

Check that the entry point in `bootloader/src/main.rs` matches the actual kernel entry point:
```rust
const KERNEL_ENTRY_PHYS: u64 = 0xXXXXXX; // Must match kernel ELF entry point
```

### QEMU "cannot set up guest memory"

Kill existing QEMU processes:
```powershell
taskkill /F /IM qemu-system-x86_64.exe
```

## Testing

```bash
# Run unit tests (host platform)
cargo test -p webbos-shared

# Run kernel tests (requires QEMU)
qemu-system-x86_64 -bios OVMF.fd -drive format=raw,file=webbos.img -m 128M -smp 1 -nographic -serial stdio
```

## Release Builds

For optimized builds:

```powershell
cargo +nightly-2025-01-15 build --release -p kernel --target x86_64-unknown-none -Z build-std=core,compiler_builtins,alloc
cargo +nightly-2025-01-15 build --release -p bootloader --target x86_64-unknown-uefi -Z build-std=core,compiler_builtins,alloc
```
