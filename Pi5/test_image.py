#!/usr/bin/env python3
"""Test script to check SD card image contents."""
import sys
import importlib.util

# Load the create-sdcard module (has hyphen in name)
spec = importlib.util.spec_from_file_location("create_sdcard", "scripts/create-sdcard.py")
create_sdcard = importlib.util.module_from_spec(spec)
sys.modules["create_sdcard"] = create_sdcard
spec.loader.exec_module(create_sdcard)

parse_fat32_bpb = create_sdcard.parse_fat32_bpb
list_directory = create_sdcard.list_directory
find_file_in_dir = create_sdcard.find_file_in_dir
read_partition_table = create_sdcard.read_partition_table

with open('webbos-pi.img', 'rb') as f:
    partitions = read_partition_table(f)
    boot_partition = partitions[0]
    print(f'Boot partition starts at: {boot_partition["start_lba"]}')
    
    bpb = parse_fat32_bpb(f, boot_partition['start_lba'])
    print(f'Root cluster: {bpb["root_cluster"]}')
    
    files = list_directory(f, bpb, bpb['root_cluster'])
    for entry, cluster, offset in files:
        print(f'File: "{entry["name"]}" size: {entry["size"]}')
    
    # Try to find kernel8.img
    entry, cluster, offset = find_file_in_dir(f, bpb, bpb['root_cluster'], 'KERNEL8  IMG')
    if entry:
        print(f'Found KERNEL8.IMG: {entry}')
    else:
        print('KERNEL8.IMG not found!')
        print('Trying case variations...')
        for entry, cluster, offset in files:
            if 'KERNEL' in entry['name'].upper():
                print(f'  Found similar: "{entry["name"]}"')
