# WebbOS - Agent Documentation

**For:** Future AI agents working on this project  
**Date:** 2026-01-30  
**Status:** System fully booting and operational

---

## Quick Reference

### Build Commands (Copy-Paste Ready)

**Windows 11 (Primary Platform):**
```powershell
# Build
cargo +nightly-2025-01-15 build -p bootloader --target x86_64-unknown-uefi -Z build-std=core,compiler_builtins,alloc
cargo +nightly-2025-01-15 build -p kernel --target x86_64-unknown-none -Z build-std=core,compiler_builtins,alloc

# Update disk image
python update-image.py webbos.img "EFI/BOOT/BOOTX64.EFI" target/x86_64-unknown-uefi/debug/bootloader.efi
python update-image.py webbos.img kernel.elf target/x86_64-unknown-none/debug/kernel

# Run
qemu-system-x86_64 -bios OVMF.fd -drive format=raw,file=webbos.img -m 128M -smp 1 -nographic -serial stdio
```

**One-liner:**
```powershell
cargo +nightly-2025-01-15 build -p bootloader --target x86_64-unknown-uefi -Z build-std=core,compiler_builtins,alloc; cargo +nightly-2025-01-15 build -p kernel --target x86_64-unknown-none -Z build-std=core,compiler_builtins,alloc; python update-image.py webbos.img "EFI/BOOT/BOOTX64.EFI" target/x86_64-unknown-uefi/debug/bootloader.efi; python update-image.py webbos.img kernel.elf target/x86_64-unknown-none/debug/kernel; qemu-system-x86_64 -bios OVMF.fd -drive format=raw,file=webbos.img -m 128M -smp 1 -nographic -serial stdio
```

---

## Development Environment

### Platform
- **OS:** Windows 11
- **Shell:** PowerShell
- **No WSL Required** - Uses native Windows toolchain

### Required Tools
1. **Rust** with nightly toolchain:
   ```
   rustup toolchain install nightly-2025-01-15
   rustup component add rust-src --toolchain nightly-2025-01-15
   rustup target add x86_64-unknown-none x86_64-unknown-uefi --toolchain nightly-2025-01-15
   ```

2. **QEMU** for Windows: `choco install qemu`

3. **Python 3** (usually pre-installed on Windows 11)

### Toolchain Configuration
- **Rust Toolchain:** `nightly-2025-01-15` (specified in `rust-toolchain.toml`)
- **Kernel Target:** `x86_64-unknown-none`
- **Bootloader Target:** `x86_64-unknown-uefi`

---

## Project Structure

```
webbOs/
├── bootloader/          # UEFI bootloader
│   └── src/
│       ├── main.rs      # Entry point, kernel loading
│       ├── paging.rs    # Page table setup
│       └── memory.rs    # Memory allocation
├── kernel/              # Kernel
│   └── src/
│       ├── main.rs      # Kernel entry
│       ├── arch/        # x86_64 specific code
│       ├── mm/          # Memory management
│       ├── drivers/     # Device drivers
│       ├── net/         # Network stack
│       ├── browser/     # Browser engine
│       ├── graphics/    # Graphics subsystem
│       ├── desktop/     # Desktop environment
│       └── ...
├── shared/              # Shared types
│   └── src/
│       ├── types.rs     # PhysAddr, VirtAddr
│       └── bootinfo.rs  # Boot protocol
├── webbos.img           # Boot disk (FAT32)
├── OVMF.fd              # UEFI firmware
└── update-image.py      # Disk image updater
```

---

## Build Process

### 1. Build Bootloader
```powershell
cargo +nightly-2025-01-15 build -p bootloader --target x86_64-unknown-uefi -Z build-std=core,compiler_builtins,alloc
```
**Output:** `target/x86_64-unknown-uefi/debug/bootloader.efi`

### 2. Build Kernel
```powershell
cargo +nightly-2025-01-15 build -p kernel --target x86_64-unknown-none -Z build-std=core,compiler_builtins,alloc
```
**Output:** `target/x86_64-unknown-none/debug/kernel`

### 3. Update Disk Image
The disk image `webbos.img` is a FAT32 filesystem. Use the Python script:

```powershell
python update-image.py webbos.img "EFI/BOOT/BOOTX64.EFI" target/x86_64-unknown-uefi/debug/bootloader.efi
python update-image.py webbos.img kernel.elf target/x86_64-unknown-none/debug/kernel
```

