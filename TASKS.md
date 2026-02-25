# WebbOS Development Tasks

## Overview

This document tracks all development tasks across the three WebbOS ports:
- **PC**: x86_64 UEFI (68 files, ~1.05MB, 666 warnings)
- **Pi**: ARM64 Pi 3/4 (88 files, ~1.34MB, 1342 warnings) - **Most Complete**
- **Pi5**: ARM64 Pi 5 (81 files, ~1.25MB, 1206 warnings)

Last Updated: 2026-02-20

## Recent Changes
- **2026-02-20**: Fixed all static_mut_refs and weak password hashing for Pi port (9 critical security issues resolved)

---

## Priority Legend

- 🔴 **CRITICAL**: Security vulnerabilities or critical functionality broken
- 🟠 **HIGH**: Major features missing or significant technical debt
- 🟡 **MEDIUM**: Important improvements, performance issues
- 🟢 **LOW**: Nice-to-have, code cleanup, documentation

---

## Part 1: Security Issues (CRITICAL)

### 🔴 CRITICAL Security Vulnerabilities

#### 1.1 Unsafe Filesystem Parsing
- **Location**: `fs/fat32/mod.rs:130`, `fs/ext2/mod.rs:155`
- **Issue**: FAT32 and EXT2 parsers use `unsafe` pointer casts on untrusted disk data without validation
- **Risk**: Buffer overflow, arbitrary code execution from malicious disk images
- **Affected Ports**: ALL (PC, Pi, Pi5)
- **Task**: Add bounds checking to all filesystem `unsafe` blocks before parsing

#### 1.2 Broken WPA2 Cryptography (Pi/Pi5)
- **Location**: `drivers/wifi/wpa2.rs:66-81`
- **Issue**: Uses custom XOR-based "crypto" instead of proper PBKDF2
- **Risk**: WiFi passwords easily crackable
- **Affected Ports**: Pi, Pi5
- **Task**: Implement proper PBKDF2-SHA1 for WPA2 key derivation

#### ~~1.3 Weak Password Hashing~~ ✅ FIXED for Pi
- **Location**: `users/mod.rs:304-310`
- **Issue**: SHA-256 with static salt instead of PBKDF2/Argon2
- **Risk**: Passwords vulnerable to rainbow table attacks
- **Affected Ports**: ALL (Fixed for Pi, pending PC/Pi5)
- **Task**: ~~Replace with PBKDF2 or Argon2 with per-user salt~~
- **Fix Applied**: Implemented PBKDF2-like construction with 100,000 iterations of SHA-256, per-user salt derived from username

#### ~~1.4 Static Mutable State~~ ✅ FIXED for Pi
- **Location**: 
  - ~~`net/dhcp.rs:63-64`~~
  - ~~`net/ip.rs:349`~~
  - ~~`drivers/timer.rs:205`~~
  - ~~`desktop/mod.rs:585,592,698`~~
  - ~~`arch/exceptions.rs:120,186`~~
  - ~~`browser/js_bindings.rs:19,32`~~
  - ~~`process/scheduler.rs:16`~~
  - ~~`mm/mod.rs:25`~~
  - ~~`drivers/sdio/mod.rs:701`~~
  - ~~`drivers/wifi/bcm43438.rs:1072`~~
  - ~~`drivers/wifi/sdio_spi.rs:638`~~
  - ~~`drivers/usb/dwc_otg.rs:29`~~
  - ~~`bootloader/src/main.rs:39`~~
- **Issue**: Multiple `static mut` variables without synchronization
- **Risk**: Data races, undefined behavior
- **Affected Ports**: ALL (Fixed for Pi, pending PC/Pi5)
- **Task**: ~~Replace with `AtomicU32`, `Mutex<T>`, or thread-safe alternatives~~
- **Fix Applied**: 
  - Replaced all `static mut` with `AtomicU64`, `AtomicU32`, `AtomicU16`, `AtomicUsize`
  - Used `Mutex<T>` for complex types
  - Used `UnsafeCell` with `Sync` impl for bootloader allocator
  - Used `lazy_static!` with `Mutex` for optional singletons

