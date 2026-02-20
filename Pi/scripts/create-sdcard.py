#!/usr/bin/env python3
"""
Create a Raspberry Pi SD card image with MBR partition table.

The SD card image has:
- Partition 1: FAT32 boot partition (256MB, type 0x0C, bootable)
- Partition 2: ext4 root partition (remaining space, type 0x83)

The boot partition contains:
- bootcode.bin / start4.elf - GPU firmware
- fixup.dat / fixup4.dat - GPU memory fixup
- config.txt - Boot configuration
- cmdline.txt - Kernel command line
- kernel8.img - 64-bit kernel
- *.dtb - Device tree blobs
- overlays/*.dtbo - Device tree overlays
- firmware/brcm/ - WiFi firmware (optional)

Usage:
    python create-sdcard.py [--size SIZE_MB] [--output IMAGE_PATH] [KERNEL]
    
Examples:
    # Create 2GB image with kernel
    python create-sdcard.py
    
    # Create with custom kernel path
    python create-sdcard.py target/aarch64-unknown-none/release/kernel
    
    # Create larger image
    python create-sdcard.py --size 4096 --output large-sdcard.img
"""

import sys
import struct
import os
import argparse

# Partition constants
PARTITION_TYPE_FAT32_LBA = 0x0C
PARTITION_TYPE_LINUX = 0x83
SECTOR_SIZE = 512

# Default partition sizes
DEFAULT_BOOT_PARTITION_SIZE_MB = 256
MIN_ROOT_PARTITION_MB = 128  # Minimum size for root partition

# Alignment: Start at 1MB (sector 2048) for optimal SD card performance
FIRST_PARTITION_START = 2048


def create_mbr_partition_entry(status, partition_type, start_lba, num_sectors):
    """Create a 16-byte MBR partition entry."""
    entry = bytearray(16)
    
    # Boot status (0x80 = active/bootable, 0x00 = inactive)
    entry[0] = status
    
    # CHS start address (ignored for LBA, but set to typical values)
    entry[1] = 0xFE  # Head
    entry[2] = 0xFF  # Sector/Cylinder high
    entry[3] = 0xFF  # Cylinder low
    
    # Partition type
    entry[4] = partition_type
    
    # CHS end address (ignored for LBA)
    entry[5] = 0xFE  # Head
    entry[6] = 0xFF  # Sector/Cylinder high
    entry[7] = 0xFF  # Cylinder low
    
    # Start LBA (4 bytes, little-endian)
    entry[8:12] = struct.pack('<I', start_lba)
    
    # Number of sectors (4 bytes, little-endian)
    entry[12:16] = struct.pack('<I', num_sectors)
    
    return bytes(entry)


def create_mbr(boot_partition_sectors, root_partition_sectors):
    """Create a 512-byte MBR with partition table."""
    mbr = bytearray(512)
    
    # Boot code (first 446 bytes) - just zeros for non-bootable MBR
    # Could add a simple bootloader here if needed
    mbr[0:446] = b'\x00' * 446
    
    # Partition table entries (4 entries, 16 bytes each)
    # Entry 1: FAT32 boot partition
    boot_partition = create_mbr_partition_entry(
        status=0x80,  # Bootable
        partition_type=PARTITION_TYPE_FAT32_LBA,
        start_lba=FIRST_PARTITION_START,
        num_sectors=boot_partition_sectors
    )
    mbr[446:462] = boot_partition
    
    # Entry 2: Linux root partition
    root_partition = create_mbr_partition_entry(
        status=0x00,  # Not bootable
        partition_type=PARTITION_TYPE_LINUX,
        start_lba=FIRST_PARTITION_START + boot_partition_sectors,
        num_sectors=root_partition_sectors
    )
    mbr[462:478] = root_partition
    
    # Entries 3 and 4: Unused (zeros)
    mbr[478:494] = b'\x00' * 16
    mbr[494:510] = b'\x00' * 16
    
    # Boot signature
    mbr[510:512] = b'\x55\xAA'
    
    return bytes(mbr)


