#!/usr/bin/env python3
"""
Add files and directories to a FAT32 disk image.
"""

import sys
import struct
import os

def parse_fat32_bpb(f):
    """Parse FAT32 BIOS Parameter Block."""
    f.seek(0)
    boot_sector = f.read(512)

    if boot_sector[510:512] != b'\x55\xAA':
        raise ValueError("Invalid boot sector signature")

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
    """Convert cluster number to byte offset."""
    first_data_sector = bpb['reserved_sectors'] + (bpb['num_fats'] * bpb['sectors_per_fat'])
    sector = first_data_sector + ((cluster - 2) * bpb['sectors_per_cluster'])
    return sector * bpb['bytes_per_sector']

def read_cluster(f, cluster, bpb):
    """Read a cluster from the filesystem."""
    offset = cluster_to_offset(cluster, bpb)
    size = bpb['sectors_per_cluster'] * bpb['bytes_per_sector']
    f.seek(offset)
    return f.read(size)

def write_cluster(f, cluster, bpb, data):
    """Write data to a cluster."""
    offset = cluster_to_offset(cluster, bpb)
    cluster_size = bpb['sectors_per_cluster'] * bpb['bytes_per_sector']

    # Pad data to cluster size
    if len(data) < cluster_size:
        data = data + b'\x00' * (cluster_size - len(data))

    f.seek(offset)
    f.write(data[:cluster_size])

def read_fat(f, bpb):
    """Read the FAT table."""
    fat_size = bpb['sectors_per_fat'] * bpb['bytes_per_sector']
    f.seek(bpb['reserved_sectors'] * bpb['bytes_per_sector'])
    fat_data = bytearray(f.read(fat_size))

    entries = []
    for i in range(0, len(fat_data), 4):
        entry = struct.unpack('<I', fat_data[i:i+4])[0] & 0x0FFFFFFF
        entries.append(entry)
    return entries, fat_data

def write_fat(f, bpb, fat_data):
    """Write the FAT table to all FAT copies."""
    fat_size = bpb['sectors_per_fat'] * bpb['bytes_per_sector']

    for fat_num in range(bpb['num_fats']):
        offset = (bpb['reserved_sectors'] + fat_num * bpb['sectors_per_fat']) * bpb['bytes_per_sector']
        f.seek(offset)
        f.write(fat_data)

def allocate_cluster(fat_entries, fat_data):
    """Find and allocate a free cluster."""
    for i in range(2, len(fat_entries)):
        if fat_entries[i] == 0:
            # Mark as end of chain
            new_value = 0x0FFFFFFF
            fat_entries[i] = new_value
            struct.pack_into('<I', fat_data, i * 4, new_value)
            return i
    raise ValueError("No free clusters available")

def get_cluster_chain(fat, start_cluster):
    """Get the chain of clusters."""
    chain = [start_cluster]
    current = start_cluster

    while current < len(fat):
        next_cluster = fat[current]
        if next_cluster >= 0x0FFFFFF8:
            break
        if next_cluster == 0:
            break
        chain.append(next_cluster)
        current = next_cluster

    return chain

def parse_directory_entry(data, offset):
    """Parse a single directory entry."""
    entry = data[offset:offset+32]

    if entry[0] == 0x00:
        return None, 'end'
    if entry[0] == 0xE5:
        return None, 'deleted'
    if entry[11] == 0x0F:
        return None, 'lfn'
    if entry[11] & 0x08:
        return None, 'label'

    name = entry[0:11].decode('latin-1').strip()
    attr = entry[11]
    start_cluster = struct.unpack('<H', entry[26:28])[0] | (struct.unpack('<H', entry[20:22])[0] << 16)
    size = struct.unpack('<I', entry[28:32])[0]

    return {
        'name': name,
        'attr': attr,
        'start_cluster': start_cluster,
        'size': size,
        'offset': offset
    }, 'file'

def create_dir_entry(name, ext, attr, start_cluster, size):
    """Create a directory entry."""
    entry = bytearray(32)

    # Name (8.3 format)
    entry[0:8] = name.upper().ljust(8).encode('latin-1')[:8]
    entry[8:11] = ext.upper().ljust(3).encode('latin-1')[:3]

    # Attributes
    entry[11] = attr

    # Cluster
    entry[26:28] = struct.pack('<H', start_cluster & 0xFFFF)
    entry[20:22] = struct.pack('<H', (start_cluster >> 16) & 0xFFFF)

    # Size
    entry[28:32] = struct.pack('<I', size)

    return bytes(entry)

def list_directory(f, bpb, start_cluster):
    """List all entries in a directory."""
    fat_entries, _ = read_fat(f, bpb)
    clusters = get_cluster_chain(fat_entries, start_cluster)

    entries = []
    for cluster in clusters:
        data = read_cluster(f, cluster, bpb)
        for i in range(0, len(data), 32):
            entry, entry_type = parse_directory_entry(data, i)
            if entry_type == 'end':
                return entries, cluster, i
            if entry and entry_type == 'file':
                entries.append(entry)

    return entries, clusters[-1], 0

