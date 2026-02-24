# WebbOS Architecture

This document describes the architecture of WebbOS, a web browser operating system written in Rust for x86_64 platforms.

> **Status:** ~85% Complete (see STATUS.md for details)

## Overview

WebbOS is a monolithic kernel designed specifically for running a web browser directly on bare metal. It provides a complete networking stack, modern cryptography (TLS 1.3), filesystem support, and a web rendering engine.

```
┌─────────────────────────────────────────────────────────────────┐
│                        WebbOS Kernel                             │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────┐│
│  │   Browser   │  │   Shell     │  │   Tests     │  │   FS    ││
│  │   Engine    │  │   Command   │  │   Suite     │  │   Utils ││
│  └──────┬──────┘  └─────────────┘  └─────────────┘  └─────────┘│
├───────┬─┴──────────────────────────────────────────────────────┤
│       │                     Web Stack                          │
│  ┌────┴────┐  ┌─────────────┐  ┌─────────────┐  ┌────────────┐│
│  │   HTTP  │  │    TLS      │  │    DNS      │  │   TCP/IP   ││
│  │  Client │  │    1.3      │  │  Resolver   │  │   Stack    ││
│  └─────────┘  └─────────────┘  └─────────────┘  └────────────┘│
├─────────────────────────────────────────────────────────────────┤
│                        System Services                           │
│  ┌───────────┐  ┌───────────┐  ┌───────────┐  ┌───────────────┐│
│  │   VFS     │  │  Process  │  │  Syscall  │  │    Graphics   ││
│  │   Layer   │  │  Manager  │  │ Interface │  │    (VESA)     ││
│  └───────────┘  └───────────┘  └───────────┘  └───────────────┘│
├─────────────────────────────────────────────────────────────────┤
│                        Hardware Abstraction                      │
│  ┌───────────┐  ┌───────────┐  ┌───────────┐  ┌───────────────┐│
│  │   VirtIO  │  │  Storage  │  │   Input   │  │    PCI Bus    ││
│  │   Net     │  │  (ATA/NVMe)│  │  KB/Mouse │  │   Scanner     ││
│  └───────────┘  └───────────┘  └───────────┘  └───────────────┘│
├─────────────────────────────────────────────────────────────────┤
│                        Kernel Core                               │
│  ┌───────────┐  ┌───────────┐  ┌───────────┐  ┌───────────────┐│
│  │  Memory   │  │ Interrupt │  │  Console  │  │    Panic      ││
│  │  Manager  │  │  Handler  │  │   (VGA)   │  │   Handler     ││
│  └───────────┘  └───────────┘  └───────────┘  └───────────────┘│
└─────────────────────────────────────────────────────────────────┘
```

## Boot Process

```
1. UEFI Firmware
        ↓
2. Bootloader (bootloader/)
   - UEFI services initialization
   - Framebuffer setup
   - Load kernel ELF at 0xFFFF_8000_0000_0000
        ↓
3. Kernel Entry (kernel/src/main.rs:kernel_entry)
   - CPU initialization
   - Memory management setup
   - Interrupt system
   - Driver initialization
   - Network stack
   - Desktop environment
```

## Memory Layout

```
Higher Half Kernel Mapping (0xFFFF_8000_0000_0000+)

0xFFFF_FFFF_FFFF_FFFF  ┌─────────────────┐
                       │   Kernel Stack  │  Stack grows down
0xFFFF_FFFF_8000_0000  ├─────────────────┤
                       │   Kernel Code   │  Text, Data, BSS
0xFFFF_8000_0100_0000  ├─────────────────┤
                       │   Kernel Heap   │  Dynamic allocation
0xFFFF_8000_0010_0000  ├─────────────────┤
                       │   Boot Info     │  UEFI boot data
0xFFFF_8000_0000_0000  └─────────────────┘

Identity Mapped (First 1GB)
0x0000_0000_0000_0000  ┌─────────────────┐
                       │   Physical      │  MMIO, DMA buffers
                       │   Memory        │  Identity mapped
                       └─────────────────┘
```

