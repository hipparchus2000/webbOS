# WebbOS Raspberry Pi Port - Agent Documentation

**For:** Future AI agents working on the ARM64/Pi port  
**Date:** 2026-02-20  
**Status:** Core architecture conversion complete, driver implementation pending

---

## Quick Reference

### Build Commands

```bash
# Build kernel
cargo +nightly-2025-01-15 build -p kernel --target aarch64-unknown-none -Z build-std=core,compiler_builtins,alloc

# Build bootloader
cargo +nightly-2025-01-15 build -p bootloader --target aarch64-unknown-none -Z build-std=core,compiler_builtins,alloc

# Create kernel8.img
rust-objcopy -O binary target/aarch64-unknown-none/debug/kernel kernel8.img

# Run in QEMU
qemu-system-aarch64 -M raspi3b -m 1G -serial stdio -kernel kernel8.img

# Or use Makefile
make build
make run
```

---

## Architecture Changes from x86_64

### Boot Sequence

1. **GPU firmware** loads `kernel8.img` at `0x80000`
2. **pi_start.rs** takes over:
   - Checks current Exception Level (EL)
   - Transitions from EL2 → EL1 (or EL3 → EL2 → EL1)
   - Clears BSS section
   - Sets up temporary stack
   - Jumps to `rust_main()` with DTB pointer in x0

3. **bootloader/main.rs**:
   - Initializes UART (PL011) for debug output
   - Parses Device Tree Blob for hardware info
   - Loads kernel ELF
   - Sets up ARM64 page tables (4-level)
   - Enables MMU
   - Jumps to kernel

4. **kernel/main.rs**:
   - Initializes exception vectors (VBAR_EL1)
   - Sets up memory management
   - Initializes drivers
   - Runs main loop

### Key Register Differences

| Purpose | x86_64 | ARM64 |
|---------|--------|-------|
| Page table | CR3 | TTBR0_EL1 / TTBR1_EL1 |
| Control | CR0/CR4 | SCTLR_EL1 |
| Translation control | - | TCR_EL1 |
| Memory attributes | - | MAIR_EL1 |
| Vector table | IDTR | VBAR_EL1 |
| Interrupt flags | EFLAGS.IF | DAIF bits |
| Timer | TSC | CNTPCT_EL0 |

---

## File Organization

### Bootloader (`bootloader/src/`)

```
main.rs         - Entry point, kernel loading, MMU enable
pi_start.rs     - Assembly entry, EL transition
mmu.rs          - ARM64 page table setup
uart.rs         - PL011 UART driver
dtb.rs          - Device Tree parser
memory.rs       - Memory utilities
Cargo.toml      - (No UEFI dependencies)
```

### Kernel Arch (`kernel/src/arch/`)

```
mod.rs          - Exports cpu, exceptions, mmu
cpu.rs          - CPU features, EL, timers
exceptions.rs   - Exception vectors, IRQ handling
mmu.rs          - MMU management, page tables
linker.ld       - ARM64 memory layout
```

**Removed:** `gdt.rs`, `interrupts.rs`, `paging.rs` (x86-specific)

---

## Memory Layout

### Physical Memory Map

```
0x0000_0000 - 0x0008_0000:  GPU firmware / DTB
0x0008_0000 - 0x0010_0000:  Bootloader (512KB)
0x0010_0000 - 0x0020_0000:  Kernel code/data (1MB)
0x0040_0000 - 0x0100_0000:  Bootloader heap
```

### Virtual Memory Map (Higher Half)

```
0xFFFF_0000_0000_0000:      Kernel base
0xFFFF_0000_0010_0000:      Kernel code
0xFFFF_0000_0050_0000:      Kernel stack (128KB)
0xFFFF_0000_1000_0000:      Kernel heap
```

---

## Key Implementation Details

### ARM64 MMU (4KB granule)

- **Level 0 (PGD)**: 512GB per entry, usually one entry for kernel space
- **Level 1 (PUD)**: 1GB per entry (can be 1GB block)
- **Level 2 (PMD)**: 2MB per entry (can be 2MB block)
- **Level 3 (PTE)**: 4KB page

Page table entry flags:
- Bit 0: Valid
- Bit 1: Table (1=pointer to next level, 0=block/page)
- Bits [4:2]: Memory attribute index (MAIR)
- Bits [7:6]: Access permissions
- Bit 10: Access flag (must be set)
- Bits [54:53]: PXN/UXN (execute never)

### Exception Levels

ARM64 uses Exception Levels instead of rings:
- **EL3**: Secure monitor (highest privilege)
- **EL2**: Hypervisor
- **EL1**: OS kernel
- **EL0**: User applications

The Pi typically boots to EL2, we drop to EL1 for the kernel.

### Exception Vectors

VBAR_EL1 points to a table with 16 entries (4 exception types × 4 sources):
- Synchronous exceptions
- IRQ
- FIQ
- SError

Each from: EL1t, EL1h, EL0_64, EL0_32

Each slot is 128 bytes, allowing simple inline handlers.

---

## What's Working

- ✅ Assembly entry point with EL transition
- ✅ UART output (PL011)
- ✅ Device Tree parsing
- ✅ Page table setup
- ✅ MMU enable/disable
- ✅ Exception vector setup
- ✅ CPU feature detection
- ✅ Basic kernel boot flow

## What's Not Implemented

- ❌ SD card (EMMC) driver
- ❌ GIC (interrupt controller)
- ❌ USB host
- ❌ SMP (multi-core)
- ❌ GPU mailbox interface
- ❌ Real framebuffer setup

---

## Common Issues

### "Invalid instruction" in QEMU
Make sure you're targeting `aarch64-unknown-none` not `aarch64-unknown-linux-gnu`.

### No UART output
- Check QEMU machine type (`raspi3b` not `virt`)
- Verify UART initialization happens early
- DTB may specify different UART base address

### Page fault on boot
- Ensure TTBR0/TTBR1 are set before enabling MMU
- Check TCR_EL1 configuration matches page table format
- Verify all page table entries have AF (access flag) set

---

## Testing

```bash
# Quick test in QEMU
make run

# With GDB debugging
make debug
# In another terminal: gdb-multiarch -ex "target remote :1234" target/aarch64-unknown-none/debug/kernel

# Check kernel symbols
rust-objdump -t target/aarch64-unknown-none/debug/kernel | grep "T "
```

---

## References

- [ARMv8-A Architecture Reference Manual](https://developer.arm.com/documentation/ddi0487/latest/)
- [Raspberry Pi Firmware Wiki](https://github.com/raspberrypi/firmware/wiki)
- [Device Tree Specification](https://www.devicetree.org/specifications/)
- [OSDev ARM64 Bare Bones](https://wiki.osdev.org/ARM64_Bare_Bones)

---

**Last Updated:** 2026-02-20
