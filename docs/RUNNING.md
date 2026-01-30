# Running WebbOS

This guide explains how to build and run WebbOS on your system.

> **Platform Note:** WebbOS was developed and tested on **Windows 11**. While it should work on Linux and macOS, the primary development toolchain is Windows-native (PowerShell + Python).

## Prerequisites

### Required Tools
- **Rust** (nightly toolchain: `nightly-2025-01-15`)
- **QEMU** (for virtualization)
- **Python 3** (for disk image updates on Windows)

### Installation

#### Windows 11 (PowerShell) - Primary Platform
```powershell
# Install Rust
irm https://win.rustup.rs | iex

# Install QEMU (using chocolatey)
choco install qemu

# Install nightly toolchain
rustup toolchain install nightly-2025-01-15
rustup component add rust-src --toolchain nightly-2025-01-15
rustup target add x86_64-unknown-none x86_64-unknown-uefi --toolchain nightly-2025-01-15

# Verify Python is installed (usually pre-installed on Windows 11)
python --version
```

#### Linux (Ubuntu/Debian)
```bash
# Install dependencies
sudo apt update
sudo apt install qemu-system-x86 mtools python3

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install nightly toolchain
rustup toolchain install nightly-2025-01-15
rustup component add rust-src --toolchain nightly-2025-01-15
rustup target add x86_64-unknown-none x86_64-unknown-uefi --toolchain nightly-2025-01-15
```

#### macOS
```bash
# Install dependencies
brew install qemu mtools python3

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install nightly toolchain
rustup toolchain install nightly-2025-01-15
rustup component add rust-src --toolchain nightly-2025-01-15
rustup target add x86_64-unknown-none x86_64-unknown-uefi --toolchain nightly-2025-01-15
```

## Building WebbOS

### Quick Build (Windows 11)

```powershell
# Build bootloader
cargo +nightly-2025-01-15 build -p bootloader --target x86_64-unknown-uefi -Z build-std=core,compiler_builtins,alloc

# Build kernel
cargo +nightly-2025-01-15 build -p kernel --target x86_64-unknown-none -Z build-std=core,compiler_builtins,alloc

# Update disk image
python update-image.py webbos.img "EFI/BOOT/BOOTX64.EFI" target/x86_64-unknown-uefi/debug/bootloader.efi
python update-image.py webbos.img kernel.elf target/x86_64-unknown-none/debug/kernel
```

### Detailed Build Process

#### 1. Build the Kernel

```bash
cargo +nightly-2025-01-15 build -p kernel --target x86_64-unknown-none -Z build-std=core,compiler_builtins,alloc
```

**Output:** `target/x86_64-unknown-none/debug/kernel` (~10MB ELF binary)

**What it does:**
- Compiles the kernel for bare-metal x86_64
- Links with custom start code
- Outputs an ELF64 binary

#### 2. Build the Bootloader

```bash
cargo +nightly-2025-01-15 build -p bootloader --target x86_64-unknown-uefi -Z build-std=core,compiler_builtins,alloc
```

**Output:** `target/x86_64-unknown-uefi/debug/bootloader.efi` (~220KB UEFI executable)

**What it does:**
- Compiles UEFI bootloader
- Links as EFI application
- Can be loaded by UEFI firmware

#### 3. Create/Update Boot Disk Image

The disk image (`webbos.img`) is a pre-formatted FAT32 image containing:
- `/EFI/BOOT/BOOTX64.EFI` - The bootloader
- `/kernel.elf` - The kernel binary

**On Windows (using Python script):**
```powershell
python update-image.py webbos.img "EFI/BOOT/BOOTX64.EFI" target/x86_64-unknown-uefi/debug/bootloader.efi
python update-image.py webbos.img kernel.elf target/x86_64-unknown-none/debug/kernel
```

The `update-image.py` script:
- Parses the FAT32 filesystem structure
- Locates files by their 8.3 directory entry names
- Overwrites file content in-place
- Does NOT require WSL, mtools, or mounting

