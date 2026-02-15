//! Block and metadata caching for filesystem performance
//!
//! Implements read-ahead, write-behind, FAT table caching, and
//! directory entry caching for optimal filesystem performance.

use crate::error::VFatError;
use crate::fs::block::BlockDevice;
use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::time::Duration;

/// Cache block state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CacheState {
    /// Clean - matches disk
    Clean,
    /// Dirty - modified, needs write
    Dirty,
    /// Invalid - can be reused
    Invalid,
}

/// Cache entry for a single block
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// Block number
    pub block: u64,
    /// Cached data
    pub data: Vec<u8>,
    /// Cache state
    pub state: CacheState,
    /// Last access time (for LRU)
    pub last_access: u64,
    /// Access count (for LFU)
    pub access_count: u64,
}

impl CacheEntry {
    /// Create a new empty cache entry
    pub fn new(block_size: usize) -> Self {
        Self {
            block: u64::MAX,
            data: vec![0u8; block_size],
            state: CacheState::Invalid,
            last_access: 0,
            access_count: 0,
        }
    }

    /// Check if entry is valid for given block
    pub fn is_valid_for(&self, block: u64) -> bool {
        self.state != CacheState::Invalid && self.block == block
    }

    /// Mark as accessed
    pub fn touch(&mut self, timestamp: u64) {
        self.last_access = timestamp;
        self.access_count += 1;
    }
}

/// Cache replacement policy
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CachePolicy {
    /// Least Recently Used
    Lru,
    /// Least Frequently Used
    Lfu,
    /// First In First Out
    Fifo,
}

/// Block cache for filesystem operations
pub struct BlockCache<B: BlockDevice> {
    /// Cache entries
    entries: Vec<CacheEntry>,
    /// Block size
    block_size: usize,
    /// Current timestamp for LRU
    timestamp: u64,
    /// Replacement policy
    policy: CachePolicy,
    /// Maximum dirty blocks before forced flush
    max_dirty_blocks: usize,
    /// Phantom marker for BlockDevice
    _phantom: core::marker::PhantomData<B>,
}

impl<B: BlockDevice> BlockCache<B> {
    /// Create a new block cache
    pub fn new(block_size: usize, capacity: usize) -> Self {
        let mut entries = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            entries.push(CacheEntry::new(block_size));
        }

