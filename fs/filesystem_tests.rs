//! Filesystem Integration Tests
//!
//! Tests for FAT32 filesystem operations on virtual block devices.
//! These tests can run on any platform (no hardware required).

use webbos_pi5_vfat32::fs::{
    block::{BlockDevice, VirtualBlockDevice, StatsBlockDevice},
    cache::{BlockCache, CachePolicy},
    fat32::{Fat32Filesystem, FileInfo},
    format_fat32, mount_fat32, create_virtual_disk,
    partition::{PartitionTable, Partition, PartitionType},
    vfs::{Vfs, VfsOperations, OpenFlags, SeekFrom},
};

/// Test helper: Create a formatted FAT32 filesystem
fn create_test_fs() -> Fat32Filesystem<VirtualBlockDevice> {
    let mut disk = create_virtual_disk(10000);
    format_fat32(&mut disk).expect("Failed to format disk");
    mount_fat32(disk).expect("Failed to mount filesystem")
}

/// Test helper: Create a disk with MBR partition table
fn create_partitioned_disk() -> VirtualBlockDevice {
    let mut disk = create_virtual_disk(10000);
    format_fat32(&mut disk).expect("Failed to format disk");
    
    // Add MBR partition table
    let mut data = disk.data_mut();
    
    // MBR signature
    data[510] = 0x55;
    data[511] = 0xAA;
    
    // Create FAT32 partition entry at offset 446
    let entry_offset = 446;
    data[entry_offset] = 0x80; // Bootable
    data[entry_offset + 4] = 0x0C; // FAT32 LBA type
    
    // Start LBA (sector 2048)
    data[entry_offset + 8..12].copy_from_slice(&2048u32.to_le_bytes());
    
    // Sector count
    data[entry_offset + 12..16].copy_from_slice(&7800u32.to_le_bytes());
    
    disk
}

#[test]
fn test_format_and_mount() {
    let fs = create_test_fs();
    let info = fs.info();
    
    assert!(info.total_clusters > 0);
    assert_eq!(info.bytes_per_sector, 512);
    assert!(info.bytes_per_cluster >= 512);
    println!("Filesystem info: {:?}", info);
}

#[test]
fn test_list_root_directory() {
    let mut fs = create_test_fs();
    let entries = fs.list_root().expect("Failed to list root directory");
    
    // Root should have volume label entry
    assert!(!entries.is_empty());
    println!("Root directory entries: {:?}", entries);
}

#[test]
fn test_create_file() {
    let mut fs = create_test_fs();
    let root_cluster = fs.info().root_cluster;
    
    // Create a file
    let file_info = fs.create_file(root_cluster, "test.txt")
        .expect("Failed to create file");
    
    assert_eq!(file_info.name, "test.txt");
    assert_eq!(file_info.size, 0);
    assert!(file_info.is_file);
    
    // Verify file appears in directory listing
    let entries = fs.list_root().expect("Failed to list root");
    let found = entries.iter().find(|e| e.name == "test.txt");
    assert!(found.is_some(), "Created file not found in directory listing");
}

#[test]
fn test_write_and_read_file() {
    let mut fs = create_test_fs();
    let root_cluster = fs.info().root_cluster;
    
    // Create a file
    let file_info = fs.create_file(root_cluster, "test.txt")
        .expect("Failed to create file");
    
    // Write data to file
    let test_data = b"Hello, FAT32 World! This is a test file.";
    fs.write_file(root_cluster, "test.txt", test_data)
        .expect("Failed to write file");
    
    // Read back the data
    let read_data = fs.read_file(&file_info)
        .expect("Failed to read file");
    
    assert_eq!(read_data, test_data.to_vec());
}

