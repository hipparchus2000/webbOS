#!/usr/bin/env python3
"""
Create a new FAT32 disk image for WebbOS from scratch.

This script creates a bootable FAT32 disk image with:
- EFI/BOOT/BOOTX64.EFI (bootloader)
- kernel.elf (kernel)

Usage:
    python create-image.py [--size SIZE_MB] [--output IMAGE_PATH] [BOOTLOADER] [KERNEL]
    
Examples:
    # Create with default settings (64MB image)
    python create-image.py
    
    # Create with custom size and files
    python create-image.py --size 128 --output webbos.img \\
        target/x86_64-unknown-uefi/debug/bootloader.efi \\
        target/x86_64-unknown-none/debug/kernel
    
    # Just create an empty image
    python create-image.py --empty
"""

import sys
import struct
import os
import argparse


def create_fat32_image(output_path, size_mb=64, label="WEBBOS"):
    """
    Create a blank FAT32 disk image.
    
    Args:
        output_path: Path for the output image file
        size_mb: Size of the image in megabytes
        label: Volume label (up to 11 characters)
    """
    # Calculate sizes
    size_bytes = size_mb * 1024 * 1024
    bytes_per_sector = 512
    sectors_per_cluster = 1
    sectors_per_fat = 0  # Will calculate
    num_fats = 2
    reserved_sectors = 32
    root_cluster = 2
    
    total_sectors = size_bytes // bytes_per_sector
    
    # Calculate FAT size
    # Each FAT entry is 4 bytes
    # We need entries for all data clusters
    # Data clusters = total - reserved - (num_fats * sectors_per_fat)
    # This is circular, so we iterate to find the right value
    
    data_sectors = total_sectors - reserved_sectors
    # Approximate: each cluster needs 4 bytes in FAT
    clusters_approx = data_sectors // sectors_per_cluster
    fat_sectors_approx = (clusters_approx * 4 + bytes_per_sector - 1) // bytes_per_sector
    
    # Recalculate with FAT overhead
    data_sectors = total_sectors - reserved_sectors - (num_fats * fat_sectors_approx)
    clusters = data_sectors // sectors_per_cluster
    sectors_per_fat = ((clusters + 2) * 4 + bytes_per_sector - 1) // bytes_per_sector
    
    print(f"Creating {size_mb}MB FAT32 image...")
    print(f"  Total sectors: {total_sectors}")
    print(f"  Sectors per FAT: {sectors_per_fat}")
    print(f"  Data clusters: {clusters}")
    
    # Create blank image
    with open(output_path, 'wb') as f:
        f.write(b'\x00' * size_bytes)
    
    # Write boot sector
    with open(output_path, 'r+b') as f:
        boot_sector = bytearray(512)
        
        # Jump instruction
        boot_sector[0:3] = b'\xEB\x58\x90'
        
        # OEM name
        boot_sector[3:11] = b'MSDOS5.0'
        
        # BPB
        boot_sector[11:13] = struct.pack('<H', bytes_per_sector)  # Bytes per sector
        boot_sector[13] = sectors_per_cluster  # Sectors per cluster
        boot_sector[14:16] = struct.pack('<H', reserved_sectors)  # Reserved sectors
        boot_sector[16] = num_fats  # Number of FATs
        boot_sector[17:19] = struct.pack('<H', 0)  # Root entries (0 for FAT32)
        boot_sector[19:21] = struct.pack('<H', 0)  # Total sectors (0 for FAT32)
        boot_sector[21] = 0xF8  # Media descriptor (fixed disk)
        boot_sector[22:24] = struct.pack('<H', 0)  # Sectors per FAT (0 for FAT32)
        boot_sector[24:26] = struct.pack('<H', 0x3F)  # Sectors per track
        boot_sector[26:28] = struct.pack('<H', 0xFF)  # Number of heads
        boot_sector[28:32] = struct.pack('<I', 0)  # Hidden sectors
        boot_sector[32:36] = struct.pack('<I', total_sectors)  # Total sectors (FAT32)
        
        # FAT32 specific
        boot_sector[36:40] = struct.pack('<I', sectors_per_fat)  # Sectors per FAT
        boot_sector[40:42] = struct.pack('<H', 0)  # Flags
        boot_sector[42:44] = struct.pack('<H', 0)  # FAT version
        boot_sector[44:48] = struct.pack('<I', root_cluster)  # Root cluster
        boot_sector[48:50] = struct.pack('<H', 1)  # FSInfo sector
        boot_sector[50:52] = struct.pack('<H', 6)  # Backup boot sector
        boot_sector[52:64] = b'\x00' * 12  # Reserved
        boot_sector[64] = 0x80  # Drive number
        boot_sector[65] = 0  # Reserved
        boot_sector[66] = 0x29  # Boot signature
        boot_sector[67:71] = struct.pack('<I', 0x12345678)  # Volume serial
        boot_sector[71:82] = label.ljust(11).encode('ascii')[:11]  # Volume label
        boot_sector[82:90] = b'FAT32   '  # File system type
        
        # Boot code (minimal - just halt)
        boot_sector[90:510] = b'\xF4' * 420  # HLT instructions
        
        # Boot signature
        boot_sector[510:512] = b'\x55\xAA'
        
        f.write(boot_sector)
        
        # Write FSInfo sector (sector 1)
        fsinfo = bytearray(512)
        fsinfo[0:4] = b'RRaA'  # Signature
        fsinfo[484:488] = b'rrAa'  # Signature
        fsinfo[488:492] = struct.pack('<I', 0xFFFFFFFF)  # Free cluster count (unknown)
        fsinfo[492:496] = struct.pack('<I', 0xFFFFFFFF)  # Next free cluster (unknown)
        fsinfo[508:512] = b'\x00\x00\x55\xAA'  # Signature
        f.write(fsinfo)
        
        # Write FATs
        fat_data = bytearray(sectors_per_fat * bytes_per_sector)
        # FAT32 media type in first entry
        fat_data[0:4] = struct.pack('<I', 0x0FFFFFF8)  # Media type marker
        fat_data[4:8] = struct.pack('<I', 0x0FFFFFFF)  # Reserved
        fat_data[8:12] = struct.pack('<I', 0x0FFFFFFF)  # Root directory (cluster 2 = end of chain)
        
        fat_offset = reserved_sectors * bytes_per_sector
        f.seek(fat_offset)
        f.write(fat_data)
        f.write(fat_data)  # Second FAT copy
        
        print(f"  Blank FAT32 image created at: {output_path}")
        
        return {
            'bytes_per_sector': bytes_per_sector,
            'sectors_per_cluster': sectors_per_cluster,
            'reserved_sectors': reserved_sectors,
            'num_fats': num_fats,
            'sectors_per_fat': sectors_per_fat,
            'root_cluster': root_cluster,
        }