## Components

### 1. Bootloader (`bootloader/`)

UEFI-compliant bootloader that:
- Locates and loads the kernel ELF
- Sets up higher-half kernel mapping
- Provides boot information to kernel
- Exits UEFI boot services properly

**Status:** ✅ Complete

### 2. Memory Management (`kernel/src/mm/`)

| Module | Purpose | Status |
|--------|---------|--------|
| `frame_allocator.rs` | Physical page allocation | ✅ |
| `paging.rs` | Virtual memory mapping | ✅ |
| `heap.rs` | Kernel heap allocator | ✅ |
| `gdt.rs` | Global Descriptor Table | ✅ |

**Frame Allocation:** Bitmap-based allocator for physical frames.

**Page Tables:** 4-level paging for x86_64 with identity mapping for first 1GB.

**Heap:** 1MB initial kernel heap using `linked_list_allocator`.

### 3. Interrupts (`kernel/src/arch/`)

| Component | Description | Status |
|-----------|-------------|--------|
| IDT | 256 interrupt descriptors | ✅ |
| PIC | 8259A Programmable Interrupt Controller | ✅ |
| Exceptions | CPU exceptions with error codes | ✅ |
| IRQs | Hardware interrupts 0-15 | ✅ |

**Status:** ✅ Complete

### 4. Process Management (`kernel/src/process/`)

| Structure | Description | Status |
|-----------|-------------|--------|
| PCB | Process Control Block | ✅ |
| TCB | Thread Control Block | ✅ |
| Scheduler | 32-priority round-robin | ✅ |

**Context Switch:** Saves/restores:
- General-purpose registers (RAX-R15)
- Instruction pointer (RIP)
- Stack pointer (RSP)
- Segment registers (CS, SS)
- Page table (CR3)

**Status:** ✅ Complete

### 5. Syscall Interface (`kernel/src/syscall.rs`)

Uses `syscall`/`sysret` instructions.

| Syscall | Number | Description | Status |
|---------|--------|-------------|--------|
| exit | 0 | Terminate process | ✅ |
| write | 1 | Write to file descriptor | ✅ |
| read | 2 | Read from file descriptor | ✅ |
| open | 3 | Open file | ✅ |
| close | 4 | Close file descriptor | ✅ |
| socket | 10 | Create socket | ✅ |
| connect | 11 | Connect socket | ✅ |

**Status:** ✅ Complete

### 6. Virtual Filesystem (`kernel/src/fs/`)

Layered architecture:

```
┌─────────────────────────────────────┐
│  System Calls (open, read, write)   │
├─────────────────────────────────────┤
│        VFS Layer (vfs.rs)           │
├─────────────────────────────────────┤
│  File Systems (ext2.rs, fat32.rs)   │
├─────────────────────────────────────┤
│     Block Devices (block.rs)        │
├─────────────────────────────────────┤
│  Storage Drivers (ata, nvme)        │
└─────────────────────────────────────┘
```

**Status:** ✅ Complete (drivers implemented, needs hardware testing)

### 7. Network Stack (`kernel/src/net/`)

**Layer 2:** VirtIO network driver ✅
**Layer 3:** IPv4, ARP, ICMP ✅
**Layer 4:** TCP, UDP with BSD sockets API ✅
**Layer 5+:** DNS resolver ✅, HTTP/1.1 & HTTP/2 client ✅

```
┌─────────────────────────────────────────┐
│         Application (Browser)           │
├─────────────────────────────────────────┤
│  HTTP/1.1  │  HTTP/2  │  DNS Resolver  │
├────────────┴──────────┴────────────────┤
│            BSD Socket API               │
├─────────────────┬───────────────────────┤
│      TCP        │         UDP           │
├─────────────────┴───────────────────────┤
│              IPv4 / ICMP                │
├─────────────────────────────────────────┤
│           ARP Resolution                │
├─────────────────────────────────────────┤
│         VirtIO Network Driver           │
└─────────────────────────────────────────┘
```

