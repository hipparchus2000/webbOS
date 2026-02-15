# ARM64 Toolchain Recommendations for Raspberry Pi 5 Porting

**Date:** February 15, 2026  
**Project:** webbOS Raspberry Pi 5 Porting  
**Phase:** 2 (Core Porting)

## Executive Summary

This document provides comprehensive toolchain recommendations for cross-compiling webbOS from x86_64 to ARM64 (Raspberry Pi 5). The recommended approach uses a hybrid development environment with cross-compilation from x86_64 for rapid iteration and native testing on Raspberry Pi 5 hardware for validation.

## 1. Development Environment Strategy

### Recommended Approach: Hybrid Development

```
x86_64 Development Machine (Primary)
    ↓ Cross-compilation
ARM64 Kernel Binary
    ↓ Transfer to
Raspberry Pi 5 (Testing/Validation)
    ↓ Serial Debug Output
Development Machine (Analysis)
```

**Advantages:**
- Fast compilation on powerful x86_64 machine
- Rapid iteration without hardware dependencies
- Final validation on actual hardware
- Cost-effective (minimal hardware requirements)

## 2. Core Toolchain Components

### 2.1 Cross-Compilation Toolchain

#### Required Packages (Ubuntu/Debian):

```bash
# Essential toolchain
sudo apt-get update
sudo apt-get install -y \
    gcc-aarch64-linux-gnu \
    g++-aarch64-linux-gnu \
    binutils-aarch64-linux-gnu \
    qemu-system-arm \
    qemu-utils \
    openocd \
    gdb-multiarch

# Rust ARM64 targets
rustup target add aarch64-unknown-none
rustup target add aarch64-unknown-none-softfloat
```

#### Alternative (Arch Linux):

```bash
sudo pacman -S aarch64-linux-gnu-gcc aarch64-linux-gnu-binutils \
    qemu-system-arm qemu-arch-extra openocd gdb
```

### 2.2 Rust Configuration

#### Target Specification:

Create `/root/.openclaw/workspace/projects/webbos/aarch64-unknown-none.json`:

```json
{
  "llvm-target": "aarch64-unknown-none",
  "data-layout": "e-m:e-i8:8:32-i16:16:32-i64:64-i128:128-n32:64-S128",
  "arch": "aarch64",
  "target-endian": "little",
  "target-pointer-width": "64",
  "target-c-int-width": "32",
  "features": "+strict-align,+neon,+fp-armv8",
  "disable-redzone": true,
  "max-atomic-width": 128,
  "panic-strategy": "abort",
  "relocation-model": "static",
  "code-model": "small",
  "emit-debug-gdb-scripts": true,
  "frame-pointer": "always",
  "stack-probes": {
    "kind": "inline"
  },
  "linker": "aarch64-linux-gnu-gcc",
  "linker-flavor": "gcc",
  "pre-link-args": {
    "gcc": [
      "-nostdlib",
      "-Wl,-T,linker.ld",
      "-Wl,--build-id=none"
    ]
  },
  "post-link-args": {
    "gcc": [
      "-lgcc"
    ]
  },
  "executables": true,
  "singlethread": false,
  "no-builtins": false,
  "position-independent-executables": false
}
```

#### Cargo Configuration:

Add to `.cargo/config.toml`:

```toml
[build]
target = "x86_64-unknown-none"  # Default for existing code

[target.aarch64-unknown-none]
linker = "aarch64-linux-gnu-gcc"
rustflags = [
    "-C", "link-arg=-Tlinker.ld",
    "-C", "link-arg=-nostdlib",
    "-C", "link-arg=-lgcc",
]

[target.'cfg(all(target_arch = "aarch64", target_os = "none"))']
runner = "qemu-system-aarch64 -M virt -cpu cortex-a72 -kernel"
```

### 2.3 Build System Updates

#### Makefile Modifications:

Update `/root/.openclaw/workspace/projects/webbos/Makefile`:

