# Build Instructions for WebbOS

## Overview

WebbOS supports building on **Linux** (primary), **macOS**, and **Windows**. The project includes a Makefile for simplified building on Unix-like systems.

## Quick Start (Linux/macOS)

```bash
# Build and run x86_64
make run-x64

# Build and run aarch64 (Raspberry Pi)
make run-aarch64

# Build everything
make kernel bootloader aarch64-kernel aarch64-image
```

## Prerequisites

### All Platforms

1. **Rust nightly toolchain** (specified in `rust-toolchain.toml`):
   ```bash
   rustup install nightly-2025-01-15
   rustup component add rust-src --toolchain nightly-2025-01-15
   rustup target add x86_64-unknown-none x86_64-unknown-uefi --toolchain nightly-2025-01-15
   rustup target add aarch64-unknown-none --toolchain nightly-2025-01-15
   ```

2. **QEMU** for testing:
   - Linux: `sudo apt-get install qemu-system-x86 qemu-system-arm`
   - macOS: `brew install qemu`
   - Windows: `choco install qemu`

3. **LLD** linker (Linux):
   ```bash
   sudo apt-get install lld
   ```

## Supported Architectures

| Architecture | Target | Bootloader | Status |
|--------------|--------|------------|--------|
| x86_64 | `x86_64-unknown-none` | UEFI (`bootloader/`) | ✅ Working |
| ARM64 | `aarch64-unknown-none` | Pi Bootloader (`bootloader-pi/`) | ✅ Working |

## Linux (Recommended)

### Using Makefile

```bash
# Build x86_64 kernel
make kernel

# Build x86_64 bootloader
make bootloader

# Run x86_64 in QEMU
make run-x64

# Build aarch64 kernel
make aarch64-kernel

# Create aarch64 image for Raspberry Pi
make aarch64-image

# Run aarch64 in QEMU
make run-aarch64
```

### Manual Build (x86_64)

```bash
# Build kernel
cargo build --target x86_64-unknown-none --release -p kernel

# Build bootloader
cargo build --target x86_64-unknown-uefi --release -p bootloader

# Prepare boot files
mkdir -p build/iso/EFI/BOOT
cp target/x86_64-unknown-uefi/release/bootloader.efi build/iso/EFI/BOOT/BOOTX64.EFI
cp target/x86_64-unknown-none/release/kernel build/iso/kernel.elf

# Run in QEMU
qemu-system-x86_64 -m 512M -smp 2 -cpu qemu64 -bios OVMF.fd \
    -drive format=raw,file=fat:rw:build/iso -serial stdio -display none
```

### Manual Build (aarch64)

```bash
# Build kernel
cargo build --target aarch64-unknown-none --release -p kernel

# Build Pi bootloader
cargo build --target aarch64-unknown-none --release -p bootloader-pi

# Create image (uses objcopy)
./scripts/create-pi-image.sh

# Run in QEMU (Raspberry Pi 3)
qemu-system-aarch64 -M raspi3b -kernel build/aarch64/kernel8.img \
    -serial stdio -display none
```

## Windows

### Using PowerShell

```powershell
# Install Rust
irm https://win.rustup.rs | iex

# Install QEMU
choco install qemu

# Build (using same commands as Linux)
cargo build --target x86_64-unknown-none --release -p kernel
cargo build --target x86_64-unknown-uefi --release -p bootloader

# Update disk image with Python
python scripts/create-image.py
python scripts/update-image.py webbos.img "EFI/BOOT/BOOTX64.EFI" target/x86_64-unknown-uefi/release/bootloader.efi
python scripts/update-image.py webbos.img kernel.elf target/x86_64-unknown-none/release/kernel

# Run
qemu-system-x86_64 -bios OVMF.fd -drive format=raw,file=webbos.img -m 512M -serial stdio
```

## macOS

```bash
# Install dependencies
brew install qemu llvm

# Build (same as Linux)
make kernel
make bootloader
make run-x64
```

## Raspberry Pi Deployment

### Create SD Card Image

```bash
# Build combined bootloader + kernel image
./scripts/create-pi-image.sh

# Output files:
# - build/aarch64/kernel8.img
# - build/aarch64/config.txt
# - build/aarch64/cmdline.txt
```

### Deploy to SD Card

1. Format SD card with FAT32
2. Copy files to root:
   - `build/aarch64/kernel8.img`
   - `build/aarch64/config.txt`
   - `build/aarch64/cmdline.txt`
3. Insert into Raspberry Pi 4/5 and power on

## Build Output

After successful build:

```
target/
├── x86_64-unknown-none/release/kernel          # x86_64 kernel
├── x86_64-unknown-uefi/release/bootloader.efi  # x86_64 UEFI bootloader
└── aarch64-unknown-none/release/
    ├── kernel                                  # aarch64 kernel
    └── bootloader-pi                           # Pi bootloader

build/
├── iso/                                        # x86_64 boot files
│   ├── EFI/BOOT/BOOTX64.EFI
│   └── kernel.elf
└── aarch64/                                    # aarch64 boot files
    ├── kernel8.img                             # Combined Pi image
    ├── config.txt
    └── cmdline.txt
```

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

## Makefile Targets

| Target | Description |
|--------|-------------|
| `make kernel` | Build x86_64 kernel |
| `make bootloader` | Build x86_64 UEFI bootloader |
| `make run-x64` | Build and run x86_64 in QEMU |
| `make aarch64-kernel` | Build aarch64 kernel |
| `make aarch64-image` | Create Raspberry Pi image |
| `make run-aarch64` | Build and run aarch64 in QEMU |
| `make clean` | Clean all build artifacts |
| `make test` | Run tests |
| `make fmt` | Format code |
| `make lint` | Run clippy |

## Troubleshooting

### "linker rust-lld not found"

Install LLD:
```bash
# Ubuntu/Debian
sudo apt-get install lld

# Fedora
sudo dnf install lld
```

### "target not found"

```bash
rustup target add x86_64-unknown-none --toolchain nightly-2025-01-15
rustup target add x86_64-unknown-uefi --toolchain nightly-2025-01-15
rustup target add aarch64-unknown-none --toolchain nightly-2025-01-15
```

### "rust-src component not found"

```bash
rustup component add rust-src --toolchain nightly-2025-01-15
```

### Kernel crashes in QEMU

Check that OVMF.fd exists:
```bash
ls -la OVMF.fd || curl -L -o OVMF.fd https://github.com/retrage/edk2-nightly/raw/master/bin/RELEASEX64_OVMF.fd
```

## Testing

```bash
# Run unit tests (host platform)
cargo test -p webbos-shared

# Run kernel in QEMU (x86_64)
make run-x64

# Run kernel in QEMU (aarch64)
make run-aarch64
```

## Release Builds

For optimized builds, use `--release`:

```bash
cargo build --release --target x86_64-unknown-none -p kernel
cargo build --release --target x86_64-unknown-uefi -p bootloader
```

Or with make:
```bash
make kernel RELEASE=1
```