        Self {
            entries,
            block_size,
            timestamp: 0,
            policy: CachePolicy::Lru,
            max_dirty_blocks: capacity / 4, // Flush when 25% dirty
            _phantom: core::marker::PhantomData,
        }
    }

    /// Set cache replacement policy
    pub fn set_policy(&mut self, policy: CachePolicy) {
        self.policy = policy;
    }

    /// Get number of dirty blocks
    pub fn dirty_count(&self) -> usize {
        self.entries.iter().filter(|e| e.state == CacheState::Dirty).count()
    }

    /// Read a sector from cache or device
    pub fn read_sector(&mut self, device: &mut B, block: u64) -> Result<&[u8], VFatError> {
        self.timestamp += 1;

        // Check if in cache
        for i in 0..self.entries.len() {
            if self.entries[i].is_valid_for(block) {
                self.entries[i].touch(self.timestamp);
                return Ok(&self.entries[i].data);
            }
        }

        // Not in cache, need to load
        self.load_block(device, block)
    }

    /// Write a sector to cache
    pub fn write_sector(&mut self, device: &mut B, block: u64, data: &[u8]) -> Result<(), VFatError> {
        self.timestamp += 1;

        if data.len() != self.block_size {
            return Err(VFatError::InvalidParameter(
                "Data size mismatch".to_string()
            ));
        }

        // Check if in cache
        for i in 0..self.entries.len() {
            if self.entries[i].is_valid_for(block) {
                self.entries[i].data.copy_from_slice(data);
                self.entries[i].state = CacheState::Dirty;
                self.entries[i].touch(self.timestamp);
                return Ok(());
            }
        }

        // Not in cache, need to evict and load
        let entry_idx = self.find_victim(device)?;
        self.entries[entry_idx].block = block;
        self.entries[entry_idx].data.copy_from_slice(data);
        self.entries[entry_idx].state = CacheState::Dirty;
        self.entries[entry_idx].touch(self.timestamp);

        // Flush if too many dirty blocks
        if self.dirty_count() >= self.max_dirty_blocks {
            self.flush(device)?;
        }

        Ok(())
    }

    /// Load a block into cache
    fn load_block(&mut self, device: &mut B, block: u64) -> Result<&[u8], VFatError> {
        let entry_idx = self.find_victim(device)?;
        
        // Read from device
        device.read_block(block, &mut self.entries[entry_idx].data)?;
        
        self.entries[entry_idx].block = block;
        self.entries[entry_idx].state = CacheState::Clean;
        self.entries[entry_idx].touch(self.timestamp);

        Ok(&self.entries[entry_idx].data)
    }

    /// Find a victim entry to evict
    fn find_victim(&mut self, device: &mut B) -> Result<usize, VFatError> {
        // First, look for invalid entries
        for i in 0..self.entries.len() {
            if self.entries[i].state == CacheState::Invalid {
                return Ok(i);
            }
        }

        // Flush dirty entries if needed
        self.flush(device)?;

        // Apply replacement policy
        match self.policy {
            CachePolicy::Lru => {
                let mut min_access = u64::MAX;
                let mut victim = 0;
                for i in 0..self.entries.len() {
                    if self.entries[i].last_access < min_access {
                        min_access = self.entries[i].last_access;
                        victim = i;
                    }
                }
                Ok(victim)
            }
            CachePolicy::Lfu => {
                let mut min_count = u64::MAX;
                let mut victim = 0;
                for i in 0..self.entries.len() {
                    if self.entries[i].access_count < min_count {
                        min_count = self.entries[i].access_count;
                        victim = i;
                    }
                }
                Ok(victim)
            }
            CachePolicy::Fifo => {
                // Just evict the oldest (lowest block number as simple approximation)
                let mut min_block = u64::MAX;
                let mut victim = 0;
                for i in 0..self.entries.len() {
                    if self.entries[i].block < min_block {
                        min_block = self.entries[i].block;
                        victim = i;
                    }
                }
                Ok(victim)
            }
        }
    }

    /// Flush all dirty blocks to device
    pub fn flush(&mut self, device: &mut B) -> Result<(), VFatError> {
        for entry in self.entries.iter_mut() {
            if entry.state == CacheState::Dirty {
                device.write_block(entry.block, &entry.data)?;
                entry.state = CacheState::Clean;
            }
        }
        device.flush()
    }

    /// Invalidate all entries
    pub fn invalidate(&mut self) {
        for entry in self.entries.iter_mut() {
            entry.state = CacheState::Invalid;
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            capacity: self.entries.len(),
            valid_entries: self.entries.iter().filter(|e| e.state != CacheState::Invalid).count(),
            dirty_entries: self.dirty_count(),
            clean_entries: self.entries.iter().filter(|e| e.state == CacheState::Clean).count(),
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone, Copy)]
pub struct CacheStats {
    /// Total cache capacity
    pub capacity: usize,
    /// Number of valid entries
    pub valid_entries: usize,
    /// Number of dirty entries
    pub dirty_entries: usize,
    /// Number of clean entries
    pub clean_entries: usize,
}

/// Read-ahead cache for sequential access
pub struct ReadAheadCache<B: BlockDevice> {
    /// Underlying block cache
    cache: BlockCache<B>,
    /// Read-ahead window size
    window_size: usize,
    /// Last accessed block (for detecting sequential access)
    last_block: Option<u64>,
    /// Pending read-ahead blocks
    read_ahead_queue: VecDeque<u64>,
}

impl<B: BlockDevice> ReadAheadCache<B> {
    /// Create a new read-ahead cache
    pub fn new(block_size: usize, cache_capacity: usize, window_size: usize) -> Self {
        Self {
            cache: BlockCache::new(block_size, cache_capacity),
            window_size,
            last_block: None,
            read_ahead_queue: VecDeque::new(),
        }
    }

    /// Read with read-ahead
    pub fn read(&mut self, device: &mut B, block: u64) -> Result<&[u8], VFatError> {
        // Check if sequential
        let is_sequential = self.last_block.map(|last| block == last + 1).unwrap_or(false);
        self.last_block = Some(block);

        // Perform read
        let result = self.cache.read_sector(device, block);

        // Trigger read-ahead if sequential
        if is_sequential {
            self.trigger_read_ahead(device, block);
        }

        result
    }