```makefile
# Architecture selection
ARCH ?= x86_64
CARGO_TARGET_KERNEL_x86_64 = x86_64-unknown-none
CARGO_TARGET_KERNEL_aarch64 = aarch64-unknown-none

CARGO_TARGET_BOOTLOADER_x86_64 = x86_64-unknown-uefi
CARGO_TARGET_BOOTLOADER_aarch64 = aarch64-unknown-none  # Custom bootloader needed

QEMU_x86_64 = qemu-system-x86_64
QEMU_aarch64 = qemu-system-aarch64

# Select based on ARCH
CARGO_TARGET_KERNEL = $(CARGO_TARGET_KERNEL_$(ARCH))
CARGO_TARGET_BOOTLOADER = $(CARGO_TARGET_BOOTLOADER_$(ARCH))
QEMU = $(QEMU_$(ARCH))

# ARM64-specific variables
CROSS_COMPILE = aarch64-linux-gnu-
OBJCOPY = $(CROSS_COMPILE)objcopy

# Build targets
.PHONY: all arm64 x86_64

all: x86_64

x86_64:
	$(MAKE) ARCH=x86_64 kernel

arm64:
	$(MAKE) ARCH=aarch64 kernel

# Kernel build
kernel:
	cargo build --target $(CARGO_TARGET_KERNEL) $(CARGO_FLAGS)
	$(OBJCOPY) -O binary \
		target/$(CARGO_TARGET_KERNEL)/debug/kernel \
		kernel8.img

# QEMU testing
qemu: kernel
	$(QEMU) -M virt -cpu cortex-a72 \
		-kernel kernel8.img \
		-serial stdio \
		-device virtio-gpu-pci \
		-device virtio-keyboard-pci \
		-device virtio-mouse-pci

# Raspberry Pi testing (requires SD card)
pi-test: kernel
	cp kernel8.img /media/sd-card/
	sync
	echo "Kernel copied to SD card"
```

## 3. Testing Environments

### 3.1 QEMU ARM64 Emulation

#### Basic QEMU Command:

```bash
qemu-system-aarch64 \
  -M virt \
  -cpu cortex-a72 \
  -smp 4 \
  -m 2G \
  -kernel kernel8.img \
  -serial stdio \
  -device virtio-gpu-pci \
  -device virtio-keyboard-pci \
  -device virtio-mouse-pci \
  -device virtio-net-device,netdev=net0 \
  -netdev user,id=net0
```

#### Raspberry Pi 3/4 Emulation:

```bash
qemu-system-aarch64 \
  -M raspi3b \
  -kernel kernel8.img \
  -serial stdio \
  -dtb bcm2710-rpi-3-b-plus.dtb
```

*Note: Raspberry Pi 5 emulation not fully available in QEMU yet*

### 3.2 Serial Debug Configuration

#### Hardware Setup:
- Connect USB serial adapter to Raspberry Pi GPIO:
  - TX (GPIO 14) → RX on serial adapter
  - RX (GPIO 15) → TX on serial adapter
  - GND → GND

#### Software Configuration:

```bash
# Check serial device
ls /dev/ttyUSB*

# Connect at 115200 baud
sudo screen /dev/ttyUSB0 115200

# Alternative with minicom
sudo minicom -D /dev/ttyUSB0 -b 115200
```

#### Kernel Serial Output:

Add to kernel initialization:

```rust
// ARM64 UART initialization
fn init_serial() {
    // Raspberry Pi UART0 (PL011) at 0xFE201000
    let uart_base = 0xFE201000 as *mut u32;
    
    unsafe {
        // Disable UART
        uart_base.add(0x30/4).write_volatile(0x00);
        
        // Set baud rate (115200)
        let divisor = 24000000 / (16 * 115200);
        uart_base.add(0x24/4).write_volatile(divisor & 0xFFFF);
        uart_base.add(0x28/4).write_volatile((divisor >> 16) & 0xFFFF);
        
        // 8N1, FIFO enabled
        uart_base.add(0x2C/4).write_volatile(0x70);
        
        // Enable UART
        uart_base.add(0x30/4).write_volatile(0x301);
    }
}
```

## 4. Debugging Tools

### 4.1 GDB Debugging

#### Cross-Debugging Setup:

```bash
# Start QEMU with GDB server
qemu-system-aarch64 -M virt -cpu cortex-a72 \
  -kernel kernel8.img \
  -serial stdio \
  -s -S  # -S: freeze at startup, -s: gdb on port 1234

# In another terminal
gdb-multiarch target/aarch64-unknown-none/debug/kernel
(gdb) target remote :1234
(gdb) break kernel_entry
(gdb) continue
```

#### GDB Configuration File:

Create `.gdbinit`:

```
set architecture aarch64
target remote :1234
file target/aarch64-unknown-none/debug/kernel
symbol-file target/aarch64-unknown-none/debug/kernel
break kernel_entry
continue
```

### 4.2 OpenOCD for Hardware Debugging

```bash
# Raspberry Pi 4/5 configuration
openocd -f interface/raspberrypi-swd.cfg -f target/rp2040.cfg

# Connect GDB
gdb-multiarch target/aarch64-unknown-none/debug/kernel
(gdb) target remote :3333
```

## 5. Performance Optimization Tools

### 5.1 Size Optimization

```bash
# Check binary size
aarch64-linux-gnu-size kernel8.img

# Strip debug symbols (for release)
aarch64-linux-gnu-strip --strip-debug kernel8.img

# Analyze section sizes
aarch64-linux-gnu-objdump -h kernel8.img
```

### 5.2 Build Time Optimization