**On Linux (using mtools):**
```bash
mcopy -o -i webbos.img target/x86_64-unknown-uefi/debug/bootloader.efi ::/EFI/BOOT/BOOTX64.EFI
mcopy -o -i webbos.img target/x86_64-unknown-none/debug/kernel ::/kernel.elf
```

**Creating a new disk image (if needed):**
```bash
# Using WSL or Linux
dd if=/dev/zero of=webbos.img bs=1M count=64
mkfs.fat -F 32 webbos.img

# Create directory structure and copy files...
```

## Running WebbOS

### Method 1: QEMU with Serial Output (Recommended)

```powershell
qemu-system-x86_64 `
    -bios OVMF.fd `
    -drive format=raw,file=webbos.img `
    -m 128M `
    -smp 1 `
    -nographic `
    -serial stdio
```

**Parameters:**
- `-bios OVMF.fd` - UEFI firmware (included in repo)
- `-drive format=raw,file=webbos.img` - Boot disk
- `-m 128M` - 128MB RAM (sufficient for WebbOS)
- `-smp 1` - Single CPU core
- `-nographic` - No GUI window, use serial for display
- `-serial stdio` - Connect serial port to terminal

### Method 2: QEMU with Graphics

```powershell
qemu-system-x86_64 `
    -bios OVMF.fd `
    -drive format=raw,file=webbos.img `
    -vga std `
    -m 256M `
    -smp 1 `
    -serial stdio
```

### Method 3: QEMU with Network

```powershell
qemu-system-x86_64 `
    -bios OVMF.fd `
    -drive format=raw,file=webbos.img `
    -m 256M `
    -smp 1 `
    -serial stdio `
    -netdev user,id=net0,hostfwd=tcp::8080-:80 `
    -device virtio-net-pci,netdev=net0
```

### Method 4: Debug Mode (with GDB)

Terminal 1:
```powershell
qemu-system-x86_64 `
    -bios OVMF.fd `
    -drive format=raw,file=webbos.img `
    -m 128M `
    -smp 1 `
    -nographic `
    -serial stdio `
    -s -S
```

Terminal 2:
```bash
# Connect GDB
gdb target/x86_64-unknown-none/debug/kernel
(gdb) target remote :1234
(gdb) break kernel_entry
(gdb) continue
```

## First Boot

When WebbOS boots successfully, you'll see:

```
╔══════════════════════════════════════════════════╗
║                                                  ║
║  ██╗    ██╗███████╗██████╗ ██████╗  ██████╗ ███████╗
║  ██║    ██║██╔════╝██╔══██╗██╔══██╗██╔═══██╗██╔════╝
║  ██║ █╗ ██║█████╗  ██████╔╝██████╔╝██║   ██║███████╗
║  ██║███╗██║██╔══╝  ██╔══██╗██╔══██╗██║   ██║╚════██║
║  ╚███╔███╔╝███████╗██████╔╝██║  ██║╚██████╔╝███████║
║   ╚══╝╚══╝ ╚══════╝╚═════╝ ╚═╝  ╚═╝ ╚═════╝ ╚══════╝
║                                                  ║
║           Version 0.1.0 - x86_64                 ║
╚══════════════════════════════════════════════════╝

Boot Info:
  Version: 1
  Kernel: PhysAddr(1048576) (size: ... bytes)
  Stack: top=VirtAddr(...), size=128KB
  Memory map: 114 entries
  Bootloader: WebbOS Bootloader

[cpu] Initializing...
[cpu] CPU features detected

[mm] Initializing memory management...
  Total available memory: ... MB
  Heap initialized: 8192 KB at ...
[mm] Memory management initialized

... (more initialization) ...

✓ WebbOS kernel initialized successfully!

System is ready. Type 'help' for available commands.
$ 
```

### Default Credentials

When the desktop environment starts:

| Username | Password | Type |
|----------|----------|------|
| `admin` | `admin` | Administrator |
| `user` | `user` | Standard User |

## Available Commands

Once at the command prompt, try these commands:

