#!/usr/bin/env python3
"""
Create a proper bootable FAT32 disk image for WebbOS.
This creates a disk with MBR partition table and EFI System Partition.
"""

import struct
import sys

def create_lba_chs(lba):
    """Convert LBA to CHS format for MBR"""
    # Simplified CHS calculation
    heads = 255
    sectors_per_track = 63
    cylinder = lba // (heads * sectors_per_track)
    head = (lba // sectors_per_track) % heads
    sector = (lba % sectors_per_track) + 1
    
    if cylinder > 1023:
        cylinder = 1023
    
    chs = bytearray(3)
    chs[0] = head
    chs[1] = ((cylinder >> 2) & 0xC0) | (sector & 0x3F)
    chs[2] = cylinder & 0xFF
    return bytes(chs)

def create_mbr_partition_table():
    """Create MBR partition table with one FAT32 partition"""
    # Partition entry format (16 bytes each)
    # Byte 0: Boot indicator (0x80 = bootable, 0x00 = not bootable)
    # Byte 1-3: Starting CHS
    # Byte 4: Partition type
    # Byte 5-7: Ending CHS
    # Byte 8-11: Starting LBA
    # Byte 12-15: Size in sectors
    
    partition = bytearray(16)
    partition[0] = 0x80  # Bootable
    partition[1:4] = create_lba_chs(2048)  # Starting CHS (sector 2048 = 1MB offset)
    partition[4] = 0xEF  # EFI System Partition
    partition[5:8] = create_lba_chs(0xFFFFFFFF)  # Ending CHS (use max)
    partition[8:12] = struct.pack('<I', 2048)  # Starting LBA
    # Size will be set later
    return partition

def create_fat32_boot_sector(sectors_per_cluster, reserved_sectors, num_fats, 
                              sectors_per_fat, total_sectors, root_cluster, 
                              volume_start_sector):
    """Create FAT32 boot sector (first sector of partition)"""
    boot = bytearray(512)
    
    # Jump instruction
    boot[0:3] = b'\xEB\x58\x90'
    # OEM name
    boot[3:11] = b'EFI FAT32'
    # Bytes per sector
    boot[11:13] = struct.pack('<H', 512)
    # Sectors per cluster
    boot[13] = sectors_per_cluster
    # Reserved sectors
    boot[14:16] = struct.pack('<H', reserved_sectors)
    # Number of FATs
    boot[16] = num_fats
    # Root entries (0 for FAT32)
    boot[17:19] = struct.pack('<H', 0)
    # Total sectors (0 for FAT32, use field at offset 32)
    boot[19:21] = struct.pack('<H', 0)
    # Media descriptor
    boot[21] = 0xF8
    # Sectors per FAT (0 for FAT32)
    boot[22:24] = struct.pack('<H', 0)
    # Sectors per track
    boot[24:26] = struct.pack('<H', 32)
    # Number of heads
    boot[26:28] = struct.pack('<H', 64)
    # Hidden sectors (sectors before this partition)
    boot[28:32] = struct.pack('<I', volume_start_sector)
    # Total sectors (FAT32)
    boot[32:36] = struct.pack('<I', total_sectors)
    
    # FAT32 specific fields (offset 36+)
    # Sectors per FAT
    boot[36:40] = struct.pack('<I', sectors_per_fat)
    # Mirror flags
    boot[40:42] = struct.pack('<H', 0)
    # Version
    boot[42:44] = struct.pack('<H', 0)
    # Root directory cluster
    boot[44:48] = struct.pack('<I', root_cluster)
    # FSInfo sector
    boot[48:50] = struct.pack('<H', 1)
    # Backup boot sector
    boot[50:52] = struct.pack('<H', 6)
    # Reserved
    boot[52:64] = b'\x00' * 12
    # Drive number
    boot[64] = 0x00
    # Reserved
    boot[65] = 0x00
    # Boot signature
    boot[66] = 0x29
    # Volume serial
    boot[67:71] = struct.pack('<I', 0x12345678)
    # Volume label
    boot[71:82] = b'WEBBOS     '
    # File system type
    boot[82:90] = b'FAT32   '
    
    # Boot code (minimal)
    boot[90:510] = b'\x00' * 420
    # Boot signature
    boot[510:512] = b'\x55\xAA'
    
    return boot

def create_fsinfo():
    """Create FAT32 FSInfo sector"""
    fsinfo = bytearray(512)
    # Lead signature
    fsinfo[0:4] = struct.pack('<I', 0x41615252)
    # Reserved
    fsinfo[4:484] = b'\x00' * 480
    # Struct signature
    fsinfo[484:488] = struct.pack('<I', 0x61417272)
    # Free cluster count (0xFFFFFFFF = unknown)
    fsinfo[488:492] = struct.pack('<I', 0xFFFFFFFF)
    # Next free cluster
    fsinfo[492:496] = struct.pack('<I', 0xFFFFFFFF)
    # Reserved
    fsinfo[496:508] = b'\x00' * 12
    # Trail signature
    fsinfo[508:512] = struct.pack('<I', 0xAA550000)
    return fsinfo

def create_directory_entry(name, ext, attr, cluster, size):
    """Create a short directory entry (32 bytes)"""
    entry = bytearray(32)
    
    # Pad name to 8 chars
    name = name.upper().encode('ascii')
    name = name[:8].ljust(8, b' ')
    
    # Pad extension to 3 chars
    ext = ext.upper().encode('ascii')
    ext = ext[:3].ljust(3, b' ')
    
    entry[0:8] = name
    entry[8:11] = ext
    entry[11] = attr  # Attributes
    entry[12:13] = b'\x00'  # Reserved
    entry[13] = 0  # Creation time tenths
    entry[14:16] = struct.pack('<H', 0)  # Creation time
    entry[16:18] = struct.pack('<H', 0)  # Creation date
    entry[18:20] = struct.pack('<H', 0)  # Access date
    entry[20:22] = struct.pack('<H', cluster >> 16)  # High cluster
    entry[22:24] = struct.pack('<H', 0)  # Modification time
    entry[24:26] = struct.pack('<H', 0)  # Modification date
    entry[26:28] = struct.pack('<H', cluster & 0xFFFF)  # Low cluster
    entry[28:32] = struct.pack('<I', size)  # File size
    
    return entry

def create_efi_bootable_image(image_path):
    """Create a complete EFI-bootable disk image"""
    
    # Disk parameters
    image_size = 64 * 1024 * 1024  # 64 MB
    bytes_per_sector = 512
    total_sectors = image_size // bytes_per_sector
    
    # Partition parameters
    partition_start = 2048  # Start at sector 2048 (1MB offset, standard practice)
    partition_sectors = total_sectors - partition_start
    
    # FAT32 parameters
    sectors_per_cluster = 8  # 4KB clusters
    reserved_sectors = 32
    num_fats = 2
    
    # Calculate FAT size
    # Each FAT32 entry is 4 bytes
    # Number of clusters = (partition_sectors - reserved_sectors) / sectors_per_cluster
    data_sectors = partition_sectors - reserved_sectors
    num_clusters = data_sectors // sectors_per_cluster
    sectors_per_fat = ((num_clusters * 4 + bytes_per_sector - 1) // bytes_per_sector + 1)
    
    # Adjust for FATs
    data_sectors = partition_sectors - reserved_sectors - (num_fats * sectors_per_fat)
    num_clusters = data_sectors // sectors_per_cluster
    
    # Root directory at cluster 2
    root_cluster = 2
    
    print(f"Creating disk image:")
    print(f"  Total size: {image_size} bytes ({image_size // (1024*1024)} MB)")
    print(f"  Partition start: sector {partition_start}")
    print(f"  Partition sectors: {partition_sectors}")
    print(f"  Sectors per cluster: {sectors_per_cluster}")
    print(f"  Reserved sectors: {reserved_sectors}")
    print(f"  Sectors per FAT: {sectors_per_fat}")
    print(f"  Number of clusters: {num_clusters}")
    
    # Create empty image
    image = bytearray(image_size)
    
    # Create MBR
    mbr = bytearray(512)
    # Boot code (minimal)
    mbr[0:446] = b'\x00' * 446
    # Partition table
    partition = create_mbr_partition_table()
    # Update partition size
    partition[12:16] = struct.pack('<I', partition_sectors)
    mbr[446:462] = partition
    # Boot signature
    mbr[510:512] = b'\x55\xAA'
    
    image[0:512] = mbr
    
    # Create FAT32 boot sector for the partition
    boot_sector = create_fat32_boot_sector(
        sectors_per_cluster, reserved_sectors, num_fats,
        sectors_per_fat, partition_sectors, root_cluster, partition_start
    )
    
    # Write boot sector to partition start
    partition_offset = partition_start * bytes_per_sector
    image[partition_offset:partition_offset + 512] = boot_sector
    
    # Write backup boot sector at sector 6
    backup_offset = partition_offset + 6 * bytes_per_sector
    image[backup_offset:backup_offset + 512] = boot_sector
    
    # Write FSInfo sector at sector 1
    fsinfo = create_fsinfo()
    fsinfo_offset = partition_offset + bytes_per_sector
    image[fsinfo_offset:fsinfo_offset + 512] = fsinfo
    
    # Create FAT tables
    fat_size = sectors_per_fat * bytes_per_sector
    fat1_offset = partition_offset + reserved_sectors * bytes_per_sector
    fat2_offset = fat1_offset + fat_size
    
    fat = bytearray(fat_size)
    # FAT32 media type marker
    fat[0:4] = struct.pack('<I', 0x0FFFFFF8)
    # Reserved
    fat[4:8] = struct.pack('<I', 0xFFFFFFFF)
    # Root directory (cluster 2) - end of chain
    fat[8:12] = struct.pack('<I', 0x0FFFFFFF)
    
    image[fat1_offset:fat1_offset + fat_size] = fat
    image[fat2_offset:fat2_offset + fat_size] = fat
    
    # Data area starts after FATs
    data_offset = fat2_offset + fat_size
    
    # Root directory is at cluster 2
    root_dir_offset = data_offset
    
    # Create EFI directory entry
    efi_entry = create_directory_entry('EFI', '', 0x10, 3, 0)  # Directory, cluster 3
    image[root_dir_offset:root_dir_offset + 32] = efi_entry
    
    # Mark cluster 3 as end of chain (for EFI directory contents)
    fat[12:16] = struct.pack('<I', 0x0FFFFFFF)
    image[fat1_offset:fat1_offset + 16] = fat[:16]
    image[fat2_offset:fat2_offset + 16] = fat[:16]
    
    # EFI directory contents at cluster 3
    efi_dir_offset = data_offset + (3 - 2) * sectors_per_cluster * bytes_per_sector
    
    # Create . and .. entries
    dot_entry = create_directory_entry('.', '', 0x10, 2, 0)
    dotdot_entry = create_directory_entry('..', '', 0x10, 0, 0)
    boot_entry = create_directory_entry('BOOT', '', 0x10, 4, 0)  # Cluster 4
    
    image[efi_dir_offset:efi_dir_offset + 32] = dot_entry
    image[efi_dir_offset + 32:efi_dir_offset + 64] = dotdot_entry
    image[efi_dir_offset + 64:efi_dir_offset + 96] = boot_entry
    
    # Mark cluster 4 as end of chain (for BOOT directory contents)
    fat[16:20] = struct.pack('<I', 0x0FFFFFFF)
    image[fat1_offset:fat1_offset + 20] = fat[:20]
    image[fat2_offset:fat2_offset + 20] = fat[:20]
    
    # BOOT directory contents at cluster 4
    boot_dir_offset = data_offset + (4 - 2) * sectors_per_cluster * bytes_per_sector
    
    # Create . and .. entries for BOOT
    boot_dot = create_directory_entry('.', '', 0x10, 4, 0)
    boot_dotdot = create_directory_entry('..', '', 0x10, 3, 0)
    
    image[boot_dir_offset:boot_dir_offset + 32] = boot_dot
    image[boot_dir_offset + 32:boot_dir_offset + 64] = boot_dotdot
    
    # Write image to file
    with open(image_path, 'wb') as f:
        f.write(image)
    
    print(f"\nCreated bootable FAT32 image: {image_path}")
    return True

if __name__ == '__main__':
    if len(sys.argv) < 2:
        print("Usage: create-fat32-image.py <image_path>")
        sys.exit(1)
    
    image_path = sys.argv[1]
    
    if create_efi_bootable_image(image_path):
        print("Success!")
    else:
        print("Failed!")
        sys.exit(1)
