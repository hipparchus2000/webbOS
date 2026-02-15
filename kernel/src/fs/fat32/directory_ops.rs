//! Directory operations for FAT32 filesystem
//!
//! Provides functions for creating, deleting, and modifying directory entries.

use alloc::string::String;
use alloc::vec::Vec;
use alloc::vec;

use crate::fs::{FsResult, FsError, FileType};
use crate::storage::BlockDevice;

use super::fat_table::FatTableManager;
use super::{DirEntry, LfnEntry, BootSector};

/// File attributes
const ATTR_READ_ONLY: u8 = 0x01;
const ATTR_HIDDEN: u8 = 0x02;
const ATTR_SYSTEM: u8 = 0x04;
const ATTR_VOLUME_ID: u8 = 0x08;
const ATTR_DIRECTORY: u8 = 0x10;
const ATTR_ARCHIVE: u8 = 0x20;
const ATTR_LFN: u8 = 0x0F;

/// Directory operations manager
pub struct DirectoryManager<'a> {
    device: &'a dyn BlockDevice,
    fat_manager: &'a mut FatTableManager<'a>,
    bytes_per_sector: u16,
    sectors_per_cluster: u8,
    bytes_per_cluster: u32,
    data_start_sector: u32,
}

impl<'a> DirectoryManager<'a> {
    /// Create a new directory manager
    pub fn new(
        device: &'a dyn BlockDevice,
        fat_manager: &'a mut FatTableManager<'a>,
        bytes_per_sector: u16,
        sectors_per_cluster: u8,
        data_start_sector: u32,
    ) -> Self {
        let bytes_per_cluster = (bytes_per_sector as u32) * (sectors_per_cluster as u32);
        
        Self {
            device,
            fat_manager,
            bytes_per_sector,
            sectors_per_cluster,
            bytes_per_cluster,
            data_start_sector,
        }
    }
    
    /// Cluster to sector
    fn cluster_to_sector(&self, cluster: u32) -> u64 {
        let cluster_offset = cluster.saturating_sub(2);
        (self.data_start_sector as u64) + 
        (cluster_offset as u64 * self.sectors_per_cluster as u64)
    }
    
    /// Read cluster
    fn read_cluster(&self, cluster: u32, buf: &mut [u8]) -> FsResult<()> {
        let sector = self.cluster_to_sector(cluster);
        self.device.read_blocks(sector, self.sectors_per_cluster as usize, buf)
            .map_err(|_| FsError::IoError)
    }
    
    /// Write cluster
    fn write_cluster(&self, cluster: u32, buf: &[u8]) -> FsResult<()> {
        let sector = self.cluster_to_sector(cluster);
        self.device.write_blocks(sector, self.sectors_per_cluster as usize, buf)
            .map_err(|_| FsError::IoError)
    }
    
    /// Find free directory entry
    pub fn find_free_entry(&self, dir_cluster: u32) -> FsResult<(u32, usize)> {
        let mut current_cluster = dir_cluster;
        let mut cluster_offset = 0;
        
        loop {
            let mut cluster_data = vec![0u8; self.bytes_per_cluster as usize];
            self.read_cluster(current_cluster, &mut cluster_data)?;
            
            let entry_count = cluster_data.len() / 32;
            for i in 0..entry_count {
                let entry_offset = i * 32;
                let first_byte = cluster_data[entry_offset];
                
                // Free entry (0x00 or 0xE5)
                if first_byte == 0x00 || first_byte == 0xE5 {
                    return Ok((current_cluster, i));
                }
            }
            
            // Check if we need to allocate more clusters
            if let Some(next_cluster) = self.fat_manager.next_cluster(current_cluster) {
                current_cluster = next_cluster;
                cluster_offset += 1;
            } else {
                // End of chain, need to extend
                break;
            }
        }
        
        // Need to extend directory
        let new_cluster = self.fat_manager.extend_cluster_chain(current_cluster, 1)?;
        
        // Initialize new cluster with zeros
        let mut new_cluster_data = vec![0u8; self.bytes_per_cluster as usize];
        new_cluster_data[0] = 0x00; // Mark as end of directory
        self.write_cluster(new_cluster, &new_cluster_data)?;
        
        Ok((new_cluster, 0))
    }
    
