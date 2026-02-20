# Raspberry Pi SD Card Scripts

These scripts create and manage SD card images for running WebbOS on Raspberry Pi (ARM64/aarch64).

## Overview

The Raspberry Pi boots from an SD card with a specific layout:

```
+--------------------------------+  Sector 0
| MBR (Master Boot Record)       |  512 bytes
| - Partition table              |
| - Boot code (not used)         |
+--------------------------------+  Sector 2048 (1MB offset)
| Partition 1: FAT32 Boot        |  256MB
| - bootcode.bin / start4.elf    |  GPU firmware (Pi 4)
| - fixup.dat / fixup4.dat       |  GPU memory fixup
| - config.txt                   |  Boot configuration
| - cmdline.txt                  |  Kernel command line
| - kernel8.img                  |  64-bit kernel (our OS)
| - *.dtb                        |  Device tree blobs
| - overlays/*.dtbo              |  Device tree overlays
+--------------------------------+
| Partition 2: ext4 Root         |  Remaining space
| - OS root filesystem           |  (not yet implemented)
+--------------------------------+
```

## Quick Start

### 1. Build the kernel

```powershell
cd Pi
.\build.bat
```

This will:
1. Build the kernel for aarch64
2. Create `webbos-pi.img` if it doesn't exist
3. Or update the kernel in the existing image

### 2. Run in QEMU

```powershell
# Run with Raspberry Pi 3B (default)
.\run.bat raspi3b

# Run with Raspberry Pi 4
.\run.bat raspi4b
```

### 3. Write to SD card

Use a tool like [Raspberry Pi Imager](https://www.raspberrypi.com/software/),
[Rufus](https://rufus.ie/), or [Etcher](https://www.balena.io/etcher/).

Or from command line (be very careful with device name!):

```powershell
# Windows (using dd for Windows or WSL)
dd if=webbos-pi.img of=\\.\PhysicalDriveN bs=4M status=progress

# Linux/macOS
sudo dd if=webbos-pi.img of=/dev/sdX bs=4M status=progress
```

## Scripts

### create-sdcard.py

Creates a new SD card image with MBR partition table and FAT32 boot partition.

```bash
# Create 2GB image (default)
python scripts/create-sdcard.py

# Create with specific kernel
python scripts/create-sdcard.py target/aarch64-unknown-none/release/kernel

# Create larger image
python scripts/create-sdcard.py --size 4096 --output large-sdcard.img

# Include Raspberry Pi firmware files
python scripts/create-sdcard.py --firmware-dir /path/to/firmware --include-firmware
```

### update-sdcard.py

Updates files in an existing SD card image.

```bash
# Update kernel
python scripts/update-sdcard.py webbos-pi.img kernel target/aarch64-unknown-none/release/kernel

# Update config.txt
python scripts/update-sdcard.py webbos-pi.img config my-config.txt

# Update cmdline.txt
python scripts/update-sdcard.py webbos-pi.img cmdline my-cmdline.txt

# List all files in boot partition
python scripts/update-sdcard.py webbos-pi.img ls
```

## Configuration Files

### config.txt

The `config.txt` file is read by the GPU firmware at boot. Key settings:

```ini
# Enable 64-bit mode (required)
arm_64bit=1

# Load our kernel
kernel=kernel8.img

# Disable auto cmdline - we parse device tree
disable_commandline_tags=1

# Enable UART for debugging
enable_uart=1
uart_2ndstage=1

# GPU memory split
gpu_mem=16
```

See `config.txt` in the Pi directory for the full configuration.

### cmdline.txt

The kernel command line passed by the firmware:

```
console=ttyS0,115200 root=/dev/mmcblk0p2 rw rootwait
```

## Firmware Files

The Raspberry Pi requires proprietary GPU firmware files that are not included
in this repository. You can download them from the official repository:

```bash
# Download firmware files
git clone --depth 1 https://github.com/raspberrypi/firmware.git

# Copy boot files
cp firmware/boot/bootcode.bin .
cp firmware/boot/start*.elf .
cp firmware/boot/fixup*.dat .
cp firmware/boot/*.dtb .
cp -r firmware/boot/overlays .
```

For QEMU testing, these files are not strictly required since QEMU emulates
the CPU directly. However, for real hardware, they are essential.

## Device Tree Blobs

Device Tree Blobs (DTBs) describe the hardware to the kernel:

- **bcm2710-rpi-3-b.dtb** - Raspberry Pi 3 Model B
- **bcm2710-rpi-3-b-plus.dtb** - Raspberry Pi 3 Model B+
- **bcm2711-rpi-4-b.dtb** - Raspberry Pi 4 Model B

These should be placed in the boot partition. The kernel parses them to
discover hardware like UART, GPIO, and timers.

## QEMU Testing

QEMU can emulate Raspberry Pi hardware for testing without physical hardware:

```bash
# Pi 3B
qemu-system-aarch64 \
    -M raspi3b \
    -cpu cortex-a53 \
    -m 1G \
    -kernel target/aarch64-unknown-none/release/kernel \
    -dtb bcm2710-rpi-3-b-plus.dtb \
    -serial stdio \
    -display none

# Pi 4B
qemu-system-aarch64 \
    -M raspi4b \
    -cpu cortex-a72 \
    -m 2G \
    -kernel target/aarch64-unknown-none/release/kernel \
    -dtb bcm2711-rpi-4-b.dtb \
    -serial stdio \
    -display none
```

## Troubleshooting

### QEMU fails to start

- Make sure you have a recent version of QEMU (6.0+)
- Check that the kernel file exists and is a valid aarch64 binary
- Verify the DTB file matches the machine type

### Pi doesn't boot from SD card

- Verify the SD card is properly written (try re-writing)
- Check that all required firmware files are present
- Connect UART serial cable for debug output
- Ensure config.txt has `arm_64bit=1`

### Kernel crashes on boot

- Enable debug UART in config.txt
- Check that the kernel is compiled for the correct architecture (aarch64)
- Verify the device tree blob matches your Pi model

## References

- [Raspberry Pi Documentation - config.txt](https://www.raspberrypi.com/documentation/computers/config_txt.html)
- [Raspberry Pi Firmware Repository](https://github.com/raspberrypi/firmware)
- [Device Tree Specification](https://www.devicetree.org/specification/)
- [QEMU ARM/Raspberry Pi](https://qemu-project.gitlab.io/qemu/system/arm/raspi.html)
