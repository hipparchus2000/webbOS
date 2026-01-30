#!/usr/bin/env python3
"""
Create a GPT-based EFI-bootable disk image for WebbOS.
UEFI firmware typically expects GPT for EFI systems.
"""

import struct
import sys
import uuid

def crc32(data):
    """Calculate CRC32 checksum"""
    import binascii
    return binascii.crc32(data) & 0xffffffff

def create_gpt_efi_image(image_path):
    """Create a GPT disk with EFI System Partition"""
    
    # Disk parameters
    image_size = 64 * 1024 * 1024  # 64 MB
    sector_size = 512
    total_sectors = image_size // sector_size
    
    # GPT layout:
    # Sector 0: Protective MBR
    # Sector 1: GPT Header
    # Sectors 2-33: Partition Entry Array (typically 32 sectors)
    # ... data ...
    # Last 33 sectors: Backup GPT
    
    first_usable_lba = 34
    last_usable_lba = total_sectors - 34
    
    # ESP will use all available space
    esp_start = 2048  # Start at sector 2048 for alignment
    esp_end = last_usable_lba - 1
    
    # FAT32 parameters for ESP
    sectors_per_cluster = 8
    reserved_sectors = 32
    num_fats = 2
    
    esp_sectors = esp_end - esp_start + 1
    data_sectors = esp_sectors - reserved_sectors
    num_clusters = data_sectors // sectors_per_cluster
    
    # Calculate FAT size
    sectors_per_fat = ((num_clusters * 4 + sector_size - 1) // sector_size + 1)
    # Recalculate with actual FAT size
    data_sectors = esp_sectors - reserved_sectors - (num_fats * sectors_per_fat)
    num_clusters = data_sectors // sectors_per_cluster
    
    print(f"Creating GPT disk image:")
    print(f"  Total size: {image_size} bytes ({image_size // (1024*1024)} MB)")
    print(f"  Total sectors: {total_sectors}")
    print(f"  First usable LBA: {first_usable_lba}")
    print(f"  Last usable LBA: {last_usable_lba}")
    print(f"  ESP start: {esp_start}")
    print(f"  ESP sectors: {esp_sectors}")
    print(f"  Sectors per cluster: {sectors_per_cluster}")
    print(f"  Sectors per FAT: {sectors_per_fat}")
    
    # Create empty image
    image = bytearray(image_size)
    
    # Protective MBR (sector 0)
    mbr = bytearray(512)
    # Boot code
    mbr[0:440] = b'\x00' * 440
    # Disk signature
    mbr[440:444] = struct.pack('<I', 0)
    # Reserved
    mbr[444:446] = b'\x00\x00'
    
    # Protective partition entry (covers entire disk)
    # Entry 1: EE type (GPT protective)
    mbr[446] = 0x00  # Not bootable
    mbr[447:450] = b'\x00\x00\x00'  # Start CHS
    mbr[450] = 0xEE  # GPT protective
    mbr[451:454] = b'\xFF\xFF\xFF'  # End CHS
    mbr[454:458] = struct.pack('<I', 1)  # Start LBA
    mbr[458:462] = struct.pack('<I', total_sectors - 1)  # Size
    
    # Boot signature
    mbr[510:512] = b'\x55\xAA'
    image[0:512] = mbr
    
    # GPT Header (sector 1)
    gpt_header = bytearray(512)
    gpt_header[0:8] = b'EFI PART'  # Signature
    gpt_header[8:12] = struct.pack('<I', 0x00010000)  # Revision (1.0)
    gpt_header[12:16] = struct.pack('<I', 92)  # Header size
    gpt_header[16:20] = struct.pack('<I', 0)  # CRC32 (will calculate)
    gpt_header[20:24] = struct.pack('<I', 0)  # Reserved
    gpt_header[24:32] = struct.pack('<Q', 1)  # Current LBA (this header)
    gpt_header[32:40] = struct.pack('<Q', total_sectors - 1)  # Backup LBA
    gpt_header[40:48] = struct.pack('<Q', first_usable_lba)  # First usable LBA
    gpt_header[48:56] = struct.pack('<Q', last_usable_lba)  # Last usable LBA
    gpt_header[56:72] = uuid.uuid4().bytes_le  # Disk GUID
    gpt_header[72:80] = struct.pack('<Q', 2)  # Partition entry array start LBA
    gpt_header[80:84] = struct.pack('<I', 128)  # Number of partition entries
    gpt_header[84:88] = struct.pack('<I', 128)  # Size of each partition entry
    gpt_header[88:92] = struct.pack('<I', 0)  # CRC32 of partition entry array
    
    # Calculate CRC32 of header (before writing CRC field)
    header_crc = crc32(gpt_header[0:92])
    gpt_header[16:20] = struct.pack('<I', header_crc)
    
    image[512:1024] = gpt_header
    
    # Partition Entry Array (sectors 2-33)
    # EFI System Partition entry (entry 0)
    esp_entry = bytearray(128)
    # Partition type GUID: EFI System Partition
    # C12A7328-F81F-11D2-BA4B-00A0C93EC93B
    esp_type_guid = bytes.fromhex('28732AC11FF8D211BA4B00A0C93EC93B')
    esp_entry[0:16] = esp_type_guid
    # Unique partition GUID
    esp_entry[16:32] = uuid.uuid4().bytes_le
    # Starting LBA
    esp_entry[32:40] = struct.pack('<Q', esp_start)
    # Ending LBA
    esp_entry[40:48] = struct.pack('<Q', esp_end)
    # Attributes
    esp_entry[48:56] = struct.pack('<Q', 0)
    # Partition name (UTF-16LE)
    name = "EFI System Partition".encode('utf-16-le')
    esp_entry[56:56+len(name)] = name
    
    image[1024:1152] = esp_entry
    
    # Calculate CRC32 of partition entry array (first 16KB typically)
    entry_array_size = 128 * 128  # 128 entries * 128 bytes
    partition_crc = crc32(image[1024:1024+entry_array_size])
    gpt_header[88:92] = struct.pack('<I', partition_crc)
    # Recalculate header CRC
    header_crc = crc32(gpt_header[0:92])
    gpt_header[16:20] = struct.pack('<I', header_crc)
    image[512:1024] = gpt_header
    
    # Create FAT32 boot sector for ESP
    boot_offset = esp_start * sector_size
    boot = bytearray(512)
    
    # Jump instruction
    boot[0:3] = b'\xEB\x58\x90'
    # OEM name
    boot[3:11] = b'MSDOS5.0'
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
    # Total sectors (0 for FAT32)
    boot[19:21] = struct.pack('<H', 0)
    # Media descriptor
    boot[21] = 0xF8
    # Sectors per FAT (0 for FAT32)
    boot[22:24] = struct.pack('<H', 0)
    # Sectors per track
    boot[24:26] = struct.pack('<H', 32)
    # Number of heads
    boot[26:28] = struct.pack('<H', 64)
    # Hidden sectors
    boot[28:32] = struct.pack('<I', esp_start)
    # Total sectors (FAT32)
    boot[32:36] = struct.pack('<I', esp_sectors)
    # Sectors per FAT
    boot[36:40] = struct.pack('<I', sectors_per_fat)
    # Mirror flags
    boot[40:42] = struct.pack('<H', 0)
    # Version
    boot[42:44] = struct.pack('<H', 0)
    # Root directory cluster
    boot[44:48] = struct.pack('<I', 2)
    # FSInfo sector
    boot[48:50] = struct.pack('<H', 1)
    # Backup boot sector
    boot[50:52] = struct.pack('<H', 6)
    # Drive number
    boot[64] = 0x80
    # Boot signature
    boot[66] = 0x29
    # Volume serial
    boot[67:71] = struct.pack('<I', 0x12345678)
    # Volume label
    boot[71:82] = b'EFI SYSTEM '
    # File system type
    boot[82:90] = b'FAT32   '
    # Boot signature
    boot[510:512] = b'\x55\xAA'
    
    image[boot_offset:boot_offset+512] = boot
    
    # Backup boot sector at sector 6 (relative to ESP)
    backup_offset = boot_offset + 6 * sector_size
    image[backup_offset:backup_offset+512] = boot
    
    # FSInfo sector at sector 1 (relative to ESP)
    fsinfo_offset = boot_offset + sector_size
    fsinfo = bytearray(512)
    fsinfo[0:4] = struct.pack('<I', 0x41615252)  # Lead signature
    fsinfo[484:488] = struct.pack('<I', 0x61417272)  # Struct signature
    fsinfo[488:492] = struct.pack('<I', 0xFFFFFFFF)  # Free clusters
    fsinfo[492:496] = struct.pack('<I', 0xFFFFFFFF)  # Next free cluster
    fsinfo[508:512] = struct.pack('<I', 0xAA550000)  # Trail signature
    image[fsinfo_offset:fsinfo_offset+512] = fsinfo
    
    # FAT tables
    fat_size = sectors_per_fat * sector_size
    fat1_offset = boot_offset + reserved_sectors * sector_size
    fat2_offset = fat1_offset + fat_size
    
    fat = bytearray(fat_size)
    fat[0:4] = struct.pack('<I', 0x0FFFFFF8)  # Media type
    fat[4:8] = struct.pack('<I', 0xFFFFFFFF)  # Reserved
    fat[8:12] = struct.pack('<I', 0x0FFFFFFF)  # Root directory (end of chain)
    
    image[fat1_offset:fat1_offset+fat_size] = fat
    image[fat2_offset:fat2_offset+fat_size] = fat
    
    # Data area
    data_offset = fat2_offset + fat_size
    
    # Root directory at cluster 2
    root_offset = data_offset
    
    # Create EFI directory entry
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
    
    # EFI directory in root
    efi_entry = create_dir_entry('EFI', '', 0x10, 3, 0)
    image[root_offset:root_offset+32] = efi_entry
    
    # Update FAT for cluster 3
    fat[12:16] = struct.pack('<I', 0x0FFFFFFF)
    image[fat1_offset:fat1_offset+16] = fat[:16]
    image[fat2_offset:fat2_offset+16] = fat[:16]
    
    # EFI directory contents at cluster 3
    efi_offset = data_offset + (3 - 2) * sectors_per_cluster * sector_size
    image[efi_offset:efi_offset+32] = create_dir_entry('.', '', 0x10, 3, 0)
    image[efi_offset+32:efi_offset+64] = create_dir_entry('..', '', 0x10, 2, 0)
    image[efi_offset+64:efi_offset+96] = create_dir_entry('BOOT', '', 0x10, 4, 0)
    
    # Update FAT for cluster 4
    fat[16:20] = struct.pack('<I', 0x0FFFFFFF)
    image[fat1_offset:fat1_offset+20] = fat[:20]
    image[fat2_offset:fat2_offset+20] = fat[:20]
    
    # BOOT directory at cluster 4
    boot_dir_offset = data_offset + (4 - 2) * sectors_per_cluster * sector_size
    image[boot_dir_offset:boot_dir_offset+32] = create_dir_entry('.', '', 0x10, 4, 0)
    image[boot_dir_offset+32:boot_dir_offset+64] = create_dir_entry('..', '', 0x10, 3, 0)
    
    # Write image to file
    with open(image_path, 'wb') as f:
        f.write(image)
    
    print(f"\nCreated GPT EFI disk image: {image_path}")
    return True

if __name__ == '__main__':
    if len(sys.argv) < 2:
        print("Usage: create-gpt-image.py <image_path>")
        sys.exit(1)
    
    image_path = sys.argv[1]
    
    if create_gpt_efi_image(image_path):
        print("Success!")
    else:
        print("Failed!")
        sys.exit(1)
