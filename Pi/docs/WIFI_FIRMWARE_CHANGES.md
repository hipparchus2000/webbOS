# WiFi Firmware Support - Implementation Summary

## What Was Implemented

### 1. Firmware Loader Module (`kernel/src/drivers/wifi/firmware_loader.rs`)

A new module that handles loading WiFi firmware files from the SD card filesystem:

**Features:**
- `load_firmware_files()` - Loads firmware binary, NVRAM config, and CLM blob from `/firmware/brcm/`
- `parse_nvram()` - Parses NVRAM text files into key-value pairs
- `validate_firmware_header()` - Validates firmware integrity
- `get_mac_address_from_nvram()` - Extracts MAC address from NVRAM parameters
- `build_nvram_binary()` - Builds binary NVRAM blob for chip download
- `check_firmware_available()` - Checks if firmware files exist
- `print_firmware_info()` - Prints firmware status for debugging

**Supported Hardware:**
- Raspberry Pi 3 / 3B+ (BCM43438)
- Raspberry Pi 4 / 4B (BCM43455)

### 2. Updated WiFi Driver (`kernel/src/drivers/wifi/bcm43438.rs`)

Modified the `load_firmware()` method to:
- Use the new firmware loader to read files from SD card
- Validate firmware files are present and readable
- Extract and display MAC address from NVRAM
- Report firmware sizes and status

### 3. Firmware Download Script (`scripts/download-wifi-firmware.py`)

New Python script that:
- Downloads WiFi firmware from official Linux firmware repository
- Supports Pi 3 (BCM43438) and Pi 4 (BCM43455)
- Has fallback URLs if primary source fails
- Creates proper directory structure (`pi-wifi-firmware/brcm/`)
- Includes verification mode (`--verify` flag)

**Usage:**
```bash
# Download all firmware
python scripts/download-wifi-firmware.py

# Download to specific directory
python scripts/download-wifi-firmware.py --output-dir firmware/wifi

# Download only Pi 4 firmware
python scripts/download-wifi-firmware.py --models pi4

# Verify existing firmware
python scripts/download-wifi-firmware.py --verify
```

### 4. Updated SD Card Creation Script (`scripts/create-sdcard.py`)

Added `--wifi-firmware-dir` option to include WiFi firmware in the SD card image:

**Usage:**
```bash
# Create SD card with WiFi firmware
python scripts/create-sdcard.py --wifi-firmware-dir pi-wifi-firmware

# Create with both GPU and WiFi firmware
python scripts/create-sdcard.py \
    --firmware-dir pi-firmware \
    --wifi-firmware-dir pi-wifi-firmware
```

### 5. Documentation (`docs/WIFI_SETUP.md`)

Comprehensive WiFi setup guide covering:
- Why firmware is required
- Step-by-step setup instructions
- Manual firmware installation
- Troubleshooting common issues
- Firmware sources and download links
- Technical details about the driver
- Current limitations

## Firmware Files Required

### For Raspberry Pi 3 (BCM43438):
| File | Size | Purpose |
|------|------|---------|
| `brcmfmac43430-sdio.bin` | ~300KB | Main firmware binary |
| `brcmfmac43430-sdio.clm_blob` | ~10KB | Calibration data |
| `brcmfmac43430-sdio.txt` | ~2KB | NVRAM configuration |

### For Raspberry Pi 4 (BCM43455):
| File | Size | Purpose |
|------|------|---------|
| `brcmfmac43455-sdio.bin` | ~500KB | Main firmware binary |
| `brcmfmac43455-sdio.clm_blob` | ~15KB | Calibration data |
| `brcmfmac43455-sdio.txt` | ~2KB | NVRAM configuration |

## Current Status

### What's Working ✅
- Firmware file loading from SD card
- NVRAM parsing and validation
- MAC address extraction
- Firmware integrity checks
- File presence detection

### What's Partially Working ⚠️
- Firmware binary validation (reads and validates, but doesn't download to chip)
- Driver initialization (loads files, but doesn't fully boot firmware)

### What's Not Yet Implemented ❌
- Full firmware binary download to chip RAM
- SDPCM event handling loop
- Scan result processing
- WPA/WPA2 authentication
- DHCP client integration

## Next Steps for Full WiFi Support

1. **Implement Firmware Download Protocol**
   - Parse firmware binary sections
   - Transfer sections to chip memory via backplane
   - Validate download completion

2. **Implement SDPCM Protocol**
   - Event channel handling
   - Control channel (IOCTL) responses
   - Data channel packet processing

3. **Add Scan Support**
   - Send scan IOCTL commands
   - Parse scan result events
   - Cache and expose network list

4. **Add Connection Support**
   - WPA2 key negotiation (4-way handshake)
   - Association and authentication
   - DHCP client for IP configuration

## Testing

To test the firmware loading:

1. Download firmware:
   ```bash
   python scripts/download-wifi-firmware.py
   ```

2. Create SD card image:
   ```bash
   python scripts/create-sdcard.py --wifi-firmware-dir pi-wifi-firmware
   ```

3. Boot on Raspberry Pi and check serial output:
   ```
   [bcm43438] Loading firmware...
   [wifi/firmware] Loading firmware files...
   [wifi/firmware]  Reading: /firmware/brcm/brcmfmac43430-sdio.bin
   [wifi/firmware]   -> 371603 bytes
   [wifi/firmware]  Reading: /firmware/brcm/brcmfmac43430-sdio.txt
   [wifi/firmware]   -> 2073 bytes
   [wifi/firmware]   -> 45 NVRAM parameters
   [wifi/firmware] Firmware files loaded successfully
   ```

## Architecture

```
User Application
       |
Network Stack (TCP/IP)
       |
Network Interface Trait
       |
  BCM43438 Driver
       |-- firmware_loader (NEW)
       |     |
       |     +-- File System (VFS)
       |     |       |
       |     |       +-- FAT32 Driver
       |     |       |       |
       |     |       |       +-- SD Card
       |     |       |
       |     |       +-- Files loaded:
       |     |           - brcmfmac*.bin
       |     |           - brcmfmac*.txt
       |     |           - brcmfmac*.clm_blob
       |     |
       |     +-- Validation
       |     +-- NVRAM parsing
       |
SDIO Core (Function 2)
       |
SDIO Host Controller (Arasan SDHCI)
       |
BCM43438/BCM43455 Hardware
```

## Files Modified/Created

### New Files:
- `kernel/src/drivers/wifi/firmware_loader.rs`
- `scripts/download-wifi-firmware.py`
- `docs/WIFI_SETUP.md`
- `docs/WIFI_FIRMWARE_CHANGES.md`

### Modified Files:
- `kernel/src/drivers/wifi/mod.rs` - Added firmware_loader module
- `kernel/src/drivers/wifi/bcm43438.rs` - Updated load_firmware()
- `scripts/create-sdcard.py` - Added --wifi-firmware-dir option
- `README.md` - Updated WiFi status

## References

- Linux Firmware Repository: https://git.kernel.org/pub/scm/linux/kernel/git/firmware/linux-firmware.git
- Raspberry Pi Firmware: https://github.com/RPi-Distro/firmware-nonfree
- Linux brcmfmac Driver: https://github.com/torvalds/linux/tree/master/drivers/net/wireless/broadcom/brcm80211/brcmfmac
