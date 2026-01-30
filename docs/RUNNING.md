# Running WebbOS

This guide explains how to build and run WebbOS on your system.

## Prerequisites

### Required Tools
- **Rust** (nightly toolchain)
- **QEMU** (for virtualization)
- **mtools** (for creating disk images)

### Installation

#### Windows (PowerShell)
```powershell
# Install Rust
irm https://win.rustup.rs | iex

# Install QEMU (using chocolatey)
choco install qemu

# Install nightly toolchain
rustup toolchain install nightly-2025-01-15
rustup component add rust-src --toolchain nightly-2025-01-15
```

#### Linux (Ubuntu/Debian)
```bash
# Install dependencies
sudo apt update
sudo apt install qemu-system-x86 mtools

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install nightly toolchain
rustup toolchain install nightly-2025-01-15
rustup component add rust-src --toolchain nightly-2025-01-15
```

#### macOS
```bash
# Install dependencies
brew install qemu mtools

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install nightly toolchain
rustup toolchain install nightly-2025-01-15
rustup component add rust-src --toolchain nightly-2025-01-15
```

## Building WebbOS

### 1. Build the Kernel

```bash
# From the webbOs directory
cargo +nightly-2025-01-15 build -p kernel --target x86_64-unknown-none -Z build-std=core,compiler_builtins,alloc
```

### 2. Build the Bootloader

```bash
cargo +nightly-2025-01-15 build -p bootloader --target x86_64-unknown-uefi -Z build-std=core,compiler_builtins,alloc
```

### 3. Create Boot Disk Image

We provide a PowerShell script to create the disk image:

```powershell
# Windows
.\scripts\create-image.ps1
```

Or manually:

```bash
# Create disk image
dd if=/dev/zero of=webbos.img bs=1M count=64

# Create FAT32 filesystem
mkfs.fat -F 32 webbos.img

# Mount and copy files
mkdir -p mnt
sudo mount -o loop webbos.img mnt

# Create EFI directory structure
sudo mkdir -p mnt/EFI/BOOT

# Copy bootloader
sudo cp target/x86_64-unknown-uefi/debug/bootloader.efi mnt/EFI/BOOT/BOOTX64.EFI

# Copy kernel
sudo cp target/x86_64-unknown-none/debug/kernel mnt/

# Unmount
sudo umount mnt
rmdir mnt
```

## Running WebbOS

### Method 1: QEMU (Recommended for Development)

#### Basic Run
```bash
qemu-system-x86_64 -bios OVMF.fd -drive format=raw,file=webbos.img
```

#### With Graphics (Recommended)
```bash
qemu-system-x86_64 \
    -bios OVMF.fd \
    -drive format=raw,file=webbos.img \
    -vga std \
    -m 512M \
    -smp 2 \
    -serial stdio \
    -display sdl
```

#### With Network
```bash
qemu-system-x86_64 \
    -bios OVMF.fd \
    -drive format=raw,file=webbos.img \
    -vga std \
    -m 512M \
    -smp 2 \
    -serial stdio \
    -netdev user,id=net0,hostfwd=tcp::8080-:80 \
    -device virtio-net-pci,netdev=net0
```

#### Debug Mode (with GDB)
```bash
qemu-system-x86_64 \
    -bios OVMF.fd \
    -drive format=raw,file=webbos.img \
    -vga std \
    -m 512M \
    -smp 1 \
    -serial stdio \
    -s -S

# In another terminal:
./scripts/gdb.sh
```

### Method 2: Quick Test Script

We provide a convenient script to build and run:

```powershell
# Windows PowerShell
.\scripts\run-qemu.ps1

# With networking
.\scripts\run-qemu.ps1 -Network

# Debug mode
.\scripts\run-qemu.ps1 -Debug
```

### Method 3: Real Hardware (USB Boot)

**⚠️ Warning: This will erase your USB drive!**

```bash
# Find your USB device (e.g., /dev/sdb)
lsblk

# Copy the image to USB (replace /dev/sdX with your device)
sudo dd if=webbos.img of=/dev/sdX bs=4M status=progress
sync
```

Then boot from the USB drive on your target computer.