**Status:** ✅ Complete (needs real network testing)

### 8. Cryptography (`kernel/src/crypto/`)

Modern cryptographic primitives:

| Algorithm | Purpose | Status |
|-----------|---------|--------|
| SHA-256 | Hashing | ✅ |
| SHA-384 | Hashing | ✅ |
| ChaCha20 | Symmetric cipher | ✅ |
| Poly1305 | MAC | ✅ |
| X25519 | Key exchange | ✅ |
| HKDF | Key derivation | ✅ |

**Status:** ✅ Complete

### 9. TLS 1.3 (`kernel/src/tls/`)

Full TLS 1.3 implementation supporting:
- `TLS_CHACHA20_POLY1305_SHA256` cipher suite
- X25519 key exchange
- 1-RTT handshake
- Encrypted records (TLSInnerPlaintext)

```
Client                                  Server
  │                                        │
  │────────── ClientHello ────────────────>│
  │          + KeyShare (X25519)           │
  │                                        │
  │<───────── ServerHello ────────────────│
  │          + KeyShare (X25519)           │
  │<───────── {EncryptedExtensions} ──────│
  │<───────── {Certificate} ──────────────│
  │<───────── {CertificateVerify} ────────│
  │<───────── {Finished} ─────────────────│
  │                                        │
  │────────── {Finished} ────────────────>│
  │                                        │
  │========== Application Data ===========>│
  │<========== Application Data ==========│
```

**Status:** ✅ Complete

### 10. HTTP Client (`kernel/src/net/http/`)

Features:
- HTTP/1.1 and HTTP/2 support
- Connection pooling
- TLS integration for HTTPS
- Request/response parsing
- Automatic protocol selection

**Status:** ✅ Complete

### 11. Browser Engine (`kernel/src/browser/`)

Web rendering pipeline:

```
┌──────────────┐   ┌──────────────┐   ┌──────────────┐
│ HTML Parser  │──>│   CSS Parser │──>│ Layout Engine│
│  (Tokenizer) │   │  (Selector)  │   │  (Box Model) │
└──────────────┘   └──────────────┘   └──────┬───────┘
                                              │
┌──────────────┐   ┌──────────────┐   ┌──────▼───────┐
│   Renderer   │<──│  Framebuffer │<──│ Render Tree  │
│  (Display)   │   │   (VESA)     │   │  Generation  │
└──────────────┘   └──────────────┘   └──────────────┘
                                              ↑
┌──────────────┐   ┌──────────────┐           │
│  JavaScript  │──>│   WASM VM    │───────────┘
│ Interpreter  │   │  (Runtime)   │
└──────────────┘   └──────────────┘
```

**Status:** ⚠️ Partial (70% - parsers complete, integration pending)

### 12. Graphics (`kernel/src/graphics/`)

- Graphics context for rendering
- 2D drawing primitives (lines, circles, rectangles)
- Bitmap font rendering
- Color utilities

**Status:** ✅ Complete

### 13. VESA Driver (`kernel/src/drivers/vesa/`)

- VESA/VBE framebuffer support
- 1024x768 @ 32bpp default
- 2D primitives (lines, rectangles, circles)
- Text rendering with bitmap fonts

**Status:** ✅ Complete

### 14. Input Drivers (`kernel/src/drivers/input/`)

- PS/2 Keyboard driver with scancode translation
- PS/2 Mouse driver with 3 buttons
- Event queue system

**Status:** ✅ Complete

### 15. Storage Drivers (`kernel/src/drivers/storage/`)

| Driver | Interface | Supported Devices | Status |
|--------|-----------|-------------------|--------|
| ATA/IDE | PIO/DMA | Hard drives, CD-ROMs | ✅ |
| AHCI | SATA | Modern SATA drives | ✅ |
| NVMe | PCIe | SSDs (high performance) | ✅ |

**Status:** ✅ Complete (needs hardware testing)

### 16. User Management (`kernel/src/users/`)