```
help              - Show all commands
info              - System information
memory            - Memory statistics
processes         - Show running processes
network           - Network status
users             - List user accounts
desktop           - Desktop environment info
launch notepad    - Open Notepad
launch paint      - Open Paint
launch filemanager - Open File Manager
launch browser    - Open WebbBrowser
test              - Run test suite
reboot            - Reboot system
shutdown          - Shutdown system
```

## Development Workflow

### Quick Development Cycle

```powershell
# 1. Make changes to source code
# 2. Build kernel
cargo +nightly-2025-01-15 build -p kernel --target x86_64-unknown-none -Z build-std=core,compiler_builtins,alloc

# 3. Update disk image
python update-image.py webbos.img kernel.elf target/x86_64-unknown-none/debug/kernel

# 4. Run
qemu-system-x86_64 -bios OVMF.fd -drive format=raw,file=webbos.img -m 128M -smp 1 -nographic -serial stdio
```

### One-Line Build and Run

```powershell
cargo +nightly-2025-01-15 build -p kernel --target x86_64-unknown-none -Z build-std=core,compiler_builtins,alloc; python update-image.py webbos.img kernel.elf target/x86_64-unknown-none/debug/kernel; qemu-system-x86_64 -bios OVMF.fd -drive format=raw,file=webbos.img -m 128M -smp 1 -nographic -serial stdio
```

## Troubleshooting

### Build Errors

#### "cargo not found"
```powershell
# Ensure Rust is installed and in PATH
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
# Or restart your terminal
```

#### "target not found"
```bash
# Install the target
rustup target add x86_64-unknown-none --toolchain nightly-2025-01-15
rustup target add x86_64-unknown-uefi --toolchain nightly-2025-01-15
```

#### "rust-src component not found"
```bash
rustup component add rust-src --toolchain nightly-2025-01-15
```

### QEMU Issues

#### "OVMF.fd not found"
The `OVMF.fd` file is included in the repository. If missing, download from:
https://github.com/retrage/edk2-nightly/raw/master/bin/RELEASEX64_OVMF.fd

#### "cannot set up guest memory"
Kill existing QEMU processes:
```powershell
taskkill /F /IM qemu-system-x86_64.exe
```

#### Kernel crashes immediately
The kernel entry point changes with each build. The bootloader has a hardcoded address that must match. Check the entry point:
```powershell
python -c "import struct; f=open('target/x86_64-unknown-none/debug/kernel','rb'); f.seek(0x18); print(f'Entry: {struct.unpack('<Q', f.read(8))[0]:#x}')"
```

Then update `bootloader/src/main.rs`:
```rust
const KERNEL_ENTRY_PHYS: u64 = 0xXXXXXX; // Use the printed address
```

### Disk Image Issues

#### "File not found" in update-image.py
The script looks for files by their 8.3 FAT32 names:
- `EFI/BOOT/BOOTX64.EFI` → looks for `BOOTX64 EFI`
- `kernel.elf` → looks for `KERNEL  ELF`

Ensure the disk image has these files already present (the script updates existing files, doesn't create new ones).

## Performance Tips

### Faster Builds
```bash
# Use release mode (slower compile, faster runtime)
cargo build --release ...
```

### Faster QEMU
```bash
# Reduce memory
qemu-system-x86_64 ... -m 128M

# Disable graphics
qemu-system-x86_64 ... -nographic
```

## Real Hardware (USB Boot)

**⚠️ Warning: This will erase your USB drive!**

```bash
# Find your USB device (e.g., /dev/sdb on Linux, /dev/disk2 on macOS)
lsblk  # Linux
diskutil list  # macOS

# Copy the image to USB (replace X with your device)
sudo dd if=webbos.img of=/dev/sdX bs=4M status=progress
sync
```

Then boot from the USB drive on your target computer.

## Support

For help and questions:
- Check `docs/ARCHITECTURE.md` for system details
- Check `docs/BUILD.md` for build instructions
- Check `docs/FEATURES.md` for feature list

Happy hacking! 🌐
