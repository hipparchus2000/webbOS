# webbOS Filesystem Documentation

## Overview

The webbOS filesystem layer provides complete FAT32 filesystem support for Raspberry Pi 5, including SD card block device drivers, partition table support, and a Virtual File System (VFS) layer.

## Architecture

```
┌─────────────────────────────────────┐
│  VFS (Virtual File System)          │
│  - File descriptor management       │
│  - Path resolution                  │
│  - POSIX-like API                   │
├─────────────────────────────────────┤
│  FAT32 Filesystem                   │
│  - File/directory operations        │
│  - Cluster allocation               │
│  - Long filename (LFN) support      │
├─────────────────────────────────────┤
│  Partition Layer                    │
│  - MBR (Master Boot Record)         │
│  - GPT (GUID Partition Table)       │
├─────────────────────────────────────┤
│  Cache Layer                        │
│  - Block cache (LRU/LFU)            │
│  - FAT table caching                │
│  - Directory entry cache            │
│  - Read-ahead                       │
├─────────────────────────────────────┤
│  Block Device                       │
│  - SD Card (SDHCI/SDIO)             │
│  - Virtual (testing)                │
└─────────────────────────────────────┘
```

## SD Card Block Device Driver

### SDHCI Interface (Raspberry Pi 5)

The Raspberry Pi 5 uses the RP1 I/O controller with SDHCI (SD Host Controller Interface):

```rust
use webbos_pi5_vfat32::fs::block::{SdCardBlockDevice, BlockDevice};

// Initialize SD card (unsafe: requires valid SDHCI base address)
let mut sd = unsafe { SdCardBlockDevice::new(0x1F000000) }?;

// Read sector
let mut buffer = vec![0u8; 512];
sd.read_block(0, &mut buffer)?;

// Write sector
let data = vec![0xABu8; 512];
sd.write_block(0, &data)?;
```

### Features

- **Block Operations**: 512-byte sector read/write
- **Error Handling**: Automatic retry with configurable attempts
- **High Capacity Support**: SDHC/SDXC cards (up to 2TB)
- **Performance**: Up to 25MHz SD clock (high speed mode)

## Partition Table Support

### MBR (Master Boot Record)

```rust
use webbos_pi5_vfat32::fs::partition::PartitionTable;
use webbos_pi5_vfat32::fs::block::VirtualBlockDevice;

let mut device = VirtualBlockDevice::new(10000);
let table = PartitionTable::read(&mut device)?;

// Find boot partition
if let Some(boot_part) = table.find_boot_partition() {
    println!("Boot partition: {:?}", boot_part);
}

// Find FAT32 partition
if let Some(fat32_part) = table.find_fat32_partition() {
    println!("FAT32 at LBA {}, size {} sectors", 
        fat32_part.start_lba, fat32_part.sector_count);
}
```

### GPT (GUID Partition Table)

GPT support is automatically detected and parsed alongside MBR.

## FAT32 Filesystem

### Mounting

```rust
use webbos_pi5_vfat32::fs::{mount_fat32, format_fat32};

// Format and mount
let mut disk = VirtualBlockDevice::new(10000);
format_fat32(&mut disk)?;
let mut fs = mount_fat32(disk)?;

// Get filesystem info
let info = fs.info();
println!("Total clusters: {}", info.total_clusters);
println!("Bytes per cluster: {}", info.bytes_per_cluster);
```

### File Operations

```rust
use webbos_pi5_vfat32::fs::fat32::FileInfo;

let root_cluster = fs.info().root_cluster;

// Create file
let file = fs.create_file(root_cluster, "hello.txt")?;

// Write to file
let data = b"Hello, World!";
fs.write_file(root_cluster, "hello.txt", data)?;

// Read from file
let contents = fs.read_file(&file)?;

// Delete file
fs.delete(root_cluster, "hello.txt")?;
```

### Directory Operations

```rust
// Create directory
let dir = fs.create_directory(root_cluster, "mydir")?;

// List directory contents
let entries = fs.list_directory(dir.cluster)?;
for entry in entries {
    println!("{} - {} bytes", entry.name, entry.size);
}

// Create file in subdirectory
fs.create_file(dir.cluster, "nested.txt")?;
```

### Long Filename Support

FAT32 LFN entries are automatically created for long filenames:

```rust
// Creates LFN entries automatically
let file = fs.create_file(root_cluster, "very_long_filename_for_testing.txt")?;
// Generates short name: "VERY_L~1.TXT"
```

## Virtual File System (VFS)

The VFS provides a POSIX-like interface for filesystem operations:

