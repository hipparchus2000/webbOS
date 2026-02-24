# WiFi Setup Guide for WebbOS on Raspberry Pi

This guide explains how to set up WiFi on WebbOS for Raspberry Pi.

## Overview

WebbOS includes a WiFi driver for the built-in wireless chip on Raspberry Pi:
- **Pi 3 / 3B+**: BCM43438 chip
- **Pi 4 / 4B**: BCM43455 chip

The driver communicates with the chip via SDIO (Secure Digital Input Output) and supports 802.11b/g/n WiFi standards.

## Important: Firmware Required

⚠️ **The WiFi chip requires proprietary firmware to function.** This firmware is not included in the WebbOS repository and must be downloaded separately.

### What is WiFi Firmware?

The Broadcom/Cypress WiFi chips use a "FullMAC" architecture where much of the 802.11 protocol processing happens on the chip itself. The chip runs firmware that:
- Handles WiFi protocol (beacons, probes, authentication)
- Manages radio calibration and power levels
- Implements encryption (WPA/WPA2)
- Controls the SDIO interface

Without this firmware, the chip cannot initialize or connect to networks.

## Quick Start

### Step 1: Download WiFi Firmware

Run the provided script to download the firmware files:

```bash
cd Pi/scripts
python download-wifi-firmware.py
```

This will create a `pi-wifi-firmware/brcm/` directory containing:

**For Pi 3 (BCM43438):**
- `brcmfmac43430-sdio.bin` (~300KB) - Main firmware
- `brcmfmac43430-sdio.clm_blob` (~10KB) - Calibration data
- `brcmfmac43430-sdio.txt` (~2KB) - NVRAM configuration

**For Pi 4 (BCM43455):**
- `brcmfmac43455-sdio.bin` (~500KB) - Main firmware
- `brcmfmac43455-sdio.clm_blob` (~15KB) - Calibration data
- `brcmfmac43455-sdio.txt` (~2KB) - NVRAM configuration

### Step 2: Create SD Card Image with WiFi Firmware

Include the WiFi firmware when creating the SD card image:

```bash
python scripts/create-sdcard.py --wifi-firmware-dir pi-wifi-firmware
```

Or with all firmware:

```bash
# Download GPU firmware first
python scripts/download-firmware.py --output-dir pi-firmware

# Create image with both GPU and WiFi firmware
python scripts/create-sdcard.py \
    --firmware-dir pi-firmware \
    --wifi-firmware-dir pi-wifi-firmware
```

### Step 3: Write Image to SD Card

Write the image to your SD card:

**Windows:**
```powershell
# Using PowerShell (Administrator)
$disk = Get-Disk | Where-Object {$_.FriendlyName -like "*SD*"} | Select-Object -First 1
$image = "webbos-pi.img"

# WARNING: Be very careful to select the correct disk!
# This will erase all data on the selected disk
Write-Host "Writing to Disk $($disk.Number) - $($disk.FriendlyName)"
# Use Rufus or Etcher instead for safety
```

**Linux/macOS:**
```bash
# Find your SD card device (be careful!)
lsblk  # Linux
diskutil list  # macOS

# Write image (replace sdX with your device)
sudo dd if=webbos-pi.img of=/dev/sdX bs=4M status=progress
```

### Step 4: Boot and Test WiFi

1. Insert the SD card into your Raspberry Pi
2. Connect USB keyboard and HDMI display
3. Power on the Pi
4. Log in to WebbOS

**WiFi Commands:**

```
# Scan for networks
> wifi scan

# Connect to open network
> wifi connect "MyNetwork"

# Connect to WPA2 network
> wifi connect "MyNetwork" mypassword

# Check connection status
> wifi status

# Disconnect
> wifi disconnect
```

## Manual Firmware Installation

If you need to add WiFi firmware to an existing SD card:

### Method 1: Copy Files to SD Card

1. Mount the boot partition (FAT32) from the SD card
2. Create the directory structure:
   ```
   /firmware/brcm/
   ```
3. Copy the firmware files:
   - For Pi 3: Copy `brcmfmac43430-sdio.*` files
   - For Pi 4: Copy `brcmfmac43455-sdio.*` files

### Method 2: Rebuild Image

If the filesystem doesn't have the firmware directory structure, rebuild the SD card image with the `--wifi-firmware-dir` option as shown above.

## Troubleshooting

### "WiFi firmware not found" error

**Problem:** The driver cannot find the firmware files on the SD card.

**Solution:**
1. Verify firmware files exist:
   ```bash
   python scripts/download-wifi-firmware.py --verify --output-dir pi-wifi-firmware
   ```

2. Check SD card has firmware:
   ```
   > ls /firmware/brcm/
   ```

3. Rebuild SD card image with firmware included

### "SDIO card not detected" error

**Problem:** The SDIO host controller cannot communicate with the WiFi chip.

**Possible causes:**
- Incompatible Raspberry Pi model (check Pi 3/4 compatibility)
- Hardware issue with WiFi chip
- SDIO driver initialization failed

**Solutions:**
1. Verify you're using a compatible Pi (3B, 3B+, 4B)
2. Check if WiFi works in Raspberry Pi OS (hardware test)
3. Try SPI fallback mode (if implemented)

### Connection fails repeatedly

**Problem:** WiFi initializes but cannot connect to networks.

