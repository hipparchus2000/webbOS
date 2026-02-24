#!/usr/bin/env python3
"""
Update files in a Raspberry Pi SD card image.

This script updates files in the FAT32 boot partition of a Pi SD card image.
It can update the kernel, config.txt, or add/remove files.

Usage:
    python update-sdcard.py <image> <command> [options]
    
Commands:
    kernel <kernel_path>     Update kernel8.img
    config <config_path>     Update config.txt
    cmdline <cmdline_path>   Update cmdline.txt
    add <source> <dest>      Add a file to boot partition
    rm <filename>            Remove a file from boot partition
    ls                       List files in boot partition
    
Examples:
    python update-sdcard.py webbos-pi.img kernel target/aarch64-unknown-none/release/kernel
    python update-sdcard.py webbos-pi.img config my-config.txt
    python update-sdcard.py webbos-pi.img ls
"""

import sys
import struct
import os
import argparse

SECTOR_SIZE = 512
FIRST_PARTITION_START = 2048  # 1MB offset


def parse_mbr(f):
    """Parse MBR and return partition information."""
    f.seek(0)
    mbr = f.read(512)
    
    if mbr[510:512] != b'\x55\xAA':
        raise ValueError("Invalid MBR signature")
    
    partitions = []
    for i in range(4):
        offset = 446 + (i * 16)
        entry = mbr[offset:offset+16]
        
        status = entry[0]
        partition_type = entry[4]
        start_lba = struct.unpack('<I', entry[8:12])[0]
        num_sectors = struct.unpack('<I', entry[12:16])[0]
        
        if partition_type != 0:  # Non-empty partition
            partitions.append({
                'number': i + 1,
                'status': status,
                'type': partition_type,
                'start_lba': start_lba,
                'num_sectors': num_sectors,
                'type_name': get_partition_type_name(partition_type)
            })
    
    return partitions


def get_partition_type_name(ptype):
    """Get human-readable partition type name."""
    types = {
        0x0C: 'FAT32 (LBA)',
        0x0B: 'FAT32',
        0x83: 'Linux',
        0x07: 'NTFS/HPFS',
        0xEE: 'GPT',
    }
    return types.get(ptype, f'0x{ptype:02X}')


def parse_fat32_bpb(f, partition_start):
    """Parse FAT32 BIOS Parameter Block from boot sector."""
    f.seek(partition_start * SECTOR_SIZE)
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
        'partition_start': partition_start,
        'bytes_per_sector': bytes_per_sector,
        'sectors_per_cluster': sectors_per_cluster,
        'reserved_sectors': reserved_sectors,
        'num_fats': num_fats,
        'total_sectors': total_sectors,
        'sectors_per_fat': sectors_per_fat,
        'root_cluster': root_cluster,
    }


def cluster_to_offset(cluster, bpb):
    """Convert cluster number to byte offset in image."""
    first_data_sector = bpb['partition_start'] + bpb['reserved_sectors'] + \
                        (bpb['num_fats'] * bpb['sectors_per_fat'])
    sector = first_data_sector + ((cluster - 2) * bpb['sectors_per_cluster'])
    return sector * bpb['bytes_per_sector']


def read_fat(f, bpb):
    """Read FAT entries."""
    fat_size = bpb['sectors_per_fat'] * bpb['bytes_per_sector']
    fat_offset = (bpb['partition_start'] + bpb['reserved_sectors']) * bpb['bytes_per_sector']
    
    f.seek(fat_offset)
    fat_data = bytearray(f.read(fat_size))
    
    entries = []
    for i in range(0, len(fat_data), 4):
        entry = struct.unpack('<I', fat_data[i:i+4])[0] & 0x0FFFFFFF
        entries.append(entry)
    
    return entries, fat_data


def write_fat(f, bpb, fat_data):
    """Write FAT to all copies."""
    fat_size = bpb['sectors_per_fat'] * bpb['bytes_per_sector']
    
    for fat_num in range(bpb['num_fats']):
        offset = (bpb['partition_start'] + bpb['reserved_sectors'] + 
                  fat_num * bpb['sectors_per_fat']) * bpb['bytes_per_sector']
        f.seek(offset)
        f.write(fat_data)


