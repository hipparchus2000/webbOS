# WebbOS ARM64/Raspberry Pi Porting Guide

This document describes the changes made to port WebbOS from x86_64 UEFI to Raspberry Pi ARM64 bare metal.

## Overview

The porting effort involved converting:
1. **Bootloader**: From UEFI-based to bare metal Pi boot
2. **Architecture code**: From x86_64 to ARM64 (AArch64)
3. **Memory management**: From x86 paging to ARM64 MMU
4. **Interrupts**: From x86 IDT/PIC to ARM64 exception vectors/GIC

## Key Differences

### Boot Process

| Aspect | x86_64 (PC) | ARM64 (Pi) |
|--------|-------------|------------|
| Boot mechanism | UEFI firmware | GPU firmware loads kernel8.img |
| Load address | Configurable | Fixed at 0x80000 |
| Initial mode | Long mode (64-bit) | EL2 or EL3, transition to EL1 |
| Hardware info | UEFI services, ACPI | Device Tree Blob (DTB) |
| Entry register | RDI = BootInfo pointer | X0 = DTB physical address |

### Memory Management

| Feature | x86_64 | ARM64 |
|---------|--------|-------|
| Page size | 4KB (can use 2MB large pages) | 4KB (4KB, 2MB, 1GB granules) |
| Page table levels | 4 (PML4, PDPT, PD, PT) | 4 (L0 PGD, L1 PUD, L2 PMD, L3 PTE) |
| Page table register | CR3 | TTBR0_EL1 / TTBR1_EL1 |
| TLB invalidation | INVLPG | TLBI instruction |
| Control register | CR0, CR4 | SCTLR_EL1, TCR_EL1, MAIR_EL1 |

### Interrupts and Exceptions

| Feature | x86_64 | ARM64 |
|---------|--------|-------|
| Exception table | IDT (Interrupt Descriptor Table) | VBAR (Vector Base Address Register) |
| Table entries | 256 IDT entries | 16 vector entries per EL |
| Entry size | 16 bytes | 128 bytes (per vector slot) |
| Hardware interrupts | PIC/APIC | GIC (Generic Interrupt Controller) |
| System calls | SYSCALL/SYSRET | SVC instruction |

### CPU Features

| Feature | x86_64 | ARM64 |
|---------|--------|-------|
| Segmentation | GDT/TSS required | Not used (flat memory model) |
| Privilege levels | Rings 0-3 | Exception Levels (EL0-EL3) |
| FPU/SIMD | SSE/AVX automatically available | FP/SIMD must be enabled via CPACR_EL1 |
| Timer | TSC (Time Stamp Counter) | CNTFRQ_EL0 / CNTPCT_EL0 |

## File Changes

### New Files

#### Bootloader (`bootloader/src/`)

| File | Purpose |
|------|---------|
| `pi_start.rs` | ARM64 assembly entry point, EL transition, BSS clear |
| `main.rs` | Main bootloader logic, kernel loading, MMU setup |
| `mmu.rs` | ARM64 MMU page table setup |
| `uart.rs` | PL011 UART driver for serial output |
| `dtb.rs` | Device Tree Blob parser |
| `memory.rs` | Simple memory utilities |

#### Kernel Architecture (`kernel/src/arch/`)

| File | Purpose |
|------|---------|
| `mod.rs` | Architecture module exports |
| `cpu.rs` | CPU initialization, exception levels, timers |
| `exceptions.rs` | Exception vector table, interrupt handlers |
| `mmu.rs` | MMU management, page table operations |
| `linker.ld` | ARM64 kernel linker script |

#### Build Configuration

| File | Purpose |
|------|---------|
| `.cargo/config.toml` | Cargo configuration for aarch64 target |
| `rust-toolchain.toml` | Rust toolchain with aarch64 support |
| `Makefile` | Build automation for Pi target |
| `bootstub/boot.S` | Alternative assembly boot stub |

### Removed Files (x86-specific)

- `bootloader/src/paging.rs` - Replaced by `mmu.rs`
- `kernel/src/arch/gdt.rs` - Not needed on ARM (no segmentation)
- `kernel/src/arch/interrupts.rs` - Replaced by `exceptions.rs`
- `kernel/src/arch/paging.rs` - Replaced by `mmu.rs`

### Modified Files

- `bootloader/Cargo.toml` - Removed UEFI dependencies
- `kernel/Cargo.toml` - Minor updates for ARM64
- `kernel/src/main.rs` - Updated for ARM64 entry, removed x86-specific code

## Memory Layout

### Bootloader (Physical Addresses)

```
0x0000_0000 - 0x0008_0000:  GPU firmware / DTB
0x0008_0000 - 0x0010_0000:  Bootloader (kernel8.img loaded here)
0x0010_0000 - 0x0020_0000:  Kernel code/data
0x0040_0000 - 0x0100_0000:  Heap/allocations
```

### Kernel (Virtual Addresses)

```
0xFFFF_0000_0000_0000:      Kernel higher half base
0xFFFF_0000_0010_0000:      Kernel code
0xFFFF_0000_0050_0000:      Kernel stack
0xFFFF_0000_1000_0000:      Kernel heap
```

## Building and Running

### Prerequisites

1. Install Rust nightly with aarch64 target:
```bash
rustup toolchain install nightly-2025-01-15
rustup component add rust-src --toolchain nightly-2025-01-15
rustup target add aarch64-unknown-none --toolchain nightly-2025-01-15
```

2. Install QEMU with aarch64 support (for emulation)

### Build

```bash
# Build everything
make build

# Or build individual components
cargo +nightly-2025-01-15 build -p kernel --target aarch64-unknown-none -Z build-std=core,compiler_builtins,alloc
cargo +nightly-2025-01-15 build -p bootloader --target aarch64-unknown-none -Z build-std=core,compiler_builtins,alloc
```

### Run in QEMU

```bash
# Run with QEMU (Raspberry Pi 3 model)
make run

# Or directly:
qemu-system-aarch64 -M raspi3b -m 1G -serial stdio -kernel kernel8.img
```

### Run on Real Hardware

1. Copy `kernel8.img` to SD card
2. Add `config.txt` with appropriate settings
3. Insert SD card into Raspberry Pi and power on

## Known Limitations

1. **SD Card Driver**: Not yet implemented - kernel assumes it's loaded by firmware
2. **GIC**: Generic Interrupt Controller support is stubbed
3. **USB**: No USB driver (keyboard/mouse would need this)
4. **Graphics**: Framebuffer support from DTB only
5. **SMP**: Only CPU0 is started; multi-core support pending

## Next Steps

To complete the port:

1. Implement SD card (EMMC) driver for filesystem access
2. Implement GIC driver for proper interrupt handling
3. Add UART keyboard input support
4. Implement mailbox interface for GPU communication
5. Add SMP support to start additional CPU cores
6. Implement USB host controller driver
