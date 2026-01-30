#!/usr/bin/env python3
"""
Update files in a FAT32 disk image without requiring WSL.
This script locates files by name in the FAT32 filesystem and overwrites them.
"""

import sys
import struct

def read_sector(f, sector_size=512):
    """Read a single sector from the file."""
    return f.read(sector_size)

def parse_fat32_bpb(f):
    """Parse FAT32 BIOS Parameter Block."""
    f.seek(0)
    boot_sector = f.read(512)
    
    # Check signature
    if boot_sector[510:512] != b'\x55\xAA':
        raise ValueError("Invalid boot sector signature")
    
    # Parse BPB
    bytes_per_sector = struct.unpack('<H', boot_sector[11:13])[0]
    sectors_per_cluster = boot_sector[13]
    reserved_sectors = struct.unpack('<H', boot_sector[14:16])[0]
    num_fats = boot_sector[16]
    total_sectors = struct.unpack('<I', boot_sector[32:36])[0]
    sectors_per_fat = struct.unpack('<I', boot_sector[36:40])[0]
    root_cluster = struct.unpack('<I', boot_sector[44:48])[0]
    
    return {
        'bytes_per_sector': bytes_per_sector,
        'sectors_per_cluster': sectors_per_cluster,
        'reserved_sectors': reserved_sectors,
        'num_fats': num_fats,
        'total_sectors': total_sectors,
        'sectors_per_fat': sectors_per_fat,
        'root_cluster': root_cluster,
    }

def cluster_to_offset(cluster, bpb):
    """Convert cluster number to byte offset in the file."""
    first_data_sector = bpb['reserved_sectors'] + (bpb['num_fats'] * bpb['sectors_per_fat'])
    sector = first_data_sector + ((cluster - 2) * bpb['sectors_per_cluster'])
    return sector * bpb['bytes_per_sector']

def read_cluster(f, cluster, bpb):
    """Read a cluster from the filesystem."""
    offset = cluster_to_offset(cluster, bpb)
    size = bpb['sectors_per_cluster'] * bpb['bytes_per_sector']
    f.seek(offset)
    return f.read(size)

def read_fat(f, bpb):
    """Read the FAT table."""
    fat_size = bpb['sectors_per_fat'] * bpb['bytes_per_sector']
    f.seek(bpb['reserved_sectors'] * bpb['bytes_per_sector'])
    fat_data = f.read(fat_size)
    
    # Parse FAT entries (4 bytes each for FAT32)
    entries = []
    for i in range(0, len(fat_data), 4):
        entry = struct.unpack('<I', fat_data[i:i+4])[0] & 0x0FFFFFFF
        entries.append(entry)
    return entries

def get_cluster_chain(fat, start_cluster):
    """Get the chain of clusters starting from start_cluster."""
    chain = [start_cluster]
    current = start_cluster
    
    while current < len(fat):
        next_cluster = fat[current]
        if next_cluster >= 0x0FFFFFF8:  # End of chain
            break
        if next_cluster == 0:  # Free cluster
            break
        chain.append(next_cluster)
        current = next_cluster
    
    return chain

def parse_directory_entry(data, offset):
    """Parse a single directory entry."""
    entry = data[offset:offset+32]
    
    # Check for deleted or empty entry
    if entry[0] == 0x00:
        return None, 'end'
    if entry[0] == 0xE5:
        return None, 'deleted'
    
    # Check for long filename entry
    if entry[11] == 0x0F:
        return None, 'lfn'
    
    # Check for volume label
    if entry[11] & 0x08:
        return None, 'label'
    
    # Regular 8.3 entry
    name = entry[0:11].decode('latin-1').strip()
    attr = entry[11]
    start_cluster = struct.unpack('<H', entry[26:28])[0] | (struct.unpack('<H', entry[20:22])[0] << 16)
    size = struct.unpack('<I', entry[28:32])[0]
    
    return {
        'name': name,
        'attr': attr,
        'start_cluster': start_cluster,
        'size': size,
    }, 'file'

