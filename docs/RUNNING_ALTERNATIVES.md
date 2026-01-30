# Alternative Ways to Run WebbOS

Since the WSL setup requires a reboot, here are alternative methods to run WebbOS.

## Method 1: Use a Linux VM or Live USB (Easiest)

If you have access to any Linux system (VM, live USB, or another computer):

```bash
# 1. Copy your webbOs directory to the Linux system (or git clone)

# 2. Install required tools
sudo apt update
sudo apt install mtools qemu-system-x86

# 3. Build (if not already built)
cd webbOs
cargo +nightly-2025-01-15 build -p kernel --target x86_64-unknown-none -Z build-std=core,compiler_builtins,alloc
cargo +nightly-2025-01-15 build -p bootloader --target x86_64-unknown-uefi -Z build-std=core,compiler_builtins,alloc

# 4. Create disk image
dd if=/dev/zero of=webbos.img bs=1M count=64
mkfs.fat -F 32 webbos.img
mmd -i webbos.img ::/EFI
mmd -i webbos.img ::/EFI/BOOT
mcopy -i webbos.img target/x86_64-unknown-uefi/debug/bootloader.efi ::/EFI/BOOT/BOOTX64.EFI
mcopy -i webbos.img target/x86_64-unknown-none/debug/kernel ::/kernel

# 5. Download OVMF
wget https://github.com/retrage/edk2-nightly/raw/master/bin/RELEASEX64_OVMF.fd -O OVMF.fd

# 6. Run
qemu-system-x86_64 -bios OVMF.fd -drive format=raw,file=webbos.img -vga std -m 512M -serial stdio
```

## Method 2: Complete WSL Setup on Windows

### Step 1: Reboot
Restart your computer to complete WSL feature installation.

### Step 2: Install Ubuntu
After reboot, run as Administrator in PowerShell:
```powershell
wsl --install -d Ubuntu
```

Or install from Microsoft Store:
1. Open Microsoft Store
2. Search for "Ubuntu"
3. Click Install
4. Launch Ubuntu from Start menu

### Step 3: Set Up Ubuntu
When you first run Ubuntu, it will ask you to create a username and password.

### Step 4: Install mtools
In the Ubuntu terminal:
```bash
sudo apt update
sudo apt install mtools
exit
```

### Step 5: Run WebbOS
Back in PowerShell:
```powershell
.\scripts\run-qemu.ps1
```

## Method 3: Use Docker

If you have Docker Desktop installed:

```powershell
# Create a Dockerfile
docker run --rm -v "${PWD}:/webbos" -w /webbos ubuntu:22.04 bash -c "
    apt update && apt install -y mtools wget
    dd if=/dev/zero of=webbos.img bs=1M count=64
    mkfs.fat -F 32 webbos.img
    mmd -i webbos.img ::/EFI
    mmd -i webbos.img ::/EFI/BOOT
    mcopy -i webbos.img target/x86_64-unknown-uefi/debug/bootloader.efi ::/EFI/BOOT/BOOTX64.EFI
    mcopy -i webbos.img target/x86_64-unknown-none/debug/kernel ::/kernel
    mdir -i webbos.img -s ::
"
```

Then download OVMF and run with QEMU.

## Method 4: Use Pre-made Disk Image

### Option A: Download Alpine Linux ISO and modify

1. Download Alpine Linux standard ISO (x86_64)
2. Mount it or extract the EFI partition
3. Replace the kernel with WebbOS kernel
4. Re-pack and boot

### Option B: Use a minimal Linux VM template

You can use any minimal Linux distribution that boots via UEFI, then:
1. Mount the EFI system partition
2. Replace vmlinuz (kernel) with WebbOS kernel
3. Boot

## Method 5: Boot from USB on Real Hardware

### Create bootable USB on Linux:

```bash
# On a Linux system
dd if=webbos.img of=/dev/sdX bs=4M status=progress
sync
```

Replace `/dev/sdX` with your USB device (be careful!).

Then boot from the USB on any PC with UEFI.

## Method 6: Cloud VM

You can run WebbOS on cloud providers that support custom images:

1. Upload `webbos.img` as a custom image
2. Create a VM from that image
3. Connect via serial console

Supported providers: AWS, GCP, Azure (with custom image import)

## Troubleshooting

### "mtools not found"
Install mtools: `sudo apt install mtools` (Linux) or `wsl -d Ubuntu -e sudo apt install mtools` (Windows with WSL)

### "qemu-system-x86_64 not found"
Install QEMU from https://www.qemu.org/download/#windows

### "OVMF.fd not found"
Download from: https://github.com/retrage/edk2-nightly/raw/master/bin/RELEASEX64_OVMF.fd

### "Kernel panic" or crash
The kernel expects certain UEFI structures. Make sure you're using the full disk image creation process, not direct kernel loading.

## Quick Reference

| Method | Requirements | Difficulty | Works On |
|--------|--------------|------------|----------|
| Linux VM/USB | Linux system | Easy | Any PC with Linux |
| WSL Setup | Windows 10/11 | Medium | Windows |
| Docker | Docker Desktop | Medium | Windows/Mac/Linux |
| Real Hardware | USB drive | Hard | Physical PC |
| Cloud VM | Cloud account | Hard | Cloud providers |

## Recommendation

**For immediate testing:** Use Method 1 (Linux VM or live USB) - this is the fastest way to get running.

**For ongoing development:** Complete Method 2 (WSL Setup) - this gives you the best Windows development experience.
