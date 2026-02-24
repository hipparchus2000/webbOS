#!/usr/bin/env python3
"""
Add all apps and games to Pi SD card image in one go.
"""

import sys
import struct
import os

SECTOR_SIZE = 512
FIRST_PARTITION_START = 2048


def parse_fat32_bpb_partitioned(f, partition_start=FIRST_PARTITION_START):
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


def cluster_to_offset_partitioned(cluster, bpb):
    partition_start = bpb['partition_start']
    first_data_sector = partition_start + bpb['reserved_sectors'] + (bpb['num_fats'] * bpb['sectors_per_fat'])
    sector = first_data_sector + ((cluster - 2) * bpb['sectors_per_cluster'])
    return sector * bpb['bytes_per_sector']


def read_cluster_partitioned(f, cluster, bpb):
    offset = cluster_to_offset_partitioned(cluster, bpb)
    size = bpb['sectors_per_cluster'] * bpb['bytes_per_sector']
    f.seek(offset)
    return f.read(size)


def write_cluster_partitioned(f, cluster, bpb, data):
    offset = cluster_to_offset_partitioned(cluster, bpb)
    cluster_size = bpb['sectors_per_cluster'] * bpb['bytes_per_sector']
    if len(data) < cluster_size:
        data = data + b'\x00' * (cluster_size - len(data))
    f.seek(offset)
    f.write(data[:cluster_size])


def read_fat_partitioned(f, bpb):
    partition_start = bpb['partition_start']
    fat_size = bpb['sectors_per_fat'] * bpb['bytes_per_sector']
    f.seek(partition_start * SECTOR_SIZE + bpb['reserved_sectors'] * bpb['bytes_per_sector'])
    fat_data = bytearray(f.read(fat_size))
    entries = []
    for i in range(0, len(fat_data), 4):
        entry = struct.unpack('<I', fat_data[i:i+4])[0] & 0x0FFFFFFF
        entries.append(entry)
    return entries, fat_data


def write_fat_partitioned(f, bpb, fat_data):
    partition_start = bpb['partition_start']
    fat_size = bpb['sectors_per_fat'] * bpb['bytes_per_sector']
    for fat_num in range(bpb['num_fats']):
        offset = (partition_start + bpb['reserved_sectors'] + fat_num * bpb['sectors_per_fat']) * bpb['bytes_per_sector']
        f.seek(offset)
        f.write(fat_data)


def allocate_cluster(fat_entries, fat_data):
    for i in range(2, len(fat_entries)):
        if fat_entries[i] == 0:
            new_value = 0x0FFFFFFF
            fat_entries[i] = new_value
            struct.pack_into('<I', fat_data, i * 4, new_value)
            return i
    raise ValueError("No free clusters available")


def get_cluster_chain(fat, start_cluster):
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
    
    return {'name': name, 'attr': attr, 'start_cluster': start_cluster, 'size': size, 'offset': offset}, 'file'


def create_dir_entry(name, ext, attr, start_cluster, size):
    entry = bytearray(32)
    entry[0:8] = name.upper().ljust(8).encode('latin-1')[:8]
    entry[8:11] = ext.upper().ljust(3).encode('latin-1')[:3]
    entry[11] = attr
    entry[26:28] = struct.pack('<H', start_cluster & 0xFFFF)
    entry[20:22] = struct.pack('<H', (start_cluster >> 16) & 0xFFFF)
    entry[28:32] = struct.pack('<I', size)
    return bytes(entry)


def list_directory_partitioned(f, bpb, start_cluster):
    fat_entries, _ = read_fat_partitioned(f, bpb)
    clusters = get_cluster_chain(fat_entries, start_cluster)
    
    entries = []
    for cluster in clusters:
        data = read_cluster_partitioned(f, cluster, bpb)
        for i in range(0, len(data), 32):
            entry, entry_type = parse_directory_entry(data, i)
            if entry_type == 'end':
                return entries, cluster, i
            if entry and entry_type == 'file':
                entries.append(entry)
    return entries, clusters[-1], 0


def add_directory_entry_partitioned(f, bpb, dir_cluster, name, ext, attr, start_cluster, size):
    fat_entries, fat_data = read_fat_partitioned(f, bpb)
    clusters = get_cluster_chain(fat_entries, dir_cluster)
    
    for cluster in clusters:
        data = bytearray(read_cluster_partitioned(f, cluster, bpb))
        for i in range(0, len(data), 32):
            if data[i] == 0x00 or data[i] == 0xE5:
                entry = create_dir_entry(name, ext, attr, start_cluster, size)
                data[i:i+32] = entry
                write_cluster_partitioned(f, cluster, bpb, bytes(data))
                return True
    
    new_cluster = allocate_cluster(fat_entries, fat_data)
    last_cluster = clusters[-1]
    struct.pack_into('<I', fat_data, last_cluster * 4, new_cluster)
    fat_entries[last_cluster] = new_cluster
    write_fat_partitioned(f, bpb, fat_data)
    
    data = bytearray(bpb['sectors_per_cluster'] * bpb['bytes_per_sector'])
    entry = create_dir_entry(name, ext, attr, start_cluster, size)
    data[0:32] = entry
    write_cluster_partitioned(f, new_cluster, bpb, bytes(data))
    return True


