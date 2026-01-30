# WebbOS

A minimal, high-performance operating system written in Rust, designed around a web-first architecture.

![WebbOS Logo](docs/assets/logo.png)

## Overview

WebbOS is an experimental operating system where the entire desktop environment is implemented as a single HTML file with an integrated web browser engine. Applications are web-based (HTML/JS/WASM) and distributed through a built-in app store.

## Features

- **Rust-based kernel** - Memory safety and high performance
- **UEFI bootloader** - Modern boot process
- **x86_64 support** - Multi-core processor support
- **Web browser engine** - HTML5, CSS3, JavaScript, WebAssembly
- **HTML-based desktop** - Single-file desktop environment
- **App store** - Download and manage web applications
- **TLS 1.3** - Secure network connections

## Project Structure

```
webbos/
├── bootloader/          # UEFI bootloader
├── kernel/              # OS kernel
│   ├── arch/            # Architecture-specific code (x86_64)
│   ├── mm/              # Memory management
│   ├── console/         # VGA/serial output
│   └── ...
├── shared/              # Shared types between bootloader and kernel
└── docs/                # Documentation
```

## Building

### Prerequisites

1. **Rust nightly toolchain** (nightly-2025-01-15)
2. **Build dependencies:**
   - Windows: Visual Studio Build Tools 2019+ or LLVM/MinGW
   - Linux: `build-essential`, `lld`
   - macOS: Xcode Command Line Tools

3. **QEMU** (for testing)

### Quick Start

```bash
# Install Rust nightly
rustup install nightly-2025-01-15
rustup default nightly-2025-01-15

# Install targets
rustup target add x86_64-unknown-none x86_64-unknown-uefi
rustup component add rust-src

# Build the project
make all

# Run in QEMU
make run
```

### Building on Windows

Option 1: Using Visual Studio Build Tools
```powershell
# Install Visual Studio Build Tools with C++ workload
# Then build with cargo
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
cargo build --target x86_64-unknown-none
```

Option 2: Using MinGW/LLVM
```powershell
# Install LLVM which includes lld-link
# Set linker in .cargo/config.toml
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    User Space (Ring 3)                      │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │   Browser   │  │   Desktop   │  │  User Apps          │ │
│  │   Engine    │  │  (HTML/JS)  │  │  (WASM/JS/HTML)     │ │
│  └─────────────┘  └─────────────┘  └─────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
                              │
                    System Call Interface
                              │
┌─────────────────────────────────────────────────────────────┐
│                  Kernel Space (Ring 0)                      │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              System Call Handler                     │   │
│  └─────────────────────────────────────────────────────┘   │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │   Process   │  │    VFS      │  │   Network Stack     │ │
│  │   Manager   │  │   Layer     │  │   (TCP/IP/TLS)      │ │
│  └─────────────┘  └─────────────┘  └─────────────────────┘ │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │   Memory    │  │   Device    │  │   File Systems      │ │
│  │   Manager   │  │   Drivers   │  │   (WebbFS/FAT32)    │ │
│  └─────────────┘  └─────────────┘  └─────────────────────┘ │
│  ┌─────────────────────────────────────────────────────────┐│
│  │              Hardware Abstraction Layer (HAL)           ││
│  │         (Paging, Interrupts, Timers, I/O)               ││
│  └─────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
```

### Boot Process

1. **UEFI Firmware** → Loads bootloader
2. **Bootloader** → Sets up page tables, loads kernel
3. **Kernel** → Initializes subsystems, starts shell

### Memory Layout

```
0x0000_0000_0000_0000 - 0x0000_7FFF_FFFF_FFFF  User space
0x0000_8000_0000_0000 - 0xFFFF_7FFF_FFFF_FFFF  Non-canonical
0xFFFF_8000_0000_0000 - 0xFFFF_FFFF_FFFF_FFFF  Kernel space
  0xFFFF_8000_0010_0000  Kernel code/data
  0xFFFF_8000_4000_0000  Kernel heap
```

## Testing

```bash
# Run unit tests
make test

# Run in QEMU with GDB debugging
make debug

# Generate coverage report
make coverage
```

## Development Status

| Component | Status |
|-----------|--------|
| Bootloader | ✅ Implemented |
| Kernel Core | ✅ Implemented |
| Memory Management | ✅ Implemented |
| Interrupts | ✅ Implemented |
| Console/VGA | ✅ Implemented |
| Process Management | 🚧 Planned |
| File System | 🚧 Planned |
| Network Stack | 🚧 Planned |
| Browser Engine | 🚧 Planned |
| Desktop Environment | 🚧 Planned |
| App Store | 🚧 Planned |

## License

This project is licensed under the MIT OR Apache-2.0 license.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.
