#!/usr/bin/env python3
"""
Create a simple MBR-based disk with ESP at sector 1 (right after MBR).
"""

import struct
import sys
import uuid

def create_simple_esp_image(image_path):
    """Create a simple ESP disk image"""
    
    image_size = 64 * 1024 * 1024  # 64 MB
    sector_size = 512
    total_sectors = image_size // sector_size
    
    # ESP starts at sector 1 (right after MBR)
    esp_start = 1
    esp_sectors = total_sectors - 1
    
    # FAT32 parameters
    sectors_per_cluster = 8
    reserved_sectors = 32
    num_fats = 2
    
    data_sectors = esp_sectors - reserved_sectors
    num_clusters = data_sectors // sectors_per_cluster
    sectors_per_fat = ((num_clusters * 4 + sector_size - 1) // sector_size + 1)
    
    # Recalculate
    data_sectors = esp_sectors - reserved_sectors - (num_fats * sectors_per_fat)
    num_clusters = data_sectors // sectors_per_cluster
    
    print(f"Creating simple ESP disk:")
    print(f"  Total sectors: {total_sectors}")
    print(f"  ESP start: sector {esp_start}")
    print(f"  ESP sectors: {esp_sectors}")
    print(f"  Clusters: {num_clusters}")
    print(f"  Sectors per FAT: {sectors_per_fat}")
    
    image = bytearray(image_size)
    
    # MBR (sector 0)
    mbr = bytearray(512)
    # Boot code
    mbr[0:446] = b'\x00' * 446
    
    # Partition 1: ESP
    mbr[446] = 0x80  # Bootable
    # CHS for sector 1
    mbr[447] = 0x00  # Head 0
    mbr[448] = 0x02  # Sector 2, cylinder 0
    mbr[449] = 0x00
    mbr[450] = 0xEF  # EFI System Partition
    # CHS end (max)
    mbr[451] = 0xFE
    mbr[452] = 0xFF
    mbr[453] = 0xFF
    # Start LBA
    mbr[454:458] = struct.pack('<I', esp_start)
    # Size
    mbr[458:462] = struct.pack('<I', esp_sectors)
    
    # Boot signature
    mbr[510:512] = b'\x55\xAA'
    image[0:512] = mbr
    
    # FAT32 boot sector at sector 1
    boot_offset = esp_start * sector_size
    boot = bytearray(512)
    
    boot[0:3] = b'\xEB\x58\x90'  # Jump
    boot[3:11] = b'MSDOS5.0'       # OEM
    boot[11:13] = struct.pack('<H', 512)  # Bytes per sector
    boot[13] = sectors_per_cluster
    boot[14:16] = struct.pack('<H', reserved_sectors)
    boot[16] = num_fats
    boot[17:19] = struct.pack('<H', 0)  # Root entries
    boot[19:21] = struct.pack('<H', 0)  # Total sectors (0 for FAT32)
    boot[21] = 0xF8  # Media
    boot[22:24] = struct.pack('<H', 0)  # Sectors per FAT (0 for FAT32)
    boot[24:26] = struct.pack('<H', 32)  # Sectors per track
    boot[26:28] = struct.pack('<H', 64)  # Heads
    boot[28:32] = struct.pack('<I', esp_start)  # Hidden sectors
    boot[32:36] = struct.pack('<I', esp_sectors)  # Total sectors
    boot[36:40] = struct.pack('<I', sectors_per_fat)  # Sectors per FAT
    boot[40:42] = struct.pack('<H', 0)  # Mirror
    boot[42:44] = struct.pack('<H', 0)  # Version
    boot[44:48] = struct.pack('<I', 2)  # Root cluster
    boot[48:50] = struct.pack('<H', 1)  # FSInfo
    boot[50:52] = struct.pack('<H', 6)  # Backup boot
    boot[64] = 0x80  # Drive number
    boot[66] = 0x29  # Boot sig
    boot[67:71] = struct.pack('<I', 0x12345678)  # Serial
    boot[71:82] = b'EFI SYSTEM '  # Label
    boot[82:90] = b'FAT32   '  # Type
    boot[510:512] = b'\x55\xAA'
    
    image[boot_offset:boot_offset+512] = boot
    
    # Backup boot at sector 6
    image[boot_offset+6*512:boot_offset+6*512+512] = boot
    
    # FSInfo
    fsinfo = bytearray(512)
    fsinfo[0:4] = struct.pack('<I', 0x41615252)
    fsinfo[484:488] = struct.pack('<I', 0x61417272)
    fsinfo[488:492] = struct.pack('<I', 0xFFFFFFFF)
    fsinfo[492:496] = struct.pack('<I', 0xFFFFFFFF)
    fsinfo[508:512] = struct.pack('<I', 0xAA550000)
    image[boot_offset+512:boot_offset+1024] = fsinfo
    
    # FATs
    fat_size = sectors_per_fat * sector_size
    fat1_offset = boot_offset + reserved_sectors * sector_size
    fat2_offset = fat1_offset + fat_size
    
    fat = bytearray(fat_size)
    fat[0:4] = struct.pack('<I', 0x0FFFFFF8)
    fat[4:8] = struct.pack('<I', 0xFFFFFFFF)
    fat[8:12] = struct.pack('<I', 0x0FFFFFFF)
    
    image[fat1_offset:fat1_offset+fat_size] = fat
    image[fat2_offset:fat2_offset+fat_size] = fat
    
    # Data area
    data_offset = fat2_offset + fat_size
    
    # Root directory at cluster 2
    root_offset = data_offset
    
    def create_dir_entry(name, ext, attr, cluster, size):
        entry = bytearray(32)
        name = name.upper().encode('ascii')[:8].ljust(8, b' ')
        ext = ext.upper().encode('ascii')[:3].ljust(3, b' ')
        entry[0:8] = name
        entry[8:11] = ext
        entry[11] = attr
        entry[20:22] = struct.pack('<H', cluster >> 16)
        entry[26:28] = struct.pack('<H', cluster & 0xFFFF)
        entry[28:32] = struct.pack('<I', size)
        return entry
    
    # EFI directory
    image[root_offset:root_offset+32] = create_dir_entry('EFI', '', 0x10, 3, 0)
    
    # Update FAT for cluster 3
    fat[12:16] = struct.pack('<I', 0x0FFFFFFF)
    image[fat1_offset:fat1_offset+16] = fat[:16]
    image[fat2_offset:fat2_offset+16] = fat[:16]
    
    # EFI directory
    efi_offset = data_offset + (3-2) * sectors_per_cluster * sector_size
    image[efi_offset:efi_offset+32] = create_dir_entry('.', '', 0x10, 3, 0)
    image[efi_offset+32:efi_offset+64] = create_dir_entry('..', '', 0x10, 2, 0)
    image[efi_offset+64:efi_offset+96] = create_dir_entry('BOOT', '', 0x10, 4, 0)
    
    # Update FAT for cluster 4
    fat[16:20] = struct.pack('<I', 0x0FFFFFFF)
    image[fat1_offset:fat1_offset+20] = fat[:20]
    image[fat2_offset:fat2_offset+20] = fat[:20]
    
    # BOOT directory
    boot_dir_offset = data_offset + (4-2) * sectors_per_cluster * sector_size
    image[boot_dir_offset:boot_dir_offset+32] = create_dir_entry('.', '', 0x10, 4, 0)
    image[boot_dir_offset+32:boot_dir_offset+64] = create_dir_entry('..', '', 0x10, 3, 0)
    
    with open(image_path, 'wb') as f:
        f.write(image)
    
    print(f"\nCreated simple ESP image: {image_path}")
    return True

if __name__ == '__main__':
    if len(sys.argv) < 2:
        print("Usage: create-simple-esp.py <image_path>")
        sys.exit(1)
    
    if create_simple_esp_image(sys.argv[1]):
        print("Success!")
    else:
        sys.exit(1)