def find_directory_partitioned(f, bpb, parent_cluster, dirname):
    entries, _, _ = list_directory_partitioned(f, bpb, parent_cluster)
    dirname_83 = dirname.upper().ljust(8)[:8]  # 8.3 format
    
    for entry in entries:
        entry_name_83 = entry['name'][:8].rstrip()
        if entry_name_83.upper() == dirname_83 and entry['attr'] & 0x10:
            return entry['start_cluster']
    return None


def create_directory_partitioned(f, bpb, parent_cluster, dirname):
    fat_entries, fat_data = read_fat_partitioned(f, bpb)
    
    new_cluster = allocate_cluster(fat_entries, fat_data)
    write_fat_partitioned(f, bpb, fat_data)
    
    data = bytearray(bpb['sectors_per_cluster'] * bpb['bytes_per_sector'])
    write_cluster_partitioned(f, new_cluster, bpb, bytes(data))
    
    add_directory_entry_partitioned(f, bpb, parent_cluster, dirname, '', 0x10, new_cluster, 0)
    
    return new_cluster


def add_file_to_pi_image(f, bpb, parent_cluster, filename, source_file):
    """Add a file to a specific directory."""
    # Determine name and ext
    if '.' in filename:
        name, ext = filename.rsplit('.', 1)
        name = name[:8]
        ext = ext[:3]
    else:
        name = filename[:8]
        ext = ''
    
    with open(source_file, 'rb') as src:
        file_data = src.read()
    
    file_size = len(file_data)
    cluster_size = bpb['sectors_per_cluster'] * bpb['bytes_per_sector']
    num_clusters = (file_size + cluster_size - 1) // cluster_size
    if num_clusters == 0:
        num_clusters = 1
    
    fat_entries, fat_data = read_fat_partitioned(f, bpb)
    clusters = []
    
    for i in range(num_clusters):
        cluster = allocate_cluster(fat_entries, fat_data)
        if i > 0:
            prev_cluster = clusters[-1]
            struct.pack_into('<I', fat_data, prev_cluster * 4, cluster)
            fat_entries[prev_cluster] = cluster
        clusters.append(cluster)
    
    write_fat_partitioned(f, bpb, fat_data)
    
    for i, cluster in enumerate(clusters):
        start = i * cluster_size
        end = min(start + cluster_size, file_size)
        chunk = file_data[start:end]
        write_cluster_partitioned(f, cluster, bpb, chunk)
    
    add_directory_entry_partitioned(f, bpb, parent_cluster, name, ext, 0x20, clusters[0], file_size)
    print(f"  Added {filename} ({file_size} bytes)")


def main():
    image_path = 'webbos-pi.img'
    
    # Apps to add
    apps = [
        ('calc.html', 'system/apps/calc.html'),
        ('judge.html', 'system/apps/judge.html'),
        ('richtext.html', 'system/apps/richtext-editor.html'),
        ('sheet.html', 'system/apps/sheet.html'),
    ]
    
    games = [
        ('backgamon.html', 'system/games/backgammon.html'),
        ('invaders.html', 'system/games/invaders.html'),
        ('mahjong.html', 'system/games/mahjong.html'),
        ('solitaire.html', 'system/games/solitaire.html'),
        ('chickens.html', 'system/games/chicken-darts.html'),
        ('decision.html', 'system/games/decision.html'),
        ('platform.html', 'system/games/platform.html'),
        ('swans.html', 'system/games/swans.html'),
    ]
    
    with open(image_path, 'r+b') as f:
        bpb = parse_fat32_bpb_partitioned(f)
        root_cluster = bpb['root_cluster']
        
        # Create APPS directory
        print("Creating APPS directory...")
        apps_cluster = create_directory_partitioned(f, bpb, root_cluster, 'APPS')
        
        # Add apps
        print("Adding apps:")
        for filename, source in apps:
            add_file_to_pi_image(f, bpb, apps_cluster, filename, source)
        
        # Create GAMES subdirectory
        print("Creating APPS/GAMES directory...")
        games_cluster = create_directory_partitioned(f, bpb, apps_cluster, 'GAMES')
        
        # Add games
        print("Adding games:")
        for filename, source in games:
            add_file_to_pi_image(f, bpb, games_cluster, filename, source)
    
    print("Done!")


if __name__ == '__main__':
    main()