def list_directory(f, bpb, start_cluster):
    """List all files in a directory."""
    fat = read_fat(f, bpb)
    clusters = get_cluster_chain(fat, start_cluster)
    
    files = []
    for cluster in clusters:
        data = read_cluster(f, cluster, bpb)
        for i in range(0, len(data), 32):
            entry, entry_type = parse_directory_entry(data, i)
            if entry_type == 'end':
                break
            if entry and entry_type == 'file':
                files.append(entry)
    
    return files

def find_file_in_dir(f, bpb, dir_cluster, filename):
    """Find a file by name in a directory."""
    files = list_directory(f, bpb, dir_cluster)
    for file in files:
        # Convert 8.3 name to something we can compare
        name = file['name']
        if name.upper() == filename.upper():
            return file
    return None

def find_directory(f, bpb, parent_cluster, dirname):
    """Find a subdirectory by name."""
    files = list_directory(f, bpb, parent_cluster)
    for file in files:
        if file['name'].upper() == dirname.upper() and file['attr'] & 0x10:
            return file['start_cluster']
    return None

def update_file(f, bpb, file_entry, new_data):
    """Update a file with new content."""
    fat = read_fat(f, bpb)
    clusters = get_cluster_chain(fat, file_entry['start_cluster'])
    
    cluster_size = bpb['sectors_per_cluster'] * bpb['bytes_per_sector']
    
    # Write data to clusters
    for i, cluster in enumerate(clusters):
        offset = cluster_to_offset(cluster, bpb)
        start = i * cluster_size
        end = min(start + cluster_size, len(new_data))
        chunk = new_data[start:end]
        
        # Pad to cluster size
        if len(chunk) < cluster_size:
            chunk = chunk + b'\x00' * (cluster_size - len(chunk))
        
        f.seek(offset)
        f.write(chunk)
        
        if end >= len(new_data):
            break
    
    # Update file size in directory entry
    # This is complex as we'd need to find the directory entry again
    # For now, we just overwrite the data
    print(f"Updated file (size: {len(new_data)} bytes)")

def update_file_in_image(image_path, file_in_image, source_file):
    """Update a file in the disk image."""
    with open(image_path, 'r+b') as f:
        bpb = parse_fat32_bpb(f)
        
        # Parse the path
        parts = [p for p in file_in_image.replace('\\', '/').split('/') if p]
        
        # Start from root
        current_cluster = bpb['root_cluster']
        
        # Navigate to parent directory
        for part in parts[:-1]:
            current_cluster = find_directory(f, bpb, current_cluster, part)
            if current_cluster is None:
                raise ValueError(f"Directory not found: {part}")
        
        # Find the file
        filename = parts[-1]
        # Convert to 8.3 format for comparison
        if '.' in filename:
            name, ext = filename.rsplit('.', 1)
            name = name[:8].ljust(8)
            ext = ext[:3].ljust(3)
            fat_name = name + ext
        else:
            fat_name = filename[:11].ljust(11)
        
        file_entry = find_file_in_dir(f, bpb, current_cluster, fat_name)
        if file_entry is None:
            raise ValueError(f"File not found: {filename}")
        
        print(f"Found file: {file_entry}")
        
        # Read new content
        with open(source_file, 'rb') as src:
            new_data = src.read()
        
        # Update the file
        update_file(f, bpb, file_entry, new_data)

def main():
    if len(sys.argv) < 4:
        print(f"Usage: {sys.argv[0]} <image> <file_in_image> <source_file>")
        print(f"Example: {sys.argv[0]} webbos.img EFI/BOOT/BOOTX64.EFI target/x86_64-unknown-uefi/debug/bootloader.efi")
        sys.exit(1)
    
    image_path = sys.argv[1]
    file_in_image = sys.argv[2]
    source_file = sys.argv[3]
    
    update_file_in_image(image_path, file_in_image, source_file)

if __name__ == '__main__':
    main()
