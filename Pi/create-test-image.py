#!/usr/bin/env python3
"""Create a minimal SD card image with browser-test.html for testing."""

import struct
import os

SECTOR_SIZE = 512

def create_mbr_partition_entry(status, partition_type, start_lba, num_sectors):
    """Create a 16-byte MBR partition entry."""
    entry = bytearray(16)
    entry[0] = status
    entry[1:4] = b'\xFE\xFF\xFF'
    entry[4] = partition_type
    entry[5:8] = b'\xFE\xFF\xFF'
    entry[8:12] = struct.pack('<I', start_lba)
    entry[12:16] = struct.pack('<I', num_sectors)
    return bytes(entry)

def create_fat32_boot_sector(start_sector, num_sectors, label="WEBBOS"):
    """Create a FAT32 boot sector."""
    bytes_per_sector = SECTOR_SIZE
    sectors_per_cluster = 4
    reserved_sectors = 32
    num_fats = 2
    root_cluster = 2
    
    total_clusters = num_sectors // sectors_per_cluster
    sectors_per_fat = ((total_clusters * 4) + bytes_per_sector - 1) // bytes_per_sector
    
    boot_sector = bytearray(512)
    boot_sector[0:3] = b'\xEB\x58\x90'
    boot_sector[3:11] = b'MSDOS5.0'
    boot_sector[11:13] = struct.pack('<H', bytes_per_sector)
    boot_sector[13] = sectors_per_cluster
    boot_sector[14:16] = struct.pack('<H', reserved_sectors)
    boot_sector[16] = num_fats
    boot_sector[17:19] = struct.pack('<H', 0)
    boot_sector[19:21] = struct.pack('<H', 0)
    boot_sector[21] = 0xF8
    boot_sector[22:24] = struct.pack('<H', 0)
    boot_sector[24:26] = struct.pack('<H', 32)
    boot_sector[26:28] = struct.pack('<H', 64)
    boot_sector[28:32] = struct.pack('<I', start_sector)
    boot_sector[32:36] = struct.pack('<I', num_sectors)
    boot_sector[36:40] = struct.pack('<I', sectors_per_fat)
    boot_sector[40:42] = struct.pack('<H', 0)
    boot_sector[42:44] = struct.pack('<H', 0)
    boot_sector[44:48] = struct.pack('<I', root_cluster)
    boot_sector[48:50] = struct.pack('<H', 1)
    boot_sector[50:52] = struct.pack('<H', 6)
    boot_sector[52:64] = b'\x00' * 12
    boot_sector[64] = 0x80
    boot_sector[65] = 0
    boot_sector[66] = 0x29
    boot_sector[67:71] = struct.pack('<I', 0x12345678)
    boot_sector[71:82] = label.ljust(11).encode('ascii')[:11]
    boot_sector[82:90] = b'FAT32   '
    boot_sector[90:510] = b'\xF4' * 420
    boot_sector[510:512] = b'\x55\xAA'
    
    return boot_sector, sectors_per_fat, sectors_per_cluster

def create_directory_entry(name, ext, attr, start_cluster, size):
    """Create a 32-byte FAT directory entry."""
    entry = bytearray(32)
    entry[0:8] = name.upper().ljust(8).encode('latin-1')[:8]
    entry[8:11] = ext.upper().ljust(3).encode('latin-1')[:3]
    entry[11] = attr
    entry[12:22] = b'\x00' * 10
    entry[18:20] = struct.pack('<H', 0)
    entry[20:22] = struct.pack('<H', (start_cluster >> 16) & 0xFFFF)
    entry[22:26] = struct.pack('<I', 0)
    entry[26:28] = struct.pack('<H', start_cluster & 0xFFFF)
    entry[28:32] = struct.pack('<I', size)
    return bytes(entry)

