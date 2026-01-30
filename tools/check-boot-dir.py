#!/usr/bin/env python3
"""Check the contents of the EFI/BOOT directory"""

import struct

with open('webbos.img', 'rb') as f:
    data = bytearray(f.read())

esp_start = 2048
boot_offset = esp_start * 512
boot_sector = data[boot_offset:boot_offset+512]

bytes_per_sector = struct.unpack('<H', boot_sector[11:13])[0]
sectors_per_cluster = boot_sector[13]
reserved_sectors = struct.unpack('<H', boot_sector[14:16])[0]
num_fats = boot_sector[16]
sectors_per_fat = struct.unpack('<I', boot_sector[36:40])[0]

fat1_offset = boot_offset + reserved_sectors * bytes_per_sector
data_offset = fat1_offset + num_fats * sectors_per_fat * bytes_per_sector

# EFI/BOOT is at cluster 4
boot_dir_cluster = 4
boot_dir_offset = data_offset + (boot_dir_cluster - 2) * sectors_per_cluster * bytes_per_sector

print(f"BOOT directory at cluster {boot_dir_cluster}, offset 0x{boot_dir_offset:x}")
print("\nDirectory entries:")

for i in range(32):
    entry_offset = boot_dir_offset + i * 32
    entry = data[entry_offset:entry_offset+32]
    
    if entry[0] == 0x00:
        break
    
    status = entry[0]
    name = entry[0:11].hex()
    attr = entry[11]
    cluster_low = struct.unpack('<H', entry[26:28])[0]
    cluster_high = struct.unpack('<H', entry[20:22])[0]
    cluster = cluster_low | (cluster_high << 16)
    size = struct.unpack('<I', entry[28:32])[0]
    
    attr_str = ""
    if attr & 0x01: attr_str += "R"
    if attr & 0x02: attr_str += "H"
    if attr & 0x04: attr_str += "S"
    if attr & 0x08: attr_str += "V"
    if attr & 0x10: attr_str += "D"
    if attr & 0x20: attr_str += "A"
    
    name_decoded = entry[0:8].decode('ascii', errors='ignore').strip()
    ext = entry[8:11].decode('ascii', errors='ignore').strip()
    if ext:
        full_name = f"{name_decoded}.{ext}"
    else:
        full_name = name_decoded
    
    print(f"  Entry {i}: status=0x{status:02x} name='{full_name}' attr=0x{attr:02x}({attr_str}) cluster={cluster} size={size}")
