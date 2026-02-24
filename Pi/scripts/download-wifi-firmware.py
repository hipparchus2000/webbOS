#!/usr/bin/env python3
"""
Download WiFi firmware files for Raspberry Pi.

This script downloads the necessary Broadcom/Cypress WiFi firmware files
from the official Linux firmware repository for BCM43438 (Pi 3) and 
BCM43455 (Pi 4) wireless chips.

The firmware files are required for the WebbOS WiFi driver to function.
Without these files, the WiFi chip cannot initialize or connect to networks.

Usage:
    python download-wifi-firmware.py [--output-dir DIR]

Output files:
    For Pi 3 (BCM43438):
        - brcmfmac43430-sdio.bin      (main firmware)
        - brcmfmac43430-sdio.clm_blob (calibration data)
        - brcmfmac43430-sdio.txt      (NVRAM configuration)
    
    For Pi 4 (BCM43455):
        - brcmfmac43455-sdio.bin      (main firmware)
        - brcmfmac43455-sdio.clm_blob (calibration data)
        - brcmfmac43455-sdio.txt      (NVRAM configuration)

The files are placed in the correct directory structure:
    <output-dir>/brcm/

Example:
    # Download to default directory (pi-wifi-firmware)
    python download-wifi-firmware.py
    
    # Download to specific directory
    python download-wifi-firmware.py --output-dir firmware/wifi
    
    # Then create SD card with WiFi firmware
    python create-sdcard.py --wifi-firmware-dir pi-wifi-firmware
"""

import os
import sys
import urllib.request
import urllib.error
import argparse

# Linux firmware repository base URL
LINUX_FIRMWARE_URL = "https://git.kernel.org/pub/scm/linux/kernel/git/firmware/linux-firmware.git/plain/brcm"

# WiFi firmware files for each Pi model
WIFI_FIRMWARE = {
    "pi3": {
        "chip": "BCM43438",
        "files": [
            ("brcmfmac43430-sdio.bin", "Main firmware binary"),
            ("brcmfmac43430-sdio.clm_blob", "Calibration data (CLM)"),
            ("brcmfmac43430-sdio.txt", "NVRAM configuration"),
        ],
    },
    "pi4": {
        "chip": "BCM43455",
        "files": [
            ("brcmfmac43455-sdio.bin", "Main firmware binary"),
            ("brcmfmac43455-sdio.clm_blob", "Calibration data (CLM)"),
            ("brcmfmac43455-sdio.txt", "NVRAM configuration"),
        ],
    },
}

# Alternative URLs in case the main one fails
ALT_URLS = [
    "https://raw.githubusercontent.com/RPi-Distro/firmware-nonfree/master/debian/config/brcm80211/brcm",
    "https://raw.githubusercontent.com/RPi-Distro/firmware-nonfree/bullseye/debian/config/brcm80211/brcm",
]


def download_file(url, output_path, description=""):
    """Download a single file from URL."""
    try:
        print(f"  Downloading {description}... ", end="", flush=True)
        
        # Create request with user agent to avoid blocks
        req = urllib.request.Request(
            url,
            headers={
                'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.0'
            }
        )
        
        with urllib.request.urlopen(req, timeout=30) as response:
            data = response.read()
            
        # Write to file
        os.makedirs(os.path.dirname(output_path), exist_ok=True)
        with open(output_path, 'wb') as f:
            f.write(data)
        
        print(f"OK ({len(data)} bytes)")
        return True
        
    except urllib.error.HTTPError as e:
        if e.code == 404:
            print(f"NOT FOUND (404)")
            return False
        else:
            print(f"HTTP ERROR {e.code}")
            return False
    except urllib.error.URLError as e:
        print(f"NETWORK ERROR: {e.reason}")
        return False
    except Exception as e:
        print(f"ERROR: {e}")
        return False


def try_download_with_fallback(filename, output_dir):
    """Try to download from multiple sources."""
    output_path = os.path.join(output_dir, "brcm", filename)
    
    # Try main URL first
    urls = [f"{LINUX_FIRMWARE_URL}/{filename}"]
    
    # Add alternative URLs
    for alt_base in ALT_URLS:
        urls.append(f"{alt_base}/{filename}")
    
    for url in urls:
        if download_file(url, output_path, filename):
            return True
    
    return False