    /// Create a directory entry
    pub fn create_entry(
        &mut self,
        dir_cluster: u32,
        name: &str,
        file_type: FileType,
        start_cluster: u32,
        size: u32,
    ) -> FsResult<()> {
        // Find free entry
        let (cluster, entry_idx) = self.find_free_entry(dir_cluster)?;
        
        // Read cluster
        let mut cluster_data = vec![0u8; self.bytes_per_cluster as usize];
        self.read_cluster(cluster, &mut cluster_data)?;
        
        let entry_offset = entry_idx * 32;
        
        // Create directory entry
        let mut entry = DirEntry {
            name: [b' '; 11],
            attrs: 0,
            reserved: 0,
            create_time_tenths: 0,
            create_time: 0,
            create_date: 0,
            access_date: 0,
            cluster_high: ((start_cluster >> 16) & 0xFFFF) as u16,
            modify_time: 0,
            modify_date: 0,
            cluster_low: (start_cluster & 0xFFFF) as u16,
            size,
        };
        
        // Set attributes
        match file_type {
            FileType::Directory => entry.attrs = ATTR_DIRECTORY,
            FileType::Regular => entry.attrs = ATTR_ARCHIVE,
            _ => return Err(FsError::NotImplemented),
        }
        
        // Set name (8.3 format for now)
        let name_bytes = name.as_bytes();
        let mut name_idx = 0;
        
        // Handle 8.3 filename
        if name.len() <= 12 {
            let parts: Vec<&str> = name.split('.').collect();
            let base_name = parts[0];
            let ext = if parts.len() > 1 { parts[1] } else { "" };
            
            // Base name (up to 8 chars)
            for (i, &c) in base_name.as_bytes().iter().enumerate().take(8) {
                entry.name[i] = c.to_ascii_uppercase();
            }
            
            // Extension (up to 3 chars)
            if !ext.is_empty() {
                for (i, &c) in ext.as_bytes().iter().enumerate().take(3) {
                    entry.name[8 + i] = c.to_ascii_uppercase();
                }
            }
        } else {
            // For longer names, we'd need LFN support
            // For now, use a truncated name
            for (i, &c) in name_bytes.iter().enumerate().take(11) {
                entry.name[i] = c.to_ascii_uppercase();
            }
        }
        
        // Write entry to cluster data
        let entry_bytes = unsafe {
            core::slice::from_raw_parts(
                &entry as *const DirEntry as *const u8,
                core::mem::size_of::<DirEntry>()
            )
        };
        
        cluster_data[entry_offset..entry_offset + 32].copy_from_slice(entry_bytes);
        
        // Write cluster back
        self.write_cluster(cluster, &cluster_data)?;
        
        Ok(())
    }
    
    /// Delete a directory entry
    pub fn delete_entry(&mut self, dir_cluster: u32, name: &str) -> FsResult<()> {
        let entries = self.read_entries(dir_cluster)?;
        
        for (entry_cluster, entry_idx, entry) in entries {
            let entry_name = self.entry_to_name(&entry);
            
            if entry_name.to_ascii_lowercase() == name.to_ascii_lowercase() {
                // Mark entry as deleted
                let mut cluster_data = vec![0u8; self.bytes_per_cluster as usize];
                self.read_cluster(entry_cluster, &mut cluster_data)?;
                
                cluster_data[entry_idx * 32] = 0xE5; // Mark as deleted
                
                self.write_cluster(entry_cluster, &cluster_data)?;
                
                // If this is a directory or file, deallocate its clusters
                if entry.attrs & ATTR_DIRECTORY != 0 || entry.attrs & ATTR_ARCHIVE != 0 {
                    let start_cluster = ((entry.cluster_high as u32) << 16) | (entry.cluster_low as u32);
                    self.fat_manager.deallocate_cluster_chain(start_cluster)?;
                }
                
                return Ok(());
            }
        }
        
        Err(FsError::NotFound)
    }
    
    /// Read all directory entries with their locations
    fn read_entries(&self, dir_cluster: u32) -> FsResult<Vec<(u32, usize, DirEntry)>> {
        let mut entries = Vec::new();
        let mut current_cluster = dir_cluster;
        let mut cluster_idx = 0;
        
        loop {
            let mut cluster_data = vec![0u8; self.bytes_per_cluster as usize];
            self.read_cluster(current_cluster, &mut cluster_data)?;
            
            let entry_count = cluster_data.len() / 32;
            for i in 0..entry_count {
                let entry_offset = i * 32;
                let first_byte = cluster_data[entry_offset];
                
                // End of directory
                if first_byte == 0x00 {
                    return Ok(entries);
                }
                
                // Skip deleted entries
                if first_byte == 0xE5 {
                    continue;
                }
                
                let attrs = cluster_data[entry_offset + 11];
                
                // Skip LFN entries for now
                if attrs == ATTR_LFN {
                    continue;
                }
                
                // Skip volume label
                if attrs & ATTR_VOLUME_ID != 0 {
                    continue;
                }
                
                let entry = unsafe {
                    *(cluster_data.as_ptr().add(entry_offset) as *const DirEntry)
                };
                
                // Skip "." and ".." entries
                let name = self.entry_to_name(&entry);
                if name != "." && name != ".." {
                    entries.push((current_cluster, i, entry));
                }
            }
            
            // Move to next cluster
            if let Some(next_cluster) = self.fat_manager.next_cluster(current_cluster) {
                current_cluster = next_cluster;
                cluster_idx += 1;
            } else {
                break;
            }
        }
        
        Ok(entries)
    }
    
