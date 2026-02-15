#!/usr/bin/env python3
"""
Update files in a FAT32 disk image without requiring WSL.
This script locates files by name in the FAT32 filesystem and overwrites them.

If the new file is larger than the allocated clusters, new clusters will be
allocated automatically. If smaller, excess clusters are freed.

Usage:
    python update-image.py <image> <file_in_image> <source_file>
    
Examples:
    python update-image.py webbos.img "EFI/BOOT/BOOTX64.EFI" target/x86_64-unknown-uefi/debug/bootloader.efi
    python update-image.py webbos.img kernel.elf target/x86_64-unknown-none/debug/kernel
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


def write_cluster(f, cluster, bpb, data):
    """Write data to a cluster."""
    offset = cluster_to_offset(cluster, bpb)
    cluster_size = bpb['sectors_per_cluster'] * bpb['bytes_per_sector']
    
    # Pad data to cluster size if needed
    if len(data) < cluster_size:
        data = data + b'\x00' * (cluster_size - len(data))
    
    f.seek(offset)
    f.write(data[:cluster_size])


def read_fat(f, bpb):
    """Read the FAT table and return entries and raw data."""
    fat_size = bpb['sectors_per_fat'] * bpb['bytes_per_sector']
    f.seek(bpb['reserved_sectors'] * bpb['bytes_per_sector'])
    fat_data = bytearray(f.read(fat_size))
    
    # Parse FAT entries (4 bytes each for FAT32)
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


def free_cluster_chain(fat_entries, fat_data, start_cluster):
    """Free all clusters in a chain starting from start_cluster."""
    current = start_cluster
    while current < len(fat_entries):
        next_cluster = fat_entries[current]
        # Mark as free
        fat_entries[current] = 0
        struct.pack_into('<I', fat_data, current * 4, 0)
        if next_cluster >= 0x0FFFFFF8 or next_cluster == 0:
            break
        current = next_cluster


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
    fat, _ = read_fat(f, bpb)
    clusters = get_cluster_chain(fat, start_cluster)
    
    files = []
    for cluster in clusters:
        data = read_cluster(f, cluster, bpb)
        for i in range(0, len(data), 32):
            entry, entry_type = parse_directory_entry(data, i)
            if entry_type == 'end':
                break
            if entry and entry_type == 'file':
                files.append((entry, cluster, i))
    
    return files


def find_file_in_dir(f, bpb, dir_cluster, filename):
    """Find a file by name in a directory. Returns (entry, cluster, offset) or None."""
    files = list_directory(f, bpb, dir_cluster)
    for entry, cluster, offset in files:
        if entry['name'].upper() == filename.upper():
            return entry, cluster, offset
    return None, None, None


def find_directory(f, bpb, parent_cluster, dirname):
    """Find a subdirectory by name. Returns cluster number or None."""
    files = list_directory(f, bpb, parent_cluster)
    for entry, _, _ in files:
        if entry['name'].upper() == dirname.upper() and entry['attr'] & 0x10:
            return entry['start_cluster']
    return None


def update_directory_entry(f, bpb, dir_cluster, entry_offset, new_size):
    """Update the file size in a directory entry."""
    # Read the cluster containing the directory entry
    fat, _ = read_fat(f, bpb)
    clusters = get_cluster_chain(fat, dir_cluster)
    
    # Find which cluster contains the entry
    cluster_size = bpb['sectors_per_cluster'] * bpb['bytes_per_sector']
    entries_per_cluster = cluster_size // 32
    
    for cluster_idx, cluster in enumerate(clusters):
        start_offset = cluster_idx * cluster_size
        end_offset = start_offset + cluster_size
        if start_offset <= entry_offset < end_offset:
            # This cluster contains the entry
            offset_in_cluster = entry_offset - start_offset
            data = bytearray(read_cluster(f, cluster, bpb))
            
            # Update size (bytes 28-31)
            data[offset_in_cluster + 28:offset_in_cluster + 32] = struct.pack('<I', new_size)
            
            write_cluster(f, cluster, bpb, bytes(data))
            return True
    
    return False


def update_file(f, bpb, file_entry, entry_cluster, entry_offset, new_data):
    """Update a file with new content, allocating new clusters if needed."""
    fat_entries, fat_data = read_fat(f, bpb)
    old_clusters = get_cluster_chain(fat_entries, file_entry['start_cluster'])
    
    cluster_size = bpb['sectors_per_cluster'] * bpb['bytes_per_sector']
    new_size = len(new_data)
    old_size = file_entry['size']
    
    num_clusters_needed = (new_size + cluster_size - 1) // cluster_size
    num_clusters_available = len(old_clusters)
    
    print(f"  Old size: {old_size} bytes ({num_clusters_available} clusters)")
    print(f"  New size: {new_size} bytes ({num_clusters_needed} clusters)")
    
    # Allocate or free clusters as needed
    if num_clusters_needed > num_clusters_available:
        # Need more clusters
        clusters_to_allocate = num_clusters_needed - num_clusters_available
        print(f"  Allocating {clusters_to_allocate} new cluster(s)...")
        
        # Link last old cluster to first new cluster
        if old_clusters:
            prev_cluster = old_clusters[-1]
        else:
            # No existing clusters - this shouldn't happen for an existing file
            raise ValueError("File has no clusters allocated")
        
        for _ in range(clusters_to_allocate):
            new_cluster = allocate_cluster(fat_entries, fat_data)
            # Link previous cluster to new one
            fat_entries[prev_cluster] = new_cluster
            struct.pack_into('<I', fat_data, prev_cluster * 4, new_cluster)
            old_clusters.append(new_cluster)
            prev_cluster = new_cluster
            
    elif num_clusters_needed < num_clusters_available:
        # Have excess clusters - free them
        clusters_to_free = old_clusters[num_clusters_needed:]
        print(f"  Freeing {len(clusters_to_free)} excess cluster(s)...")
        
        # Mark end of chain at last used cluster
        if num_clusters_needed > 0:
            last_used = old_clusters[num_clusters_needed - 1]
            fat_entries[last_used] = 0x0FFFFFFF
            struct.pack_into('<I', fat_data, last_used * 4, 0x0FFFFFFF)
        
        # Free excess clusters
        for cluster in clusters_to_free:
            fat_entries[cluster] = 0
            struct.pack_into('<I', fat_data, cluster * 4, 0)
        
        old_clusters = old_clusters[:num_clusters_needed]
    
    # Write the updated FAT
    write_fat(f, bpb, fat_data)
    
    # Write data to clusters
    clusters = old_clusters if old_clusters else [file_entry['start_cluster']]
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
    update_directory_entry(f, bpb, entry_cluster, entry_offset, new_size)
    
    print(f"  Updated successfully!")


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
        
        file_entry, entry_cluster, entry_offset = find_file_in_dir(f, bpb, current_cluster, fat_name)
        if file_entry is None:
            raise ValueError(f"File not found: {filename}")
        
        print(f"Found file: {file_entry['name']} (current size: {file_entry['size']} bytes)")
        
        # Read new content
        with open(source_file, 'rb') as src:
            new_data = src.read()
        
        # Update the file
        # Calculate entry offset within the directory cluster
        cluster_size = bpb['sectors_per_cluster'] * bpb['bytes_per_sector']
        entries_per_cluster = cluster_size // 32
        
        # Find the actual byte offset of the directory entry
        fat, _ = read_fat(f, bpb)
        clusters = get_cluster_chain(fat, current_cluster)
        
        entry_byte_offset = None
        for cluster in clusters:
            data = read_cluster(f, cluster, bpb)
            for i in range(0, len(data), 32):
                entry, entry_type = parse_directory_entry(data, i)
                if entry_type == 'end':
                    break
                if entry and entry_type == 'file':
                    if entry['name'].upper() == fat_name.upper():
                        entry_byte_offset = (cluster - 2) * cluster_size + i
                        entry_cluster_actual = current_cluster
                        break
            if entry_byte_offset is not None:
                break
        
        if entry_byte_offset is None:
            raise ValueError(f"Could not locate directory entry for {filename}")
        
        update_file(f, bpb, file_entry, entry_cluster_actual, entry_byte_offset, new_data)


def main():
    if len(sys.argv) < 4:
        print(f"Usage: {sys.argv[0]} <image> <file_in_image> <source_file>")
        print(f"Example: {sys.argv[0]} webbos.img \"EFI/BOOT/BOOTX64.EFI\" target/x86_64-unknown-uefi/debug/bootloader.efi")
        print(f"         {sys.argv[0]} webbos.img kernel.elf target/x86_64-unknown-none/debug/kernel")
        print()
        print("This script updates files in a FAT32 disk image, automatically allocating")
        print("new clusters if the file has grown, or freeing excess clusters if shrunk.")
        sys.exit(1)
    
    image_path = sys.argv[1]
    file_in_image = sys.argv[2]
    source_file = sys.argv[3]
    
    try:
        update_file_in_image(image_path, file_in_image, source_file)
    except ValueError as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)
    except Exception as e:
        print(f"Unexpected error: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == '__main__':
    main()
