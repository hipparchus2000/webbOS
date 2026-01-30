#!/usr/bin/env python3
"""Verify the contents of the FAT32 disk image"""

import struct
import sys

def read_fat32_image(image_path):
    with open(image_path, 'rb') as f:
        data = f.read()
    
    # Check MBR
    print("=== MBR (Sector 0) ===")
    boot_sig = data[510:512]
    print(f"Boot signature: {boot_sig.hex()} (expected: 55aa)")
    
    # Check partition entry
    partition = data[446:462]
    bootable = partition[0]
    part_type = partition[4]
    start_lba = struct.unpack('<I', partition[8:12])[0]
    size_sectors = struct.unpack('<I', partition[12:16])[0]
    print(f"Bootable: {bootable:02x}")
    print(f"Partition type: {part_type:02x} (EF=ESP, EE=GPT protective)")
    print(f"Start LBA: {start_lba}")
    print(f"Size (sectors): {size_sectors}")
    
    # Check GPT if present
    gpt_sig = data[512:520]
    if gpt_sig == b'EFI PART':
        print("\n=== GPT Header (Sector 1) ===")
        print("GPT signature found!")
        
        # Read partition entries
        print("\n=== GPT Partition Entries ===")
        esp_type_guid = bytes.fromhex('28732AC11FF8D211BA4B00A0C93EC93B')
        for i in range(4):  # Check first 4 entries
            entry_offset = 1024 + i * 128
            entry = data[entry_offset:entry_offset+128]
            type_guid = entry[0:16]
            start = struct.unpack('<Q', entry[32:40])[0]
            end = struct.unpack('<Q', entry[40:48])[0]
            
            if type_guid == esp_type_guid:
                print(f"Entry {i}: EFI System Partition")
                print(f"  Start LBA: {start}")
                print(f"  End LBA: {end}")
                esp_start = start
                break
        else:
            print("ESP not found in first 4 entries")
            esp_start = None
    else:
        print("\nNo GPT found, using MBR partition")
        esp_start = start_lba if part_type == 0xEF else None
    
    if esp_start is None:
        print("ERROR: No ESP found!")
        return
    
    # Check FAT32 boot sector
    print(f"\n=== FAT32 Boot Sector (Sector {esp_start}) ===")
    boot_offset = esp_start * 512
    boot_sector = data[boot_offset:boot_offset+512]
    
    jump = boot_sector[0:3]
    oem = boot_sector[3:11]
    bytes_per_sector = struct.unpack('<H', boot_sector[11:13])[0]
    sectors_per_cluster = boot_sector[13]
    reserved_sectors = struct.unpack('<H', boot_sector[14:16])[0]
    num_fats = boot_sector[16]
    sectors_per_fat = struct.unpack('<I', boot_sector[36:40])[0]
    root_cluster = struct.unpack('<I', boot_sector[44:48])[0]
    
    print(f"Jump: {jump.hex()}")
    print(f"OEM: {oem}")
    print(f"Bytes per sector: {bytes_per_sector}")
    print(f"Sectors per cluster: {sectors_per_cluster}")
    print(f"Reserved sectors: {reserved_sectors}")
    print(f"Number of FATs: {num_fats}")
    print(f"Sectors per FAT: {sectors_per_fat}")
    print(f"Root cluster: {root_cluster}")
    
    boot_sig = boot_sector[510:512]
    print(f"Boot signature: {boot_sig.hex()}")
    
    # Calculate offsets
    fat1_offset = boot_offset + reserved_sectors * bytes_per_sector
    data_area_offset = fat1_offset + num_fats * sectors_per_fat * bytes_per_sector
    
    print(f"\nFAT1 offset: 0x{fat1_offset:x}")
    print(f"Data area offset: 0x{data_area_offset:x}")
    
    # Check FAT entries
    print("\n=== FAT Entries ===")
    fat = data[fat1_offset:fat1_offset + sectors_per_fat * bytes_per_sector]
    for i in range(5):
        entry = struct.unpack('<I', fat[i*4:i*4+4])[0] & 0x0FFFFFFF
        print(f"  Cluster {i}: 0x{entry:08x}")
    
    # Read root directory
    print(f"\n=== Root Directory (Cluster {root_cluster}) ===")
    root_offset = data_area_offset + (root_cluster - 2) * sectors_per_cluster * bytes_per_sector
    
    for i in range(16):  # Check first 16 entries
        entry_offset = root_offset + i * 32
        entry = data[entry_offset:entry_offset+32]
        
        # Check if entry is valid
        if entry[0] == 0x00:
            break  # End of directory
        if entry[0] == 0xE5:
            continue  # Deleted entry
        if entry[11] == 0x0F:
            continue  # Long file name entry
        
        name = entry[0:8].decode('ascii', errors='ignore').strip()
        ext = entry[8:11].decode('ascii', errors='ignore').strip()
        attr = entry[11]
        cluster = struct.unpack('<H', entry[26:28])[0] | (struct.unpack('<H', entry[20:22])[0] << 16)
        size = struct.unpack('<I', entry[28:32])[0]
        
        if ext:
            full_name = f"{name}.{ext}"
        else:
            full_name = name
        
        attr_str = ""
        if attr & 0x10:
            attr_str = "<DIR>"
        elif attr & 0x20:
            attr_str = "<FILE>"
        
        print(f"  {full_name:12s} {attr_str:8s} cluster={cluster:4d} size={size}")
        
        # If this is a directory, read it
        if attr & 0x10 and cluster >= 2:
            read_directory(data, data_area_offset, sectors_per_cluster, cluster, full_name, 1)

def read_directory(data, data_area_offset, sectors_per_cluster, cluster, parent_name, indent):
    """Read a subdirectory"""
    dir_offset = data_area_offset + (cluster - 2) * sectors_per_cluster * 512
    prefix = "  " * (indent + 1)
    
    for i in range(64):  # Check more entries
        entry_offset = dir_offset + i * 32
        entry = data[entry_offset:entry_offset+32]
        
        if entry[0] == 0x00:
            break
        if entry[0] == 0xE5:
            continue
        if entry[11] == 0x0F:
            continue
        
        name = entry[0:8].decode('ascii', errors='ignore').strip()
        ext = entry[8:11].decode('ascii', errors='ignore').strip()
        attr = entry[11]
        file_cluster = struct.unpack('<H', entry[26:28])[0] | (struct.unpack('<H', entry[20:22])[0] << 16)
        size = struct.unpack('<I', entry[28:32])[0]
        
        if name in ['.', '..']:
            continue
        
        if ext:
            full_name = f"{name}.{ext}"
        else:
            full_name = name
        
        attr_str = ""
        if attr & 0x10:
            attr_str = "<DIR>"
        elif attr & 0x20:
            attr_str = "<FILE>"
        
        print(f"{prefix}{full_name:12s} {attr_str:8s} cluster={file_cluster:4d} size={size}")

if __name__ == '__main__':
    if len(sys.argv) < 2:
        print("Usage: verify-image.py <image_path>")
        sys.exit(1)
    
    read_fat32_image(sys.argv[1])