def create_fat32_boot_sector(start_sector, num_sectors, label="BOOT"):
    """
    Create a FAT32 boot sector for the boot partition.
    
    This is a simplified boot sector that sets up FAT32 for the
    Raspberry Pi firmware to read.
    """
    bytes_per_sector = SECTOR_SIZE
    sectors_per_cluster = 4  # 2KB clusters
    reserved_sectors = 32
    num_fats = 2
    root_cluster = 2
    
    # Calculate FAT size
    # Each cluster needs 4 bytes in FAT
    total_clusters = num_sectors // sectors_per_cluster
    sectors_per_fat = ((total_clusters * 4) + bytes_per_sector - 1) // bytes_per_sector
    
    boot_sector = bytearray(512)
    
    # Jump instruction
    boot_sector[0:3] = b'\xEB\x58\x90'
    
    # OEM name
    boot_sector[3:11] = b'MSDOS5.0'
    
    # BPB
    boot_sector[11:13] = struct.pack('<H', bytes_per_sector)
    boot_sector[13] = sectors_per_cluster
    boot_sector[14:16] = struct.pack('<H', reserved_sectors)
    boot_sector[16] = num_fats
    boot_sector[17:19] = struct.pack('<H', 0)  # Root entries (0 for FAT32)
    boot_sector[19:21] = struct.pack('<H', 0)  # Total sectors (0, use 32-bit field)
    boot_sector[21] = 0xF8  # Media descriptor
    boot_sector[22:24] = struct.pack('<H', 0)  # Sectors per FAT (0 for FAT32)
    boot_sector[24:26] = struct.pack('<H', 32)  # Sectors per track
    boot_sector[26:28] = struct.pack('<H', 64)  # Number of heads
    boot_sector[28:32] = struct.pack('<I', start_sector)  # Hidden sectors
    boot_sector[32:36] = struct.pack('<I', num_sectors)  # Total sectors (32-bit)
    
    # FAT32 specific fields
    boot_sector[36:40] = struct.pack('<I', sectors_per_fat)
    boot_sector[40:42] = struct.pack('<H', 0)  # Flags
    boot_sector[42:44] = struct.pack('<H', 0)  # Version
    boot_sector[44:48] = struct.pack('<I', root_cluster)  # Root cluster
    boot_sector[48:50] = struct.pack('<H', 1)  # FSInfo sector
    boot_sector[50:52] = struct.pack('<H', 6)  # Backup boot sector
    boot_sector[52:64] = b'\x00' * 12  # Reserved
    boot_sector[64] = 0x80  # Drive number
    boot_sector[65] = 0  # Reserved
    boot_sector[66] = 0x29  # Boot signature
    boot_sector[67:71] = struct.pack('<I', 0x12345678)  # Volume serial
    boot_sector[71:82] = label.ljust(11).encode('ascii')[:11]  # Volume label
    boot_sector[82:90] = b'FAT32   '  # File system type
    
    # Boot code (just halt if executed)
    boot_sector[90:510] = b'\xF4' * 420
    
    # Boot signature
    boot_sector[510:512] = b'\x55\xAA'
    
    return boot_sector, sectors_per_fat, sectors_per_cluster


def create_fat_tables(sectors_per_fat, num_fats):
    """Create FAT tables with initial entries."""
    fat_data = bytearray(sectors_per_fat * SECTOR_SIZE)
    
    # FAT32 media type markers
    fat_data[0:4] = struct.pack('<I', 0x0FFFFFF8)  # Media type
    fat_data[4:8] = struct.pack('<I', 0x0FFFFFFF)  # Reserved
    fat_data[8:12] = struct.pack('<I', 0x0FFFFFFF)  # Root directory (cluster 2 = end)
    
    return fat_data