#### 1.5 Unchecked Pointer Arithmetic
- **Location**: `storage/nvme.rs:148-150`
- **Issue**: Unchecked `add()` operations on MMIO pointers
- **Risk**: Memory corruption, system crash
- **Affected Ports**: PC
- **Task**: Add bounds validation before pointer arithmetic

---

### 🟠 HIGH Priority Security Issues

#### 2.1 Buffer Overflows in Network Stack
- **Location**: `net/tcp.rs:39-55`
- **Issue**: Packet processing without length validation
- **Task**: Add packet length checks to all network handlers

#### 2.2 Race Conditions in Scheduler
- **Location**: `process/mod.rs`, `arch/interrupts.rs`
- **Issue**: Interrupt handlers and scheduler not properly synchronized
- **Task**: Audit all concurrent access patterns

#### 2.3 Integer Overflow in Heap Calculations
- **Location**: `mm/allocator.rs:25-27`
- **Issue**: Heap size calculations can overflow
- **Task**: Use `checked_add`, `saturating_mul` for size calculations

#### 2.4 USB Descriptor Parsing Without Validation (Pi/Pi5)
- **Location**: `drivers/usb/dwc_otg.rs`, `drivers/usb/hid.rs`
- **Issue**: USB descriptors parsed without bounds checking
- **Affected Ports**: Pi, Pi5
- **Task**: Validate all descriptor lengths before parsing

#### 2.5 Device Tree Parsing Without Bounds Checks
- **Location**: `bootloader/src/dtb.rs:65`
- **Issue**: DTB parsing trusts input data
- **Affected Ports**: Pi, Pi5
- **Task**: Add bounds checks to DTB parser

#### 2.6 Predictable TCP Sequence Numbers
- **Location**: `net/tcp.rs`
- **Issue**: TCP ISN generation is predictable
- **Risk**: TCP session hijacking
- **Task**: Implement RFC 6528 compliant ISN generation

#### ~~2.7 Unsafe Static Mutable References~~ ✅ FIXED for Pi
- **Count**: ~~15 instances across all ports~~ 0 remaining in Pi
- **Locations** (Fixed in Pi):
  - ~~`bootloader/src/main.rs:77`~~ - Used `UnsafeCell` wrapper
  - ~~`kernel/src/mm/mod.rs`~~ - Used `lazy_static!` with `Mutex`
  - ~~`kernel/src/arch/exceptions.rs`~~ - Used `AtomicU64`, `SyncUnsafeCell`
  - ~~`kernel/src/browser/js_bindings.rs`~~ - Used `AtomicUsize`, `AtomicU32`
  - ~~`kernel/src/desktop/mod.rs`~~ - Used `lazy_static!` with `Mutex`
  - ~~`kernel/src/net/ip.rs`~~ - Used `AtomicU16`
  - ~~`kernel/src/process/scheduler.rs`~~ - Used `AtomicU64` array
  - ~~`kernel/src/drivers/timer.rs`~~ - Used `AtomicU64`
  - ~~`kernel/src/drivers/sdio/mod.rs`~~ - Used `lazy_static!` with `Mutex`
  - ~~`kernel/src/drivers/wifi/bcm43438.rs`~~ - Used `lazy_static!` with `Mutex`
  - ~~`kernel/src/drivers/wifi/sdio_spi.rs`~~ - Used `lazy_static!` with `Mutex`
  - ~~`kernel/src/drivers/usb/dwc_otg.rs`~~ - Used `AtomicU64`
- **Issue**: Creating references to mutable statics is UB
- **Task**: ~~Use raw pointers or proper synchronization~~
- **Status**: ✅ All 12 static mut instances fixed in Pi port

---

### 🟡 MEDIUM Priority Security Issues

#### 3.1 Panic on Malicious Input
- **Locations**:
  - `browser/html.rs:419`
  - `browser/js.rs:359`
- **Issue**: `unwrap()` calls can panic on malformed HTML/JS
- **Task**: Replace `unwrap()` with proper error handling

#### 3.2 XSS Vulnerabilities in Browser
- **Location**: `browser/html.rs`, `browser/js.rs`
- **Issue**: No HTML sanitization for user content
- **Task**: Implement HTML entity encoding

#### 3.3 Path Traversal in Filesystem
- **Location**: `fs/mod.rs`, `fs/fat32/mod.rs`
- **Issue**: `../` sequences not sanitized
- **Task**: Normalize paths before operations