def get_cluster_chain(fat, start_cluster):
    """Get chain of clusters starting from start_cluster."""
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
    
    if len(data) < cluster_size:
        data = data + b'\x00' * (cluster_size - len(data))
    
    f.seek(offset)
    f.write(data[:cluster_size])


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
    start_cluster = struct.unpack('<H', entry[26:28])[0] | \
                   (struct.unpack('<H', entry[20:22])[0] << 16)
    size = struct.unpack('<I', entry[28:32])[0]
    
    return {
        'name': name,
        'attr': attr,
        'start_cluster': start_cluster,
        'size': size,
        'is_directory': bool(attr & 0x10),
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
    """Find a file by name in a directory."""
    files = list_directory(f, bpb, dir_cluster)
    for entry, cluster, offset in files:
        if entry['name'].upper() == filename.upper():
            return entry, cluster, offset
    return None, None, None


def update_directory_entry_size(f, bpb, dir_cluster, entry_cluster, entry_offset, new_size):
    """Update the file size in a directory entry."""
    cluster_size = bpb['sectors_per_cluster'] * bpb['bytes_per_sector']
    
    # Calculate which cluster contains the entry
    fat, _ = read_fat(f, bpb)
    clusters = get_cluster_chain(fat, dir_cluster)
    
    for cluster in clusters:
        cluster_start_offset = (cluster - 2) * cluster_size
        cluster_end_offset = cluster_start_offset + cluster_size
        
        if cluster_start_offset <= entry_offset < cluster_end_offset:
            offset_in_cluster = entry_offset - cluster_start_offset
            data = bytearray(read_cluster(f, cluster, bpb))
            
            # Update size (bytes 28-31)
            data[offset_in_cluster + 28:offset_in_cluster + 32] = struct.pack('<I', new_size)
            
            write_cluster(f, cluster, bpb, bytes(data))
            return True
    
    return False


def allocate_cluster(fat_entries, fat_data):
    """Find and allocate a free cluster."""
    for i in range(2, len(fat_entries)):
        if fat_entries[i] == 0:
            fat_entries[i] = 0x0FFFFFFF
            struct.pack_into('<I', fat_data, i * 4, 0x0FFFFFFF)
            return i
    raise ValueError("No free clusters available")


def free_cluster_chain(fat_entries, fat_data, start_cluster):
    """Free all clusters in a chain."""
    current = start_cluster
    while current < len(fat_entries):
        next_cluster = fat_entries[current]
        fat_entries[current] = 0
        struct.pack_into('<I', fat_data, current * 4, 0)
        if next_cluster >= 0x0FFFFFF8 or next_cluster == 0:
            break
        current = next_cluster


def update_file_in_image(f, bpb, file_entry, entry_cluster, entry_offset, new_data):
    """Update a file with new content."""
    fat_entries, fat_data = read_fat(f, bpb)
    old_clusters = get_cluster_chain(fat_entries, file_entry['start_cluster'])
    
    cluster_size = bpb['sectors_per_cluster'] * bpb['bytes_per_sector']
    new_size = len(new_data)
    
    num_clusters_needed = (new_size + cluster_size - 1) // cluster_size
    num_clusters_available = len(old_clusters)
    
    print(f"  Old size: {file_entry['size']} bytes ({num_clusters_available} clusters)")
    print(f"  New size: {new_size} bytes ({num_clusters_needed} clusters)")
    
    # Allocate or free clusters
    if num_clusters_needed > num_clusters_available:
        clusters_to_allocate = num_clusters_needed - num_clusters_available
        print(f"  Allocating {clusters_to_allocate} new cluster(s)...")
        
        if old_clusters:
            prev_cluster = old_clusters[-1]
        else:
            raise ValueError("File has no clusters allocated")
        
        for _ in range(clusters_to_allocate):
            new_cluster = allocate_cluster(fat_entries, fat_data)
            fat_entries[prev_cluster] = new_cluster
            struct.pack_into('<I', fat_data, prev_cluster * 4, new_cluster)
            old_clusters.append(new_cluster)
            prev_cluster = new_cluster
            
    elif num_clusters_needed < num_clusters_available:
        clusters_to_free = old_clusters[num_clusters_needed:]
        print(f"  Freeing {len(clusters_to_free)} excess cluster(s)...")
        
        if num_clusters_needed > 0:
            last_used = old_clusters[num_clusters_needed - 1]
            fat_entries[last_used] = 0x0FFFFFFF
            struct.pack_into('<I', fat_data, last_used * 4, 0x0FFFFFFF)
        
        for cluster in clusters_to_free:
            fat_entries[cluster] = 0
            struct.pack_into('<I', fat_data, cluster * 4, 0)
        
        old_clusters = old_clusters[:num_clusters_needed]
    
    write_fat(f, bpb, fat_data)
    
    # Write data to clusters
    for i, cluster in enumerate(old_clusters):
        offset = cluster_to_offset(cluster, bpb)
        start = i * cluster_size
        end = min(start + cluster_size, len(new_data))
        chunk = new_data[start:end]
        
        if len(chunk) < cluster_size:
            chunk = chunk + b'\x00' * (cluster_size - len(chunk))
        
        f.seek(offset)
        f.write(chunk)
    
    # Update directory entry
    update_directory_entry_size(f, bpb, entry_cluster, entry_cluster, entry_offset, new_size)
    
    print(f"  Updated successfully!")


def command_kernel(image_path, kernel_path):
    """Update the kernel in the image."""
    print(f"Updating kernel in {image_path}...")
    
    with open(image_path, 'r+b') as f:
        partitions = parse_mbr(f)
        
        if not partitions:
            raise ValueError("No partitions found in MBR")
        
        # Find FAT32 boot partition
        boot_partition = None
        for p in partitions:
            if p['type'] in (0x0B, 0x0C):  # FAT32
                boot_partition = p
                break
        
        if not boot_partition:
            raise ValueError("No FAT32 boot partition found")
        
        print(f"  Found boot partition at sector {boot_partition['start_lba']}")
        
        bpb = parse_fat32_bpb(f, boot_partition['start_lba'])
        
        # Find kernel8.img
        file_entry, entry_cluster, entry_offset = find_file_in_dir(
            f, bpb, bpb['root_cluster'], 'KERNEL8  IMG'
        )
        
        if file_entry is None:
            raise ValueError("kernel8.img not found in boot partition. "
                           "Run create-sdcard.py first.")
        
        print(f"  Found kernel8.img (current size: {file_entry['size']} bytes)")
        
        # Read new kernel
        with open(kernel_path, 'rb') as kf:
            kernel_data = kf.read()
        
        # Calculate entry offset within directory
        cluster_size = bpb['sectors_per_cluster'] * bpb['bytes_per_sector']
        entry_byte_offset = (entry_cluster - 2) * cluster_size + entry_offset
        
        update_file_in_image(f, bpb, file_entry, entry_cluster, entry_byte_offset, kernel_data)


def command_config(image_path, config_path):
    """Update config.txt in the image."""
    print(f"Updating config.txt in {image_path}...")
    
    with open(image_path, 'r+b') as f:
        partitions = parse_mbr(f)
        
        boot_partition = None
        for p in partitions:
            if p['type'] in (0x0B, 0x0C):
                boot_partition = p
                break
        
        if not boot_partition:
            raise ValueError("No FAT32 boot partition found")
        
        bpb = parse_fat32_bpb(f, boot_partition['start_lba'])
        
        file_entry, entry_cluster, entry_offset = find_file_in_dir(
            f, bpb, bpb['root_cluster'], 'CONFIG   TXT'
        )
        
        if file_entry is None:
            raise ValueError("config.txt not found in boot partition")
        
        with open(config_path, 'rb') as cf:
            config_data = cf.read()
        
        cluster_size = bpb['sectors_per_cluster'] * bpb['bytes_per_sector']
        entry_byte_offset = (entry_cluster - 2) * cluster_size + entry_offset
        
        update_file_in_image(f, bpb, file_entry, entry_cluster, entry_byte_offset, config_data)


def command_cmdline(image_path, cmdline_path):
    """Update cmdline.txt in the image."""
    print(f"Updating cmdline.txt in {image_path}...")
    
    with open(image_path, 'r+b') as f:
        partitions = parse_mbr(f)
        
        boot_partition = None
        for p in partitions:
            if p['type'] in (0x0B, 0x0C):
                boot_partition = p
                break
        
        if not boot_partition:
            raise ValueError("No FAT32 boot partition found")
        
        bpb = parse_fat32_bpb(f, boot_partition['start_lba'])
        
        file_entry, entry_cluster, entry_offset = find_file_in_dir(
            f, bpb, bpb['root_cluster'], 'CMDLINE  TXT'
        )
        
        if file_entry is None:
            raise ValueError("cmdline.txt not found in boot partition")
        
        with open(cmdline_path, 'rb') as cf:
            cmdline_data = cf.read()
        
        cluster_size = bpb['sectors_per_cluster'] * bpb['bytes_per_sector']
        entry_byte_offset = (entry_cluster - 2) * cluster_size + entry_offset
        
        update_file_in_image(f, bpb, file_entry, entry_cluster, entry_byte_offset, cmdline_data)


def command_list(image_path):
    """List files in the boot partition."""
    print(f"Listing files in {image_path}...\n")
    
    with open(image_path, 'rb') as f:
        partitions = parse_mbr(f)
        
        print("Partitions:")
        for p in partitions:
            boot_marker = " (bootable)" if p['status'] == 0x80 else ""
            size_mb = p['num_sectors'] * SECTOR_SIZE // (1024 * 1024)
            print(f"  Partition {p['number']}: {p['type_name']}{boot_marker}")
            print(f"    Start: sector {p['start_lba']}, Size: {size_mb} MB")
        
        print()
        
        # Find FAT32 boot partition
        boot_partition = None
        for p in partitions:
            if p['type'] in (0x0B, 0x0C):
                boot_partition = p
                break
        
        if not boot_partition:
            print("No FAT32 boot partition found")
            return
        
        bpb = parse_fat32_bpb(f, boot_partition['start_lba'])
        
        print(f"Boot partition files (FAT32, root cluster {bpb['root_cluster']}):")
        print(f"{'Filename':<20} {'Size':>10} {'Type':<10}")
        print("-" * 45)
        
        files = list_directory(f, bpb, bpb['root_cluster'])
        for entry, cluster, offset in files:
            ftype = 'Directory' if entry['is_directory'] else 'File'
            # Format filename nicely
            name = entry['name']
            if ' ' in name and not entry['is_directory']:
                # Try to format as name.ext
                if len(name) > 8:
                    name = name[:8].rstrip() + '.' + name[8:].strip()
            print(f"{name:<20} {entry['size']:>10} {ftype:<10}")


def main():
    parser = argparse.ArgumentParser(
        description='Update files in a Raspberry Pi SD card image',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Update kernel
  python update-sdcard.py webbos-pi.img kernel target/aarch64-unknown-none/release/kernel
  
  # Update config.txt
  python update-sdcard.py webbos-pi.img config my-config.txt
  
  # List all files
  python update-sdcard.py webbos-pi.img ls
        """
    )
    
    parser.add_argument('image', help='Path to SD card image')
    parser.add_argument('command', choices=['kernel', 'config', 'cmdline', 'ls'],
                        help='Command to execute')
    parser.add_argument('args', nargs='*', help='Command arguments')
    
    args = parser.parse_args()
    
    if not os.path.exists(args.image):
        print(f"Error: Image not found: {args.image}", file=sys.stderr)
        sys.exit(1)
    
    try:
        if args.command == 'kernel':
            if len(args.args) < 1:
                parser.error("kernel command requires a kernel path")
            command_kernel(args.image, args.args[0])
        
        elif args.command == 'config':
            if len(args.args) < 1:
                parser.error("config command requires a config file path")
            command_config(args.image, args.args[0])
        
        elif args.command == 'cmdline':
            if len(args.args) < 1:
                parser.error("cmdline command requires a cmdline file path")
            command_cmdline(args.image, args.args[0])
        
        elif args.command == 'ls':
            command_list(args.image)
    
    except ValueError as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)
    except Exception as e:
        print(f"Unexpected error: {e}", file=sys.stderr)
        import traceback
        traceback.print_exc()
        sys.exit(1)


if __name__ == '__main__':
    main()