## First Boot

When WebbOS boots, you'll see:

1. **UEFI Boot** - The bootloader loads
2. **Kernel Initialization** - Memory, drivers, network init
3. **Login Screen** - Beautiful gradient background with login form

### Default Credentials

| Username | Password | Type |
|----------|----------|------|
| `admin` | `admin` | Administrator |
| `user` | `user` | Standard User |

### Using the Desktop

1. **Login** - Enter username and password
2. **Desktop** - Click the Start button (🌐 WebbOS) to open the menu
3. **Launch Apps** - Click any application to open it
4. **Shell** - Press any key in the serial console for command mode

## Common Commands

Once in the shell, try these commands:

```
help              - Show all commands
info              - System information
memory            - Memory usage
processes         - Running processes
network           - Network status
users             - List user accounts
desktop           - Desktop environment info
launch notepad    - Open Notepad
launch paint      - Open Paint
launch filemanager - Open File Manager
test              - Run test suite
reboot            - Reboot system
shutdown          - Shutdown system
```

## Troubleshooting

### Build Errors

#### "cargo not found"
```bash
# Ensure Rust is installed and in PATH
source $HOME/.cargo/env  # Linux/macOS
# Or restart your terminal
```

#### "target not found"
```bash
# Install the target
rustup target add x86_64-unknown-none --toolchain nightly-2025-01-15
rustup target add x86_64-unknown-uefi --toolchain nightly-2025-01-15
```

### QEMU Issues

#### "OVMF.fd not found"
```bash
# Install OVMF (UEFI firmware)
# Ubuntu/Debian:
sudo apt install ovmf

# Then use:
qemu-system-x86_64 -bios /usr/share/ovmf/OVMF.fd ...

# Or download manually from:
# https://github.com/retrage/edk2-nightly/raw/master/bin/RELEASEX64_OVMF.fd
```

#### Black Screen
```bash
# Try VGA cirrus instead of std
qemu-system-x86_64 -vga cirrus ...

# Or disable graphics and use serial only
qemu-system-x86_64 -nographic ...
```

#### No Network
```bash
# Check QEMU network settings
qemu-system-x86_64 ... -netdev user,id=net0 -device virtio-net-pci,netdev=net0

# Test with ping from WebbOS shell:
ping 10.0.2.2
```

### Runtime Issues

#### Keyboard/Mouse Not Working
- Ensure you're using QEMU's default PS/2 emulation (not USB)
- Try: `-usb -device usb-mouse -device usb-kbd` for USB input

#### Low Resolution
WebbOS defaults to 1024x768. To change:
```bash
qemu-system-x86_64 ... -vga std -display sdl,gl=on
```

## Development Workflow

### Quick Development Cycle

```bash
# 1. Make changes to source code
# 2. Build
cargo +nightly-2025-01-15 build -p kernel --target x86_64-unknown-none -Z build-std=core,compiler_builtins,alloc

# 3. Run
.\scripts\run-qemu.ps1
```

### Running Tests

```bash
# From WebbOS shell
test

# Or specific test suites:
test memory
test network
test crypto
```

### Debugging

```bash
# 1. Start QEMU in debug mode
qemu-system-x86_64 ... -s -S

# 2. Connect GDB
target remote :1234
```

## Performance Tips

### Faster Builds
```bash
# Use release mode (slower compile, faster runtime)
cargo build --release ...

# Use sccache for caching
export RUSTC_WRAPPER=sccache
```

### Faster QEMU
```bash
# Use KVM (Linux only)
qemu-system-x86_64 ... -enable-kvm -cpu host

# Reduce memory
qemu-system-x86_64 ... -m 256M

# Disable unnecessary features
qemu-system-x86_64 ... -vga none -nographic
```

## Next Steps

- **Read the code**: Start with `kernel/src/main.rs`
- **Add features**: See `docs/ARCHITECTURE.md`
- **Report issues**: Open an issue on GitHub
- **Contribute**: Submit pull requests!

## Support

For help and questions:
- Check `docs/ARCHITECTURE.md` for system details
- Check `docs/FEATURES.md` for feature list
- Review build logs for errors

Happy hacking! 🌐