#[test]
fn test_large_file_write() {
    let mut fs = create_test_fs();
    let root_cluster = fs.info().root_cluster;
    
    // Create a large file (multiple clusters)
    let file_info = fs.create_file(root_cluster, "large.bin")
        .expect("Failed to create file");
    
    // Write 10KB of data
    let test_data: Vec<u8> = (0..10240).map(|i| (i % 256) as u8).collect();
    fs.write_file(root_cluster, "large.bin", &test_data)
        .expect("Failed to write large file");
    
    // Read back and verify
    let read_data = fs.read_file(&file_info)
        .expect("Failed to read large file");
    
    assert_eq!(read_data.len(), test_data.len());
    assert_eq!(read_data, test_data);
}

#[test]
fn test_create_directory() {
    let mut fs = create_test_fs();
    let root_cluster = fs.info().root_cluster;
    
    // Create a directory
    let dir_info = fs.create_directory(root_cluster, "testdir")
        .expect("Failed to create directory");
    
    assert_eq!(dir_info.name, "testdir");
    assert!(dir_info.is_directory);
    
    // Verify directory appears in listing
    let entries = fs.list_root().expect("Failed to list root");
    let found = entries.iter().find(|e| e.name == "testdir" && e.is_directory);
    assert!(found.is_some(), "Created directory not found in listing");
}

#[test]
fn test_delete_file() {
    let mut fs = create_test_fs();
    let root_cluster = fs.info().root_cluster;
    
    // Create and delete a file
    fs.create_file(root_cluster, "delete_me.txt")
        .expect("Failed to create file");
    
    // Verify file exists
    let entries_before = fs.list_root().expect("Failed to list root");
    assert!(entries_before.iter().any(|e| e.name == "delete_me.txt"));
    
    // Delete the file
    fs.delete(root_cluster, "delete_me.txt")
        .expect("Failed to delete file");
    
    // Verify file is gone
    let entries_after = fs.list_root().expect("Failed to list root");
    assert!(!entries_after.iter().any(|e| e.name == "delete_me.txt"));
}

#[test]
fn test_long_filename() {
    let mut fs = create_test_fs();
    let root_cluster = fs.info().root_cluster;
    
    // Create file with long name
    let long_name = "this_is_a_very_long_filename_for_testing.txt";
    let file_info = fs.create_file(root_cluster, long_name)
        .expect("Failed to create file with long name");
    
    assert_eq!(file_info.name, long_name);
    
    // Verify short name was generated
    assert!(!file_info.short_name.is_empty());
    println!("Long name: {}, Short name: {}", long_name, file_info.short_name);
}

#[test]
fn test_partition_table_detection() {
    let mut disk = create_partitioned_disk();
    
    // Read partition table
    let table = PartitionTable::read(&mut disk)
        .expect("Failed to read partition table");
    
    assert_eq!(table.table_type, partition::PartitionTableType::Mbr);
    assert!(!table.partitions.is_empty());
    
    let partition = &table.partitions[0];
    assert!(partition.bootable);
    assert!(partition.is_fat32());
    
    println!("Partition table: {:?}", table);
}

#[test]
fn test_vfs_operations() {
    let disk = create_virtual_disk(10000);
    let mut disk_for_format = disk;
    format_fat32(&mut disk_for_format).expect("Failed to format");
    
    let fs = mount_fat32(disk_for_format).expect("Failed to mount");
    let mut vfs = Vfs::new(fs);
    
    // Test create and write
    let fd = vfs.open("/test.txt", OpenFlags::from_mode("w+").unwrap())
        .expect("Failed to open file");
    
    let test_data = b"VFS test data";
    let written = vfs.write(fd, test_data).expect("Failed to write");
    assert_eq!(written, test_data.len());
    
    // Test seek and read
    vfs.seek(fd, SeekFrom::Start(0)).expect("Failed to seek");
    
    let mut read_buf = vec![0u8; test_data.len()];
    let read = vfs.read(fd, &mut read_buf).expect("Failed to read");
    assert_eq!(read, test_data.len());
    assert_eq!(&read_buf, test_data);
    
    // Test close
    vfs.close(fd).expect("Failed to close file");
    
    // Test directory operations
    vfs.mkdir("/testdir").expect("Failed to create directory");
    
    let entries = vfs.readdir("/").expect("Failed to list directory");
    assert!(entries.iter().any(|e| e.name == "test.txt"));
    assert!(entries.iter().any(|e| e.name == "testdir"));
}