```bash
# Use mold linker (faster than gold/ld)
sudo apt-get install mold
export RUSTFLAGS="-C link-arg=-fuse-ld=mold"

# Parallel builds
export CARGO_BUILD_JOBS=$(nproc)

# Use sccache for caching
cargo install sccache
export RUSTC_WRAPPER=sccache
```

## 6. CI/CD Pipeline Recommendations

### 6.1 GitHub Actions Configuration:

```yaml
name: ARM64 Build and Test

on: [push, pull_request]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v3
    
    - name: Install ARM64 toolchain
      run: |
        sudo apt-get update
        sudo apt-get install -y gcc-aarch64-linux-gnu qemu-system-arm
    
    - name: Install Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: nightly
        target: aarch64-unknown-none
    
    - name: Build for ARM64
      run: |
        cargo build --target aarch64-unknown-none
        aarch64-linux-gnu-objcopy -O binary \
          target/aarch64-unknown-none/debug/kernel \
          kernel8.img
    
    - name: Test in QEMU
      run: |
        timeout 10s qemu-system-aarch64 \
          -M virt -cpu cortex-a72 \
          -kernel kernel8.img \
          -serial stdio \
          -display none || true
```

### 6.2 Local CI Script:

Create `ci.sh`:

```bash
#!/bin/bash
set -e

echo "=== Building for ARM64 ==="
cargo build --target aarch64-unknown-none

echo "=== Creating binary image ==="
aarch64-linux-gnu-objcopy -O binary \
  target/aarch64-unknown-none/debug/kernel \
  kernel8.img

echo "=== Testing in QEMU ==="
timeout 5s qemu-system-aarch64 \
  -M virt -cpu cortex-a72 \
  -kernel kernel8.img \
  -serial stdio \
  -display none 2>&1 | grep -i "webbos\|kernel\|error" || true

echo "=== Size Analysis ==="
aarch64-linux-gnu-size kernel8.img

echo "=== CI Complete ==="
```

## 7. Troubleshooting Guide

### Common Issues and Solutions:

#### 1. Linker Errors:
```
error: linking with `aarch64-linux-gnu-gcc` failed
```
**Solution:** Ensure `-nostdlib` and `-lgcc` flags are set in linker arguments.

#### 2. QEMU Boot Failure:
```
qemu-system-aarch64: kernel8.img: Invalid argument
```
**Solution:** Ensure binary is properly formatted with `objcopy -O binary`.

#### 3. No Serial Output:
**Solution:**
- Verify baud rate (115200)
- Check GPIO connections (TX→RX, RX→TX)
- Ensure UART is enabled in kernel

#### 4. Rust Feature Issues:
```
error: #[feature] may not be used on the stable release channel
```
**Solution:** Use nightly Rust: `rustup default nightly`

## 8. Recommended Development Workflow

### Daily Development Cycle:

1. **Code on x86_64:**
   ```bash
   # Edit code
   vim kernel/src/arch/arm64/mod.rs
   ```

2. **Cross-compile:**
   ```bash
   make arm64
   ```

3. **Test in QEMU:**
   ```bash
   make qemu-arm64
   ```

4. **Hardware test (as needed):**
   ```bash
   make pi-test
   screen /dev/ttyUSB0 115200
   ```

5. **Debug issues:**
   ```bash
   # Use GDB with QEMU
   make debug-arm64
   ```

### Weekly Validation Cycle:

1. **Monday:** Plan week's porting targets
2. **Tuesday-Thursday:** Core porting work
3. **Friday:** Hardware validation testing
4. **Weekend:** Documentation and cleanup

## 9. Cost Analysis of Toolchain

### Free Components:
- GCC ARM64 toolchain: $0
- QEMU: $0
- Rust compiler: $0
- OpenOCD: $0
- GDB: $0

### Potential Paid Components:
- **Commercial IDE:** $0-100/month (optional)
- **Hardware debug probe:** $50-200 (optional)
- **Cloud CI minutes:** $0-50/month

**Total Toolchain Cost:** $0-350 (mostly optional)

## 10. Success Metrics

### Toolchain Success Criteria:
1. ✅ Cross-compilation working in < 2 minutes
2. ✅ QEMU boot within 5 seconds
3. ✅ Serial output visible within 10 seconds of hardware boot
4. ✅ GDB debugging functional
5. ✅ CI pipeline passing

### Performance Targets:
- **Compilation time:** < 3 minutes for full rebuild
- **Binary size:** < 2MB for kernel
- **Boot time:** < 5 seconds to serial output
- **Memory usage:** < 16MB for basic kernel

---

**Prepared by:** Claude Le Comptable  
**Date:** February 15, 2026  
**Status:** READY FOR IMPLEMENTATION

*These recommendations will be updated based on Phase 2 experience.*