# BCM43438/BCM43455 WiFi Driver

This directory contains the WiFi driver implementation for Raspberry Pi's built-in wireless chip.

## Overview

The BCM43438 (Pi 3) and BCM43455 (Pi 4) are FullMAC WiFi chips that communicate via SDIO. The driver consists of:

- **SDIO Host Driver** (`../sdio/mod.rs`) - Low-level SDIO communication
- **BCM43438 Driver** (`bcm43438.rs`) - Main WiFi driver
- **SDIO-over-SPI Fallback** (`sdio_spi.rs`) - Alternative SPI-based SDIO

## Architecture

### SDIO Communication

The chip uses three SDIO functions:
- **Function 0**: SDIO bus (standard SDIO registers)
- **Function 1**: Backplane (core register access, firmware download)
- **Function 2**: WLAN data (SDPCM packet transfer)

### Protocol Stack

```
User Application
      |
Network Stack (TCP/IP)
      |
NetworkInterface trait
      |
  BCM43438 Driver
      |
SDIO Core (Function 2) / Backplane (Function 1)
      |
SDIO Host Controller (Ar SDHCI)
      |
BCM43438/BCM43455 Hardware
```

### SDPCM Protocol

The Broadcom SDIO Protocol for Control and Management (SDPCM) is used:

1. **Control Channel (0)**: IOCTL commands/responses
2. **Event Channel (1)**: Asynchronous events (connect/disconnect/link status)
3. **Data Channel (2)**: Ethernet frame encapsulation (BDC header + 802.3 frame)

## Firmware Files

Firmware must be loaded from SD card:

**Raspberry Pi 3 (BCM43438):**
- `/lib/firmware/brcm/brcmfmac43430-sdio.bin` - Main firmware
- `/lib/firmware/brcm/brcmfmac43430-sdio.clm_blob` - Calibration data
- `/lib/firmware/brcm/brcmfmac43430-sdio.txt` - NVRAM configuration

**Raspberry Pi 4 (BCM43455):**
- `/lib/firmware/brcm/brcmfmac43455-sdio.bin` - Main firmware
- `/lib/firmware/brcm/brcmfmac43455-sdio.clm_blob` - Calibration data
- `/lib/firmware/brcm/brcmfmac43455-sdio.txt` - NVRAM configuration

## Usage

### Initialization

```rust
// In kernel main.rs or platform initialization
use kernel::drivers;

// Initialize Pi-specific drivers (pass true for Pi 4)
drivers::init_pi_drivers(false); // For Pi 3
```

### Scanning Networks

```rust
if let Some(device) = bcm43438::device() {
    let networks = device.lock().scan()?;
    for net in networks {
        println!("Found: {} ({} dBm)", 
            net.ssid_string(), 
            net.rssi
        );
    }
}
```

### Connecting

```rust
let config = WiFiConfig::with_password(
    b"MyNetwork",
    b"MyPassword",
    SecurityType::Wpa2
);

device.lock().connect(&config.ssid, Some(&config.password))?;
```

## Implementation Status

### Completed
- ✅ SDIO host controller driver (CMD52/CMD53)
- ✅ BCM43438/BCM43455 driver structure
- ✅ NetworkInterface trait implementation
- ✅ SDPCM protocol frame structures
- ✅ IOCTL command definitions
- ✅ SDIO-over-SPI fallback

### Pending (requires firmware loading)
- ⏳ Firmware binary loading from SD card
- ⏳ Firmware download protocol
- ⏳ NVRAM configuration parsing
- ⏳ Full IOCTL response handling
- ⏳ Scan result processing
- ⏳ Event handling loop
- ⏳ Power management

## Hardware Notes

### SDIO Base Addresses
- **Pi 3**: 0x3F300000 (BCM2837)
- **Pi 4**: 0xFE300000 (BCM2711)

### Clock Configuration
- Initialization: 400 KHz
- Operational: 50 MHz (high speed mode)

### SPI Pinout (Fallback Mode)
- GPIO 9 (Pin 21): MISO
- GPIO 10 (Pin 19): MOSI
- GPIO 11 (Pin 23): SCLK
- GPIO 8 (Pin 24): CS0

## References

- BCM43438 Datasheet (Broadcom)
- SDIO Simplified Specification (SD Association)
- SD Host Controller Simplified Specification
- Linux brcmfmac driver (reference implementation)