#[test]
fn test_cache_performance() {
    let disk = create_virtual_disk(10000);
    let mut disk_for_format = disk;
    format_fat32(&mut disk_for_format).expect("Failed to format");
    
    // Wrap with stats tracking
    let mut stats_disk = StatsBlockDevice::new(disk_for_format);
    
    // Create filesystem
    let fs = mount_fat32(stats_disk).expect("Failed to mount");
    
    // Perform operations and check stats
    println!("Stats after mount: {:?}", fs.stats());
}

#[test]
fn test_file_truncation() {
    let mut fs = create_test_fs();
    let root_cluster = fs.info().root_cluster;
    
    // Create file with data
    fs.create_file(root_cluster, "truncate.txt")
        .expect("Failed to create file");
    
    let original_data = b"This is the original content of the file.";
    fs.write_file(root_cluster, "truncate.txt", original_data)
        .expect("Failed to write file");
    
    // Truncate to smaller size
    let truncated_data = b"This is";
    fs.write_file(root_cluster, "truncate.txt", truncated_data)
        .expect("Failed to truncate file");
    
    // Read and verify
    let entries = fs.list_root().expect("Failed to list root");
    let file_info = entries.iter().find(|e| e.name == "truncate.txt")
        .expect("File not found");
    
    assert_eq!(file_info.size as usize, truncated_data.len());
    
    let read_data = fs.read_file(file_info).expect("Failed to read file");
    assert_eq!(&read_data, truncated_data);
}

#[test]
fn test_multiple_files() {
    let mut fs = create_test_fs();
    let root_cluster = fs.info().root_cluster;
    
    // Create multiple files
    for i in 0..10 {
        let name = format!("file{}.txt", i);
        let data = format!("Content of file {}", i);
        
        fs.create_file(root_cluster, &name).expect("Failed to create file");
        fs.write_file(root_cluster, &name, data.as_bytes()).expect("Failed to write file");
    }
    
    // Verify all files exist
    let entries = fs.list_root().expect("Failed to list root");
    
    for i in 0..10 {
        let name = format!("file{}.txt", i);
        let found = entries.iter().find(|e| e.name == name);
        assert!(found.is_some(), "File {} not found", name);
    }
}

#[test]
fn test_nested_directories() {
    let mut fs = create_test_fs();
    let root_cluster = fs.info().root_cluster;
    
    // Create nested directory structure
    let dir1_info = fs.create_directory(root_cluster, "level1")
        .expect("Failed to create level1");
    
    let dir2_info = fs.create_directory(dir1_info.cluster, "level2")
        .expect("Failed to create level2");
    
    let _dir3_info = fs.create_directory(dir2_info.cluster, "level3")
        .expect("Failed to create level3");
    
    // Create file in nested directory
    fs.create_file(dir2_info.cluster, "nested.txt")
        .expect("Failed to create nested file");
    
    // List directories
    let level1_entries = fs.list_directory(dir1_info.cluster)
        .expect("Failed to list level1");
    
    let found_level2 = level1_entries.iter().find(|e| e.name == "level2");
    assert!(found_level2.is_some());
}

#[test]
fn test_empty_file_operations() {
    let mut fs = create_test_fs();
    let root_cluster = fs.info().root_cluster;
    
    // Create empty file
    fs.create_file(root_cluster, "empty.txt")
        .expect("Failed to create empty file");
    
    // Read empty file
    let entries = fs.list_root().expect("Failed to list root");
    let file_info = entries.iter().find(|e| e.name == "empty.txt")
        .expect("File not found");
    
    assert_eq!(file_info.size, 0);
    
    let data = fs.read_file(file_info).expect("Failed to read empty file");
    assert!(data.is_empty());
}