#### 3.4 Missing Certificate Validation in TLS
- **Location**: `tls/mod.rs`
- **Issue**: Certificate chain not properly validated
- **Task**: Implement proper X.509 validation

---

### 🔵 LOW Priority Security Issues

#### 4.1 Generic Error Messages
- **Issue**: `expect()` messages don't aid debugging
- **Task**: Improve error messages

#### 4.2 Missing Exploit Mitigations
- **Issue**: No ASLR, NX bit, or stack canaries
- **Task**: Implement basic exploit mitigations

#### 4.3 No Kernel Module Signing
- **Issue**: No verification of loaded modules
- **Task**: Add module signature verification

---

## Part 2: Port Synchronization Tasks

### PC Port - Missing Features from Pi

#### 🔴 Graphics: Dirty Rectangle Tracking
- **Status**: Pi has it, PC missing
- **Impact**: Full screen redraw every frame causes flickering, poor Paint app performance
- **Files to Port**:
  - `Pi/kernel/src/desktop/ui.rs` → `PC/kernel/src/desktop/ui.rs`
    - `DirtyRect` struct
    - `mark_dirty()`, `mark_mouse_dirty()`, `mark_full_redraw()` methods
    - Modified `draw()` with partial redraw support
    - `draw_region()` for selective updates
- **Estimated Effort**: Medium (2-3 hours)
- **Can Copy**: Partial - needs adaptation for VesaDriver vs PiFramebuffer

#### 🔴 Browser: DOM API and Events
- **Status**: Pi has it, PC missing
- **Impact**: HTML5 apps can't interact with JavaScript properly
- **Files to Copy** (straight copy, no changes needed):
  - `Pi/kernel/src/browser/dom_api.rs` → `PC/kernel/src/browser/`
  - `Pi/kernel/src/browser/event.rs` → `PC/kernel/src/browser/`
  - `Pi/kernel/src/browser/window.rs` → `PC/kernel/src/browser/`
  - `Pi/kernel/src/browser/js_bindings.rs` → `PC/kernel/src/browser/`
- **Files to Modify**:
  - `PC/kernel/src/browser/mod.rs` - Add module declarations
  - `PC/kernel/src/browser/html.rs` - Add `#[derive(Clone)]` to Element/Node
  - `PC/kernel/src/browser/js.rs` - Add `NativeFn` type alias
- **Estimated Effort**: Low (1 hour for copy + integration)

#### 🟠 Filesystem: High-Level File Operations
- **Status**: Pi has `read_file()` and `read_dir()`, PC missing
- **Impact**: Apps can't easily read files or list directories
- **Files to Update**:
  - `PC/kernel/src/fs/mod.rs` - Add functions (copy from Pi)
- **Can Copy**: Yes, straight copy of functions
- **Estimated Effort**: Low (30 minutes)

#### 🟠 DHCP: Robust Client Implementation
- **Status**: Pi has advanced DHCP client with renewal, PC has basic version
- **Impact**: PC DHCP doesn't handle timeouts, retries, or lease renewal
- **Files**:
  - `Pi/kernel/src/net/dhcp_client.rs` → `PC/kernel/src/net/`
  - `Pi/kernel/src/net/dhcp.rs` - Replace PC version with Pi version
- **Can Copy**: Mostly, but check network driver integration
- **Estimated Effort**: Medium (2 hours testing)

#### 🟡 HTML/CSS/JS Support Improvements
- **Status**: Pi has better HTML5 support
- **Files to Sync**:
  - `PC/kernel/src/browser/html.rs` - Add `Clone` derives
  - `PC/kernel/src/browser/js.rs` - Add `NativeFn` type alias
  - `PC/kernel/src/net/ip.rs` - Add `process_ip_packet` alias
  - `PC/kernel/src/net/mod.rs` - Add `from_bytes()` to `Ipv4Address`
- **Can Copy**: Yes, all simple additions
- **Estimated Effort**: Low (1 hour)

#### 🟢 Desktop: HTML Integration
- **Status**: Pi has `launch_html()`, message passing, file manager integration
- **Impact**: PC desktop can't launch HTML apps properly
- **Files**:
  - `Pi/kernel/src/desktop/mod.rs` - Copy HTML-related functions (behind feature flag)
