# WebbOS SD Card Setup Guide

This guide explains how to create a bootable SD card for WebbOS on your Raspberry Pi.

## Prerequisites

- Raspberry Pi 3B, 3B+, or 4B
- MicroSD card (8GB or larger, Class 10 recommended)
- SD card reader
- WebbOS image file: `webbos-pi.img` (256MB)

## Image Contents

The `webbos-pi.img` file contains:

| Partition | Size | Type | Contents |
|-----------|------|------|----------|
| Partition 1 | 127MB | FAT32 | Boot files, kernel, apps |
| Partition 2 | 128MB | ext4 | Reserved for root filesystem |

### Boot Partition Files

```
/
├── kernel8.img          (608KB) - WebbOS kernel with browser engine
├── config.txt           - Boot configuration
├── cmdline.txt          - Kernel command line
├── Apps/
│   ├── calc.html        - Calculator
│   ├── judge.html       - Judge app
│   ├── richtext.html    - Rich text editor
│   ├── sheet.html       - Spreadsheet
│   └── Games/
│       ├── backgamon.html   - Backgammon
│       ├── invaders.html    - Space Invaders
│       ├── mahjong.html     - Mahjong
│       ├── solitaire.html   - Solitaire
│       ├── chickens.html    - Chicken Darts
│       ├── decision.html    - Decision Game
│       ├── platform.html    - Platform Game
│       └── swans.html       - Swans
```

## Option 1: Windows

### Method A: Raspberry Pi Imager (Recommended)

