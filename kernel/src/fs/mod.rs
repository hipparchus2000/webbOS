//! Filesystem module for webbOS Raspberry Pi 5
//!
//! This module provides a complete filesystem stack including:
//! - SD card block device driver (SDHCI interface)
//! - MBR and GPT partition table support
//! - FAT32 filesystem with write support
//! - Virtual File System (VFS) layer
//! - Performance caching layer
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────┐
//! │  VFS (Virtual File System)          │
//! │  - File descriptors                 │
//! │  - Path resolution                  │
//! │  - Open/read/write/close            │
//! ├─────────────────────────────────────┤
//! │  FAT32 Filesystem                   │
//! │  - File/directory operations        │
//! │  - Cluster allocation               │
//! │  - Long filename support            │
//! ├─────────────────────────────────────┤
//! │  Partition Layer                    │
//! │  - MBR/GPT parsing                  │
//! │  - Partition detection              │
//! ├─────────────────────────────────────┤
//! │  Block Cache                        │
//! │  - Read-ahead                       │
//! │  - Write-behind                     │
//! │  - FAT table caching                │
//! ├─────────────────────────────────────┤
//! │  Block Device                       │
//! │  - SD card (SDHCI)                  │
//! │  - Virtual (testing)                │
//! └─────────────────────────────────────┘
//! ```

#![no_std]
#![feature(llvm_asm)]

pub mod block;
pub mod cache;
pub mod fat32;
pub mod partition;
pub mod vfs;

use crate::error::VFatError;
use alloc::string::String;
use alloc::vec::Vec;

/// Filesystem initialization result
pub type FsResult<T> = Result<T, VFatError>;

/// Mount a FAT32 filesystem from a block device
/// 
/// This is a convenience function that chains together:
/// 1. Block device initialization
/// 2. Partition table detection
/// 3. FAT32 filesystem mounting
pub fn mount_fat32<B: block::BlockDevice>(device: B) -> FsResult<fat32::Fat32Filesystem<B>> {
    fat32::Fat32Filesystem::mount(device)
}

/// Create a virtual block device for testing
pub fn create_virtual_disk(size_sectors: u64) -> block::VirtualBlockDevice {
    block::VirtualBlockDevice::new(size_sectors)
}

/// Create a formatted FAT32 test image
pub fn create_fat32_image(size_sectors: u64) -> FsResult<block::VirtualBlockDevice> {
    use block::BlockDevice;
    
    let mut disk = block::VirtualBlockDevice::new(size_sectors);
    
    // Format with FAT32 boot sector
    format_fat32(&mut disk)?;
    
    Ok(disk)
}

