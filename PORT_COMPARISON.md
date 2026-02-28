# WebbOS Port Comparison

## Overview

| Port | Target | Files | Code Size | Status | Warnings |
|------|--------|-------|-----------|--------|----------|
| **PC** | x86_64 UEFI | 68 | ~1.05 MB | Working | ~474 |
| **Pi** | ARM64 (Pi 3/4) | 88 | ~1.34 MB | Most Complete | ~500 |
| **Pi5** | ARM64 (Pi 5) | 81 | ~1.25 MB | In Progress | ~1067 |

## Architecture Differences

### Directory Structure

**PC Port:**
```
kernel/src/
├── arch/x86_64/          # x86_64 architecture code
├── drivers/
│   ├── input/            # Keyboard/mouse
│   ├── storage/          # Disk/ATA/NVMe
│   ├── usb/              # USB controller detection (xHCI/EHCI)
│   └── vesa/             # VESA framebuffer
├── graphics/             # Graphics context
└── ...
```

**Pi/Pi5 Port:**
```
kernel/src/
├── arch/aarch64/         # ARM64 architecture code
├── drivers/
│   ├── display/          # Pi framebuffer (mailbox)
│   ├── input/            # Keyboard/mouse
│   ├── mailbox/          # VideoCore mailbox
│   ├── sdio/             # SD card I/O
│   ├── storage/          # SD card storage
│   ├── usb/              # USB controller (DWC OTG)
│   └── wifi/             # WiFi (BCM43438/BCM43455)
├── graphics/             # Graphics context
└── ...
```

## Feature Comparison

| Feature | PC | Pi5 | Pi |
|---------|----|-----|-----|
| **Graphics** ||||
| VESA framebuffer | ✅ | - | - |
| Pi mailbox framebuffer | - | ✅ | ✅ |
| Dirty rectangle tracking | ✅ | ✅ | ✅ |
| Double buffering | 🚧 | 🚧 | 🚧 |
| **Storage** ||||
| ATA/NVMe | ✅ | - | - |
| SD card (SDIO) | - | ✅ | ✅ |
| FAT32 write | ✅ | ✅ | ✅ |
| **Network** ||||
| Ethernet (virtio/e1000) | ✅ | - | - |
| DHCP client (advanced) | ✅ | ✅ | ✅ |
| WiFi (WPA2) | - | 🚧 | ✅ |
| **USB** ||||
| USB controller detection | ✅ | - | ✅ |
| USB keyboard/mouse | ✅ | - | ✅ |
| **Input** ||||
| PS/2 keyboard | ✅ | - | - |
| PS/2 mouse | ✅ | - | - |
| USB HID | ✅ | - | ✅ |
| **Process/Scheduler** ||||
| Preemptive scheduler | ✅ | ✅ | ✅ |
| Kernel threads | ✅ | ✅ | ✅ |
| Context switching | ✅ | ✅ | ✅ |
| Sleep/wake queues | ✅ | ✅ | ✅ |
| **Other** ||||
| PCI enumeration | ✅ | - | - |
| Real-time clock | ✅ | - | - |
| **Security** ||||
| TCP ISN (RFC 6528) | ✅ | ✅ | ✅ |
| Filesystem bounds checking | ✅ | ✅ | ✅ |
| PBKDF2 password hashing | ✅ | ✅ | ✅ |
| WPA2 crypto | N/A | 🚧 | ✅ |
| Static mut safety | ✅ | ✅ | ✅ |

Legend: ✅ Working, 🚧 In Progress, ❌ Missing, - Not applicable

## Security Improvements (Completed)

### TCP Sequence Number Generation (All Ports)
- **Status**: ✅ Implemented RFC 6528 compliant ISN generation
- **Details**: Uses hardware entropy (RDTSC on PC, CNTPCT_EL0 on Pi) + timer ticks + secret key
- **Benefit**: Prevents TCP session hijacking attacks

### Filesystem Bounds Checking (All Ports)
- **Status**: ✅ Comprehensive bounds checking on all filesystem operations
- **Details**: 
  - FAT32: Boot sector, FAT table, directory entries validated
  - EXT2: Superblock, group descriptors, inodes validated
  - Initrd: Path traversal protection, offset validation