def cluster_to_offset(cluster, start_sector, sectors_per_cluster):
    """Convert cluster number to byte offset in image."""
    # First data sector = reserved sectors + FAT sectors
    # But we need to account for the partition offset
    first_data_sector = start_sector + 32 + (2 * ((cluster + 2) * 4 + SECTOR_SIZE - 1) // SECTOR_SIZE)
    sector = first_data_sector + ((cluster - 2) * sectors_per_cluster)
    return sector * SECTOR_SIZE


def create_directory_entry(name, ext, attr, start_cluster, size):
    """Create a 32-byte FAT directory entry."""
    entry = bytearray(32)
    
    # Name (8.3 format)
    entry[0:8] = name.upper().ljust(8).encode('latin-1')[:8]
    entry[8:11] = ext.upper().ljust(3).encode('latin-1')[:3]
    
    # Attributes
    entry[11] = attr
    
    # Reserved
    entry[12:22] = b'\x00' * 10
    
    # Access date
    entry[18:20] = struct.pack('<H', 0)
    
    # High cluster (bits 16-31)
    entry[20:22] = struct.pack('<H', (start_cluster >> 16) & 0xFFFF)
    
    # Modification time/date
    entry[22:26] = struct.pack('<I', 0)
    
    # Low cluster (bits 0-15)
    entry[26:28] = struct.pack('<H', start_cluster & 0xFFFF)
    
    # File size
    entry[28:32] = struct.pack('<I', size)
    
    return bytes(entry)


def add_file_to_image(f, boot_start_sector, boot_partition_sectors, sectors_per_fat, 
                      sectors_per_cluster, parent_cluster, filename, data, is_directory=False):
    """Add a file to the FAT32 boot partition."""
    cluster_size = sectors_per_cluster * SECTOR_SIZE
    
    # Parse filename
    if '.' in filename and not is_directory:
        name, ext = filename.rsplit('.', 1)
        name = name[:8]
        ext = ext[:3]
    else:
        name = filename[:8]
        ext = ''
    
    attr = 0x10 if is_directory else 0x20
    
    # Calculate sectors per FAT for offset calculation
    total_clusters = boot_partition_sectors // sectors_per_cluster
    actual_sectors_per_fat = ((total_clusters * 4) + SECTOR_SIZE - 1) // SECTOR_SIZE
    
    if is_directory:
        # Allocate one cluster for empty directory
        start_cluster = 3  # After root cluster (2)
        # Initialize directory cluster to zeros
        first_data_sector = boot_start_sector + 32 + (2 * actual_sectors_per_fat)
        dir_sector = first_data_sector + ((start_cluster - 2) * sectors_per_cluster)
        f.seek(dir_sector * SECTOR_SIZE)
        f.write(b'\x00' * cluster_size)
        size = 0
    else:
        # Calculate clusters needed
        num_clusters = (len(data) + cluster_size - 1) // cluster_size
        if num_clusters == 0:
            num_clusters = 1
        
        # Allocate clusters (start after root at cluster 3)
        start_cluster = 3
        
        # Write file data
        first_data_sector = boot_start_sector + 32 + (2 * actual_sectors_per_fat)
        for i in range(num_clusters):
            sector = first_data_sector + ((start_cluster + i - 2) * sectors_per_cluster)
            f.seek(sector * SECTOR_SIZE)
            start = i * cluster_size
            end = min(start + cluster_size, len(data))
            chunk = data[start:end]
            if len(chunk) < cluster_size:
                chunk = chunk + b'\x00' * (cluster_size - len(chunk))
            f.write(chunk)
        
        # Update FAT to link clusters
        fat_offset = boot_start_sector + 32
        for i in range(num_clusters):
            f.seek((fat_offset * SECTOR_SIZE) + ((start_cluster + i) * 4))
            if i < num_clusters - 1:
                f.write(struct.pack('<I', start_cluster + i + 1))
            else:
                f.write(struct.pack('<I', 0x0FFFFFFF))  # End of chain
        
        size = len(data)
    
    # Add directory entry to root (cluster 2)
    entry = create_directory_entry(name, ext, attr, start_cluster, size)
    
    # Write entry at beginning of root directory (cluster 2)
    first_data_sector = boot_start_sector + 32 + (2 * actual_sectors_per_fat)
    root_sector = first_data_sector  # Cluster 2 is first data cluster
    f.seek(root_sector * SECTOR_SIZE)
    root_data = bytearray(f.read(cluster_size))
    
    # Find free slot
    for i in range(0, len(root_data), 32):
        if root_data[i] == 0x00 or root_data[i] == 0xE5:
            root_data[i:i+32] = entry
            break
    
    f.seek(root_sector * SECTOR_SIZE)
    f.write(root_data)
    
    return start_cluster


def create_sdcard_image(output_path, kernel_path=None, size_mb=2048, 
                        include_firmware=False, firmware_dir=None):
    """
    Create a complete Raspberry Pi SD card image.
    
    Args:
        output_path: Path for output image
        kernel_path: Path to kernel8.img (optional)
        size_mb: Total image size in MB
        include_firmware: Whether to include Raspberry Pi firmware files
        firmware_dir: Directory containing firmware files (bootcode.bin, start.elf, etc.)
    """
    size_bytes = size_mb * 1024 * 1024
    total_sectors = size_bytes // SECTOR_SIZE
    
    # Calculate boot partition size - ensure we leave room for root partition
    available_sectors = total_sectors - FIRST_PARTITION_START
    requested_boot_sectors = DEFAULT_BOOT_PARTITION_SIZE_MB * 1024 * 1024 // SECTOR_SIZE
    min_root_sectors = MIN_ROOT_PARTITION_MB * 1024 * 1024 // SECTOR_SIZE
    
    if requested_boot_sectors + min_root_sectors > available_sectors:
        # Reduce boot partition to fit
        boot_partition_sectors = available_sectors - min_root_sectors
        if boot_partition_sectors < 64 * 1024 * 1024 // SECTOR_SIZE:  # Min 64MB
            raise ValueError(f"Image size ({size_mb}MB) too small. Minimum is {MIN_ROOT_PARTITION_MB + 64}MB")
    else:
        boot_partition_sectors = requested_boot_sectors
    
    # Calculate root partition size
    root_partition_sectors = available_sectors - boot_partition_sectors
    
    boot_size_mb = boot_partition_sectors * SECTOR_SIZE // 1024 // 1024
    root_size_mb = root_partition_sectors * SECTOR_SIZE // 1024 // 1024
    
    print(f"Creating {size_mb}MB Raspberry Pi SD card image...")
    print(f"  Total sectors: {total_sectors}")
    print(f"  Boot partition: {boot_partition_sectors} sectors ({boot_size_mb}MB)")
    print(f"  Root partition: {root_partition_sectors} sectors ({root_size_mb}MB)")
    
    # Create blank image
    with open(output_path, 'wb') as f:
        f.write(b'\x00' * size_bytes)
    
    # Write MBR
    with open(output_path, 'r+b') as f:
        mbr = create_mbr(boot_partition_sectors, root_partition_sectors)
        f.write(mbr)
        
        # Create FAT32 boot sector at partition 1 start
        boot_sector, sectors_per_fat, sectors_per_cluster = create_fat32_boot_sector(
            FIRST_PARTITION_START, boot_partition_sectors, "BOOT"
        )
        f.seek(FIRST_PARTITION_START * SECTOR_SIZE)
        f.write(boot_sector)
        
        # Write FSInfo sector
        fsinfo = bytearray(512)
        fsinfo[0:4] = b'RRaA'
        fsinfo[484:488] = b'rrAa'
        fsinfo[488:492] = struct.pack('<I', 0xFFFFFFFF)
        fsinfo[492:496] = struct.pack('<I', 0xFFFFFFFF)
        fsinfo[508:512] = b'\x00\x00\x55\xAA'
        f.seek((FIRST_PARTITION_START + 1) * SECTOR_SIZE)
        f.write(fsinfo)
        
        # Write FAT tables
        fat_data = create_fat_tables(sectors_per_fat, 2)
        fat_offset = FIRST_PARTITION_START + 32
        f.seek(fat_offset * SECTOR_SIZE)
        f.write(fat_data)
        f.write(fat_data)  # Second FAT copy
        
        print(f"\n  FAT32 boot partition created")
        print(f"    Sectors per FAT: {sectors_per_fat}")
        print(f"    Sectors per cluster: {sectors_per_cluster}")
    
    # Add files to boot partition
    with open(output_path, 'r+b') as f:
        print("\n  Adding boot files...")
        
        # Create config.txt
        config_txt = b"""# Raspberry Pi Boot Configuration for WebbOS
# Enable 64-bit mode
arm_64bit=1

# Load our kernel
kernel=kernel8.img

# Disable command line tags - we'll parse device tree
disable_commandline_tags=1

# UART for debugging
enable_uart=1
uart_2ndstage=1

# GPU memory split (minimum for headless, increase if using graphics)
gpu_mem=16

# Overclock (optional, remove for stability testing)
#arm_freq=1500
#over_voltage=2
"""
        add_file_to_image(f, FIRST_PARTITION_START, boot_partition_sectors, sectors_per_fat, sectors_per_cluster,
                          2, 'CONFIG.TXT', config_txt)
        print("    Added config.txt")
        
        # Create cmdline.txt
        cmdline_txt = b"console=ttyS0,115200 root=/dev/mmcblk0p2 rw rootwait\n"
        add_file_to_image(f, FIRST_PARTITION_START, boot_partition_sectors, sectors_per_fat, sectors_per_cluster,
                          2, 'CMDLINE.TXT', cmdline_txt)
        print("    Added cmdline.txt")
        
        # Add kernel
        if kernel_path and os.path.exists(kernel_path):
            with open(kernel_path, 'rb') as kf:
                kernel_data = kf.read()
            # Rename kernel to kernel8.img for 64-bit mode
            add_file_to_image(f, FIRST_PARTITION_START, boot_partition_sectors, sectors_per_fat, sectors_per_cluster,
                              2, 'KERNEL8.IMG', kernel_data)
            print(f"    Added kernel8.img ({len(kernel_data)} bytes)")
        else:
            # Create placeholder
            add_file_to_image(f, FIRST_PARTITION_START, boot_partition_sectors, sectors_per_fat, sectors_per_cluster,
                              2, 'KERNEL8.IMG', b'PLACEHOLDER')
            print("    Created placeholder kernel8.img")
        
        # Add firmware files if directory provided
        if include_firmware and firmware_dir and os.path.isdir(firmware_dir):
            print(f"\n  Adding firmware files from {firmware_dir}...")
            firmware_files = [
                'bootcode.bin',
                'start.elf',
                'start4.elf',
                'fixup.dat',
                'fixup4.dat',
                'LICENCE.broadcom'
            ]
            
            for fw_file in firmware_files:
                fw_path = os.path.join(firmware_dir, fw_file)
                if os.path.exists(fw_path):
                    with open(fw_path, 'rb') as fw:
                        fw_data = fw.read()
                    fat_name = fw_file.upper().replace('.', '')
                    add_file_to_image(f, FIRST_PARTITION_START, boot_partition_sectors, sectors_per_fat, sectors_per_cluster,
                                      2, fat_name, fw_data)
                    print(f"    Added {fw_file}")
        
        # Note: Device tree blobs (.dtb) would be added here
        # They can be extracted from Raspberry Pi firmware or built from kernel source
        print("\n  Note: Device tree blobs (.dtb) should be added manually")
        print("        or extracted from Raspberry Pi firmware repository")
    
    print(f"\nSD card image created successfully: {output_path}")
    print(f"\nTo write to an SD card (be careful with device name):")
    print(f"  Windows: Use Rufus or Etcher, or:\n"
          f"           dd if={output_path} of=\\\\.\\PhysicalDriveN bs=4M")
    print(f"  Linux:   sudo dd if={output_path} of=/dev/sdX bs=4M status=progress")
    
    return output_path


def main():
    parser = argparse.ArgumentParser(
        description='Create a Raspberry Pi SD card image for WebbOS',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Create 2GB image with default kernel path
  python create-sdcard.py
  
  # Create with specific kernel
  python create-sdcard.py target/aarch64-unknown-none/release/kernel
  
  # Create larger image
  python create-sdcard.py --size 4096 --output pi-sdcard.img
  
  # Include firmware files
  python create-sdcard.py --firmware-dir /path/to/pi/firmware
        """
    )
    
    parser.add_argument('kernel', nargs='?',
                        default='target/aarch64-unknown-none/release/kernel',
                        help='Path to kernel binary (default: target/aarch64-unknown-none/release/kernel)')
    parser.add_argument('-o', '--output', default='webbos-pi.img',
                        help='Output image path (default: webbos-pi.img)')
    parser.add_argument('-s', '--size', type=int, default=2048,
                        help='Image size in MB (default: 2048)')
    parser.add_argument('--firmware-dir',
                        help='Directory containing Raspberry Pi firmware files')
    parser.add_argument('--include-firmware', action='store_true',
                        help='Include firmware files from --firmware-dir')
    
    args = parser.parse_args()
    
    try:
        create_sdcard_image(
            args.output,
            args.kernel,
            args.size,
            args.include_firmware,
            args.firmware_dir
        )
    except ValueError as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)
    except Exception as e:
        print(f"Unexpected error: {e}", file=sys.stderr)
        import traceback
        traceback.print_exc()
        sys.exit(1)


if __name__ == '__main__':
    main()