    /// Trigger read-ahead for sequential reads
    fn trigger_read_ahead(&mut self, device: &mut B, current_block: u64) {
        for i in 1..=self.window_size as u64 {
            let ahead_block = current_block + i;
            
            // Check if already in cache
            let mut in_cache = false;
            for entry in &self.cache.entries {
                if entry.is_valid_for(ahead_block) {
                    in_cache = true;
                    break;
                }
            }

            if !in_cache && !self.read_ahead_queue.contains(&ahead_block) {
                self.read_ahead_queue.push_back(ahead_block);
            }
        }

        // Process read-ahead queue
        while !self.read_ahead_queue.is_empty() && self.cache.dirty_count() < self.cache.max_dirty_blocks {
            if let Some(block) = self.read_ahead_queue.pop_front() {
                let _ = self.cache.read_sector(device, block);
            }
        }
    }

    /// Write through cache
    pub fn write(&mut self, device: &mut B, block: u64, data: &[u8]) -> Result<(), VFatError> {
        self.last_block = Some(block);
        self.cache.write_sector(device, block, data)
    }

    /// Flush cache
    pub fn flush(&mut self, device: &mut B) -> Result<(), VFatError> {
        self.cache.flush(device)
    }
}

/// FAT table cache for faster cluster operations
pub struct FatCache {
    /// Cached FAT sectors
    fat_sectors: Vec<FatSectorCache>,
    /// Block size
    block_size: usize,
    /// Maximum cached FAT sectors
    max_sectors: usize,
}

/// Single cached FAT sector
#[derive(Debug, Clone)]
struct FatSectorCache {
    /// Sector number (relative to FAT start)
    sector: u32,
    /// Sector data
    data: Vec<u8>,
    /// Is modified
    dirty: bool,
    /// Access count
    access_count: u64,
}

impl FatCache {
    /// Create a new FAT cache
    pub fn new(block_size: usize, max_sectors: usize) -> Self {
        Self {
            fat_sectors: Vec::with_capacity(max_sectors),
            block_size,
            max_sectors,
        }
    }