def add_directory_entry(f, bpb, dir_cluster, name, ext, attr, start_cluster, size):
    """Add a new entry to a directory."""
    fat_entries, fat_data = read_fat(f, bpb)
    clusters = get_cluster_chain(fat_entries, dir_cluster)

    # Find free entry
    for cluster in clusters:
        data = bytearray(read_cluster(f, cluster, bpb))
        for i in range(0, len(data), 32):
            if data[i] == 0x00 or data[i] == 0xE5:
                # Found free slot
                entry = create_dir_entry(name, ext, attr, start_cluster, size)
                data[i:i+32] = entry
                write_cluster(f, cluster, bpb, bytes(data))
                return True

    # Need to allocate new cluster for directory
    new_cluster = allocate_cluster(fat_entries, fat_data)

    # Link it to the chain
    last_cluster = clusters[-1]
    struct.pack_into('<I', fat_data, last_cluster * 4, new_cluster)
    fat_entries[last_cluster] = new_cluster

    # Write FAT
    write_fat(f, bpb, fat_data)

    # Create entry in new cluster
    data = bytearray(bpb['sectors_per_cluster'] * bpb['bytes_per_sector'])
    entry = create_dir_entry(name, ext, attr, start_cluster, size)
    data[0:32] = entry
    write_cluster(f, new_cluster, bpb, bytes(data))

    return True

def find_directory(f, bpb, parent_cluster, dirname):
    """Find a subdirectory by name."""
    entries, _, _ = list_directory(f, bpb, parent_cluster)
    dirname_83 = dirname.upper().ljust(11)[:11]

    for entry in entries:
        if entry['name'].upper() == dirname_83 and entry['attr'] & 0x10:
            return entry['start_cluster']
    return None

def create_directory(f, bpb, parent_cluster, dirname):
    """Create a new directory."""
    fat_entries, fat_data = read_fat(f, bpb)

    # Allocate cluster for new directory
    new_cluster = allocate_cluster(fat_entries, fat_data)
    write_fat(f, bpb, fat_data)

    # Initialize directory cluster
    data = bytearray(bpb['sectors_per_cluster'] * bpb['bytes_per_sector'])
    write_cluster(f, new_cluster, bpb, bytes(data))

    # Add entry to parent directory
    add_directory_entry(f, bpb, parent_cluster, dirname, '', 0x10, new_cluster, 0)

    return new_cluster

def add_file_to_image(image_path, dest_path, source_file):
    """Add a file to the FAT32 image."""
    with open(image_path, 'r+b') as f:
        bpb = parse_fat32_bpb(f)

        # Parse destination path
        parts = [p for p in dest_path.replace('\\', '/').split('/') if p]

        # Start from root
        current_cluster = bpb['root_cluster']

        # Navigate/create directories
        for part in parts[:-1]:
            next_cluster = find_directory(f, bpb, current_cluster, part)
            if next_cluster is None:
                print(f"Creating directory: {part}")
                next_cluster = create_directory(f, bpb, current_cluster, part)
            current_cluster = next_cluster

        # Get filename
        filename = parts[-1]
        if '.' in filename:
            name, ext = filename.rsplit('.', 1)
            name = name[:8]
            ext = ext[:3]
        else:
            name = filename[:8]
            ext = ''

        # Read source file
        with open(source_file, 'rb') as src:
            file_data = src.read()

        file_size = len(file_data)
        cluster_size = bpb['sectors_per_cluster'] * bpb['bytes_per_sector']
        num_clusters = (file_size + cluster_size - 1) // cluster_size

        # Allocate clusters
        fat_entries, fat_data = read_fat(f, bpb)
        clusters = []

        for i in range(num_clusters):
            cluster = allocate_cluster(fat_entries, fat_data)
            if i > 0:
                # Link to previous cluster
                prev_cluster = clusters[-1]
                struct.pack_into('<I', fat_data, prev_cluster * 4, cluster)
                fat_entries[prev_cluster] = cluster
            clusters.append(cluster)

        # Write FAT
        write_fat(f, bpb, fat_data)

        # Write file data
        for i, cluster in enumerate(clusters):
            start = i * cluster_size
            end = min(start + cluster_size, file_size)
            chunk = file_data[start:end]
            write_cluster(f, cluster, bpb, chunk)

        # Add directory entry
        add_directory_entry(f, bpb, current_cluster, name, ext, 0x20, clusters[0], file_size)

        print(f"Added {dest_path} ({file_size} bytes)")

def main():
    if len(sys.argv) < 4:
        print(f"Usage: {sys.argv[0]} <image> <dest_path> <source_file>")
        print(f"Example: {sys.argv[0]} webbos.img system/icons/browser.png icons/browser.png")
        sys.exit(1)

    image_path = sys.argv[1]
    dest_path = sys.argv[2]
    source_file = sys.argv[3]

    add_file_to_image(image_path, dest_path, source_file)

if __name__ == '__main__':
    main()
