//! FAT Table manipulation for FAT32 filesystem
//!
//! Provides functions for allocating, deallocating, and modifying FAT entries.

use crate::fs::FsResult;
use crate::storage::BlockDevice;

/// FAT special values
const FAT_ENTRY_FREE: u32 = 0x00000000;
const FAT_ENTRY_RESERVED: u32 = 0x00000001;
const FAT_ENTRY_MIN: u32 = 0x00000002;
const FAT_ENTRY_MAX: u32 = 0x0FFFFFF6;
const FAT_ENTRY_BAD: u32 = 0x0FFFFFF7;
const FAT_ENTRY_EOF: u32 = 0x0FFFFFFF;

/// FAT table manager
pub struct FatTableManager<'a> {
    device: &'a dyn BlockDevice,
    fat_start_sector: u64,
    sectors_per_fat: u32,
    bytes_per_sector: u16,
    fat_cache: Vec<u32>,
    dirty: bool,
}

impl<'a> FatTableManager<'a> {
    /// Create a new FAT table manager
    pub fn new(
        device: &'a dyn BlockDevice,
        fat_start_sector: u64,
        sectors_per_fat: u32,
        bytes_per_sector: u16,
    ) -> FsResult<Self> {
        // Read FAT into memory
        let fat_size = (sectors_per_fat as usize) * (bytes_per_sector as usize);
        let fat_entries = fat_size / 4; // Each FAT entry is 4 bytes
        
        let mut fat_cache = Vec::with_capacity(fat_entries);
        let mut fat_buffer = vec![0u8; fat_size];
        
        device.read_blocks(fat_start_sector, sectors_per_fat as usize, &mut fat_buffer)
            .map_err(|_| crate::fs::FsError::IoError)?;
        
        for i in 0..fat_entries {
            let entry = unsafe {
                core::ptr::read_unaligned(fat_buffer.as_ptr().add(i * 4) as *const u32)
            } & 0x0FFFFFFF;
            fat_cache.push(entry);
        }
        
        Ok(Self {
            device,
            fat_start_sector,
            sectors_per_fat,
            bytes_per_sector,
            fat_cache,
            dirty: false,
        })
    }
    
    /// Get FAT entry for a cluster
    pub fn get_entry(&self, cluster: u32) -> Option<u32> {
        self.fat_cache.get(cluster as usize).copied()
    }
    
    /// Set FAT entry for a cluster
    pub fn set_entry(&mut self, cluster: u32, value: u32) -> FsResult<()> {
        if cluster as usize >= self.fat_cache.len() {
            return Err(crate::fs::FsError::InvalidArgument);
        }
        
        // Ensure value is masked to 28 bits
        let masked_value = value & 0x0FFFFFFF;
        self.fat_cache[cluster as usize] = masked_value;
        self.dirty = true;
        Ok(())
    }
    
    /// Find a free cluster
    pub fn find_free_cluster(&self) -> Option<u32> {
        // Start searching from cluster 2 (clusters 0 and 1 are reserved)
        for cluster in 2..self.fat_cache.len() as u32 {
            if self.fat_cache[cluster as usize] == FAT_ENTRY_FREE {
                return Some(cluster);
            }
        }
        None
    }
    
    /// Allocate a new cluster chain
    pub fn allocate_cluster_chain(&mut self, count: u32) -> FsResult<u32> {
        if count == 0 {
            return Err(crate::fs::FsError::InvalidArgument);
        }
        
        let mut first_cluster = None;
        let mut prev_cluster = 0;
        
        for i in 0..count {
            let cluster = self.find_free_cluster()
                .ok_or(crate::fs::FsError::OutOfMemory)?;
            
            if i == 0 {
                first_cluster = Some(cluster);
            } else {
                // Link previous cluster to this one
                self.set_entry(prev_cluster, cluster)?;
            }
            
            // Mark this cluster as EOF if it's the last one
            let value = if i == count - 1 {
                FAT_ENTRY_EOF
            } else {
                // Temporary value, will be updated when we allocate next cluster
                FAT_ENTRY_RESERVED
            };
            
            self.set_entry(cluster, value)?;
            prev_cluster = cluster;
        }
        
        first_cluster.ok_or(crate::fs::FsError::OutOfMemory)
    }
    
