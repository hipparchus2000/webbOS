#!/usr/bin/env python3
"""Create a combined bootloader+kernel image for Raspberry Pi."""

import sys
import os

def create_image(bootloader_path, kernel_path, output_path):
    """Create combined image with bootloader at 0x80000 and kernel at 0x100000."""
    
    with open(bootloader_path, 'rb') as f:
        bootloader = f.read()
    
    with open(kernel_path, 'rb') as f:
        kernel = f.read()
    
    print(f"Bootloader size: {len(bootloader)} bytes")
    print(f"Kernel size: {len(kernel)} bytes")
    
    # Bootloader starts at 0x80000 (512KB)
    # Kernel starts at 0x100000 (1MB)
    boot_offset = 0x80000
    kernel_offset = 0x100000
    
    # Total image size must cover both
    total_size = kernel_offset + len(kernel)
    
    # Create image
    image = bytearray(total_size)
    
    # Place bootloader at 0x80000
    image[boot_offset:boot_offset + len(bootloader)] = bootloader
    
    # Place kernel at 0x100000
    image[kernel_offset:kernel_offset + len(kernel)] = kernel
    
    with open(output_path, 'wb') as f:
        f.write(image)
    
    print(f"Created {output_path}: {len(image)} bytes ({len(image)/1024/1024:.2f} MB)")
    print(f"  Bootloader at: 0x{boot_offset:06X}")
    print(f"  Kernel at: 0x{kernel_offset:06X}")

if __name__ == '__main__':
    bootloader = sys.argv[1] if len(sys.argv) > 1 else 'target/aarch64-unknown-none/release/bootloader'
    kernel = sys.argv[2] if len(sys.argv) > 2 else 'target/aarch64-unknown-none/release/kernel'
    output = sys.argv[3] if len(sys.argv) > 3 else 'webbos-pi-boot.img'
    
    create_image(bootloader, kernel, output)