**Note:** This script locates files by their 8.3 FAT32 names and overwrites them. It does NOT require WSL, mtools, or mounting.

### 4. Run in QEMU
```powershell
qemu-system-x86_64 -bios OVMF.fd -drive format=raw,file=webbos.img -m 128M -smp 1 -nographic -serial stdio
```

---

## Critical Implementation Details

### Page Tables
- **Kernel mapped with 4KB pages** (not 2MB large pages)
- Kernel at physical 0x100000 is NOT 2MB aligned
- 512MB identity + higher-half mappings for bootloader transition

### Memory Layout
| Region | Address | Size |
|--------|---------|------|
| Kernel (phys) | 0x100000 | ~2MB |
| Kernel (virt) | 0xFFFF800000100000 | ~2MB |
| Stack | 0xFFFF800000500000 | 128KB |
| Heap | 0xFFFF800040000000 | 8MB |
| Framebuffer | 0xFFFF8000FD000000 | 3MB |

### Kernel Entry Point
The entry point **changes with each build**. The bootloader reads the ELF header, but if there are issues:

```powershell
# Check current entry point
python -c "import struct; f=open('target/x86_64-unknown-none/debug/kernel','rb'); f.seek(0x18); print(f'0x{struct.unpack('<Q', f.read(8))[0] & 0xFFFFFF:x}')"
```

Update in `bootloader/src/main.rs`:
```rust
const KERNEL_ENTRY_PHYS: u64 = 0xXXXXXX; // Must match actual entry
```

### Large Allocations
The browser and graphics subsystems use **deferred allocation**:
- Browser render context: Framebuffer allocated on first render, not at init
- Graphics context: Pixel buffer allocated on first draw
- This avoids 3MB allocation failures during early boot

---

## Troubleshooting

### Kernel crashes immediately after boot
Check that `KERNEL_ENTRY_PHYS` in bootloader matches the actual kernel entry point.

### QEMU "cannot set up guest memory"
Kill existing QEMU processes:
```powershell
taskkill /F /IM qemu-system-x86_64.exe
```

### "File not found" in update-image.py
The script looks for 8.3 FAT32 names:
- `BOOTX64 EFI` for bootloader
- `KERNEL  ELF` for kernel

### Build errors
Ensure correct toolchain:
```powershell
rustup show
rustup component add rust-src --toolchain nightly-2025-01-15
```

---

## Testing

### Manual Testing
1. Build and run
2. At prompt, type `help`
3. Try commands: `info`, `memory`, `network`, `users`
4. Test desktop: `launch notepad`

### Expected Boot Output
```
✓ WebbOS kernel initialized successfully!
System is ready. Type 'help' for available commands.
$ 
```

---

## Key Files for Common Tasks

| Task | Files |
|------|-------|
| Kernel boot | `kernel/src/main.rs` |
| Memory mgmt | `kernel/src/mm/mod.rs`, `kernel/src/mm/allocator.rs` |
| Page tables | `bootloader/src/paging.rs`, `kernel/src/arch/paging.rs` |
| Drivers | `kernel/src/drivers/` |
| Network | `kernel/src/net/` |
| Browser | `kernel/src/browser/` |
| Desktop | `kernel/src/desktop/` |
| Build config | `.cargo/config.toml`, `rust-toolchain.toml` |

---

## Documentation

- [docs/BUILD.md](docs/BUILD.md) - Detailed build instructions
- [docs/RUNNING.md](docs/RUNNING.md) - How to run WebbOS
- [STATUS.md](STATUS.md) - Current implementation status
- [KIMI_CONTEXT.md](KIMI_CONTEXT.md) - Development history

---

## Status Summary

**Current State:** ✅ FULLY BOOTING

All major subsystems operational:
- ✅ UEFI Bootloader
- ✅ Kernel Core (memory, processes, interrupts)
- ✅ Network Stack (TCP/IP, TLS 1.3, HTTP)
- ✅ Browser Engine (HTML/CSS/JS/WASM parsers)
- ✅ Desktop Environment (7 apps)
- ✅ Graphics (VESA framebuffer)
- ✅ Input (PS/2 keyboard, mouse)

**Not Implemented:**
- ❌ App Store (requirement #4 from urs.md)
- ⚠️ WebAssembly runtime (parser complete, execution stubbed)

---

**Last Updated:** 2026-01-30
