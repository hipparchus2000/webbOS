# Build Instructions for WebbOS

## Overview

WebbOS requires a cross-compilation toolchain to build the kernel and bootloader. This document provides platform-specific build instructions.

## Common Prerequisites

1. **Rust nightly toolchain** (specified in `rust-toolchain.toml`):
   ```bash
   rustup install nightly-2025-01-15
   rustup component add rust-src --toolchain nightly-2025-01-15
   rustup target add x86_64-unknown-none x86_64-unknown-uefi --toolchain nightly-2025-01-15
   ```

2. **QEMU** for testing:
   - Windows: `choco install qemu` or download from https://www.qemu.org/download/#windows
   - macOS: `brew install qemu`
   - Linux: `sudo apt-get install qemu-system-x86`

## Platform-Specific Instructions

### Linux (Ubuntu/Debian)

```bash
# Install build dependencies
sudo apt-get update
sudo apt-get install -y build-essential lld qemu-system-x86

# Build
make all

# Run in QEMU
make run
```

### macOS

```bash
# Install dependencies
brew install llvm qemu

# Build
make all

# Run in QEMU
make run
```

### Windows

#### Option 1: Using WSL2 (Recommended)

Install WSL2 with Ubuntu, then follow Linux instructions.

#### Option 2: Native Build with LLVM

1. Install LLVM from https://github.com/llvm/llvm-project/releases
2. Add LLVM to PATH
3. Build:
   ```powershell
   $env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
   cargo build --target x86_64-unknown-none -Z build-std=core,compiler_builtins,alloc
   ```

#### Option 3: Using Visual Studio Build Tools

1. Install Visual Studio Build Tools 2019 or later with C++ workload
2. Build:
   ```powershell
   $env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
   cargo build --target x86_64-unknown-none -Z build-std=core,compiler_builtins,alloc
   ```

## Build Output

After a successful build, you should have:

- `target/x86_64-unknown-uefi/release/bootloader.efi` - UEFI bootloader
- `target/x86_64-unknown-none/release/kernel` - Kernel binary
- `build/webbos.iso` - Bootable ISO image (created by Makefile)

## Troubleshooting

### "link.exe not found"

Install Visual Studio Build Tools or use LLVM linker by setting in `.cargo/config.toml`:
```toml
[target.x86_64-unknown-none]
linker = "rust-lld"
```

### "rust-src component not found"

```bash
rustup component add rust-src --toolchain nightly-2025-01-15
```

### "cannot find -lgcc"

When using GNU toolchain, you may need to install the appropriate target libraries.

## Testing

```bash
# Run unit tests (host platform)
cargo test -p webbos-shared

# Run kernel tests (requires QEMU)
make test-kernel

# Run in QEMU with GDB server
make debug
```

Then in another terminal:
```bash
gdb target/x86_64-unknown-none/release/kernel
(gdb) target remote :1234
(gdb) break kernel_entry
(gdb) continue
```