def cluster_to_offset(cluster, bpb):
    """Convert cluster number to byte offset."""
    first_data_sector = bpb['reserved_sectors'] + (bpb['num_fats'] * bpb['sectors_per_fat'])
    sector = first_data_sector + ((cluster - 2) * bpb['sectors_per_cluster'])
    return sector * bpb['bytes_per_sector']


def read_fat(f, bpb):
    """Read FAT entries and data."""
    fat_size = bpb['sectors_per_fat'] * bpb['bytes_per_sector']
    f.seek(bpb['reserved_sectors'] * bpb['bytes_per_sector'])
    fat_data = bytearray(f.read(fat_size))
    
    entries = []
    for i in range(0, len(fat_data), 4):
        entry = struct.unpack('<I', fat_data[i:i+4])[0] & 0x0FFFFFFF
        entries.append(entry)
    return entries, fat_data


def write_fat(f, bpb, fat_data):
    """Write FAT to all copies."""
    for fat_num in range(bpb['num_fats']):
        offset = (bpb['reserved_sectors'] + fat_num * bpb['sectors_per_fat']) * bpb['bytes_per_sector']
        f.seek(offset)
        f.write(fat_data)


def allocate_clusters(f, bpb, num_clusters):
    """Allocate clusters in the FAT."""
    fat_entries, fat_data = read_fat(f, bpb)
    
    allocated = []
    for _ in range(num_clusters):
        # Find free cluster
        for i in range(2, len(fat_entries)):
            if fat_entries[i] == 0:
                allocated.append(i)
                fat_entries[i] = 0x0FFFFFFF  # End of chain temporarily
                struct.pack_into('<I', fat_data, i * 4, 0x0FFFFFFF)
                break
        else:
            raise ValueError(f"Not enough free clusters (needed {num_clusters}, found {len(allocated)})")
    
    # Link clusters
    for i in range(len(allocated) - 1):
        fat_entries[allocated[i]] = allocated[i + 1]
        struct.pack_into('<I', fat_data, allocated[i] * 4, allocated[i + 1])
    
    # Last cluster is end of chain
    if allocated:
        fat_entries[allocated[-1]] = 0x0FFFFFFF
        struct.pack_into('<I', fat_data, allocated[-1] * 4, 0x0FFFFFFF)
    
    write_fat(f, bpb, fat_data)
    return allocated