    /// Extend an existing cluster chain
    pub fn extend_cluster_chain(&mut self, last_cluster: u32, count: u32) -> FsResult<u32> {
        if count == 0 {
            return Ok(last_cluster);
        }
        
        // Find the end of the chain
        let mut current = last_cluster;
        loop {
            let entry = self.get_entry(current)
                .ok_or(crate::fs::FsError::InvalidArgument)?;
            
            if entry >= FAT_ENTRY_EOF {
                break;
            }
            current = entry;
        }
        
        // Allocate new clusters
        let first_new_cluster = self.allocate_cluster_chain(count)?;
        
        // Link the old chain to the new chain
        self.set_entry(current, first_new_cluster)?;
        
        Ok(first_new_cluster)
    }
    
    /// Deallocate a cluster chain
    pub fn deallocate_cluster_chain(&mut self, start_cluster: u32) -> FsResult<()> {
        let mut current = start_cluster;
        
        while current < FAT_ENTRY_EOF {
            let entry = self.get_entry(current)
                .ok_or(crate::fs::FsError::InvalidArgument)?;
            
            // Mark cluster as free
            self.set_entry(current, FAT_ENTRY_FREE)?;
            
            if entry >= FAT_ENTRY_EOF || entry < FAT_ENTRY_MIN {
                break;
            }
            
            current = entry;
        }
        
        Ok(())
    }
    
    /// Get chain length
    pub fn get_chain_length(&self, start_cluster: u32) -> FsResult<u32> {
        let mut count = 0;
        let mut current = start_cluster;
        
        while current < FAT_ENTRY_EOF {
            count += 1;
            
            let entry = self.get_entry(current)
                .ok_or(crate::fs::FsError::InvalidArgument)?;
            
            if entry >= FAT_ENTRY_EOF || entry < FAT_ENTRY_MIN {
                break;
            }
            
            current = entry;
        }
        
        Ok(count)
    }
    
    /// Sync FAT table to disk
    pub fn sync(&mut self) -> FsResult<()> {
        if !self.dirty {
            return Ok(());
        }
        
        // Write FAT to all copies
        for fat_copy in 0..2 { // Usually 2 FAT copies
            let fat_start = self.fat_start_sector + (fat_copy as u64 * self.sectors_per_fat as u64);
            
            // Convert FAT cache to bytes
            let mut fat_buffer = vec![0u8; (self.sectors_per_fat as usize) * (self.bytes_per_sector as usize)];
            
            for (i, &entry) in self.fat_cache.iter().enumerate() {
                let bytes = entry.to_le_bytes();
                fat_buffer[i * 4..i * 4 + 4].copy_from_slice(&bytes);
            }
            
            self.device.write_blocks(fat_start, self.sectors_per_fat as usize, &fat_buffer)
                .map_err(|_| crate::fs::FsError::IoError)?;
        }
        
        self.dirty = false;
        Ok(())
    }
    
    /// Check if cluster is free
    pub fn is_cluster_free(&self, cluster: u32) -> bool {
        self.get_entry(cluster) == Some(FAT_ENTRY_FREE)
    }
    
    /// Check if cluster is allocated
    pub fn is_cluster_allocated(&self, cluster: u32) -> bool {
        if let Some(entry) = self.get_entry(cluster) {
            entry >= FAT_ENTRY_MIN && entry <= FAT_ENTRY_MAX
        } else {
            false
        }
    }
    
    /// Check if cluster is EOF
    pub fn is_cluster_eof(&self, cluster: u32) -> bool {
        if let Some(entry) = self.get_entry(cluster) {
            entry >= FAT_ENTRY_EOF
        } else {
            false
        }
    }
    
    /// Get next cluster in chain
    pub fn next_cluster(&self, cluster: u32) -> Option<u32> {
        let entry = self.get_entry(cluster)?;
        
        if entry >= FAT_ENTRY_MIN && entry <= FAT_ENTRY_MAX {
            Some(entry)
        } else {
            None
        }
    }
}

impl<'a> Drop for FatTableManager<'a> {
    fn drop(&mut self) {
        // Try to sync on drop, but ignore errors
        let _ = self.sync();
    }
}