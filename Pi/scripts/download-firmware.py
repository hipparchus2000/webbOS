#!/usr/bin/env python3
"""
Download Raspberry Pi firmware files for SD card boot.

This script downloads the necessary proprietary firmware files from the
official Raspberry Pi firmware repository.

Required files for boot:
- bootcode.bin (Pi 1-3) or start4.elf (Pi 4)
- fixup.dat / fixup4.dat
- start.elf / start4.elf
- Device tree blobs (*.dtb)
- Device tree overlays (overlays/*.dtbo)

Usage:
    python download-firmware.py [--output-dir DIR]
"""

import os
import sys
import urllib.request
import urllib.error
import argparse

# GitHub raw content base URL
FIRMWARE_BASE_URL = "https://github.com/raspberrypi/firmware/raw/master/boot"

# Essential firmware files
REQUIRED_FILES = [
    "bootcode.bin",
    "start.elf",
    "start4.elf",
    "start4cd.elf",
    "start4x.elf",
    "fixup.dat",
    "fixup4.dat",
    "fixup4cd.dat",
    "fixup4x.dat",
    "LICENCE.broadcom",
]

# Device tree blobs for different Pi models
DTB_FILES = [
    "bcm2708-rpi-b.dtb",           # Pi 1 B
    "bcm2708-rpi-b-plus.dtb",      # Pi 1 B+
    "bcm2709-rpi-2-b.dtb",         # Pi 2 B
    "bcm2710-rpi-3-b.dtb",         # Pi 3 B
    "bcm2710-rpi-3-b-plus.dtb",    # Pi 3 B+
    "bcm2711-rpi-4-b.dtb",         # Pi 4 B
    "bcm2711-rpi-400.dtb",         # Pi 400
    "bcm2711-rpi-cm4.dtb",         # Pi CM4
]

# Common overlay files
OVERLAY_FILES = [
    "disable-bt.dtbo",
    "disable-wifi.dtbo",
    "uart0.dtbo",
    "uart1.dtbo",
    "uart2.dtbo",
    "uart3.dtbo",
    "uart4.dtbo",
    "uart5.dtbo",
    "vc4-kms-v3d.dtbo",
    "vc4-fkms-v3d.dtbo",
]


def download_file(url, output_path):
    """Download a single file from URL."""
    try:
        urllib.request.urlretrieve(url, output_path)
        return True
    except urllib.error.HTTPError as e:
        if e.code == 404:
            return None  # File not found (not an error for optional files)
        raise


def download_firmware(output_dir, include_dtbs=True, include_overlays=True):
    """Download all firmware files."""
    os.makedirs(output_dir, exist_ok=True)
    
    print("Downloading Raspberry Pi firmware files...")
    print(f"Output directory: {output_dir}")
    print()
    
    # Download required firmware files
    print("Downloading core firmware files...")
    downloaded = []
    failed = []
    
    for filename in REQUIRED_FILES:
        url = f"{FIRMWARE_BASE_URL}/{filename}"
        output_path = os.path.join(output_dir, filename)
        
        print(f"  {filename}... ", end="", flush=True)
        result = download_file(url, output_path)
        
        if result is True:
            print("OK")
            downloaded.append(filename)
        elif result is None:
            print("Not found (optional)")
        else:
            print("FAILED")
            failed.append(filename)
    
    # Download device tree blobs
    if include_dtbs:
        print("\nDownloading device tree blobs...")
        dtb_dir = output_dir
        os.makedirs(dtb_dir, exist_ok=True)
        
        for filename in DTB_FILES:
            url = f"{FIRMWARE_BASE_URL}/{filename}"
            output_path = os.path.join(dtb_dir, filename)
            
            print(f"  {filename}... ", end="", flush=True)
            result = download_file(url, output_path)
            
            if result is True:
                print("OK")
                downloaded.append(filename)
            elif result is None:
                print("Not found")
            else:
                print("FAILED")
                failed.append(filename)
    
    # Download overlays
    if include_overlays:
        print("\nDownloading device tree overlays...")
        overlays_dir = os.path.join(output_dir, "overlays")
        os.makedirs(overlays_dir, exist_ok=True)
        
        for filename in OVERLAY_FILES:
            url = f"{FIRMWARE_BASE_URL}/overlays/{filename}"
            output_path = os.path.join(overlays_dir, filename)
            
            print(f"  overlays/{filename}... ", end="", flush=True)
            result = download_file(url, output_path)
            
            if result is True:
                print("OK")
                downloaded.append(f"overlays/{filename}")
            elif result is None:
                print("Not found (optional)")
            else:
                print("FAILED")
                failed.append(f"overlays/{filename}")
    
    # Summary
    print()
    print("=" * 50)
    print(f"Download complete: {len(downloaded)} files")
    
    if failed:
        print(f"Failed: {len(failed)} files")
        for f in failed:
            print(f"  - {f}")
    
    print()
    print("Files saved to:", output_dir)
    print()
    print("Next steps:")
    print("  1. Create SD card image:")
    print(f"     python create-sdcard.py --firmware-dir {output_dir} --include-firmware")
    print("  2. Or copy files manually to SD card boot partition")


def main():
    parser = argparse.ArgumentParser(
        description='Download Raspberry Pi firmware files',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Download to default directory (pi-firmware)
  python download-firmware.py
  
  # Download to specific directory
  python download-firmware.py --output-dir firmware
  
  # Download only core firmware (no DTBs or overlays)
  python download-firmware.py --no-dtbs --no-overlays
        """
    )
    
    parser.add_argument('-o', '--output-dir', default='pi-firmware',
                        help='Output directory (default: pi-firmware)')
    parser.add_argument('--no-dtbs', action='store_true',
                        help='Skip downloading device tree blobs')
    parser.add_argument('--no-overlays', action='store_true',
                        help='Skip downloading device tree overlays')
    
    args = parser.parse_args()
    
    try:
        download_firmware(
            args.output_dir,
            include_dtbs=not args.no_dtbs,
            include_overlays=not args.no_overlays
        )
    except KeyboardInterrupt:
        print("\nDownload interrupted by user")
        sys.exit(1)
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == '__main__':
    main()
