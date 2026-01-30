#!/usr/bin/env python3
"""
Copy files directly into a FAT32 disk image.
"""

import struct
import sys
import os
from datetime import datetime

def get_fat_time():
    """Get current time in FAT format"""
    now = datetime.now()
    time_val = (now.hour << 11) | (now.minute << 5) | (now.second // 2)
    date_val = ((now.year - 1980) << 9) | (now.month << 5) | now.day
    return time_val, date_val

def fat32_copy_file(image_path, src_file, dest_path):
    """Copy a file into the FAT32 image"""
    
    # Read image
    with open(image_path, 'r+b') as f:
        image = bytearray(f.read())
    
    # Parse boot sector (check for ESP at sector 1 or 2048)
    # Check MBR partition type
    mbr = image[0:512]
    part_type = mbr[450]
    if part_type == 0xEF:
        esp_start = struct.unpack('<I', mbr[454:458])[0]
    else:
        esp_start = 2048  # GPT default
    boot_offset = esp_start * 512
    boot_sector = image[boot_offset:boot_offset+512]
    
    bytes_per_sector = struct.unpack('<H', boot_sector[11:13])[0]
    sectors_per_cluster = boot_sector[13]
    reserved_sectors = struct.unpack('<H', boot_sector[14:16])[0]
    num_fats = boot_sector[16]
    sectors_per_fat = struct.unpack('<I', boot_sector[36:40])[0]
    root_cluster = struct.unpack('<I', boot_sector[44:48])[0]
    
    fat1_offset = boot_offset + reserved_sectors * bytes_per_sector
    data_offset = fat1_offset + num_fats * sectors_per_fat * bytes_per_sector
    
    # Parse destination path
    path_parts = [p for p in dest_path.split('/') if p]
    
    # Navigate to target directory
    current_cluster = root_cluster
    current_name = "ROOT"
    
    for part in path_parts[:-1]:  # Navigate through directories
        found = False
        dir_offset = data_offset + (current_cluster - 2) * sectors_per_cluster * bytes_per_sector
        
        for i in range(sectors_per_cluster * bytes_per_sector // 32):
            entry_offset = dir_offset + i * 32
            entry = image[entry_offset:entry_offset+32]
            
            if entry[0] == 0x00:
                break
            if entry[0] == 0xE5 or entry[11] == 0x0F:
                continue
            
            name = entry[0:8].decode('ascii', errors='ignore').strip()
            attr = entry[11]
            
            if name == part.upper() and attr & 0x10:
                current_cluster = struct.unpack('<H', entry[26:28])[0] | (struct.unpack('<H', entry[20:22])[0] << 16)
                current_name = part
                found = True
                break
        
        if not found:
            print(f"ERROR: Directory '{part}' not found in {current_name}")
            return False
    
    # Read source file
    with open(src_file, 'rb') as f:
        src_data = f.read()
    
    file_size = len(src_data)
    file_name = path_parts[-1].upper()
    
    # Split filename into name and extension
    if '.' in file_name:
        name, ext = file_name.rsplit('.', 1)
    else:
        name, ext = file_name, ''
    
    name = name[:8].ljust(8, ' ')
    ext = ext[:3].ljust(3, ' ')
    
    # Find free directory entry
    dir_offset = data_offset + (current_cluster - 2) * sectors_per_cluster * bytes_per_sector
    entry_idx = None
    
    for i in range(sectors_per_cluster * bytes_per_sector // 32):
        entry_offset = dir_offset + i * 32
        entry = image[entry_offset:entry_offset+32]
        
        if entry[0] == 0x00 or entry[0] == 0xE5:
            entry_idx = i
            break
    
    if entry_idx is None:
        print("ERROR: No free directory entries")
        return False
    
    # Find free clusters for file data
    clusters_needed = (file_size + sectors_per_cluster * bytes_per_sector - 1) // (sectors_per_cluster * bytes_per_sector)
    if clusters_needed == 0:
        clusters_needed = 1
    
    fat = image[fat1_offset:fat1_offset + sectors_per_fat * bytes_per_sector]
    free_clusters = []
    
    for i in range(2, sectors_per_fat * bytes_per_sector // 4):
        entry = struct.unpack('<I', fat[i*4:i*4+4])[0] & 0x0FFFFFFF
        if entry == 0:
            free_clusters.append(i)
            if len(free_clusters) >= clusters_needed:
                break
    
    if len(free_clusters) < clusters_needed:
        print(f"ERROR: Not enough free clusters (need {clusters_needed}, found {len(free_clusters)})")
        return False
    
    # Write file data to clusters
    data_remaining = file_size
    data_ptr = 0
    
    for i, cluster in enumerate(free_clusters):
        cluster_offset = data_offset + (cluster - 2) * sectors_per_cluster * bytes_per_sector
        
        to_write = min(data_remaining, sectors_per_cluster * bytes_per_sector)
        image[cluster_offset:cluster_offset + to_write] = src_data[data_ptr:data_ptr + to_write]
        
        data_ptr += to_write
        data_remaining -= to_write
        
        # Update FAT
        if i < len(free_clusters) - 1:
            # Point to next cluster
            fat[cluster*4:cluster*4+4] = struct.pack('<I', free_clusters[i+1] & 0x0FFFFFFF)
        else:
            # End of chain
            fat[cluster*4:cluster*4+4] = struct.pack('<I', 0x0FFFFFFF)
    
    # Write FATs back
    image[fat1_offset:fat1_offset + sectors_per_fat * bytes_per_sector] = fat
    fat2_offset = fat1_offset + sectors_per_fat * bytes_per_sector
    image[fat2_offset:fat2_offset + sectors_per_fat * bytes_per_sector] = fat
    
    # Create directory entry
    entry_offset = dir_offset + entry_idx * 32
    time_val, date_val = get_fat_time()
    
    entry = bytearray(32)
    entry[0:8] = name.encode('ascii')
    entry[8:11] = ext.encode('ascii')
    entry[11] = 0x20  # Archive attribute
    entry[14:16] = struct.pack('<H', time_val)  # Creation time
    entry[16:18] = struct.pack('<H', date_val)  # Creation date
    entry[18:20] = struct.pack('<H', date_val)  # Access date
    entry[20:22] = struct.pack('<H', free_clusters[0] >> 16)  # High cluster
    entry[22:24] = struct.pack('<H', time_val)  # Modification time
    entry[24:26] = struct.pack('<H', date_val)  # Modification date
    entry[26:28] = struct.pack('<H', free_clusters[0] & 0xFFFF)  # Low cluster
    entry[28:32] = struct.pack('<I', file_size)
    
    image[entry_offset:entry_offset+32] = entry
    
    # Write image back
    with open(image_path, 'wb') as f:
        f.write(image)
    
    print(f"Copied {src_file} -> {dest_path} ({file_size} bytes, {clusters_needed} clusters)")
    return True

if __name__ == '__main__':
    if len(sys.argv) < 4:
        print("Usage: copy-to-image.py <image_path> <src_file> <dest_path>")
        print("Example: copy-to-image.py webbos.img bootloader.efi EFI/BOOT/BOOTX64.EFI")
        sys.exit(1)
    
    image_path = sys.argv[1]
    src_file = sys.argv[2]
    dest_path = sys.argv[3]
    
    if fat32_copy_file(image_path, src_file, dest_path):
        print("Success!")
    else:
        print("Failed!")
        sys.exit(1)