    /// Convert directory entry to name
    fn entry_to_name(&self, entry: &DirEntry) -> String {
        let mut name = String::new();
        
        // Name (first 8 bytes, trim spaces)
        for j in 0..8 {
            if entry.name[j] != b' ' {
                let c = if entry.name[j] >= b'A' && entry.name[j] <= b'Z' {
                    entry.name[j] + 32 // Convert to lowercase
                } else {
                    entry.name[j]
                };
                name.push(c as char);
            }
        }
        
        // Extension
        let has_ext = entry.name[8..11].iter().any(|&b| b != b' ');
        if has_ext {
            name.push('.');
            for j in 8..11 {
                if entry.name[j] != b' ' {
                    let c = if entry.name[j] >= b'A' && entry.name[j] <= b'Z' {
                        entry.name[j] + 32
                    } else {
                        entry.name[j]
                    };
                    name.push(c as char);
                }
            }
        }
        
        name
    }
    
    /// Initialize a directory cluster
    pub fn init_directory_cluster(&mut self, cluster: u32, parent_cluster: u32) -> FsResult<()> {
        let mut cluster_data = vec![0u8; self.bytes_per_cluster as usize];
        
        // Create "." entry
        let dot_entry = DirEntry {
            name: [b'.', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' '],
            attrs: ATTR_DIRECTORY,
            reserved: 0,
            create_time_tenths: 0,
            create_time: 0,
            create_date: 0,
            access_date: 0,
            cluster_high: ((cluster >> 16) & 0xFFFF) as u16,
            modify_time: 0,
            modify_date: 0,
            cluster_low: (cluster & 0xFFFF) as u16,
            size: 0,
        };
        
        // Create ".." entry
        let dotdot_entry = DirEntry {
            name: [b'.', b'.', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' '],
            attrs: ATTR_DIRECTORY,
            reserved: 0,
            create_time_tenths: 0,
            create_time: 0,
            create_date: 0,
            access_date: 0,
            cluster_high: ((parent_cluster >> 16) & 0xFFFF) as u16,
            modify_time: 0,
            modify_date: 0,
            cluster_low: (parent_cluster & 0xFFFF) as u16,
            size: 0,
        };
        
        // Write entries
        let dot_bytes = unsafe {
            core::slice::from_raw_parts(
                &dot_entry as *const DirEntry as *const u8,
                core::mem::size_of::<DirEntry>()
            )
        };
        
        let dotdot_bytes = unsafe {
            core::slice::from_raw_parts(
                &dotdot_entry as *const DirEntry as *const u8,
                core::mem::size_of::<DirEntry>()
            )
        };
        
        cluster_data[0..32].copy_from_slice(dot_bytes);
        cluster_data[32..64].copy_from_slice(dotdot_bytes);
        
        // Mark end of directory
        cluster_data[64] = 0x00;
        
        self.write_cluster(cluster, &cluster_data)?;
        
        Ok(())
    }
    
    /// Update file size in directory entry
    pub fn update_file_size(
        &mut self,
        dir_cluster: u32,
        name: &str,
        new_size: u32,
    ) -> FsResult<()> {
        let entries = self.read_entries(dir_cluster)?;
        
        for (entry_cluster, entry_idx, mut entry) in entries {
            let entry_name = self.entry_to_name(&entry);
            
            if entry_name.to_ascii_lowercase() == name.to_ascii_lowercase() {
                // Update size
                entry.size = new_size;
                
                // Write updated entry
                let mut cluster_data = vec![0u8; self.bytes_per_cluster as usize];
                self.read_cluster(entry_cluster, &mut cluster_data)?;
                
                let entry_bytes = unsafe {
                    core::slice::from_raw_parts(
                        &entry as *const DirEntry as *const u8,
                        core::mem::size_of::<DirEntry>()
                    )
                };
                
                cluster_data[entry_idx * 32..entry_idx * 32 + 32].copy_from_slice(entry_bytes);
                
                self.write_cluster(entry_cluster, &cluster_data)?;
                
                return Ok(());
            }
        }
        
        Err(FsError::NotFound)
    }
}