```rust
use webbos_pi5_vfat32::fs::vfs::{Vfs, VfsOperations, OpenFlags, SeekFrom};

let fs = mount_fat32(disk)?;
let mut vfs = Vfs::new(fs);

// Open file
let fd = vfs.open("/test.txt", OpenFlags::from_mode("w+")?)?;

// Write data
vfs.write(fd, b"Hello, VFS!")?;

// Seek to beginning
vfs.seek(fd, SeekFrom::Start(0))?;

// Read data
let mut buffer = vec![0u8; 100];
let bytes_read = vfs.read(fd, &mut buffer)?;

// Close file
vfs.close(fd)?;
```

### Directory Operations via VFS

```rust
// Create directory
vfs.mkdir("/mydir")?;

// List directory
let entries = vfs.readdir("/")?;
for entry in entries {
    println!("{}", entry.name);
}

// Remove file
vfs.remove("/old_file.txt")?;
```

## Caching Layer

### Block Cache

```rust
use webbos_pi5_vfat32::fs::cache::{BlockCache, CachePolicy};

// Create cache with 64-block capacity
let mut cache = BlockCache::new(512, 64);

// Set replacement policy
cache.set_policy(CachePolicy::Lru);  // or CachePolicy::Lfu

// Cached read
let data = cache.read_sector(&mut device, block_num)?;

// Cached write
cache.write_sector(&mut device, block_num, data)?;

// Flush dirty blocks
cache.flush(&mut device)?;
```

### Cache Statistics

```rust
let stats = cache.stats();
println!("Capacity: {}", stats.capacity);
println!("Dirty entries: {}", stats.dirty_entries);
println!("Clean entries: {}", stats.clean_entries);
```

### FAT Table Cache

Specialized cache for FAT table entries:

```rust
use webbos_pi5_vfat32::fs::cache::FatCache;

let mut fat_cache = FatCache::new(512, 16); // 16 FAT sectors cached

// Read cluster chain entry
let next_cluster = fat_cache.read_entry(&mut device, fat_start, cluster)?;

// Write cluster chain entry
fat_cache.write_entry(&mut device, fat_start, cluster, next_cluster)?;
```

## System Call Interface

The VFS provides a system call compatible interface:

```rust
use webbos_pi5_vfat32::fs::vfs::FileSystemSyscall as syscall;

// Open file
let fd = syscall::sys_open(&mut vfs, "/file.txt", "r+")?;

// Read
let mut buf = [0u8; 1024];
let n = syscall::sys_read(&mut vfs, fd, &mut buf)?;

// Write
let n = syscall::sys_write(&mut vfs, fd, b"data")?;

// Seek
let pos = syscall::sys_lseek(&mut vfs, fd, 0, 0)?; // SEEK_SET

// Close
syscall::sys_close(&mut vfs, fd)?;
```

## Testing

### Virtual Block Device

For testing without hardware:

```rust
use webbos_pi5_vfat32::fs::block::VirtualBlockDevice;

// Create virtual 10000-sector disk
let disk = VirtualBlockDevice::new(10000);

// Create from existing data
let disk = VirtualBlockDevice::from_data(vec![0u8; 5_242_880]);
```

### Running Tests

```bash
# Run filesystem tests
cargo test --test filesystem_tests

# Run with output
cargo test --test filesystem_tests -- --nocapture
```

## Performance Optimization

### Read-Ahead

```rust
use webbos_pi5_vfat32::fs::cache::ReadAheadCache;

let mut cache = ReadAheadCache::new(512, 64, 4); // 4-block read-ahead window

// Sequential reads trigger read-ahead
let data = cache.read(&mut device, block_num)?;
```

### Write-Behind

Writes are cached and flushed:
- On cache full (LRU eviction)
- On explicit flush
- On filesystem unmount
- When dirty threshold exceeded (default 25%)

## Error Handling

All filesystem operations return `Result<T, VFatError>`:

```rust
use webbos_pi5_vfat32::error::VFatError;

match fs.create_file(root_cluster, "test.txt") {
    Ok(file) => println!("Created: {:?}", file),
    Err(VFatError::Io(e)) => println!("IO error: {}", e),
    Err(VFatError::Corruption(msg)) => println!("Corruption: {}", msg),
    Err(VFatError::InvalidParameter(msg)) => println!("Invalid: {}", msg),
    _ => println!("Other error"),
}
```

## Implementation Notes

### ARM64 Optimizations

- Uses 64-bit block addresses for large storage support
- Aligned memory operations for cache efficiency
- NEON SIMD ready for future optimization

### Memory Safety

- No unsafe code in high-level API
- Bounds checking on all buffer operations
- Automatic resource cleanup via Drop trait

### Limitations

- Maximum file size: 4GB (FAT32 limitation)
- Maximum partition size: 2TB
- No journaling (FAT32 limitation)
- Short names limited to 8.3 format

## API Reference

See source code documentation:
- `src/fs/block/` - Block device layer
- `src/fs/partition/` - Partition table support
- `src/fs/fat32/` - FAT32 filesystem
- `src/fs/cache/` - Caching layer
- `src/fs/vfs/` - Virtual file system