def write_cluster(f, cluster, bpb, data):
    """Write data to a cluster."""
    offset = cluster_to_offset(cluster, bpb)
    cluster_size = bpb['sectors_per_cluster'] * bpb['bytes_per_sector']
    
    if len(data) < cluster_size:
        data = data + b'\x00' * (cluster_size - len(data))
    
    f.seek(offset)
    f.write(data[:cluster_size])


def create_directory_entry(name, ext, attr, start_cluster, size):
    """Create a 32-byte directory entry."""
    entry = bytearray(32)
    
    # Name (8.3 format)
    entry[0:8] = name.upper().ljust(8).encode('latin-1')[:8]
    entry[8:11] = ext.upper().ljust(3).encode('latin-1')[:3]
    
    # Attributes
    entry[11] = attr
    
    # Creation time/date (set to 0)
    entry[12:22] = b'\x00' * 10
    
    # Access date
    entry[18:20] = struct.pack('<H', 0)
    
    # High cluster (bits 16-31)
    entry[20:22] = struct.pack('<H', (start_cluster >> 16) & 0xFFFF)
    
    # Modification time/date
    entry[22:26] = struct.pack('<I', 0)
    
    # Low cluster (bits 0-15)
    entry[26:28] = struct.pack('<H', start_cluster & 0xFFFF)
    
    # File size
    entry[28:32] = struct.pack('<I', size)
    
    return bytes(entry)


def add_file_to_image(f, bpb, parent_cluster, filename, data, is_directory=False):
    """Add a file or directory to the image."""
    cluster_size = bpb['sectors_per_cluster'] * bpb['bytes_per_sector']
    
    # Parse filename
    if '.' in filename and not is_directory:
        name, ext = filename.rsplit('.', 1)
        name = name[:8]
        ext = ext[:3]
    else:
        name = filename[:8]
        ext = ''
    
    attr = 0x10 if is_directory else 0x20  # Directory or Archive
    
    if is_directory:
        # Allocate one cluster for empty directory
        clusters = allocate_clusters(f, bpb, 1)
        size = 0
        # Initialize directory cluster to zeros
        write_cluster(f, clusters[0], bpb, b'\x00' * cluster_size)
    else:
        # Calculate clusters needed
        num_clusters = (len(data) + cluster_size - 1) // cluster_size
        if num_clusters == 0:
            num_clusters = 1  # Always allocate at least one cluster
        
        clusters = allocate_clusters(f, bpb, num_clusters)
        size = len(data)
        
        # Write file data
        for i, cluster in enumerate(clusters):
            start = i * cluster_size
            end = min(start + cluster_size, len(data))
            chunk = data[start:end]
            write_cluster(f, cluster, bpb, chunk)
    
    # Add directory entry to parent
    entry = create_directory_entry(name, ext, attr, clusters[0] if clusters else 0, size)
    
    # Find free slot in parent directory
    fat_entries, _ = read_fat(f, bpb)
    parent_clusters = []
    current = parent_cluster
    while current < len(fat_entries):
        parent_clusters.append(current)
        next_cluster = fat_entries[current]
        if next_cluster >= 0x0FFFFFF8 or next_cluster == 0:
            break
        current = next_cluster
    
    # Search for free entry
    entry_written = False
    for cluster in parent_clusters:
        offset = cluster_to_offset(cluster, bpb)
        f.seek(offset)
        cluster_data = bytearray(f.read(cluster_size))
        
        for i in range(0, len(cluster_data), 32):
            if cluster_data[i] == 0x00 or cluster_data[i] == 0xE5:
                # Free slot found
                cluster_data[i:i+32] = entry
                write_cluster(f, cluster, bpb, bytes(cluster_data))
                entry_written = True
                break
        
        if entry_written:
            break
    
    if not entry_written:
        raise ValueError("No free directory entries in parent")
    
    return clusters[0] if clusters else 0


