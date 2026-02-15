//! Write operations for FAT32 filesystem
//!
//! Provides functions for writing to files and handling write errors.

use alloc::vec;
use alloc::vec::Vec;

use crate::fs::{FsResult, FsError};
use crate::storage::BlockDevice;

use super::fat_table::FatTableManager;
use super::directory_ops::DirectoryManager;

/// Write operations manager
pub struct WriteManager<'a> {
    device: &'a dyn BlockDevice,
    fat_manager: &'a mut FatTableManager<'a>,
    dir_manager: &'a mut DirectoryManager<'a>,
    bytes_per_cluster: u32,
    sectors_per_cluster: u8,
    data_start_sector: u32,
}

impl<'a> WriteManager<'a> {
    /// Create a new write manager
    pub fn new(
        device: &'a dyn BlockDevice,
        fat_manager: &'a mut FatTableManager<'a>,
        dir_manager: &'a mut DirectoryManager<'a>,
        bytes_per_cluster: u32,
        sectors_per_cluster: u8,
        data_start_sector: u32,
    ) -> Self {
        Self {
            device,
            fat_manager,
            dir_manager,
            bytes_per_cluster,
            sectors_per_cluster,
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
    
    /// Append data to file (append-only initially)
    pub fn append_to_file(
        &mut self,
        start_cluster: u32,
        current_size: u32,
        data: &[u8],
    ) -> FsResult<u32> {
        if data.is_empty() {
            return Ok(current_size);
        }
        
        // Calculate how many bytes we need to add
        let bytes_to_add = data.len() as u32;
        let new_size = current_size + bytes_to_add;
        
        // Calculate how many clusters we currently have
        let current_clusters = self.fat_manager.get_chain_length(start_cluster)?;
        let current_capacity = current_clusters * self.bytes_per_cluster;
        
        // Check if we need more clusters
        let mut bytes_written = 0;
        let mut data_offset = 0;
        
        if new_size > current_capacity {
            // Need to allocate more clusters
            let additional_bytes_needed = new_size - current_capacity;
            let additional_clusters_needed = (additional_bytes_needed + self.bytes_per_cluster - 1) / self.bytes_per_cluster;
            
            // Extend the cluster chain
            self.fat_manager.extend_cluster_chain(start_cluster, additional_clusters_needed)?;
        }
        
        // Find the last cluster in the chain
        let mut last_cluster = start_cluster;
        loop {
            if let Some(next) = self.fat_manager.next_cluster(last_cluster) {
                last_cluster = next;
            } else {
                break;
            }
        }
        
        // Calculate offset within the last cluster
        let offset_in_last_cluster = current_size % self.bytes_per_cluster;
        
        if offset_in_last_cluster > 0 {
            // We have space in the last cluster
            let space_in_cluster = self.bytes_per_cluster - offset_in_last_cluster;
            let to_write = bytes_to_add.min(space_in_cluster) as usize;
            
            // Read the cluster
            let mut cluster_data = vec![0u8; self.bytes_per_cluster as usize];
            self.read_cluster(last_cluster, &mut cluster_data)?;
            
            // Write data to cluster
            cluster_data[offset_in_last_cluster as usize..offset_in_last_cluster as usize + to_write]
                .copy_from_slice(&data[0..to_write]);
            
            // Write cluster back
            self.write_cluster(last_cluster, &cluster_data)?;
            
            bytes_written += to_write as u32;
            data_offset += to_write;
            
            // Move to next cluster if we wrote all available space
            if to_write == space_in_cluster as usize {
                if let Some(next) = self.fat_manager.next_cluster(last_cluster) {
                    last_cluster = next;
                }
            }
        }
        
        // Write remaining data in full clusters
        while bytes_written < bytes_to_add {
            let remaining = (bytes_to_add - bytes_written) as usize;
            let to_write = remaining.min(self.bytes_per_cluster as usize);
            
            // Read cluster (in case it's not zeroed)
            let mut cluster_data = vec![0u8; self.bytes_per_cluster as usize];
            self.read_cluster(last_cluster, &mut cluster_data)?;
            
            // Write data to cluster
            cluster_data[0..to_write].copy_from_slice(&data[data_offset..data_offset + to_write]);
            
            // Write cluster back
            self.write_cluster(last_cluster, &cluster_data)?;
            
            bytes_written += to_write as u32;
            data_offset += to_write;
            
            // Move to next cluster
            if let Some(next) = self.fat_manager.next_cluster(last_cluster) {
                last_cluster = next;
            }
        }
        
        Ok(new_size)
    }
    
    /// Write data to file at specific offset
    pub fn write_to_file(
        &mut self,
        start_cluster: u32,
        offset: u64,
        data: &[u8],
    ) -> FsResult<u32> {
        if data.is_empty() {
            return Ok(0);
        }
        
        let mut bytes_written = 0;
        let mut current_offset = offset;
        let mut data_offset = 0;
        let mut current_cluster = start_cluster;
        
        // Skip to the right cluster
        let mut cluster_offset = (offset / self.bytes_per_cluster as u64) as u32;
        for _ in 0..cluster_offset {
            if let Some(next) = self.fat_manager.next_cluster(current_cluster) {
                current_cluster = next;
            } else {
                // Need to extend file
                let new_cluster = self.fat_manager.extend_cluster_chain(current_cluster, 1)?;
                current_cluster = new_cluster;
            }
        }
        
        let byte_offset_in_cluster = (offset % self.bytes_per_cluster as u64) as usize;
        
        // Write first (possibly partial) cluster
        if byte_offset_in_cluster > 0 || data.len() < self.bytes_per_cluster as usize {
            let mut cluster_data = vec![0u8; self.bytes_per_cluster as usize];
            self.read_cluster(current_cluster, &mut cluster_data)?;
            
            let space_in_cluster = self.bytes_per_cluster as usize - byte_offset_in_cluster;
            let to_write = data.len().min(space_in_cluster);
            
            cluster_data[byte_offset_in_cluster..byte_offset_in_cluster + to_write]
                .copy_from_slice(&data[0..to_write]);
            
            self.write_cluster(current_cluster, &cluster_data)?;
            
            bytes_written += to_write as u32;
            data_offset += to_write;
            current_offset += to_write as u64;
            
            // Move to next cluster
            if let Some(next) = self.fat_manager.next_cluster(current_cluster) {
                current_cluster = next;
            }
        }
        
        // Write full clusters
        while data_offset + self.bytes_per_cluster as usize <= data.len() {
            let mut cluster_data = vec![0u8; self.bytes_per_cluster as usize];
            cluster_data[..self.bytes_per_cluster as usize]
                .copy_from_slice(&data[data_offset..data_offset + self.bytes_per_cluster as usize]);
            
            self.write_cluster(current_cluster, &cluster_data)?;
            
            bytes_written += self.bytes_per_cluster;
            data_offset += self.bytes_per_cluster as usize;
            current_offset += self.bytes_per_cluster as u64;
            
            // Move to next cluster
            if let Some(next) = self.fat_manager.next_cluster(current_cluster) {
                current_cluster = next;
            }
        }
        
        // Write last (possibly partial) cluster
        let remaining = data.len() - data_offset;
        if remaining > 0 {
            let mut cluster_data = vec![0u8; self.bytes_per_cluster as usize];
            self.read_cluster(current_cluster, &mut cluster_data)?;
            
            cluster_data[0..remaining].copy_from_slice(&data[data_offset..]);
            
            self.write_cluster(current_cluster, &cluster_data)?;
            
            bytes_written += remaining as u32;
        }
        
        Ok(bytes_written)
    }
    
    /// Create a new file
    pub fn create_file(
        &mut self,
        dir_cluster: u32,
        name: &str,
        initial_data: &[u8],
    ) -> FsResult<u32> {
        // Allocate cluster for file
        let bytes_needed = initial_data.len() as u32;
        let clusters_needed = if bytes_needed == 0 {
            1 // Minimum 1 cluster
        } else {
            (bytes_needed + self.bytes_per_cluster - 1) / self.bytes_per_cluster
        };
        
        let start_cluster = self.fat_manager.allocate_cluster_chain(clusters_needed)?;
        
        // Write initial data
        if !initial_data.is_empty() {
            self.write_to_file(start_cluster, 0, initial_data)?;
        }
        
        // Create directory entry
        self.dir_manager.create_entry(
            dir_cluster,
            name,
            crate::fs::FileType::Regular,
            start_cluster,
            bytes_needed,
        )?;
        
        Ok(start_cluster)
    }
    
    /// Create a new directory
    pub fn create_directory(
        &mut self,
        parent_dir_cluster: u32,
        name: &str,
    ) -> FsResult<u32> {
        // Allocate cluster for directory
        let start_cluster = self.fat_manager.allocate_cluster_chain(1)?;
        
        // Initialize directory with "." and ".." entries
        self.dir_manager.init_directory_cluster(start_cluster, parent_dir_cluster)?;
        
        // Create directory entry in parent
        self.dir_manager.create_entry(
            parent_dir_cluster,
            name,
            crate::fs::FileType::Directory,
            start_cluster,
            0, // Directories have size 0 in FAT
        )?;
        
        Ok(start_cluster)
    }
    
    /// Truncate file to specified size
    pub fn truncate_file(
        &mut self,
        start_cluster: u32,
        new_size: u32,
    ) -> FsResult<()> {
        // Calculate how many clusters we need
        let clusters_needed = if new_size == 0 {
            1 // Keep at least one cluster
        } else {
            (new_size + self.bytes_per_cluster - 1) / self.bytes_per_cluster
        };
        
        // Get current chain length
        let current_length = self.fat_manager.get_chain_length(start_cluster)?;
        
        if clusters_needed < current_length {
            // Need to deallocate extra clusters
            // Find the cluster where we need to cut
            let mut cut_cluster = start_cluster;
            for _ in 0..clusters_needed {
                if let Some(next) = self.fat_manager.next_cluster(cut_cluster) {
                    cut_cluster = next;
                } else {
                    break;
                }
            }
            
            // Deallocate from cut_cluster onward
            if let Some(next) = self.fat_manager.next_cluster(cut_cluster) {
                self.fat_manager.deallocate_cluster_chain(next)?;
                
                // Mark cut_cluster as EOF
                self.fat_manager.set_entry(cut_cluster, 0x0FFFFFFF)?;
            }
        } else if clusters_needed > current_length {
            // Need to allocate more clusters
            let additional = clusters_needed - current_length;
            self.fat_manager.extend_cluster_chain(start_cluster, additional)?;
        }
        
        // If new_size is 0, clear the first cluster
        if new_size == 0 {
            let zero_cluster = vec![0u8; self.bytes_per_cluster as usize];
            self.write_cluster(start_cluster, &zero_cluster)?;
        }
        
        Ok(())
    }
    
    /// Handle write error and attempt recovery
    pub fn handle_write_error(&mut self, error: FsError) -> FsResult<()> {
        match error {
            FsError::IoError => {
                // Try to sync FAT table to ensure consistency
                self.fat_manager.sync()?;
                Err(FsError::IoError)
            }
            FsError::OutOfMemory => {
                // Try to free some clusters if possible
                // For now, just return the error
                Err(FsError::OutOfMemory)
            }
            FsError::WriteProtected => {
                // Cannot recover from write protection
                Err(FsError::WriteProtected)
            }
            _ => Err(error),
        }
    }
    
    /// Sync all pending writes
    pub fn sync(&mut self) -> FsResult<()> {
        self.fat_manager.sync()?;
        self.device.flush().map_err(|_| FsError::IoError)
    }
}