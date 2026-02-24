#!/usr/bin/env python3
"""
Add sample PWAs to the WebbOS disk image in /Apps folder.

Usage:
    python add-apps-to-image.py <image_file> <system_folder>
    
Example:
    python add-apps-to-image.py webbos-pi.img system
"""

import sys
import struct
import os

# Import FAT32 functions from update-image.py
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from update_image import parse_fat32_bpb, cluster_to_offset, read_fat, write_fat, find_free_clusters
from update_image import allocate_clusters, read_directory, write_directory_entry, fat32_name

def create_directory_structure(f, bpb, path_parts, parent_cluster=2):
    """Create a directory path if it doesn't exist."""
    current_cluster = parent_cluster
    
    for part in path_parts:
        # Check if directory already exists
        entries = read_directory(f, bpb, current_cluster)
        found = False
        for entry in entries:
            if entry['name'].strip() == part.upper():
                found = True
                current_cluster = entry['cluster']
                break
        
        if not found:
            # Create new directory
            new_cluster = allocate_clusters(f, bpb, 1)[0]
            
            # Create . and .. entries
            dir_data = bytearray(512)
            # . entry
            dir_data[0:11] = b'.          '
            dir_data[11] = 0x10  # Directory attribute
            dir_data[26:28] = struct.pack('<H', new_cluster & 0xFFFF)
            dir_data[20:22] = struct.pack('<H', new_cluster >> 16)
            # .. entry
            dir_data[32:43] = b'..         '
            dir_data[43] = 0x10
            dir_data[38:40] = struct.pack('<H', current_cluster & 0xFFFF)
            dir_data[32:34] = struct.pack('<H', current_cluster >> 16)
            
            # Write directory cluster
            offset = cluster_to_offset(new_cluster, bpb)
            f.seek(offset)
            f.write(dir_data)
            
            # Add entry to parent
            fat_name = fat32_name(part)
            entry_data = bytearray(32)
            entry_data[0:11] = fat_name.encode('latin-1')
            entry_data[11] = 0x10  # Directory
            entry_data[26:28] = struct.pack('<H', new_cluster & 0xFFFF)
            entry_data[20:22] = struct.pack('<H', new_cluster >> 16)
            
            # Find free entry in parent
            entries = read_directory(f, bpb, current_cluster)
            for i, entry in enumerate(entries):
                if entry['name'][0] == 0x00 or entry['name'][0] == 0xE5:
                    # Free entry
                    offset = cluster_to_offset(current_cluster, bpb) + i * 32
                    f.seek(offset)
                    f.write(entry_data)
                    break
            
            current_cluster = new_cluster
            print(f"  Created directory: {part} (cluster {new_cluster})")
    
    return current_cluster

def add_file_to_image(f, bpb, src_path, dest_path, parent_cluster=2):
    """Add a file to the image."""
    # Read source file
    with open(src_path, 'rb') as src:
        data = src.read()
    
    size = len(data)
    filename = os.path.basename(dest_path)
    
    # Calculate clusters needed
    cluster_size = bpb['sectors_per_cluster'] * bpb['bytes_per_sector']
    clusters_needed = (size + cluster_size - 1) // cluster_size
    if clusters_needed == 0:
        clusters_needed = 1
    
    # Allocate clusters
    clusters = allocate_clusters(f, bpb, clusters_needed)
    
    # Write data to clusters
    for i, cluster in enumerate(clusters):
        offset = cluster_to_offset(cluster, bpb)
        f.seek(offset)
        start = i * cluster_size
        end = min(start + cluster_size, size)
        f.write(data[start:end])
    
    # Create directory entry
    fat_name = fat32_name(filename)
    entry = bytearray(32)
    entry[0:11] = fat_name.encode('latin-1')
    entry[11] = 0x00  # Archive attribute
    entry[26:28] = struct.pack('<H', clusters[0] & 0xFFFF)
    entry[20:22] = struct.pack('<H', clusters[0] >> 16)
    entry[28:32] = struct.pack('<I', size)
    
    # Find free entry in directory
    entries = read_directory(f, bpb, parent_cluster)
    for i, e in enumerate(entries):
        if e['name'][0] == 0x00 or e['name'][0] == 0xE5:
            offset = cluster_to_offset(parent_cluster, bpb) + i * 32
            f.seek(offset)
            f.write(entry)
            break
    
    print(f"  Added: {filename} -> {dest_path} ({size} bytes, {clusters_needed} clusters)")

def main():
    if len(sys.argv) < 3:
        print("Usage: python add-apps-to-image.py <image_file> <system_folder>")
        sys.exit(1)
    
    image_path = sys.argv[1]
    system_folder = sys.argv[2]
    
    if not os.path.exists(image_path):
        print(f"Error: Image file not found: {image_path}")
        sys.exit(1)
    
    apps_folder = os.path.join(system_folder, 'apps')
    games_folder = os.path.join(system_folder, 'games')
    
    print(f"Adding PWAs to {image_path}...")
    
    with open(image_path, 'r+b') as f:
        bpb = parse_fat32_bpb(f)
        
        # Create /Apps directory
        apps_cluster = create_directory_structure(f, bpb, ['APPS'], 2)
        
        # Add app files
        if os.path.exists(apps_folder):
            for filename in os.listdir(apps_folder):
                if filename.endswith('.html'):
                    src_path = os.path.join(apps_folder, filename)
                    add_file_to_image(f, bpb, src_path, f'/Apps/{filename}', apps_cluster)
        
        # Create /Apps/Games directory
        games_cluster = create_directory_structure(f, bpb, ['APPS', 'GAMES'], 2)
        
        # Add game files
        if os.path.exists(games_folder):
            for filename in os.listdir(games_folder):
                if filename.endswith('.html'):
                    src_path = os.path.join(games_folder, filename)
                    add_file_to_image(f, bpb, src_path, f'/Apps/Games/{filename}', games_cluster)
    
    print("Done!")

if __name__ == '__main__':
    main()
