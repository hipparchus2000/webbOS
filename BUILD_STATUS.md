# WebbOS Build Status

## Summary

Both **x86_64** and **aarch64** architectures build successfully and boot in QEMU.

## Build Status

### x86_64 (Intel/AMD)
- ✅ Kernel compiles without errors
- ✅ Bootloader compiles without errors  
- ✅ Boots in QEMU with UEFI
- ✅ All drivers compile (PCI, VESA, Input, Network, etc.)

### aarch64 (ARM64)
- ✅ Kernel compiles without errors
- ✅ Raspberry Pi bootloader compiles
- ✅ Combined kernel8.img created for SD card boot
- ✅ All Raspberry Pi drivers compile (GPIO, UART, USB, Ethernet)
- ✅ HAL (Hardware Abstraction Layer) compiles

## Quick Start

### Build Everything
```bash
# Build x86_64 kernel and bootloader
make kernel
make bootloader

# Build aarch64 kernel with Pi bootloader
./scripts/create-pi-image.sh
```

### Run in QEMU

#### x86_64
```bash
make run-x64
# Or manually:
qemu-system-x86_64 -m 512M -smp 2 -cpu qemu64 -bios OVMF.fd \
    -drive format=raw,file=fat:rw:build/iso -serial stdio -display none
```

#### aarch64 (Raspberry Pi 3)
```bash
qemu-system-aarch64 -M raspi3b -kernel build/aarch64/kernel8.img \
    -serial stdio -display none
```

## Architecture Details

### x86_64
- **Target**: `x86_64-unknown-none`
- **Bootloader**: UEFI (`bootloader/` crate)
- **Boot Method**: UEFI firmware (OVMF) loads bootloader, which loads kernel
- **Drivers**: PCI, VESA Graphics, Keyboard, Mouse, ATA, VirtIO, Network

### aarch64
- **Target**: `aarch64-unknown-none`
- **Bootloader**: Custom Pi bootloader (`bootloader-pi/` crate)
- **Boot Method**: GPU firmware → Bootloader (0x80000) → Kernel (0x100000)
- **Drivers**: GPIO (RP1/BCM2711), UART (PL011), USB (XHCI), Ethernet (RTL8168)
- **HAL**: Platform detection, MMIO, interrupt controller

## Bootloaders

### x86_64 UEFI Bootloader (`bootloader/`)
- Written in Rust using UEFI protocols
- Loads `kernel.elf` from FAT filesystem
- Sets up page tables and memory mapping
- Obtains framebuffer from GOP (Graphics Output Protocol)
- Exits boot services and jumps to kernel in 64-bit mode

### Raspberry Pi Bootloader (`bootloader-pi/`)
- Written in Rust with ARM64 assembly stub
- Loaded by GPU firmware at address 0x80000
- Drops from EL3/EL2 to EL1 (kernel privilege level)
- Parses device tree blob (DTB) for hardware configuration
- Initializes PL011 UART for serial output
- Loads ELF or raw binary kernel from 0x100000
- Constructs BootInfo structure for kernel
- Jumps to kernel with proper BootInfo pointer

## File Structure

```
build/
├── iso/                    # x86_64 boot files
│   ├── EFI/BOOT/BOOTX64.EFI
│   └── kernel.elf
└── aarch64/                # aarch64 boot files
    ├── kernel8.img         # Combined bootloader+kernel for Raspberry Pi
    ├── config.txt          # Pi configuration file
    └── cmdline.txt         # Kernel command line
```

## Testing

### x86_64 Boot Test Output
```
╔═══════════════════════════════════════╗
║      WebbOS UEFI Bootloader           ║
║      Version 0.1.0                    ║
╚═══════════════════════════════════════╝

Kernel file size: 769984 bytes
ELF entry point: 0xffff80000011e5c0
...
Exiting boot services and jumping to kernel...
```

### Raspberry Pi Bootloader Output
```
╔═══════════════════════════════════════╗
║      WebbOS Pi Bootloader             ║
║      Version 0.1.0                    ║
╚═══════════════════════════════════════╝

DTB address: 0x...
Memory: base=0x0000000000000000 size=0x0000000040000000
Framebuffer: 1024x768 @ 0x000000003E000000
Loading kernel...
Kernel entry: 0x...
Boot info prepared
Jumping to kernel...
```

## Scripts

- `scripts/run-qemu-x64.sh` - Run x86_64 in QEMU
- `scripts/run-qemu-aarch64.sh` - Run aarch64 in QEMU
- `scripts/create-pi-image.sh` - Build Pi bootloader + kernel image
- `build-aarch64.sh` - Build aarch64 kernel
- `build-aarch64-drivers.sh` - Build with driver support

## Makefile Targets

- `make kernel` - Build x86_64 kernel
- `make bootloader` - Build UEFI bootloader
- `make run-x64` - Run x86_64 in QEMU
- `make aarch64-kernel` - Build aarch64 kernel
- `make aarch64-image` - Create aarch64 kernel image
- `make run-aarch64` - Run aarch64 in QEMU
- `make clean` - Clean all build artifacts

## Known Issues

1. **QEMU Raspberry Pi 4**: QEMU does not yet support Raspberry Pi 4 machine type. Use Pi 3B (`raspi3b`) for testing or test on real hardware.

2. **Warnings**: Both architectures have compiler warnings (unused imports, static references) but compile successfully.

3. **Device Tree**: Full device tree parsing is not yet implemented; using hardcoded defaults for Pi 4.
