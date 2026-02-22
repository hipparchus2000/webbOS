# Building WebbOS for Raspberry Pi

This guide covers building WebbOS Pi from source.

## Quick Start

```powershell
cd Pi
./build.bat
```

This builds everything and creates:
- `webbos-pi-raw.img` - For QEMU testing
- `webbos-pi.img` - For SD card deployment

## Prerequisites

### Windows 11
```powershell
# Install Rust
irm https://win.rustup.rs | iex

# Install nightly toolchain
rustup toolchain install nightly-2025-01-15
rustup component add rust-src --toolchain nightly-2025-01-15
rustup target add aarch64-unknown-none --toolchain nightly-2025-01-15

# Install Python (usually pre-installed)
python --version
```

### Linux/macOS
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install nightly toolchain
rustup toolchain install nightly-2025-01-15
rustup component add rust-src --toolchain nightly-2025-01-15
rustup target add aarch64-unknown-none --toolchain nightly-2025-01-15
```

## Build Process

### 1. Build Bootloader

```bash
cargo +nightly-2025-01-15 build -p bootloader \
    --target aarch64-unknown-none \
    -Z build-std=core,compiler_builtins,alloc \
    --release
```

**Output:** `target/aarch64-unknown-none/release/bootloader`

The bootloader:
- Loads at address 0x80000
- Initializes ARM64 CPU (EL2→EL1)
- Parses device tree blob
- Sets up MMU page tables
- Loads kernel at 0x100000
- Jumps to kernel at 0xFFFF000000100000

### 2. Build Kernel

```bash
cargo +nightly-2025-01-15 build -p kernel \
    --target aarch64-unknown-none \
    -Z build-std=core,compiler_builtins,alloc \
    --release
```

**Output:** `target/aarch64-unknown-none/release/kernel`

The kernel includes:
- ARM64 MMU management
- Process scheduler
- Network stack (TCP/IP, TLS 1.3)
- Pi drivers (mailbox, USB, SDIO)
- Desktop environment
- Browser engine

### 3. Create Raw Image (for QEMU)

```bash
python make-raw-image.py \
    target/aarch64-unknown-none/release/bootloader \
    target/aarch64-unknown-none/release/kernel \
    webbos-pi-raw.img
```

This creates a combined image with:
- Bootloader at offset 0x80000 (512KB)
- Kernel at offset 0x100000 (1MB)

### 4. Create SD Card Image (for real Pi)

```bash
python scripts/create-sdcard.py \
    target/aarch64-unknown-none/release/bootloader \
    -o webbos-pi.img
```

This creates a partitioned SD card image with:
- FAT32 boot partition (256MB)
- Root filesystem partition (remaining space)
- `kernel8.img` in boot partition
- `config.txt` and `cmdline.txt`

## Build Options

### Debug Build
```bash
./build.bat debug
# or
cargo +nightly-2025-01-15 build -p kernel --target aarch64-unknown-none -Z build-std=core,compiler_builtins,alloc
```

### Release Build (Optimized)
```bash
./build.bat release
# or
cargo +nightly-2025-01-15 build -p kernel --target aarch64-unknown-none -Z build-std=core,compiler_builtins,alloc --release
```

## Output Files

| File | Size | Purpose |
|------|------|---------|
| `bootloader` | ~70KB | Bare metal bootloader |
| `kernel` | ~420KB | Kernel ELF |
| `webbos-pi-raw.img` | ~1.3MB | QEMU test image |
| `webbos-pi.img` | ~256MB | SD card image |

## Troubleshooting

### "target not found" Error
```bash
rustup target add aarch64-unknown-none --toolchain nightly-2025-01-15
```

### "rust-src not found" Error
```bash
rustup component add rust-src --toolchain nightly-2025-01-15
```

### Clean Build
```bash
cargo clean
cargo +nightly-2025-01-15 build -p bootloader --target aarch64-unknown-none -Z build-std=core,compiler_builtins,alloc --release
cargo +nightly-2025-01-15 build -p kernel --target aarch64-unknown-none -Z build-std=core,compiler_builtins,alloc --release
```

### Linker Errors
Ensure you're using `rust-lld` (specified in `.cargo/config.toml`).

## Cross-Compilation Notes

WebbOS Pi is cross-compiled from x86_64 Windows/Linux to ARM64:

| Host | Target | Status |
|------|--------|--------|
| Windows x86_64 | ARM64 | ✅ Primary |
| Linux x86_64 | ARM64 | ✅ Supported |
| macOS x86_64 | ARM64 | ✅ Supported |
| macOS ARM64 | ARM64 | ✅ Native |

## Next Steps

- [Running Guide](RUNNING.md) - How to run on QEMU or real Pi
- [HARDWARE.md](../HARDWARE.md) - Hardware specifications
- [PORTING.md](../PORTING.md) - ARM64 port details