1. Download [Raspberry Pi Imager](https://www.raspberrypi.com/software/)
2. Install and run the application
3. Insert your SD card into the computer
4. Click **"Operating System"**
5. Scroll down and select **"Use custom image"**
6. Navigate to `Pi/webbos-pi.img` and select it
7. Click **"Storage"** and select your SD card
8. Click **"Write"** and confirm
9. Wait for the verification to complete
10. Remove the SD card

### Method B: Win32 Disk Imager

1. Download [Win32 Disk Imager](https://sourceforge.net/projects/win32diskimager/)
2. Run as Administrator
3. Select `webbos-pi.img` as the image file
4. Select your SD card drive letter
5. Click **"Write"**
6. Wait for completion and remove the SD card

### Method C: Command Line (PowerShell as Administrator)

⚠️ **WARNING: Be extremely careful with the disk number. Writing to the wrong disk will destroy data!**

```powershell
# List all disks to identify your SD card
# Look for the removable disk with matching size
Get-Disk | Select-Object Number, FriendlyName, Size, MediaType

# Example output:
# Number FriendlyName     Size          MediaType
# ------ ------------     ----          ---------
# 0      NVMe SSD         512GB         SSD
# 1      USB Hard Drive   1000GB        HDD
# 2      SD Card          31.9GB        Removable  <- Your SD card

# Set the disk number (replace 2 with your actual SD card disk number)
$diskNumber = 2

# Get the physical drive path
$physicalDrive = "\\.\PhysicalDrive$diskNumber"

# Verify before writing (optional but recommended)
# Check that it's the right size (should be ~256MB for webbos-pi.img)
$disk = Get-Disk -Number $diskNumber
Write-Host "Selected disk: $($disk.FriendlyName), Size: $([math]::Round($disk.Size/1MB, 2)) MB"
Read-Host "Press Enter to continue or Ctrl+C to abort"

# Write the image using fsutil and copy
cd Pi
fsutil file createnew temp.pad 0
$image = [System.IO.File]::OpenRead("webbos-pi.img")
$diskStream = [System.IO.FileStream]::new($physicalDrive, [System.IO.FileMode]::Open, [System.IO.FileAccess]::Write)
Write-Host "Writing image..."
$image.CopyTo($diskStream)
$diskStream.Close()
$image.Close()
Write-Host "Done!"
```

## Option 2: macOS

### Using Terminal

⚠️ **WARNING: Be extremely careful with the disk identifier. Writing to the wrong disk will destroy data!**

```bash
# List all disks to identify your SD card
# Look for something like /dev/disk2 (external, physical)
diskutil list

# Example output:
# /dev/disk0 (internal, physical):
#    #:                       TYPE NAME                    SIZE       IDENTIFIER
#    0:      GUID_partition_scheme                        512.1 GB   disk0
#
# /dev/disk2 (external, physical):          <- Your SD card
#    #:                       TYPE NAME                    SIZE       IDENTIFIER
#    0:     FDisk_partition_scheme                        *31.9 GB   disk2

# Unmount the SD card (don't eject)
# Replace disk2 with your actual disk identifier
diskutil unmountDisk /dev/disk2

# Write the image
# Use rdisk2 (raw disk) for faster writing
# Replace disk2 with your actual disk identifier
sudo dd if=Pi/webbos-pi.img of=/dev/rdisk2 bs=4m status=progress

# OR if status=progress is not supported:
sudo dd if=Pi/webbos-pi.img of=/dev/rdisk2 bs=4m

# Flush writes
sync

# Eject the SD card
sudo diskutil eject /dev/rdisk2
```

## Option 3: Linux

### Using Terminal

⚠️ **WARNING: Be extremely careful with the device name. Writing to the wrong device will destroy data!**

```bash
# List all block devices to identify your SD card
# Look for something like /dev/sdb or /dev/mmcblk0
lsblk

# Example output:
# NAME        MAJ:MIN RM   SIZE RO TYPE MOUNTPOINT
# sda           8:0    0   512G  0 disk 
# ├─sda1        8:1    0   512G  0 part /
# sdb           8:16   1  29.7G  0 disk       <- Your SD card (no mountpoints)

# Unmount any mounted partitions
# Replace sdb with your actual device
sudo umount /dev/sdb1 /dev/sdb2 2>/dev/null

# Write the image
# Replace sdb with your actual device
# Do NOT include partition numbers (sdb1, sdb2)
sudo dd if=Pi/webbos-pi.img of=/dev/sdb bs=4M status=progress conv=fsync

# OR for older versions of dd without status:
sudo dd if=Pi/webbos-pi.img of=/dev/sdb bs=4M && sync

# Verify the write (optional)
sudo cmp -n 268435456 Pi/webbos-pi.img /dev/sdb
```

### Using GNOME Disks / Disks Utility

1. Insert SD card
2. Open "Disks" application
3. Select your SD card from the left panel
4. Click the menu button (three dots) in the top right
5. Select **"Restore Disk Image..."**
6. Choose `webbos-pi.img`
7. Click **"Start Restoring..."**
8. Enter password and confirm
9. Wait for completion

## Booting WebbOS

1. Insert the prepared SD card into your Raspberry Pi
2. Connect hardware:
   - HDMI monitor (required)
   - USB keyboard (required)
   - USB mouse (recommended)
   - Ethernet cable (optional, for network)
3. Connect power to the Pi
4. WebbOS will boot and show the login screen

## Troubleshooting

### Black Screen
- **QEMU limitation**: The VideoCore mailbox framebuffer used by WebbOS is not emulated in QEMU
- **Solution**: You must use real Raspberry Pi hardware to see display output

### "Kernel panic" or boot loop
- Try a different SD card (some cards are incompatible)
- Re-flash the image
- Check that your Pi model is supported (3B, 3B+, 4B)

### No keyboard/mouse input
- Connect USB devices directly to the Pi, not through a hub (initially)
- Try different USB ports
- Ensure devices are USB 2.0 compatible

### "No SD card" error
- Ensure the SD card is fully inserted
- Try reformatting and re-flashing
- Check that the SD card lock switch is in the unlocked position

### Corrupted display
- Use a different HDMI cable
- Try a different monitor
- For Pi 4, ensure you're using the correct HDMI port (micro-HDMI)

## Hardware Compatibility

| Component | Status | Notes |
|-----------|--------|-------|
| Raspberry Pi 3B | ✅ Supported | Full compatibility |
| Raspberry Pi 3B+ | ✅ Supported | Full compatibility |
| Raspberry Pi 4B | ✅ Supported | Use micro-HDMI port |
| HDMI Display | ✅ Required | 1024x768 or higher recommended |
| USB Keyboard | ✅ Required | Any standard USB keyboard |
| USB Mouse | ✅ Recommended | Any standard USB mouse |
| SD Card | ✅ Required | 8GB minimum, Class 10 recommended |
| Ethernet | ⚠️ Partial | Driver present, testing needed |
| WiFi | ⚠️ Partial | Driver present, firmware needed |
| Audio | ❌ Not supported | Not implemented |
| Bluetooth | ❌ Not supported | Not implemented |

## Building a Custom Image

To create a fresh SD card image with latest changes:

```bash
# Windows PowerShell
cd Pi
python scripts/create-sdcard.py --size 256 --output webbos-pi.img target/aarch64-unknown-none/release/kernel
python scripts/add-all-apps.py

# The resulting webbos-pi.img can be written to SD card using methods above
```

## Verifying the SD Card

After writing, you can verify the card contents:

```bash
# Linux/macOS
# Mount the boot partition and list files
mkdir -p /tmp/webbos_boot
sudo mount /dev/sdb1 /tmp/webbos_boot  # Replace sdb with your device
ls -la /tmp/webbos_boot/
ls -la /tmp/webbos_boot/Apps/
sudo umount /tmp/webbos_boot
```

## Additional Resources

- [Hardware Documentation](HARDWARE.md) - Detailed hardware support info
- [Browser Implementation](BROWSER_IMPLEMENTATION.md) - Browser engine roadmap
- [STATUS.md](../STATUS.md) - Current project status

## Safety Warning

⚠️ **ALWAYS triple-check the disk/device identifier before writing.**

Writing to the wrong disk will:
- Erase your computer's operating system
- Destroy all data on the target disk
- Potentially make your computer unbootable

When in doubt:
1. Remove all other external drives
2. Double-check the disk size matches your SD card
3. Verify the media type shows "Removable"
4. Make a backup of important data first