    /// Read a FAT entry
    pub fn read_entry<B: BlockDevice>(&mut self, device: &mut B, 
                                      fat_start_sector: u32, 
                                      cluster: u32) -> Result<u32, VFatError> {
        let fat_offset = cluster * 4;
        let sector = fat_start_sector + (fat_offset / self.block_size as u32);
        let offset = (fat_offset % self.block_size as u32) as usize;

        let data = self.get_sector(device, fat_start_sector, sector)?;
        
        let entry = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]);

        Ok(entry & 0x0FFFFFFF)
    }

    /// Write a FAT entry
    pub fn write_entry<B: BlockDevice>(&mut self, device: &mut B,
                                       fat_start_sector: u32,
                                       cluster: u32, value: u32) -> Result<(), VFatError> {
        let fat_offset = cluster * 4;
        let sector = fat_start_sector + (fat_offset / self.block_size as u32);
        let offset = (fat_offset % self.block_size as u32) as usize;

        let data = self.get_sector_mut(device, fat_start_sector, sector)?;
        
        let preserved = data[offset + 3] & 0xF0;
        let bytes = (value & 0x0FFFFFFF).to_le_bytes();
        
        data[offset] = bytes[0];
        data[offset + 1] = bytes[1];
        data[offset + 2] = bytes[2];
        data[offset + 3] = (bytes[3] & 0x0F) | preserved;

        // Mark as dirty
        if let Some(cache) = self.fat_sectors.iter_mut().find(|s| s.sector == sector) {
            cache.dirty = true;
        }

        Ok(())
    }

    /// Get sector data (mutable)
    fn get_sector_mut<B: BlockDevice>(&mut self, device: &mut B, 
                                      fat_start_sector: u32, 
                                      sector: u32) -> Result<&mut Vec<u8>, VFatError> {
        // Check if already cached
        if let Some(idx) = self.fat_sectors.iter().position(|s| s.sector == sector) {
            self.fat_sectors[idx].access_count += 1;
            return Ok(&mut self.fat_sectors[idx].data);
        }

        // Need to load
        self.load_sector(device, fat_start_sector, sector)?;
        
        if let Some(idx) = self.fat_sectors.iter().position(|s| s.sector == sector) {
            Ok(&mut self.fat_sectors[idx].data)
        } else {
            Err(VFatError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to load FAT sector",
            )))
        }
    }

    /// Get sector data (immutable)
    fn get_sector<B: BlockDevice>(&mut self, device: &mut B,
                                  fat_start_sector: u32,
                                  sector: u32) -> Result<&Vec<u8>, VFatError> {
        // Check if already cached
        if let Some(idx) = self.fat_sectors.iter().position(|s| s.sector == sector) {
            self.fat_sectors[idx].access_count += 1;
            return Ok(&self.fat_sectors[idx].data);
        }

        // Need to load
        self.load_sector(device, fat_start_sector, sector)?;
        
        if let Some(idx) = self.fat_sectors.iter().position(|s| s.sector == sector) {
            Ok(&self.fat_sectors[idx].data)
        } else {
            Err(VFatError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to load FAT sector",
            )))
        }
    }

    /// Load a FAT sector into cache
    fn load_sector<B: BlockDevice>(&mut self, device: &mut B,
                                   fat_start_sector: u32,
                                   sector: u32) -> Result<(), VFatError> {
        // Flush oldest if cache is full
        if self.fat_sectors.len() >= self.max_sectors {
            self.flush_oldest(device, fat_start_sector)?;
        }

        let mut data = vec![0u8; self.block_size];
        device.read_block((fat_start_sector + sector) as u64, &mut data)?;

        self.fat_sectors.push(FatSectorCache {
            sector,
            data,
            dirty: false,
            access_count: 1,
        });

        Ok(())
    }

    /// Flush oldest/dirtiest sector
    fn flush_oldest<B: BlockDevice>(&mut self, device: &mut B,
                                    fat_start_sector: u32) -> Result<(), VFatError> {
        if self.fat_sectors.is_empty() {
            return Ok(());
        }

        // Find victim (prefer clean entries, then least accessed)
        let victim_idx = self.fat_sectors.iter()
            .enumerate()
            .min_by_key(|(_, s)| {
                let dirty_penalty = if s.dirty { 1000u64 } else { 0u64 };
                s.access_count + dirty_penalty
            })
            .map(|(i, _)| i)
            .unwrap_or(0);

        // Flush if dirty
        if self.fat_sectors[victim_idx].dirty {
            let sector = self.fat_sectors[victim_idx].sector;
            let data = self.fat_sectors[victim_idx].data.clone();
            device.write_block((fat_start_sector + sector) as u64, &data)?;
        }

        self.fat_sectors.remove(victim_idx);
        Ok(())
    }

    /// Flush all dirty sectors
    pub fn flush<B: BlockDevice>(&mut self, device: &mut B, 
                                 fat_start_sector: u32) -> Result<(), VFatError> {
        for sector_cache in self.fat_sectors.iter_mut() {
            if sector_cache.dirty {
                device.write_block((fat_start_sector + sector_cache.sector) as u64, &sector_cache.data)?;
                sector_cache.dirty = false;
            }
        }
        device.flush()
    }
}

/// Directory entry cache
pub struct DirEntryCache {
    /// Cached directory clusters
    dir_clusters: Vec<DirClusterCache>,
    /// Cluster size
    cluster_size: usize,
    /// Maximum cached clusters
    max_clusters: usize,
}

/// Cached directory cluster
#[derive(Debug, Clone)]
struct DirClusterCache {
    /// Cluster number
    cluster: u32,
    /// Cluster data
    data: Vec<u8>,
    /// Is modified
    dirty: bool,
}

impl DirEntryCache {
    /// Create a new directory entry cache
    pub fn new(cluster_size: usize, max_clusters: usize) -> Self {
        Self {
            dir_clusters: Vec::with_capacity(max_clusters),
            cluster_size,
            max_clusters,
        }
    }