- **Can Copy**: Partial - feature-gate HTML integration
- **Estimated Effort**: Medium (2 hours with testing)

---

### Pi Port - Remaining Tasks

#### 🟠 WiFi: Complete SDIO Data Channel Integration
- **Status**: WPA2 and DHCP implemented, SDIO integration incomplete
- **Issues**:
  - EAPOL frames need proper routing through SDIO function 2
  - DHCP client needs UDP socket binding verification
- **Files**:
  - `kernel/src/drivers/wifi/bcm43438.rs`
  - `kernel/src/drivers/wifi/eapol.rs`
  - `kernel/src/drivers/wifi/sdio_spi.rs`
- **Estimated Effort**: High (1-2 days)

#### 🟡 USB: Complete HID Support
- **Status**: USB host controller stubbed, HID partially implemented
- **Files**: `drivers/usb/dwc_otg.rs` (112 warnings - needs cleanup)
- **Task**: Complete USB keyboard/mouse integration
- **Estimated Effort**: Medium (1 day)

#### 🟡 Browser: WebAssembly Runtime
- **Status**: Parser complete, execution stubbed
- **File**: `browser/wasm.rs` (~35 warnings)
- **Task**: Implement WASM interpreter or JIT
- **Estimated Effort**: High (weeks)

#### 🟢 Process/Scheduler: Complete Implementation
- **Status**: Infrastructure exists but not integrated
- **Files**: `process/mod.rs` (~50 warnings)
- **Task**: Wire up process creation, scheduling, termination
- **Estimated Effort**: High (weeks)

---

### Pi5 Port - Remaining Tasks

#### 🔴 Port WiFi from Pi
- **Status**: Pi5 missing WiFi drivers that Pi has
- **Files to Copy from Pi**:
  - `drivers/wifi/bcm43438.rs`
  - `drivers/wifi/sdio_spi.rs`
  - `drivers/wifi/sdpcm.rs`
  - `drivers/wifi/ioctl.rs`
  - `drivers/wifi/eapol.rs`
  - `drivers/wifi/wpa2.rs`
  - `net/dhcp_client.rs`
- **Note**: Pi5 uses BCM43455 (vs BCM43438 on Pi3), may need firmware changes
- **Estimated Effort**: High (2-3 days)

#### 🟠 Port USB from Pi
- **Status**: Pi5 missing USB support that Pi has
- **Files to Copy**:
  - `drivers/usb/dwc_otg.rs`
  - `drivers/usb/hid.rs`
- **Estimated Effort**: Medium (1-2 days)

#### 🟡 Sync with Pi Features
- **Status**: Pi5 lags behind Pi in feature completeness
- **Task**: Regular sync of new Pi features to Pi5
- **Files**: All browser, desktop, fs improvements
- **Estimated Effort**: Ongoing

---

## Part 3: Compiler Warnings Cleanup

### All Ports - Common Warning Types

| Warning Type | PC Count | Pi Count | Pi5 Count | Priority |
|--------------|----------|----------|-----------|----------|
| **dead_code** | 570 | 1142 | 900 | 🟡 |
| **unused_imports** | 44 | 57 | 50 | 🟢 |
| **unused_variables** | 26 | 37 | 40 | 🟢 |
| **unused_mut** | 6 | 6 | 10 | 🟢 |
| **unused_doc_comments** | 15 | 18 | 15 | 🟢 |
| **static_mut_refs** | ~10 | 8 | 15 | 🔴 |

### Top Files by Warning Count

#### PC Port (666 total warnings)
1. `browser/layout.rs` - 25 warnings
2. `drivers/pci.rs` - 24 warnings
3. `browser/wasm.rs` - 24 warnings
4. `drivers/storage/ahci.rs` - 22 warnings
5. `graphics/mod.rs` - 22 warnings
6. `browser/js.rs` - 22 warnings
7. `drivers/vesa/mod.rs` - 22 warnings
8. `crypto/hkdf/mod.rs` - 21 warnings
9. `browser/css.rs` - 19 warnings
10. `net/dhcp.rs` - 18 warnings

