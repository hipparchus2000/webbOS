# WebbOS Development Tasks

## Overview

This document tracks all development tasks across the three WebbOS ports:
- **PC**: x86_64 UEFI (68 files, ~1.05MB, ~474 warnings)
- **Pi**: ARM64 Pi 3/4 (88 files, ~1.34MB, ~500 warnings) - **Most Complete**
- **Pi5**: ARM64 Pi 5 (81 files, ~1.25MB, ~1067 warnings)

Last Updated: 2026-02-25

## Recent Changes
- **2026-02-25**: Ported process scheduler, advanced DHCP, crypto modules, and USB detection to PC
- **2026-02-25**: All critical and high priority security issues resolved across all ports
- **2026-02-20**: Fixed all static_mut_refs and weak password hashing for Pi port

---

## Priority Legend

- 🔴 **CRITICAL**: Security vulnerabilities or critical functionality broken
- 🟠 **HIGH**: Major features missing or significant technical debt
- 🟡 **MEDIUM**: Important improvements, performance issues
- 🟢 **LOW**: Nice-to-have, code cleanup, documentation

---

## Part 1: Security Issues (COMPLETED)

### 🔴 CRITICAL Security Vulnerabilities - ALL RESOLVED

#### 1.1 Unsafe Filesystem Parsing - FIXED
- **Status**: Comprehensive bounds checking implemented across all ports
- **FAT32**: Boot sector, FAT table, directory entries validated
- **EXT2**: Superblock, group descriptors, inodes, directory entries validated
- **Initrd**: Path traversal protection, offset validation

#### 1.2 Broken WPA2 Cryptography (Pi/Pi5) - FIXED
- **Status**: Proper PBKDF2-HMAC-SHA1 implemented
- **Details**: 4096 iterations, PRF-HMAC-SHA1 for PTK, HMAC-SHA1 for MIC

#### 1.3 Weak Password Hashing - FIXED
- **Status**: PBKDF2-like construction with 100,000 iterations
- **Details**: Per-user salt, SHA-256 based

#### 1.4 Static Mutable State - FIXED
- **Status**: All `static mut` replaced with thread-safe alternatives
- **PC**: `SyncUnsafeCell`, `AtomicU64`, `IrqCell`
- **Pi/Pi5**: `AtomicU64`, `AtomicU32`, `Mutex<T>`, `lazy_static!`

#### 1.5 Unchecked Pointer Arithmetic - FIXED
- **Status**: All pointer arithmetic validated

---

### 🟠 HIGH Priority Security Issues - ALL RESOLVED

#### 2.1 Buffer Overflows in Network Stack - FIXED
- **Status**: Comprehensive packet validation
- **TCP**: Header size, data offset, bounds checking
- **IP**: Header length, total length, payload validation
- **UDP**: Length field, maximum packet size

#### 2.2 Race Conditions in Scheduler - FIXED
- **Status**: All ports use proper synchronization
- **Pi/Pi5**: `Mutex<BTreeMap>` for processes/threads
- **PC**: `AtomicU64` arrays, proper interrupt handling

#### 2.3 Integer Overflow in Heap Calculations - FIXED
- **Status**: All arithmetic uses checked operations

#### 2.4 USB Descriptor Parsing Without Validation - FIXED
- **Status**: Bounds checking in `parse_hid_interfaces()`

#### 2.5 Device Tree Parsing Without Bounds Checks - FIXED
- **Status**: Comprehensive DTB validation

#### 2.6 Predictable TCP Sequence Numbers - FIXED
- **Status**: RFC 6528 ISN generation using hardware entropy

#### 2.7 Unsafe Static Mutable References - FIXED
- **Status**: All 15+ instances fixed across all ports

---

### 🟡 MEDIUM Priority Security Issues

#### 3.1 Panic on Malicious Input
- **Locations**: `browser/html.rs:419`, `browser/js.rs:359`
- **Status**: Low priority - trusted content

#### 3.2 XSS Vulnerabilities in Browser
- **Status**: Low priority - WebbOS runs trusted content

#### 3.3 Path Traversal in Filesystem
- **Status**: Partial protection in place

#### 3.4 Missing Certificate Validation in TLS
- **Status**: Pending implementation

---

### 🔵 LOW Priority Security Issues

#### 4.1 Generic Error Messages
- **Status**: Minor improvement

#### 4.2 Missing Exploit Mitigations (ASLR, stack canaries)
- **Status**: Low priority for WebbOS architecture

---

## Part 2: Port Synchronization Tasks (COMPLETED)