#[test]
fn test_cache_flush() {
    let disk = create_virtual_disk(10000);
    let mut disk_for_format = disk;
    format_fat32(&mut disk_for_format).expect("Failed to format");
    
    let fs = mount_fat32(disk_for_format).expect("Failed to mount");
    
    // Perform operations
    // Note: Actual flush testing would require internal cache inspection
    fs.flush().expect("Failed to flush");
}

#[test]
fn test_error_handling() {
    let mut fs = create_test_fs();
    let root_cluster = fs.info().root_cluster;
    
    // Try to read non-existent file
    let result = fs.read_file(&FileInfo {
        name: "nonexistent.txt".to_string(),
        short_name: "NONEXISTTXT".to_string(),
        size: 0,
        attributes: 0,
        cluster: 0,
        is_directory: false,
        is_file: true,
    });
    
    assert!(result.is_err());
    
    // Try to create file in non-existent directory
    // (Would need invalid cluster number - skipped for safety)
}

#[test]
fn test_special_characters_in_filename() {
    let mut fs = create_test_fs();
    let root_cluster = fs.info().root_cluster;
    
    // Create files with special characters (short names only)
    let names = vec!["test_file.txt", "test-file.txt", "test$file.txt"];
    
    for name in &names {
        let result = fs.create_file(root_cluster, name);
        assert!(result.is_ok(), "Failed to create file: {}", name);
    }
    
    // Verify files exist
    let entries = fs.list_root().expect("Failed to list root");
    for name in &names {
        let found = entries.iter().any(|e| &e.name == *name);
        assert!(found, "File not found: {}", name);
    }
}

/// Benchmark test for sequential reads
#[test]
fn benchmark_sequential_read() {
    use std::time::Instant;
    
    let mut fs = create_test_fs();
    let root_cluster = fs.info().root_cluster;
    
    // Create a 1MB file
    let data_size = 1024 * 1024;
    let test_data: Vec<u8> = vec![0xAB; data_size];
    
    fs.create_file(root_cluster, "benchmark.bin")
        .expect("Failed to create file");
    fs.write_file(root_cluster, "benchmark.bin", &test_data)
        .expect("Failed to write file");
    
    // Benchmark sequential read
    let entries = fs.list_root().expect("Failed to list root");
    let file_info = entries.iter().find(|e| e.name == "benchmark.bin")
        .expect("File not found");
    
    let start = Instant::now();
    let _read_data = fs.read_file(file_info).expect("Failed to read file");
    let duration = start.elapsed();
    
    let mb_per_sec = data_size as f64 / (1024.0 * 1024.0) / duration.as_secs_f64();
    println!("Sequential read: {} MB/s", mb_per_sec);
}

/// Benchmark test for random reads
#[test]
fn benchmark_random_access() {
    use std::time::Instant;
    
    let mut fs = create_test_fs();
    let root_cluster = fs.info().root_cluster;
    
    // Create multiple files
    for i in 0..100 {
        let name = format!("random{}.txt", i);
        let data = format!("Random file content number {}", i);
        fs.create_file(root_cluster, &name).expect("Failed to create");
        fs.write_file(root_cluster, &name, data.as_bytes()).expect("Failed to write");
    }
    
    // Random access pattern
    let entries = fs.list_root().expect("Failed to list root");
    
    let start = Instant::now();
    for i in (0..100).step_by(2) {
        let name = format!("random{}.txt", i);
        if let Some(file_info) = entries.iter().find(|e| e.name == name) {
            let _ = fs.read_file(file_info);
        }
    }
    let duration = start.elapsed();
    
    println!("Random access of 50 files: {:?}", duration);
}

// Import required modules for tests
use webbos_pi5_vfat32::fs::partition;