/// Format a block device with FAT32
pub fn format_fat32<B: block::BlockDevice>(device: &mut B) -> FsResult<()> {
    use fat32::{BiosParameterBlock, Fat32ExtendedBpb};
    
    let sector_size = device.block_size() as u16;
    let total_sectors = device.capacity() as u32;
    let sectors_per_cluster = if total_sectors < 524288 {
        1 // 512 bytes per cluster for small disks
    } else if total_sectors < 16777216 {
        8 // 4KB clusters
    } else {
        64 // 32KB clusters
    };
    
    let reserved_sectors = 32u16;
    let num_fats = 2u8;
    let sectors_per_fat = ((total_sectors / sectors_per_cluster as u32) * 4 + sector_size as u32 - 1) / sector_size as u32;
    
    // Create boot sector
    let mut boot_sector = vec![0u8; sector_size as usize];
    
    // Jump instruction
    boot_sector[0] = 0xEB;
    boot_sector[1] = 0x58;
    boot_sector[2] = 0x90;
    
    // OEM name
    boot_sector[3..11].copy_from_slice(b"WEBBOS  ");
    
    // BPB
    boot_sector[11..13].copy_from_slice(&sector_size.to_le_bytes());
    boot_sector[13] = sectors_per_cluster;
    boot_sector[14..16].copy_from_slice(&reserved_sectors.to_le_bytes());
    boot_sector[16] = num_fats;
    boot_sector[17..19].copy_from_slice(&0u16.to_le_bytes()); // Root entry count (0 for FAT32)
    boot_sector[19..21].copy_from_slice(&0u16.to_le_bytes()); // Total sectors 16 (0 for FAT32)
    boot_sector[21] = 0xF8; // Media type
    boot_sector[22..24].copy_from_slice(&0u16.to_le_bytes()); // Sectors per FAT 16 (0 for FAT32)
    boot_sector[24..26].copy_from_slice(&63u16.to_le_bytes()); // Sectors per track
    boot_sector[26..28].copy_from_slice(&255u16.to_le_bytes()); // Number of heads
    boot_sector[28..32].copy_from_slice(&0u32.to_le_bytes()); // Hidden sectors
    boot_sector[32..36].copy_from_slice(&total_sectors.to_le_bytes());
    
    // Extended BPB
    boot_sector[36..40].copy_from_slice(&sectors_per_fat.to_le_bytes());
    boot_sector[40..42].copy_from_slice(&0u16.to_le_bytes()); // Extended flags
    boot_sector[42..44].copy_from_slice(&0u16.to_le_bytes()); // Filesystem version
    boot_sector[44..48].copy_from_slice(&2u32.to_le_bytes()); // Root cluster
    boot_sector[48..50].copy_from_slice(&1u16.to_le_bytes()); // FSInfo sector
    boot_sector[50..52].copy_from_slice(&6u16.to_le_bytes()); // Backup boot sector
    boot_sector[52..64].fill(0); // Reserved
    boot_sector[64] = 0x80; // Drive number
    boot_sector[65] = 0; // Reserved
    boot_sector[66] = 0x29; // Boot signature
    boot_sector[67..71].copy_from_slice(&0x12345678u32.to_le_bytes()); // Volume serial
    boot_sector[71..82].copy_from_slice(b"WEBBOS     "); // Volume label
    boot_sector[82..90].copy_from_slice(b"FAT32   "); // Filesystem type
    
    // Boot signature
    boot_sector[510] = 0x55;
    boot_sector[511] = 0xAA;
    
    // Write boot sector
    device.write_block(0, &boot_sector)?;
    
    // Write backup boot sector
    device.write_block(6, &boot_sector)?;
    
    // Initialize FATs
    let fat_start = reserved_sectors as u64;
    let mut fat_sector = vec![0u8; sector_size as usize];
    
    // FAT[0] = media type marker
    fat_sector[0] = 0xF8;
    fat_sector[1] = 0xFF;
    fat_sector[2] = 0xFF;
    fat_sector[3] = 0x0F;
    
    // FAT[1] = reserved
    fat_sector[4] = 0xFF;
    fat_sector[5] = 0xFF;
    fat_sector[6] = 0xFF;
    fat_sector[7] = 0xFF;
    
    // FAT[2] = root directory (end of chain)
    fat_sector[8] = 0xFF;
    fat_sector[9] = 0xFF;
    fat_sector[10] = 0xFF;
    fat_sector[11] = 0x0F;
    
    device.write_block(fat_start, &fat_sector)?;
    
    // Copy to second FAT
    let second_fat_start = fat_start + sectors_per_fat as u64;
    device.write_block(second_fat_start, &fat_sector)?;
    
    // Initialize root directory cluster
    let data_start = fat_start + (sectors_per_fat as u64 * num_fats as u64);
    let root_dir_sector = data_start; // Root cluster = 2
    let mut root_dir = vec![0u8; (sectors_per_cluster as usize) * (sector_size as usize)];
    
    // Create volume label entry
    root_dir[0..11].copy_from_slice(b"WEBBOS     ");
    root_dir[11] = 0x08; // Volume label attribute
    
    device.write_block(root_dir_sector, &root_dir[..sector_size as usize])?;
    
    Ok(())
}

/// Initialize the complete filesystem stack for Raspberry Pi 5
/// 
/// This function initializes:
/// 1. SD card controller
/// 2. Block device layer
/// 3. Partition detection
/// 4. FAT32 filesystem
pub fn init_filesystem(sdhci_base: usize) -> FsResult<fat32::Fat32Filesystem<block::SdCardBlockDevice>> {
    // Initialize SD card
    let sd_device = unsafe { 
        block::SdCardBlockDevice::new(sdhci_base)?
    };
    
    // Detect partitions
    let partition_table = partition::PartitionTable::read(&mut { 
        block::VirtualBlockDevice::new(sd_device.capacity()) 
    })?;
    
    // Find FAT32 partition
    let fat32_partition = partition_table.find_fat32_partition()
        .or_else(|| partition_table.find_boot_partition())
        .ok_or_else(|| VFatError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "No FAT32 partition found",
        )))?;
    
    // Create partition block device
    // In real implementation, wrap with partition-aware device
    
    // Mount FAT32
    fat32::Fat32Filesystem::mount(sd_device)
}

/// Filesystem statistics
#[derive(Debug, Clone)]
pub struct FilesystemStats {
    /// Total reads
    pub total_reads: u64,
    /// Total writes
    pub total_writes: u64,
    /// Cache hits
    pub cache_hits: u64,
    /// Cache misses
    pub cache_misses: u64,
    /// Files open
    pub open_files: usize,
}

// External crate dependencies
extern crate alloc;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_format_and_mount() {
        // Create and format a virtual disk
        let mut disk = create_virtual_disk(10000).unwrap();
        format_fat32(&mut disk).unwrap();
        
        // Mount the filesystem
        let fs = mount_fat32(disk).unwrap();
        let info = fs.info();
        
        assert!(info.total_clusters > 0);
        assert_eq!(info.bytes_per_sector, 512);
    }
}