- **Benefit**: Prevents buffer overflows from malicious disk images

### WPA2 Cryptography (Pi)
- **Status**: ✅ Proper PBKDF2-HMAC-SHA1 implementation
- **Details**:
  - 4096 iterations for PMK derivation
  - PRF-HMAC-SHA1 for PTK derivation
  - HMAC-SHA1 for MIC calculation
  - Hardware entropy for nonce generation
- **Benefit**: Secure WiFi connections

### Password Hashing (All Ports)
- **Status**: ✅ PBKDF2-like construction implemented
- **Details**:
  - 100,000 iterations of SHA-256
  - Per-user salt derived from username
  - Prevents rainbow table attacks
- **Benefit**: Secure password storage

### Static Mutable Safety (All Ports)
- **Status**: ✅ All `static mut` references replaced
- **Details**:
  - PC: `SyncUnsafeCell`, `AtomicU64`, `IrqCell`
  - Pi/Pi5: `AtomicU64`, `AtomicU32`, `Mutex<T>`, `lazy_static!`
- **Benefit**: Eliminates data races and undefined behavior

## Specific Improvements in PC Port

### 1. Process Scheduler
Recently ported from Pi:
- Sleep/wake queue support with tick-based scheduling
- Kernel thread spawning with `spawn_kernel_thread()`
- Idle thread with x86_64 `hlt` instruction
- Proper context switching in `schedule_next()`
- x86_64 interrupt control (`cli`/`sti`)

### 2. Advanced DHCP Client
Ported from Pi:
- UDP socket integration
- Timeout handling with exponential backoff
- Retry logic with configurable limits
- Automatic lease renewal at 50%/87.5% intervals
- State machine: Idle → Selecting → Requesting → Bound → Renewing → Rebinding

### 3. Cryptographic Modules
Added to PC:
- SHA1 implementation (for WPA2 compatibility)
- PBKDF2 key derivation
- All crypto modules now match Pi implementation

### 4. Network Stack Improvements
- Bounds checking throughout packet processing
- Length validation for IP, UDP, TCP headers
- Receive queue limits to prevent memory exhaustion
- Consistent API with alias functions

### 5. USB Support
- USB module with PCI-based controller detection
- Detects xHCI/EHCI/OHCI/UHCI controllers
- Gracefully falls back to PS/2 input
- Prepared structure for future xHCI driver

## Recommendations

### For PC Port
- ✅ **Completed**: Dirty rectangle tracking, browser DOM, filesystem operations, DHCP client
- 🚧 **Next**: Clean remaining warnings (~474), add audio driver

### For Pi Port
- ✅ **Completed**: WiFi WPA2, scheduler, USB, SDIO
- 🚧 **Next**: Test on real hardware, documentation

### For Pi5 Port
- ✅ **Completed**: Security improvements, scheduler, context switching
- 🚧 **Next**: Port WiFi drivers from Pi, USB support

## Code Quality

### Warning Counts
| Port | Before | After | Change |
|------|--------|-------|--------|
| PC | ~1182 | ~474 | -708 ✅ |
| Pi | ~1342 | ~500 | -842 ✅ |
| Pi5 | ~1206 | ~1067 | -139 ✅ |

All ports build successfully with **0 errors**.

## Testing Status

| Feature | PC (QEMU) | Pi 3 | Pi 4 | Pi 5 |
|---------|-----------|------|------|------|
| Boot | ✅ | ? | ? | ? |
| Display | ✅ | ? | ? | ? |
| Keyboard | ✅ | ? | ? | ? |
| Mouse | ✅ | ? | ? | ? |
| Network | ✅ | ✅ | ? | ? |
| WiFi | N/A | ✅ | ? | ? |
| USB | ✅ | ✅ | ? | ? |
| SD Card | ? | ✅ | ? | ? |

*✅ = tested/working, ? = needs testing, N/A = not applicable*

---

**Last Updated:** 2026-02-25