def create_image_with_file(html_path, output_path):
    """Create SD card image with browser-test.html."""
    
    # Read HTML file
    with open(html_path, 'rb') as f:
        html_data = f.read()
    
    print(f"HTML file size: {len(html_data)} bytes")
    
    # Calculate sizes
    boot_start = 2048  # 1MB offset
    boot_sectors = 524288  # 256MB partition
    
    # Calculate clusters needed for file
    sectors_per_cluster = 4
    cluster_size = sectors_per_cluster * SECTOR_SIZE
    num_clusters = (len(html_data) + cluster_size - 1) // cluster_size
    if num_clusters == 0:
        num_clusters = 1
    
    # Calculate FAT size
    total_clusters = boot_sectors // sectors_per_cluster
    sectors_per_fat = ((total_clusters * 4) + SECTOR_SIZE - 1) // SECTOR_SIZE
    
    # Total image size
    total_sectors = boot_start + boot_sectors
    image_size = total_sectors * SECTOR_SIZE
    
    print(f"Creating image: {image_size} bytes ({image_size//1024//1024} MB)")
    print(f"Boot partition: {boot_sectors} sectors ({boot_sectors*SECTOR_SIZE//1024//1024} MB)")
    print(f"Clusters needed: {num_clusters}")
    
    # Create image
    image = bytearray(image_size)
    
    # Create MBR
    mbr = bytearray(512)
    mbr[0:446] = b'\x00' * 446
    mbr[446:462] = create_mbr_partition_entry(0x80, 0x0C, boot_start, boot_sectors)
    mbr[462:478] = create_mbr_partition_entry(0x00, 0x83, boot_start + boot_sectors, 0)
    mbr[478:494] = b'\x00' * 16
    mbr[494:510] = b'\x00' * 16
    mbr[510:512] = b'\x55\xAA'
    image[0:512] = mbr
    
    # Create FAT32 boot sector
    boot_sector, actual_sectors_per_fat, _ = create_fat32_boot_sector(boot_start, boot_sectors)
    image[boot_start * SECTOR_SIZE:(boot_start + 1) * SECTOR_SIZE] = boot_sector
    
    # Create FAT tables
    fat_offset = boot_start + 32
    fat_data = bytearray(actual_sectors_per_fat * SECTOR_SIZE)
    fat_data[0:4] = struct.pack('<I', 0x0FFFFFF8)
    fat_data[4:8] = struct.pack('<I', 0x0FFFFFFF)
    fat_data[8:12] = struct.pack('<I', 0x0FFFFFFF)  # Root cluster
    
    # Link clusters for file (starting at cluster 3)
    file_start_cluster = 3
    for i in range(num_clusters):
        fat_offset_in_table = (file_start_cluster + i) * 4
        if i < num_clusters - 1:
            fat_data[fat_offset_in_table:fat_offset_in_table+4] = struct.pack('<I', file_start_cluster + i + 1)
        else:
            fat_data[fat_offset_in_table:fat_offset_in_table+4] = struct.pack('<I', 0x0FFFFFFF)
    
    # Write FAT tables
    for i in range(2):
        image[(fat_offset + i * actual_sectors_per_fat) * SECTOR_SIZE:
              (fat_offset + (i + 1) * actual_sectors_per_fat) * SECTOR_SIZE] = fat_data
    
    # Write file data to clusters
    first_data_sector = boot_start + 32 + (2 * actual_sectors_per_fat)
    for i in range(num_clusters):
        sector = first_data_sector + ((file_start_cluster + i - 2) * sectors_per_cluster)
        start = i * cluster_size
        end = min(start + cluster_size, len(html_data))
        chunk = html_data[start:end]
        if len(chunk) < cluster_size:
            chunk = chunk + b'\x00' * (cluster_size - len(chunk))
        image[sector * SECTOR_SIZE:(sector + sectors_per_cluster) * SECTOR_SIZE] = chunk
    
    # Create root directory entry for file
    entry = create_directory_entry("BROWSER-", "HTML", 0x20, file_start_cluster, len(html_data))
    root_dir_sector = first_data_sector
    image[root_dir_sector * SECTOR_SIZE:root_dir_sector * SECTOR_SIZE + 32] = entry
    
    # Write image
    with open(output_path, 'wb') as f:
        f.write(image)
    
    print(f"Created {output_path}: {len(image)} bytes")
    print(f"File: BROWSER-TEST.HTML at cluster {file_start_cluster}")

if __name__ == '__main__':
    script_dir = os.path.dirname(os.path.abspath(__file__))
    html_path = os.path.join(script_dir, 'system', 'apps', 'browser-test.html')
    output_path = os.path.join(script_dir, 'test-sdcard.img')
    
    create_image_with_file(html_path, output_path)