- Multi-user support
- SHA-256 password hashing
- Session management
- Admin and standard user types

**Status:** ✅ Complete

### 17. Desktop Environment (`kernel/src/desktop/`)

- HTML/CSS-based desktop
- Window manager with z-index
- Taskbar, start menu, desktop icons
- 7 built-in applications

**Applications:**
| App | Icon | Status |
|-----|------|--------|
| File Manager | 📁 | ✅ |
| Notepad | 📝 | ✅ |
| Paint | 🎨 | ✅ |
| Task Manager | 📊 | ✅ |
| User Manager | 👥 | ✅ |
| Terminal | 💻 | ✅ |
| Web Browser | 🌐 | ✅ |

**Status:** ✅ Complete (HTML generation works, message passing needs completion)

## Security Features

1. **Higher-half kernel:** Kernel code in high memory ✅
2. **NX bit:** No-execute for data pages ✅
3. **ASLR:** Address space layout randomization (planned)
4. **TLS 1.3:** Modern encryption for network connections ✅
5. **SMAP/SMEP:** Supervisor mode access prevention (CPU features)
6. **Password hashing:** SHA-256 with salt ✅

## Build System

```
Cargo workspace with 2 crates:
- bootloader: x86_64-unknown-uefi target
- kernel: x86_64-unknown-none target

Linker: rust-lld
Boot: UEFI → higher-half kernel
Size: ~6.7MB kernel binary
Lines: ~20,000 total
```

## Testing

```
WebbOS Test Suite
├── Memory Management ✅
│   ├── Frame allocator
│   ├── Heap allocation
│   └── Paging
├── Process Management ✅
│   ├── Process creation
│   ├── Thread creation
│   └── Context switching
├── Network Stack ⚠️
│   ├── Socket API ✅
│   ├── TCP/IP ✅
│   └── DNS ✅
├── Cryptography ✅
│   ├── SHA-256/384
│   ├── ChaCha20-Poly1305
│   └── TLS 1.3
├── Virtual Filesystem ⚠️
│   ├── VFS operations ✅
│   ├── EXT2 ✅
│   └── FAT32 ✅
└── Graphics ✅
    ├── VESA driver
    └── Primitives
```

## What's Complete vs Planned

### ✅ Completed
- UEFI Bootloader
- Memory Management
- Process/Thread Management
- Interrupt Handling
- Syscall Interface
- VFS with EXT2/FAT32
- Network Stack (TCP/IP/UDP)
- TLS 1.3
- HTTP/HTTPS Client
- DNS Resolver
- Desktop Environment
- User Management
- VESA Graphics
- PS/2 Input
- Storage Drivers

### ⚠️ Partial
- Browser Engine (parsers complete, rendering integration needed)
- WebAssembly (parser exists, runtime needed)

### ❌ Not Started
- App Store (requirement #4 from urs.md)
- USB Support
- Audio Subsystem
- IPv6
- SMP/Multi-core
- ACPI Power Management

## Architecture Decisions

### Why Rust?
- Memory safety without garbage collection
- Zero-cost abstractions
- Fearless concurrency
- Embedded/bare-metal support

### Why Monolithic Kernel?
- Performance for web browser workloads
- Simpler IPC (direct function calls)
- Easier hardware access

### Why TLS 1.3?
- Modern security standard
- Reduced handshake latency
- No legacy cipher support

### Why HTML/CSS Desktop?
- Consistent with web browser goal
- Easy to style and customize
- Familiar to web developers
- Single rendering engine for browser and desktop

## References

- [UEFI Specification](https://uefi.org/specifications)
- [Intel SDM](https://www.intel.com/content/www/us/en/developer/articles/technical/intel-sdm.html)
- [TLS 1.3 RFC 8446](https://tools.ietf.org/html/rfc8446)
- [ChaCha20-Poly1305 RFC 8439](https://tools.ietf.org/html/rfc8439)
- [X25519 RFC 7748](https://tools.ietf.org/html/rfc7748)