def download_wifi_firmware(output_dir, pi_models=None):
    """Download all WiFi firmware files."""
    if pi_models is None:
        pi_models = ["pi3", "pi4"]
    
    os.makedirs(output_dir, exist_ok=True)
    
    print("=" * 60)
    print("Raspberry Pi WiFi Firmware Downloader")
    print("=" * 60)
    print()
    print(f"Output directory: {output_dir}")
    print(f"Models: {', '.join(pi_models).upper()}")
    print()
    
    downloaded = []
    failed = []
    total_size = 0
    
    for model in pi_models:
        if model not in WIFI_FIRMWARE:
            print(f"Unknown model: {model}")
            continue
        
        info = WIFI_FIRMWARE[model]
        chip = info["chip"]
        
        print(f"\n{'='*60}")
        print(f"Raspberry Pi {model.upper()} ({chip})")
        print(f"{'='*60}")
        
        for filename, description in info["files"]:
            print(f"\n  {description}:")
            print(f"    File: {filename}")
            
            if try_download_with_fallback(filename, output_dir):
                output_path = os.path.join(output_dir, "brcm", filename)
                size = os.path.getsize(output_path)
                total_size += size
                downloaded.append((filename, size))
            else:
                failed.append(filename)
                print(f"    FAILED - Tried multiple sources")
    
    # Summary
    print()
    print("=" * 60)
    print("DOWNLOAD SUMMARY")
    print("=" * 60)
    print(f"Successfully downloaded: {len(downloaded)} files")
    print(f"Total size: {total_size / 1024:.1f} KB")
    
    if downloaded:
        print("\nDownloaded files:")
        for filename, size in downloaded:
            print(f"  ✓ {filename} ({size / 1024:.1f} KB)")
    
    if failed:
        print(f"\nFailed downloads: {len(failed)} files")
        for filename in failed:
            print(f"  ✗ {filename}")
        print("\nYou may need to download these files manually from:")
        print("  - https://git.kernel.org/pub/scm/linux/kernel/git/firmware/linux-firmware.git/tree/brcm")
        print("  - https://github.com/RPi-Distro/firmware-nonfree")
    
    print()
    print("Files saved to:")
    print(f"  {os.path.join(output_dir, 'brcm')}")
    
    # Print next steps
    print()
    print("=" * 60)
    print("NEXT STEPS")
    print("=" * 60)
    print()
    print("1. Include WiFi firmware in SD card image:")
    print(f"   python create-sdcard.py --wifi-firmware-dir {output_dir}")
    print()
    print("2. Or manually copy firmware to SD card:")
    print("   - Mount the boot partition")
    print("   - Create /firmware/brcm/ directory")
    print("   - Copy all .bin, .txt, and .clm_blob files")
    print()
    print("3. Boot WebbOS and test WiFi:")
    print("   > wifi scan")
    print("   > wifi connect <SSID> <password>")
    
    return len(failed) == 0


def verify_firmware(output_dir):
    """Verify that firmware files exist and are valid."""
    brcm_dir = os.path.join(output_dir, "brcm")
    
    if not os.path.exists(brcm_dir):
        return False
    
    all_valid = True
    
    print()
    print("Verifying firmware files...")
    print()
    
    for model in ["pi3", "pi4"]:
        info = WIFI_FIRMWARE[model]
        print(f"\n  {model.upper()} ({info['chip']}):")
        
        for filename, description in info["files"]:
            filepath = os.path.join(brcm_dir, filename)
            if os.path.exists(filepath):
                size = os.path.getsize(filepath)
                print(f"    ✓ {filename} ({size / 1024:.1f} KB)")
            else:
                print(f"    ✗ {filename} - MISSING")
                all_valid = False
    
    return all_valid


def main():
    parser = argparse.ArgumentParser(
        description='Download Raspberry Pi WiFi firmware files',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Download all firmware (Pi 3 and Pi 4)
  python download-wifi-firmware.py
  
  # Download to specific directory
  python download-wifi-firmware.py --output-dir firmware/wifi
  
  # Download only Pi 4 firmware
  python download-wifi-firmware.py --models pi4
  
  # Verify existing firmware
  python download-wifi-firmware.py --verify --output-dir pi-wifi-firmware
        """
    )
    
    parser.add_argument('-o', '--output-dir', default='pi-wifi-firmware',
                        help='Output directory (default: pi-wifi-firmware)')
    parser.add_argument('--models', default='pi3,pi4',
                        help='Comma-separated list of models (default: pi3,pi4)')
    parser.add_argument('--verify', action='store_true',
                        help='Verify existing firmware files without downloading')
    
    args = parser.parse_args()
    
    # Parse models
    models = [m.strip().lower() for m in args.models.split(',')]
    valid_models = ['pi3', 'pi4']
    models = [m for m in models if m in valid_models]
    
    if not models:
        print("Error: No valid models specified. Use: pi3, pi4")
        sys.exit(1)
    
    try:
        if args.verify:
            if verify_firmware(args.output_dir):
                print("\n✓ All firmware files are present")
                sys.exit(0)
            else:
                print("\n✗ Some firmware files are missing")
                sys.exit(1)
        else:
            success = download_wifi_firmware(args.output_dir, models)
            sys.exit(0 if success else 1)
            
    except KeyboardInterrupt:
        print("\n\nDownload interrupted by user")
        sys.exit(1)
    except Exception as e:
        print(f"\nError: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == '__main__':
    main()