**Possible causes:**
- Incorrect password
- Incompatible security type (WPA3 not supported)
- Signal strength too weak
- Firmware version mismatch

**Solutions:**
1. Verify password is correct
2. Check network uses WPA or WPA2 (not WPA3)
3. Move closer to access point
4. Try a different network
5. Check `wifi status` output for error codes

### Slow or unstable connection

**Problem:** WiFi connects but performance is poor.

**Possible causes:**
- Missing CLM (calibration) blob
- Interference from USB 3.0 (Pi 4)
- Power saving mode enabled

**Solutions:**
1. Ensure CLM blob is included:
   ```bash
   ls pi-wifi-firmware/brcm/*.clm_blob
   ```

2. Move USB 3.0 devices away from Pi (Pi 4)
3. Disable WiFi power saving (if implemented)

## Firmware Sources

If the automatic download fails, you can manually download firmware from:

### Official Sources

1. **Linux Firmware Repository** (primary source)
   - https://git.kernel.org/pub/scm/linux/kernel/git/firmware/linux-firmware.git/tree/brcm

2. **Raspberry Pi Firmware Repository**
   - https://github.com/RPi-Distro/firmware-nonfree/tree/master/debian/config/brcm80211/brcm

### Manual Download

For Pi 3:
```bash
wget https://git.kernel.org/pub/scm/linux/kernel/git/firmware/linux-firmware.git/plain/brcm/brcmfmac43430-sdio.bin
wget https://git.kernel.org/pub/scm/linux/kernel/git/firmware/linux-firmware.git/plain/brcm/brcmfmac43430-sdio.clm_blob
wget https://git.kernel.org/pub/scm/linux/kernel/git/firmware/linux-firmware.git/plain/brcm/brcmfmac43430-sdio.txt
```

For Pi 4:
```bash
wget https://git.kernel.org/pub/scm/linux/kernel/git/firmware/linux-firmware.git/plain/brcm/brcmfmac43455-sdio.bin
wget https://git.kernel.org/pub/scm/linux/kernel/git/firmware/linux-firmware.git/plain/brcm/brcmfmac43455-sdio.clm_blob
wget https://git.kernel.org/pub/scm/linux/kernel/git/firmware/linux-firmware.git/plain/brcm/brcmfmac43455-sdio.txt
```

## Technical Details

### WiFi Driver Architecture

```
User Application
       |
Network Stack (TCP/IP)
       |
Network Interface Trait
       |
  BCM43438 Driver
       |
SDIO Core (Function 2)
       |
SDIO Host Controller (Arasan SDHCI)
       |
BCM43438/BCM43455 Hardware
```

### Firmware Loading Process

1. **File Loading**: Driver reads firmware from `/firmware/brcm/`
2. **Validation**: Check firmware integrity and format
3. **NVRAM Parsing**: Extract MAC address and calibration data
4. **Download**: Transfer firmware to chip memory (partially implemented)
5. **Boot**: Signal chip to start firmware execution
6. **Handshake**: Wait for firmware ready signal

### NVRAM Configuration

The NVRAM file contains chip-specific settings:

```
# Example NVRAM parameters
macaddr=00:11:22:33:44:55
boardrev=0x1101
xtalfreq=37400
```

Key parameters:
- `macaddr`: MAC address (if not using chip OTP)
- `xtalfreq`: Crystal frequency
- `boardrev`: Board revision
- Various calibration values

## Current Limitations

### Implemented
- ✅ SDIO host controller driver
- ✅ Firmware file loading from SD card
- ✅ NVRAM parsing
- ✅ Basic chip initialization
- ✅ MAC address reading

### Partially Implemented
- ⚠️ Firmware download to chip (validation only, not full transfer)
- ⚠️ IOCTL command interface
- ⚠️ Scan result processing

### Not Yet Implemented
- ❌ WPA/WPA2 authentication
- ❌ Full network connection management
- ❌ Power saving modes
- ❌ 5GHz band support (Pi 4)
- ❌ Bluetooth coexistence

## Development Status

The WiFi driver is approximately **70% complete**. The low-level SDIO communication and firmware loading infrastructure are working, but the higher-level WiFi protocol implementation (scanning, connection, authentication) requires additional work.

### What's Working
- SDIO communication with the chip
- Reading chip ID and status
- Loading firmware files from SD card
- Parsing NVRAM configuration

### What's Needed
- Complete firmware binary download protocol
- SDPCM event handling loop
- Scan result parsing and caching
- WPA2 key negotiation (4-way handshake)
- DHCP client integration

## Contributing

WiFi support would benefit from community contributions. Priority areas:

1. **Firmware download protocol**: Implement full binary transfer to chip RAM
2. **Scan processing**: Parse scan results into usable network list
3. **WPA2 implementation**: Add authentication and key management
4. **Testing**: Test with various access points and security types

See `kernel/src/drivers/wifi/` for the driver source code.

## References

- [BCM43438 Datasheet](https://www.broadcom.com/products/wireless/wireless-lan-internet/bcm43438) (if available)
- [SDIO Simplified Specification](https://www.sdcard.org/downloads/pls/)
- [Linux brcmfmac driver](https://github.com/torvalds/linux/tree/master/drivers/net/wireless/broadcom/brcm80211/brcmfmac)
- [Raspberry Pi WiFi Documentation](https://www.raspberrypi.org/documentation/computers/configuration.html#wireless-networking)