#### Pi Port (1342 total warnings)
1. `drivers/usb/dwc_otg.rs` - 112 warnings
2. `drivers/wifi/sdio_spi.rs` - 75 warnings
3. `drivers/sdio/mod.rs` - 72 warnings
4. `drivers/mailbox/mod.rs` - 71 warnings
5. `drivers/wifi/ioctl.rs` - 64 warnings
6. `drivers/usb/hid.rs` - 50 warnings
7. `drivers/wifi/bcm43438.rs` - 45 warnings
8. `drivers/wifi/sdpcm.rs` - 38 warnings
9. `browser/wasm.rs` - 31 warnings
10. `browser/dom_api.rs` - 30 warnings

#### Pi5 Port (1206 total warnings)
Similar to Pi but slightly fewer due to some missing drivers.

### Warning Cleanup Tasks

#### 🔴 Fix static_mut_refs (Safety Issue)
- **Count**: 8-15 instances per port
- **Task**: Replace with AtomicU32 or Mutex
- **Estimated Effort**: Medium (4 hours per port)

#### 🟠 Address dead_code in Core Modules
- **Priority files**:
  - `browser/*.rs` - many stubbed features
  - `net/*.rs` - unused protocol handlers
  - `process/mod.rs` - unimplemented scheduler
- **Strategy**: Either implement features or remove dead code
- **Estimated Effort**: High (ongoing)

#### 🟢 Clean Up Unused Imports
- **Task**: Run `cargo fix --bin "kernel"` to auto-fix
- **Estimated Effort**: Low (30 minutes per port)

#### 🟢 Fix Non-Camel Case Types
- **Location**: `arch/exceptions.rs`
- **Issue**: `MCRMRC_CP15`, `MCRRMRRC_CP15`, etc.
- **Task**: Rename to `McrmrcCp15`, `McrrmrrcCp15`
- **Estimated Effort**: Low (30 minutes)

#### 🟢 Remove Unused Doc Comments
- **Issue**: Doc comments on macro invocations don't generate docs
- **Files**: Network modules, desktop modules
- **Task**: Remove or move doc comments
- **Estimated Effort**: Low (1 hour)

---

## Part 4: Code Review Recommendations

### Architecture Improvements

#### 1. Unify Common Code
**Recommendation**: Create `kernel/src/common/` for platform-agnostic code

**Files to Move**:
- Browser engine (css, html, layout, render, wasm)
- Network stack (tcp, udp, ip, arp, dns, http)
- Crypto (aes, chacha20, x25519, hkdf)
- Graphics traits and algorithms

**Benefits**:
- Single source of truth
- Reduced maintenance
- Consistent behavior across ports

#### 2. Feature Flag Organization
**Recommendation**: Use Cargo features for optional functionality

```toml
[features]
default = []
# Browser features
advanced_browser = ["dom_api", "event_system", "js_bindings"]
dom_api = []
event_system = []
js_bindings = []
# Network features
dhcp_client = []
wifi = ["wpa2"]
wpa2 = []
# Graphics features
dirty_rect = []
# Desktop features
html_ui = []
```

#### 3. Error Handling Standardization
**Recommendation**: Replace all `unwrap()` and `expect()` with proper error handling

**Current State**:
- `browser/html.rs` - uses unwrap for parsing
- `browser/js.rs` - uses unwrap for execution
- `fs/*.rs` - mixed error handling

**Target**: All functions return `Result<T, E>`

#### 4. Unsafe Code Audit
**Recommendation**: Minimize and audit all `unsafe` blocks

**Current Unsafe Usage**:
- Memory allocation
- Hardware register access
- Filesystem parsing
- Network packet processing

**Action**: Document safety invariants for each unsafe block

---

## Part 5: FAT32 Write Support Verification

### Current Status
- **FAT32 Module**: Present in all ports, identical files
- **Write Capability**: Implemented but untested

### Verification Tasks

#### 🟡 PC Port - Verify FAT32 Writable
- **Steps**:
  1. Build PC version with disk image
  2. Boot in QEMU
  3. Test file creation: `echo test > test.txt`
  4. Test directory creation: `mkdir testdir`
  5. Verify changes persist across reboots
- **Files to Check**:
  - `fs/fat32/mod.rs` - `write()`, `create_file()`, `create_dir()`
  - `fs/mod.rs` - `write_file()` wrapper
