# WebbOS Architecture

## System Overview

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

## Boot Process

### Phase 1: UEFI Boot

1. **UEFI Firmware** initializes hardware
2. **Bootloader** (`bootloader.efi`) loaded from ESP
3. Bootloader performs:
   - Memory map acquisition
   - Kernel file loading
   - Page table setup
   - Graphics output protocol initialization

### Phase 2: Kernel Initialization

1. **Assembly entry** (`_start`) sets up stack
2. **Rust entry** (`kernel_entry`) receives BootInfo
3. Subsystem initialization:
   - Console (VGA + Serial)
   - CPU features (SSE, NX bit)
   - Memory management
   - Interrupt handling
   - Device drivers

### Phase 3: Userspace

1. Initialize browser engine
2. Load desktop environment
3. Start app store service

## Memory Layout

### Physical Memory

```
0x00000000 - 0x000FFFFF  Reserved (BIOS, VGA)
0x00100000 - 0x00FFFFFF  Kernel load address
0x01000000 - ...         Available RAM
```

### Virtual Memory (Higher Half Kernel)

```
0x0000000000000000 - 0x00007FFFFFFFFFFF  User space
0x0000800000000000 - 0xFFFF7FFFFFFFFFFF  Non-canonical
0xFFFF800000000000 - 0xFFFF80003FFFFFFF  Kernel code/data (1GB)
0xFFFF800040000000 - 0xFFFF8000BFFFFFFF  Kernel heap (2GB)
0xFFFF8000C0000000 - 0xFFFF80FFFFFFFFFF  Kernel stacks
0xFFFF810000000000 - 0xFFFFFEFFFFFFFFFF  Physical memory mapping
0xFFFFFF0000000000 - 0xFFFFFFFFFFFFFFFF  Reserved
```

## Page Table Structure

4-level paging (x86_64):
- **PML4** (Page Map Level 4)
- **PDPT** (Page Directory Pointer Table)
- **PD** (Page Directory)
- **PT** (Page Table)

Each table has 512 entries (9 bits), covering 48-bit virtual addresses.

## Interrupt Handling

IDT (Interrupt Descriptor Table) layout:
- 0-31: CPU exceptions
- 32-47: PIC/IO-APIC interrupts
- 128: System call interrupt
- Others: Available for devices

## System Calls

Implemented via `syscall`/`sysret` instructions:

```rust
// Example syscall interface
pub fn syscall(num: u64, arg1: u64, arg2: u64, arg3: u64) -> u64;
```

Syscall numbers defined in `shared/src/syscalls.rs`.

## File Systems

### WebbFS (Native)
- Optimized for web assets
- Compression support
- Deduplication

### Supported External
- FAT32 (USB/removable)
- EXT2/4 (Linux compatibility)

## Network Stack

### Layers
1. **Driver** (Intel E1000/VirtIO)
2. **Link** (Ethernet)
3. **Network** (IPv4/IPv6)
4. **Transport** (TCP/UDP)
5. **Application** (HTTP/HTTPS/DNS)

### TLS
- rustls library
- TLS 1.3 support
- Certificate management

## Browser Engine

### Components
1. **HTML Parser** - Spec-compliant parsing
2. **CSS Engine** - Style computation
3. **Layout** - Box model, flex, grid
4. **Renderer** - CPU/GPU painting
5. **JavaScript** - QuickJS integration
6. **WASM** - wasmtime runtime

### Security
- Same-origin policy
- Content Security Policy
- Sandboxed iframe execution

## App Model

### App Package (.webapp)
```
manifest.json   - Metadata
main.html       - Entry point
assets/         - Static files
wasm/           - WebAssembly modules
```

### App Lifecycle
1. Download from store
2. Verify signature
3. Install to `/apps/{id}/`
4. Create user data dir
5. Launch in sandboxed context

## Development Guidelines

### Code Organization
- `arch/` - Architecture-specific code
- `mm/` - Memory management
- `fs/` - File systems
- `net/` - Networking
- `process/` - Process management
- `drivers/` - Device drivers

### Error Handling
Use `Result<T, Error>` throughout kernel. Panic on unrecoverable errors.

### Safety
- Use `unsafe` only when necessary
- Document all `unsafe` blocks
- Prefer safe abstractions

### Testing
- Unit tests: `cargo test`
- Integration: QEMU + test harness
- Coverage: `cargo tarpaulin`