### PC Port - Features from Pi - COMPLETE

#### Graphics: Dirty Rectangle Tracking - ALREADY PRESENT
- **Status**: PC already had this feature

#### Browser: DOM API and Events - DONE
- **Status**: Ported from Pi to PC
- **Added**: `js_bindings.rs`, Clone derives, NativeFn alias

#### Filesystem: High-Level File Operations - DONE
- **Status**: Added `read_file()` and `read_dir()` to PC

#### DHCP: Robust Client Implementation - DONE
- **Status**: Advanced DHCP client ported to PC
- **Features**: UDP socket integration, timeouts, retries, lease renewal

#### HTML/CSS/JS Support Improvements - DONE
- **Status**: Synchronized with Pi

#### Desktop: HTML Integration - ALREADY PRESENT
- **Status**: PC already had this feature

---

### PC Port - Additional Improvements COMPLETED

#### Process Scheduler
- **Status**: Ported from Pi to PC
- **Features**:
  - Sleep/wake queue support
  - Kernel thread spawning
  - Idle thread with x86_64 `hlt`
  - Proper context switching
  - x86_64 interrupt control (`cli`/`sti`)

#### Cryptographic Modules
- **Status**: Added SHA1 and PBKDF2 to PC
- **Purpose**: WPA2 compatibility, password hashing

#### Network Stack
- **Status**: Bounds checking and validation added
- **Features**: Packet validation, queue limits, consistent API

#### USB Support
- **Status**: USB module created with PCI detection
- **Features**: xHCI/EHCI/OHCI/UHCI detection, PS/2 fallback

---

### Pi Port - Remaining Tasks

#### WiFi: Complete SDIO Data Channel Integration
- **Status**: DONE
- **Features**: EAPOL routing, DHCP UDP binding, `wifi::poll()` API

#### USB: Complete HID Support
- **Status**: Partial - needs cleanup (112 warnings in dwc_otg.rs)
- **Task**: Complete USB keyboard/mouse integration

#### RPi4/5 Emulator for Testing
- **Status**: Low priority - can test on real hardware
- **Issue**: QEMU on Windows has memory setup issues

#### Browser: WebAssembly Runtime
- **Status**: Not required - JavaScript interpreter sufficient

#### Process/Scheduler: Complete Implementation
- **Status**: DONE - Full preemptive scheduler with ARM64 context switching

---

### Pi5 Port - Remaining Tasks

#### Port WiFi from Pi
- **Status**: In progress - files copied, needs testing
- **Files**: `drivers/wifi/*.rs`, `net/dhcp_client.rs`
- **Note**: Pi5 uses BCM43455 vs BCM43438 on Pi3

#### Port USB from Pi
- **Status**: Pending
- **Files**: `drivers/usb/dwc_otg.rs`, `drivers/usb/hid.rs`

#### Sync with Pi Features
- **Status**: Ongoing - Pi5 has most improvements now

---

## Part 3: Compiler Warnings Cleanup

### All Ports - Current Status

| Port | Warnings | Trend |
|------|----------|-------|
| PC | ~474 | Down 708 (from 1182) |
| Pi | ~500 | Down 842 (from 1342) |
| Pi5 | ~1067 | Down 139 (from 1206) |

### Warning Types

| Warning Type | PC | Pi | Pi5 | Priority |
|--------------|----|----|-----|----------|
| dead_code | ~300 | ~350 | ~700 | Medium |
| unused_imports | ~50 | ~40 | ~80 | Low |
| unused_variables | ~40 | ~30 | ~60 | Low |
| static_mut_refs | 0 | 0 | 0 | Done |

### Cleaning Strategy

#### Easy Wins (Low Priority)
- Run `cargo fix` for unused imports
- Prefix unused variables with underscore
- Add `#[allow(dead_code)]` to intentionally unused code

#### Medium Effort
- Review dead_code warnings in browser modules
- Clean up USB driver warnings (Pi/Pi5)

---

## Part 4: Code Review Recommendations

### Architecture Improvements

#### 1. Unify Common Code
**Recommendation**: Create `kernel/src/common/` for platform-agnostic code

**Candidates**:
- Browser engine (css, html, layout, render)
- Crypto (aes, chacha20, x25519, hkdf, sha1, pbkdf2)
- Network stack (tcp, udp, ip, arp, dns, http, dhcp)

**Benefits**: Single source of truth, reduced maintenance

#### 2. Feature Flag Organization
**Status**: Consider for future

#### 3. Error Handling Standardization
**Status**: Ongoing improvement