- **Estimated Effort**: Medium (2 hours testing)

#### 🟢 Document FAT32 Write API
- **Task**: Add examples to documentation
- **Examples needed**:
  - Writing a file
  - Creating a directory
  - Appending to a file
  - Deleting files

---

## Part 6: Testing Infrastructure

### Unit Tests
- **Status**: Minimal test coverage
- **Task**: Add unit tests for:
  - HTML parser
  - CSS parser
  - JavaScript interpreter
  - Network protocols
  - Cryptographic functions

### Integration Tests
- **Task**: Create test suite for:
  - Filesystem operations
  - Network stack
  - Browser rendering
  - Desktop interactions

### Hardware Testing Matrix

| Feature | PC (QEMU) | Pi 3 | Pi 4 | Pi 5 |
|---------|-----------|------|------|------|
| Boot | ✓ | ? | ? | ? |
| Display | ✓ | ? | ? | ? |
| Keyboard | ✓ | ? | ? | ? |
| Mouse | ✓ | ? | ? | ? |
| Network | ? | ? | ? | ? |
| WiFi | N/A | ? | ? | ? |
| USB | ? | ? | ? | ? |
| SD Card | ? | ? | ? | ? |

*✓ = tested, ? = needs testing, N/A = not applicable*

---

## Part 7: Documentation Tasks

### 🟢 API Documentation
- [ ] Document browser DOM API
- [ ] Document filesystem API
- [ ] Document network API
- [ ] Document graphics primitives

### 🟢 User Documentation
- [ ] Update README for each port
- [ ] Create porting guide
- [ ] Document feature flags
- [ ] Create troubleshooting guide

### 🟢 Security Documentation
- [ ] Document security model
- [ ] Document cryptographic implementations
- [ ] Security changelog

---

## Task Summary by Priority

### 🔴 CRITICAL (Do First)
1. Fix static_mut_refs in all ports (safety)
2. Add bounds checking to filesystem parsers (security)
3. Fix WPA2 crypto in Pi/Pi5 (security)
4. Fix password hashing in all ports (security)
5. Port dirty rectangle tracking to PC (usability)

### 🟠 HIGH (Do Soon)
6. Port browser DOM/events to PC
7. Port advanced DHCP to PC
8. Complete WiFi SDIO integration (Pi)
9. Port WiFi to Pi5
10. Replace unwrap() with error handling
11. Add packet validation to network stack
12. Verify FAT32 writable on PC

### 🟡 MEDIUM (Do When Possible)
13. Sync HTML/CSS/JS improvements to PC
14. Complete USB HID support (Pi/Pi5)
15. Clean up dead code warnings
16. Unify common code across ports
17. Implement feature flags
18. Add bounds checking to USB/DTB parsing

### 🟢 LOW (Backlog)
19. Fix unused import warnings
20. Fix camelCase warnings
21. Remove unused doc comments
22. Add unit tests
23. Write API documentation
24. Complete WASM runtime
25. Complete process scheduler

---

## Quick Reference: Files That Can Be Straight Copied

### From Pi to PC (No Changes Needed)
```
browser/dom_api.rs
browser/event.rs
browser/window.rs
browser/js_bindings.rs
net/dhcp_client.rs
fs/mod.rs (additions only)
desktop/ui.rs (dirty rect additions)
```

### From Pi to PC (Minor Changes)
```
browser/mod.rs - Add module declarations
browser/html.rs - Add Clone derives
browser/js.rs - Add NativeFn alias
net/ip.rs - Add process_ip_packet alias
net/mod.rs - Add from_bytes method
net/dhcp.rs - Replace with Pi version
```

### From Pi to Pi5 (No Changes Needed)
```
drivers/wifi/*.rs (may need firmware update for BCM43455)
drivers/usb/*.rs
net/dhcp_client.rs
All browser improvements
All desktop improvements
```

---

## Notes

- **Estimated Total Effort**: 4-6 weeks for critical + high priority tasks
- **Parallel Work**: PC port improvements and Pi WiFi completion can happen simultaneously
- **Dependencies**: Some tasks depend on others (e.g., browser DOM depends on browser modules)
- **Testing**: All changes need testing on actual hardware (Pi 3/4/5) and QEMU (PC)