def create_webbos_image(output_path, bootloader_path=None, kernel_path=None, size_mb=64):
    """
    Create a complete WebbOS disk image.
    
    Args:
        output_path: Path for the output image
        bootloader_path: Path to bootloader.efi (optional)
        kernel_path: Path to kernel binary (optional)
        size_mb: Size of image in MB
    """
    # Create blank image
    bpb = create_fat32_image(output_path, size_mb, label="WEBBOS")
    
    with open(output_path, 'r+b') as f:
        # Create directory structure
        print("Creating directory structure...")
        
        # Create EFI directory
        efi_cluster = add_file_to_image(f, bpb, bpb['root_cluster'], 'EFI', b'', is_directory=True)
        print(f"  Created EFI/ (cluster {efi_cluster})")
        
        # Create EFI/BOOT directory
        boot_cluster = add_file_to_image(f, bpb, efi_cluster, 'BOOT', b'', is_directory=True)
        print(f"  Created EFI/BOOT/ (cluster {boot_cluster})")
        
        # Add bootloader
        if bootloader_path and os.path.exists(bootloader_path):
            with open(bootloader_path, 'rb') as src:
                bootloader_data = src.read()
            add_file_to_image(f, bpb, boot_cluster, 'BOOTX64.EFI', bootloader_data)
            print(f"  Added EFI/BOOT/BOOTX64.EFI ({len(bootloader_data)} bytes)")
        else:
            print(f"  Warning: Bootloader not found at {bootloader_path}")
            # Create placeholder
            add_file_to_image(f, bpb, boot_cluster, 'BOOTX64.EFI', b'PLACEHOLDER')
            print(f"  Created placeholder EFI/BOOT/BOOTX64.EFI")
        
        # Add kernel
        if kernel_path and os.path.exists(kernel_path):
            with open(kernel_path, 'rb') as src:
                kernel_data = src.read()
            add_file_to_image(f, bpb, bpb['root_cluster'], 'KERNEL.ELF', kernel_data)
            print(f"  Added KERNEL.ELF ({len(kernel_data)} bytes)")
        else:
            print(f"  Warning: Kernel not found at {kernel_path}")
            # Create placeholder
            add_file_to_image(f, bpb, bpb['root_cluster'], 'KERNEL.ELF', b'PLACEHOLDER')
            print(f"  Created placeholder KERNEL.ELF")
    
    print(f"\nDisk image created successfully: {output_path}")
    print(f"Image size: {size_mb}MB")
    return output_path


def main():
    parser = argparse.ArgumentParser(
        description='Create a WebbOS FAT32 disk image',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Create with default settings (64MB, default paths)
  python create-image.py
  
  # Create with custom size
  python create-image.py --size 128
  
  # Create with specific files
  python create-image.py bootloader.efi kernel.bin
  
  # Create empty image (no files)
  python create-image.py --empty --size 32
        """
    )
    
    parser.add_argument('bootloader', nargs='?',
                        default='target/x86_64-unknown-uefi/debug/bootloader.efi',
                        help='Path to bootloader.efi (default: target/x86_64-unknown-uefi/debug/bootloader.efi)')
    parser.add_argument('kernel', nargs='?',
                        default='target/x86_64-unknown-none/debug/kernel',
                        help='Path to kernel binary (default: target/x86_64-unknown-none/debug/kernel)')
    parser.add_argument('-o', '--output', default='webbos.img',
                        help='Output image path (default: webbos.img)')
    parser.add_argument('-s', '--size', type=int, default=64,
                        help='Image size in MB (default: 64)')
    parser.add_argument('--empty', action='store_true',
                        help='Create empty image without files')
    
    args = parser.parse_args()
    
    if args.empty:
        bootloader = None
        kernel = None
    else:
        bootloader = args.bootloader
        kernel = args.kernel
    
    try:
        create_webbos_image(args.output, bootloader, kernel, args.size)
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