    /// Get directory cluster data
    pub fn get_cluster<B: BlockDevice>(&mut self, device: &mut B,
                                       cluster: u32,
                                       cluster_to_sector: impl Fn(u32) -> u64) -> Result<&[u8], VFatError> {
        // Check if cached
        if let Some(idx) = self.dir_clusters.iter().position(|c| c.cluster == cluster) {
            return Ok(&self.dir_clusters[idx].data);
        }

        // Load cluster
        self.load_cluster(device, cluster, cluster_to_sector)?;
        
        if let Some(idx) = self.dir_clusters.iter().position(|c| c.cluster == cluster) {
            Ok(&self.dir_clusters[idx].data)
        } else {
            Err(VFatError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to load directory cluster",
            )))
        }
    }

    /// Get mutable directory cluster data
    pub fn get_cluster_mut<B: BlockDevice>(&mut self, device: &mut B,
                                           cluster: u32,
                                           cluster_to_sector: impl Fn(u32) -> u64) -> Result<&mut Vec<u8>, VFatError> {
        // Check if cached
        if let Some(idx) = self.dir_clusters.iter().position(|c| c.cluster == cluster) {
            self.dir_clusters[idx].dirty = true;
            return Ok(&mut self.dir_clusters[idx].data);
        }

        // Load cluster
        self.load_cluster(device, cluster, cluster_to_sector)?;
        
        if let Some(idx) = self.dir_clusters.iter().position(|c| c.cluster == cluster) {
            self.dir_clusters[idx].dirty = true;
            Ok(&mut self.dir_clusters[idx].data)
        } else {
            Err(VFatError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to load directory cluster",
            )))
        }
    }

    /// Load a directory cluster
    fn load_cluster<B: BlockDevice>(&mut self, device: &mut B,
                                    cluster: u32,
                                    cluster_to_sector: impl Fn(u32) -> u64) -> Result<(), VFatError> {
        // Evict if necessary
        if self.dir_clusters.len() >= self.max_clusters {
            self.evict_oldest(device, &cluster_to_sector)?;
        }

        let start_sector = cluster_to_sector(cluster);
        let sectors_per_cluster = self.cluster_size / 512; // Assuming 512 byte sectors
        
        let mut data = Vec::with_capacity(self.cluster_size);
        
        for i in 0..sectors_per_cluster {
            let mut sector_data = vec![0u8; 512];
            device.read_block(start_sector + i as u64, &mut sector_data)?;
            data.extend_from_slice(&sector_data);
        }

        self.dir_clusters.push(DirClusterCache {
            cluster,
            data,
            dirty: false,
        });

        Ok(())
    }

    /// Evict oldest cluster
    fn evict_oldest<B: BlockDevice>(&mut self, device: &mut B,
                                    cluster_to_sector: impl Fn(u32) -> u64) -> Result<(), VFatError> {
        if self.dir_clusters.is_empty() {
            return Ok(());
        }

        // Flush if dirty and remove first (FIFO for simplicity)
        if self.dir_clusters[0].dirty {
            self.flush_cluster(0, device, &cluster_to_sector)?;
        }

        self.dir_clusters.remove(0);
        Ok(())
    }

    /// Flush a specific cluster
    fn flush_cluster<B: BlockDevice>(&self, idx: usize, device: &mut B,
                                     cluster_to_sector: impl Fn(u32) -> u64) -> Result<(), VFatError> {
        if idx >= self.dir_clusters.len() {
            return Ok(());
        }

        let cluster = self.dir_clusters[idx].cluster;
        let data = &self.dir_clusters[idx].data;
        let start_sector = cluster_to_sector(cluster);
        let sectors_per_cluster = self.cluster_size / 512;

        for i in 0..sectors_per_cluster {
            let offset = i * 512;
            device.write_block(start_sector + i as u64, &data[offset..offset + 512])?;
        }

        Ok(())
    }

    /// Flush all dirty clusters
    pub fn flush<B: BlockDevice>(&mut self, device: &mut B,
                                 cluster_to_sector: impl Fn(u32) -> u64) -> Result<(), VFatError> {
        for i in 0..self.dir_clusters.len() {
            if self.dir_clusters[i].dirty {
                self.flush_cluster(i, device, &cluster_to_sector)?;
                self.dir_clusters[i].dirty = false;
            }
        }
        device.flush()
    }

    /// Invalidate cluster
    pub fn invalidate(&mut self, cluster: u32) {
        if let Some(idx) = self.dir_clusters.iter().position(|c| c.cluster == cluster) {
            self.dir_clusters.remove(idx);
        }
    }
}

// External crate dependencies
extern crate alloc;