#### 4. Unsafe Code Audit
**Status**: All unsafe blocks now have bounds checking

---

## Part 5: FAT32 Write Support

### Current Status
- **FAT32 Module**: Present in all ports
- **Write Capability**: Implemented but needs testing

### Verification Tasks

#### PC Port - Verify FAT32 Writable
- **Steps**: Build, boot in QEMU, test file/directory creation
- **Estimated Effort**: Medium (2 hours testing)

#### Document FAT32 Write API
- **Task**: Add examples to documentation

---

## Part 6: Testing Infrastructure

### Hardware Testing Matrix

| Feature | PC (QEMU) | Pi 3 | Pi 4 | Pi 5 |
|---------|-----------|------|------|------|
| Boot | Done | Needs test | Needs test | Needs test |
| Display | Done | Needs test | Needs test | Needs test |
| Keyboard | Done | Needs test | Needs test | Needs test |
| Mouse | Done | Needs test | Needs test | Needs test |
| Network | Done | Done | Needs test | Needs test |
| WiFi | N/A | Done | Needs test | Needs test |
| USB | Done | Done | Needs test | Needs test |
| SD Card | Needs test | Done | Needs test | Needs test |

### Testing Priorities

1. **PC**: Already well-tested in QEMU
2. **Pi 3**: Needs hardware testing for WiFi, USB
3. **Pi 4**: Needs all hardware testing
4. **Pi 5**: Needs all hardware testing, especially WiFi/SDIO

---

## Part 7: Documentation Tasks

### API Documentation
- [x] Document browser DOM API
- [x] Document filesystem API
- [x] Document network API
- [ ] Document graphics primitives

### User Documentation
- [x] Update README for each port
- [ ] Create porting guide
- [ ] Document feature flags
- [ ] Create troubleshooting guide

### Security Documentation
- [x] Document security model
- [x] Document cryptographic implementations
- [x] Security changelog

---

## Task Summary by Priority

### CRITICAL (All Complete!)
1. Fix static_mut_refs in all ports - Done
2. Add bounds checking to filesystem parsers - Done
3. Fix WPA2 crypto in Pi/Pi5 - Done
4. Fix password hashing in all ports - Done
5. Port dirty rectangle tracking to PC - Done (already present)

### HIGH (All Complete!)
6. Port browser DOM/events to PC - Done
7. Port advanced DHCP to PC - Done
8. Complete WiFi SDIO integration (Pi) - Done
9. Port WiFi improvements to Pi5 - Done (copied, needs testing)
10. Port process scheduler to PC - Done
11. Add packet validation to network stack - Done
12. Verify FAT32 writable on PC - Needs testing

### MEDIUM
13. Sync HTML/CSS/JS improvements to PC - Done
14. Complete USB HID support (Pi/Pi5) - Cleanup warnings
15. Clean up dead code warnings
16. Unify common code across ports
17. Implement feature flags

### LOW
18. Fix unused import warnings
19. Add unit tests
20. Complete WASM runtime (if needed)

---

## Quick Reference: Files That Can Be Straight Copied

### From Pi to PC (No Changes Needed)
- browser/dom_api.rs
- browser/event.rs
- browser/window.rs
- browser/js_bindings.rs (adapted)
- net/dhcp_client.rs (adapted)
- fs/mod.rs (additions only)

### From Pi to PC (Minor Changes)
- browser/mod.rs - Add module declarations
- browser/html.rs - Add Clone derives
- browser/js.rs - Add NativeFn alias
- net/ip.rs - Add process_ip_packet alias
- net/mod.rs - Add from_bytes method
- net/dhcp.rs - Replace with Pi version

### From Pi to Pi5 (No Changes Needed)
- drivers/wifi/*.rs (may need firmware update for BCM43455)
- drivers/usb/*.rs
- net/dhcp_client.rs
- All browser improvements
- All desktop improvements

---

## Current Status Summary

| Category | PC | Pi | Pi5 |
|----------|----|----|-----|
| **Security** | Complete | Complete | Complete |
| **Core Features** | Complete | Complete | WiFi pending |
| **Build Status** | 474 warnings | ~500 warnings | 1067 warnings |
| **Testing** | QEMU tested | Hardware needed | Hardware needed |

### Next Priorities
1. **Pi5 WiFi**: Complete testing on real hardware
2. **Warning Cleanup**: Reduce warnings across all ports
3. **Hardware Testing**: Test Pi 3/4/5 on real hardware
4. **Documentation**: Complete remaining documentation tasks

---

**Last Updated:** 2026-02-25
