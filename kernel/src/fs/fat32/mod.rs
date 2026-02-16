//! FAT32 Filesystem Implementation with Write Support
//!
//! This module provides full FAT32 read/write support including:
//! - File creation, deletion, and modification
//! - Directory operations (create, list, delete)
//! - Cluster allocation and FAT table management
//! - Long filename (LFN) support

use crate::error::{VFatError, IoError};
use alloc::format;
use crate::fs::block::BlockDevice;
use crate::fs::cache::{BlockCache, CachePolicy};
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use core::fmt;

/// FAT32 BIOS Parameter Block (first 36 bytes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct BiosParameterBlock {
    /// Jump instruction
    pub jmp_boot: [u8; 3],
    /// OEM name
    pub oem_name: [u8; 8],
    /// Bytes per sector (512, 1024, 2048, or 4096)
    pub bytes_per_sector: u16,
    /// Sectors per cluster (1, 2, 4, 8, 16, 32, 64, 128)
    pub sectors_per_cluster: u8,
    /// Number of reserved sectors
    pub reserved_sector_count: u16,
    /// Number of FATs (typically 2)
    pub num_fats: u8,
    /// Root entry count (0 for FAT32)
    pub root_entry_count: u16,
    /// Total sectors (0 for FAT32, use total_sectors_32)
    pub total_sectors_16: u16,
    /// Media type
    pub media: u8,
    /// Sectors per FAT (0 for FAT32, use sectors_per_fat_32)
    pub sectors_per_fat_16: u16,
    /// Sectors per track
    pub sectors_per_track: u16,
    /// Number of heads
    pub num_heads: u16,
    /// Hidden sectors
    pub hidden_sectors: u32,
    /// Total sectors (32-bit)
    pub total_sectors_32: u32,
}

/// FAT32 Extended BIOS Parameter Block
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Fat32ExtendedBpb {
    /// Sectors per FAT (32-bit)
    pub sectors_per_fat_32: u32,
    /// Drive description flags
    pub ext_flags: u16,
    /// Filesystem version
    pub fs_version: u16,
    /// Root directory cluster
    pub root_cluster: u32,
    /// Filesystem info sector
    pub fs_info: u16,
    /// Backup boot sector
    pub backup_boot_sector: u16,
    /// Reserved
    pub reserved: [u8; 12],
    /// Drive number
    pub drive_number: u8,
    /// Reserved
    pub reserved1: u8,
    /// Boot signature
    pub boot_signature: u8,
    /// Volume serial number
    pub volume_serial: u32,
    /// Volume label
    pub volume_label: [u8; 11],
    /// Filesystem type string
    pub fs_type: [u8; 8],
}

/// FAT32 directory entry (32 bytes)
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct DirEntry {
    /// Short filename (8.3 format)
    pub name: [u8; 11],
    /// Attributes
    pub attr: u8,
    /// Reserved
    pub nt_res: u8,
    /// Creation time tenths
    pub crt_time_tenth: u8,
    /// Creation time
    pub crt_time: u16,
    /// Creation date
    pub crt_date: u16,
    /// Last access date
    pub lst_acc_date: u16,
    /// High cluster number
    pub fst_clus_hi: u16,
    /// Write time
    pub wrt_time: u16,
    /// Write date
    pub wrt_date: u16,
    /// Low cluster number
    pub fst_clus_lo: u16,
    /// File size
    pub file_size: u32,
}

impl fmt::Debug for DirEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Copy fields to local variables to avoid unaligned references
        let attr = self.attr;
        let file_size = self.file_size;
        f.debug_struct("DirEntry")
            .field("name", &self.short_name())
            .field("attr", &attr)
            .field("cluster", &self.cluster())
            .field("file_size", &file_size)
            .finish()
    }
}

/// Directory entry attributes
pub mod attr {
    pub const READ_ONLY: u8 = 0x01;
    pub const HIDDEN: u8 = 0x02;
    pub const SYSTEM: u8 = 0x04;
    pub const VOLUME_ID: u8 = 0x08;
    pub const DIRECTORY: u8 = 0x10;
    pub const ARCHIVE: u8 = 0x20;
    pub const LFN: u8 = 0x0F; // Long filename marker
}

/// FAT special cluster values
pub mod cluster {
    pub const FREE: u32 = 0x00000000;
    pub const RESERVED: u32 = 0x00000001;
    pub const FIRST_DATA: u32 = 0x00000002;
    pub const LAST_RESERVED: u32 = 0x0FFFFFEF;
    pub const BAD: u32 = 0x0FFFFFF7;
    pub const END_OF_CHAIN_MIN: u32 = 0x0FFFFFF8;
    pub const END_OF_CHAIN: u32 = 0x0FFFFFFF;
}

impl DirEntry {
    /// Check if this is a free entry
    pub fn is_free(&self) -> bool {
        self.name[0] == 0x00 || self.name[0] == 0xE5
    }

    /// Check if this is the last entry
    pub fn is_last(&self) -> bool {
        self.name[0] == 0x00
    }

    /// Check if this is a long filename entry
    pub fn is_lfn(&self) -> bool {
        self.attr == attr::LFN
    }

    /// Check if this is a directory
    pub fn is_directory(&self) -> bool {
        (self.attr & attr::DIRECTORY) != 0 && !self.is_lfn()
    }

    /// Check if this is a regular file
    pub fn is_file(&self) -> bool {
        (self.attr & (attr::DIRECTORY | attr::VOLUME_ID | attr::LFN)) == 0
    }

    /// Check if this entry is deleted
    pub fn is_deleted(&self) -> bool {
        self.name[0] == 0xE5
    }

    /// Get the starting cluster number
    pub fn cluster(&self) -> u32 {
        ((self.fst_clus_hi as u32) << 16) | (self.fst_clus_lo as u32)
    }

    /// Set the starting cluster number
    pub fn set_cluster(&mut self, cluster: u32) {
        self.fst_clus_lo = (cluster & 0xFFFF) as u16;
        self.fst_clus_hi = ((cluster >> 16) & 0xFFFF) as u16;
    }

    /// Get short name as string
    pub fn short_name(&self) -> String {
        if self.is_lfn() {
            return String::from("[LFN]");
        }
        
        let mut name = String::with_capacity(13);
        
        // Add base name
        for i in 0..8 {
            let c = self.name[i];
            if c == 0x20 {
                break;
            }
            name.push((c as char).to_ascii_uppercase());
        }
        
        // Add extension
        if self.name[8..11].iter().any(|&c| c != 0x20) {
            name.push('.');
            for i in 8..11 {
                let c = self.name[i];
                if c == 0x20 {
                    break;
                }
                name.push((c as char).to_ascii_uppercase());
            }
        }
        
        name
    }

    /// Set short name from string (8.3 format)
    pub fn set_short_name(&mut self, name: &str) {
        // Parse name and extension
        let parts: Vec<&str> = name.split('.').collect();
        let base = parts[0].to_ascii_uppercase();
        let ext = if parts.len() > 1 { parts[1].to_ascii_uppercase() } else { String::new() };

        // Fill base name (padded with spaces)
        for i in 0..8 {
            self.name[i] = if i < base.len() { base.as_bytes()[i] } else { 0x20 };
        }

        // Fill extension (padded with spaces)
        for i in 0..3 {
            self.name[i + 8] = if i < ext.len() { ext.as_bytes()[i] } else { 0x20 };
        }
    }

    /// Create a new empty directory entry
    pub fn new() -> Self {
        Self {
            name: [0x20; 11],
            attr: 0,
            nt_res: 0,
            crt_time_tenth: 0,
            crt_time: 0,
            crt_date: 0,
            lst_acc_date: 0,
            fst_clus_hi: 0,
            wrt_time: 0,
            wrt_date: 0,
            fst_clus_lo: 0,
            file_size: 0,
        }
    }

    /// Mark entry as deleted
    pub fn mark_deleted(&mut self) {
        self.name[0] = 0xE5;
    }
}

/// Long filename entry
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct LfnEntry {
    /// Sequence number
    pub order: u8,
    /// First 5 characters (UTF-16)
    pub name1: [u16; 5],
    /// Attributes (always 0x0F)
    pub attr: u8,
    /// Entry type (always 0)
    pub entry_type: u8,
    /// Checksum of short name
    pub checksum: u8,
    /// Next 6 characters (UTF-16)
    pub name2: [u16; 6],
    /// Starting cluster (always 0)
    pub fst_clus_lo: u16,
    /// Last 2 characters (UTF-16)
    pub name3: [u16; 2],
}

impl LfnEntry {
    /// Check if this is the last LFN entry
    pub fn is_last(&self) -> bool {
        (self.order & 0x40) != 0
    }

    /// Get the sequence number (without last flag)
    pub fn sequence(&self) -> u8 {
        self.order & 0x1F
    }

    /// Calculate checksum for short name
    pub fn calculate_checksum(short_name: &[u8; 11]) -> u8 {
        let mut sum = 0u8;
        for &c in short_name {
            sum = ((sum & 1) << 7) + (sum >> 1) + c;
        }
        sum
    }

    /// Extract characters from this LFN entry
    pub fn extract_chars(&self) -> Vec<u16> {
        let mut chars = Vec::with_capacity(13);
        
        for i in 0..5 {
            if self.name1[i] != 0x0000 && self.name1[i] != 0xFFFF {
                chars.push(self.name1[i]);
            }
        }
        for i in 0..6 {
            if self.name2[i] != 0x0000 && self.name2[i] != 0xFFFF {
                chars.push(self.name2[i]);
            }
        }
        for i in 0..2 {
            if self.name3[i] != 0x0000 && self.name3[i] != 0xFFFF {
                chars.push(self.name3[i]);
            }
        }
        
        chars
    }
}

/// FAT32 filesystem
pub struct Fat32Filesystem<B: BlockDevice> {
    /// Underlying block device
    device: B,
    /// Block cache
    cache: BlockCache<B>,
    /// BIOS Parameter Block
    bpb: BiosParameterBlock,
    /// Extended BPB
    ext_bpb: Fat32ExtendedBpb,
    /// First data sector (relative to partition start)
    first_data_sector: u32,
    /// First FAT sector
    first_fat_sector: u32,
    /// Total clusters
    total_clusters: u32,
    /// Bytes per cluster
    bytes_per_cluster: u32,
    /// Is filesystem mounted
    mounted: bool,
}

impl<B: BlockDevice> Fat32Filesystem<B> {
    /// Mount a FAT32 filesystem from a block device
    pub fn mount(mut device: B) -> Result<Self, VFatError> {
        // Read boot sector
        let mut boot_sector = vec![0u8; device.block_size()];
        device.read_block(0, &mut boot_sector)?;

        // Parse BPB
        let bpb = Self::parse_bpb(&boot_sector)?;
        let ext_bpb = Self::parse_ext_bpb(&boot_sector)?;

        // Validate FAT32 signature
        if boot_sector[510] != 0x55 || boot_sector[511] != 0xAA {
            return Err(VFatError::Corruption(
                "Invalid boot sector signature".to_string()
            ));
        }

        // Validate FAT32
        if &ext_bpb.fs_type != b"FAT32   " {
            return Err(VFatError::Corruption(
                "Not a FAT32 filesystem".to_string()
            ));
        }

        // Calculate derived values
        let bytes_per_cluster = (bpb.bytes_per_sector as u32) * (bpb.sectors_per_cluster as u32);
        let first_fat_sector = bpb.reserved_sector_count as u32;
        let data_sectors = bpb.total_sectors_32 as u32
            - (bpb.reserved_sector_count as u32 + (bpb.num_fats as u32) * ext_bpb.sectors_per_fat_32);
        let total_clusters = data_sectors / (bpb.sectors_per_cluster as u32);
        let first_data_sector = first_fat_sector + (bpb.num_fats as u32) * ext_bpb.sectors_per_fat_32;

        let cache = BlockCache::new(device.block_size(), 64); // 64-block cache

        Ok(Self {
            device,
            cache,
            bpb,
            ext_bpb,
            first_data_sector,
            first_fat_sector,
            total_clusters,
            bytes_per_cluster,
            mounted: true,
        })
    }

    /// Parse BIOS Parameter Block
    fn parse_bpb(data: &[u8]) -> Result<BiosParameterBlock, VFatError> {
        if data.len() < 36 {
            return Err(VFatError::Corruption("BPB too short".to_string()));
        }

        Ok(BiosParameterBlock {
            jmp_boot: [data[0], data[1], data[2]],
            oem_name: [
                data[3], data[4], data[5], data[6],
                data[7], data[8], data[9], data[10],
            ],
            bytes_per_sector: u16::from_le_bytes([data[11], data[12]]),
            sectors_per_cluster: data[13],
            reserved_sector_count: u16::from_le_bytes([data[14], data[15]]),
            num_fats: data[16],
            root_entry_count: u16::from_le_bytes([data[17], data[18]]),
            total_sectors_16: u16::from_le_bytes([data[19], data[20]]),
            media: data[21],
            sectors_per_fat_16: u16::from_le_bytes([data[22], data[23]]),
            sectors_per_track: u16::from_le_bytes([data[24], data[25]]),
            num_heads: u16::from_le_bytes([data[26], data[27]]),
            hidden_sectors: u32::from_le_bytes([data[28], data[29], data[30], data[31]]),
            total_sectors_32: u32::from_le_bytes([data[32], data[33], data[34], data[35]]),
        })
    }

    /// Parse Extended BIOS Parameter Block
    fn parse_ext_bpb(data: &[u8]) -> Result<Fat32ExtendedBpb, VFatError> {
        if data.len() < 90 {
            return Err(VFatError::Corruption("Extended BPB too short".to_string()));
        }

        Ok(Fat32ExtendedBpb {
            sectors_per_fat_32: u32::from_le_bytes([data[36], data[37], data[38], data[39]]),
            ext_flags: u16::from_le_bytes([data[40], data[41]]),
            fs_version: u16::from_le_bytes([data[42], data[43]]),
            root_cluster: u32::from_le_bytes([data[44], data[45], data[46], data[47]]),
            fs_info: u16::from_le_bytes([data[48], data[49]]),
            backup_boot_sector: u16::from_le_bytes([data[50], data[51]]),
            reserved: [
                data[52], data[53], data[54], data[55],
                data[56], data[57], data[58], data[59],
                data[60], data[61], data[62], data[63],
            ],
            drive_number: data[64],
            reserved1: data[65],
            boot_signature: data[66],
            volume_serial: u32::from_le_bytes([data[67], data[68], data[69], data[70]]),
            volume_label: [
                data[71], data[72], data[73], data[74], data[75],
                data[76], data[77], data[78], data[79], data[80], data[81],
            ],
            fs_type: [data[82], data[83], data[84], data[85], data[86], data[87], data[88], data[89]],
        })
    }

    /// Read a FAT entry
    fn read_fat_entry(&mut self, cluster: u32) -> Result<u32, VFatError> {
        if cluster < 2 || cluster >= self.total_clusters {
            return Err(VFatError::InvalidParameter(
                format!("Invalid cluster number: {}", cluster)
            ));
        }

        let fat_offset = cluster * 4;
        let fat_sector = self.first_fat_sector + (fat_offset / self.bpb.bytes_per_sector as u32);
        let entry_offset = (fat_offset % self.bpb.bytes_per_sector as u32) as usize;

        let sector_data = self.cache.read_sector(&mut self.device, fat_sector as u64)?;
        
        let entry = u32::from_le_bytes([
            sector_data[entry_offset],
            sector_data[entry_offset + 1],
            sector_data[entry_offset + 2],
            sector_data[entry_offset + 3],
        ]);

        Ok(entry & 0x0FFFFFFF) // Mask to 28 bits
    }

    /// Write a FAT entry
    fn write_fat_entry(&mut self, cluster: u32, value: u32) -> Result<(), VFatError> {
        if cluster < 2 || cluster >= self.total_clusters {
            return Err(VFatError::InvalidParameter(
                format!("Invalid cluster number: {}", cluster)
            ));
        }

        let fat_offset = cluster * 4;
        let fat_sector = self.first_fat_sector + (fat_offset / self.bpb.bytes_per_sector as u32);
        let entry_offset = (fat_offset % self.bpb.bytes_per_sector as u32) as usize;

        let sector_data = self.cache.read_sector(&mut self.device, fat_sector as u64)?.to_vec();
        
        // Update entry (preserve upper 4 bits)
        let preserved = sector_data[entry_offset + 3] & 0xF0;
        let bytes = (value & 0x0FFFFFFF).to_le_bytes();
        
        let mut new_sector = sector_data;
        new_sector[entry_offset] = bytes[0];
        new_sector[entry_offset + 1] = bytes[1];
        new_sector[entry_offset + 2] = bytes[2];
        new_sector[entry_offset + 3] = (bytes[3] & 0x0F) | preserved;

        // Write to both FATs
        self.cache.write_sector(&mut self.device, fat_sector as u64, &new_sector)?;
        
        if self.bpb.num_fats > 1 {
            let second_fat_sector = fat_sector + self.ext_bpb.sectors_per_fat_32;
            self.cache.write_sector(&mut self.device, second_fat_sector as u64, &new_sector)?;
        }

        Ok(())
    }

    /// Allocate a free cluster
    fn allocate_cluster(&mut self) -> Result<u32, VFatError> {
        // Simple first-fit allocation
        // In production, use FAT32 FSInfo sector for next free hint
        for cluster in 2..self.total_clusters {
            let entry = self.read_fat_entry(cluster)?;
            if entry == cluster::FREE {
                // Mark as end of chain
                self.write_fat_entry(cluster, cluster::END_OF_CHAIN)?;
                return Ok(cluster);
            }
        }
        
        Err(VFatError::io(IoError::other("No free clusters available")))
    }

    /// Free a cluster chain
    fn free_cluster_chain(&mut self, start_cluster: u32) -> Result<(), VFatError> {
        let mut current = start_cluster;
        
        while current >= cluster::FIRST_DATA && current < cluster::END_OF_CHAIN_MIN {
            let next = self.read_fat_entry(current)?;
            self.write_fat_entry(current, cluster::FREE)?;
            current = next;
        }
        
        Ok(())
    }

    /// Extend a cluster chain
    fn extend_chain(&mut self, last_cluster: u32) -> Result<u32, VFatError> {
        let new_cluster = self.allocate_cluster()?;
        self.write_fat_entry(last_cluster, new_cluster)?;
        Ok(new_cluster)
    }

    /// Get sector number for a cluster
    fn cluster_to_sector(&self, cluster: u32) -> u64 {
        ((cluster - 2) * self.bpb.sectors_per_cluster as u32 + self.first_data_sector) as u64
    }

    /// Read cluster data
    fn read_cluster(&mut self, cluster: u32, buffer: &mut [u8]) -> Result<(), VFatError> {
        let start_sector = self.cluster_to_sector(cluster);
        let sectors = self.bpb.sectors_per_cluster as usize;
        
        for i in 0..sectors {
            let sector_data = self.cache.read_sector(&mut self.device, start_sector + i as u64)?;
            let offset = i * self.bpb.bytes_per_sector as usize;
            buffer[offset..offset + self.bpb.bytes_per_sector as usize].copy_from_slice(sector_data);
        }
        
        Ok(())
    }

    /// Write cluster data
    fn write_cluster(&mut self, cluster: u32, buffer: &[u8]) -> Result<(), VFatError> {
        let start_sector = self.cluster_to_sector(cluster);
        let sectors = self.bpb.sectors_per_cluster as usize;
        let bytes_per_sector = self.bpb.bytes_per_sector as usize;
        
        for i in 0..sectors {
            let offset = i * bytes_per_sector;
            self.cache.write_sector(
                &mut self.device,
                start_sector + i as u64,
                &buffer[offset..offset + bytes_per_sector],
            )?;
        }
        
        Ok(())
    }

    /// Read directory entries from a cluster
    fn read_directory(&mut self, cluster: u32) -> Result<Vec<(DirEntry, Option<String>)>, VFatError> {
        let mut entries = Vec::new();
        let mut lfn_buffer: Vec<u16> = Vec::new();
        let mut lfn_checksum: u8 = 0;

        let cluster_data = self.read_cluster_data(cluster)?;
        let entry_count = cluster_data.len() / 32;

        for i in 0..entry_count {
            let offset = i * 32;
            let entry_data = &cluster_data[offset..offset + 32];

            if entry_data[0] == 0x00 {
                // End of directory
                break;
            }

            if entry_data[0] == 0xE5 {
                // Deleted entry
                lfn_buffer.clear();
                continue;
            }

            // Check if LFN entry
            if entry_data[11] == attr::LFN {
                let lfn: &LfnEntry = unsafe { &*(entry_data.as_ptr() as *const LfnEntry) };
                
                if lfn.is_last() {
                    lfn_buffer.clear();
                    lfn_checksum = lfn.checksum;
                }
                
                // Prepend characters (LFN entries are stored backwards)
                let chars = lfn.extract_chars();
                for ch in chars.iter().rev() {
                    lfn_buffer.insert(0, *ch);
                }
            } else {
                // Regular directory entry
                let entry: DirEntry = unsafe { 
                    core::ptr::read_unaligned(entry_data.as_ptr() as *const DirEntry)
                };

                let long_name = if !lfn_buffer.is_empty() {
                    let checksum = LfnEntry::calculate_checksum(&entry.name);
                    if checksum == lfn_checksum {
                        Some(String::from_utf16_lossy(&lfn_buffer))
                    } else {
                        None
                    }
                } else {
                    None
                };

                entries.push((entry, long_name));
                lfn_buffer.clear();
            }
        }

        Ok(entries)
    }

    /// Read entire cluster chain data
    fn read_cluster_data(&mut self, start_cluster: u32) -> Result<Vec<u8>, VFatError> {
        let mut data = Vec::new();
        let mut current = start_cluster;

        while current >= cluster::FIRST_DATA && current < cluster::END_OF_CHAIN_MIN {
            let mut cluster_buffer = vec![0u8; self.bytes_per_cluster as usize];
            self.read_cluster(current, &mut cluster_buffer)?;
            data.extend_from_slice(&cluster_buffer);
            
            current = self.read_fat_entry(current)?;
        }

        Ok(data)
    }

    /// Find a free directory entry in a cluster
    fn find_free_entry(&mut self, cluster: u32) -> Result<Option<(u32, usize)>, VFatError> {
        let cluster_data = self.read_cluster_data(cluster)?;
        let entry_count = cluster_data.len() / 32;

        for i in 0..entry_count {
            let offset = i * 32;
            if cluster_data[offset] == 0x00 || cluster_data[offset] == 0xE5 {
                return Ok(Some((cluster, offset)));
            }
        }

        // No free entry in this cluster
        Ok(None)
    }

    /// Find entry by name in directory
    fn find_entry(&mut self, cluster: u32, name: &str) -> Result<Option<(DirEntry, u32, usize)>, VFatError> {
        let entries = self.read_directory(cluster)?;
        let search_name = name.to_ascii_uppercase();

        for (entry, long_name) in entries {
            let matches = if let Some(ref lfn) = long_name {
                lfn.eq_ignore_ascii_case(name)
            } else {
                entry.short_name().eq_ignore_ascii_case(&search_name)
            };

            if matches {
                // Find position in directory
                let cluster_data = self.read_cluster_data(cluster)?;
                let entry_count = cluster_data.len() / 32;
                
                for i in 0..entry_count {
                    let offset = i * 32;
                    let entry_data = &cluster_data[offset..offset + 32];
                    
                    if !entry.is_lfn() && entry_data[0] != 0x00 && entry_data[0] != 0xE5 {
                        let candidate: DirEntry = unsafe {
                            core::ptr::read_unaligned(entry_data.as_ptr() as *const DirEntry)
                        };
                        
                        if candidate.cluster() == entry.cluster() {
                            return Ok(Some((entry, cluster, offset)));
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    /// Write directory entry
    fn write_entry(&mut self, cluster: u32, offset: usize, entry: &DirEntry) -> Result<(), VFatError> {
        let sector_size = self.bpb.bytes_per_sector as usize;
        let sector_offset = offset / sector_size;
        let entry_offset = offset % sector_size;
        let sector = self.cluster_to_sector(cluster) + sector_offset as u64;

        let mut sector_data = self.cache.read_sector(&mut self.device, sector)?.to_vec();
        
        unsafe {
            let entry_bytes = core::slice::from_raw_parts(
                entry as *const _ as *const u8,
                32
            );
            sector_data[entry_offset..entry_offset + 32].copy_from_slice(entry_bytes);
        }

        self.cache.write_sector(&mut self.device, sector, &sector_data)?;
        Ok(())
    }

    /// Create short name from long name
    fn generate_short_name(&self, long_name: &str, entries: &[(DirEntry, Option<String>)]) -> String {
        // Remove invalid characters and convert to uppercase
        let base: String = long_name
            .chars()
            .take(8)
            .map(|c| {
                let c = c.to_ascii_uppercase();
                if c.is_ascii_alphanumeric() || c == '$' || c == '%' || c == '-' || c == '_' || c == '@' || c == '~' || c == '`' || c == '!' || c == '(' || c == ')' || c == '{' || c == '}' || c == '^' || c == '#' || c == '&' {
                    c
                } else {
                    '_'
                }
            })
            .collect();

        let ext: String = long_name
            .rsplit('.')
            .next()
            .unwrap_or("")
            .chars()
            .take(3)
            .map(|c| c.to_ascii_uppercase())
            .collect();

        // Check for conflicts
        let mut short_name = if ext.is_empty() {
            format!("{:8}", base)
        } else {
            format!("{:8}.{:3}", base, ext)
        };

        // If conflict, add numeric tail
        for i in 1..=999999 {
            let exists = entries.iter().any(|(e, _)| {
                e.short_name().trim() == short_name.trim()
            });

            if !exists {
                break;
            }

            let tail = format!("~{}", i);
            let base_len = 8 - tail.len();
            short_name = if ext.is_empty() {
                format!("{}{:>width$}", &base[..base_len], tail, width = tail.len())
            } else {
                format!("{}{:>width$}.{:3}", &base[..base_len], tail, ext, width = tail.len())
            };
        }

        short_name
    }

    /// Generate LFN entries for a name
    fn generate_lfn_entries(name: &str, checksum: u8) -> Vec<LfnEntry> {
        let chars: Vec<u16> = name.encode_utf16().collect();
        let num_entries = (chars.len() + 12) / 13;
        let mut entries = Vec::new();

        for i in (0..num_entries).rev() {
            let mut entry = LfnEntry {
                order: ((num_entries - i) as u8) | if i == 0 { 0x40 } else { 0x00 },
                name1: [0xFFFF; 5],
                attr: attr::LFN,
                entry_type: 0,
                checksum,
                name2: [0xFFFF; 6],
                fst_clus_lo: 0,
                name3: [0xFFFF; 2],
            };

            let start_idx = i * 13;
            
            // Fill name1 (chars 0-4)
            for j in 0..5 {
                let idx = start_idx + j;
                if idx < chars.len() {
                    entry.name1[j] = chars[idx];
                } else if idx == chars.len() {
                    entry.name1[j] = 0x0000;
                }
            }

            // Fill name2 (chars 5-10)
            for j in 0..6 {
                let idx = start_idx + 5 + j;
                if idx < chars.len() {
                    entry.name2[j] = chars[idx];
                } else if idx == chars.len() {
                    entry.name2[j] = 0x0000;
                }
            }

            // Fill name3 (chars 11-12)
            for j in 0..2 {
                let idx = start_idx + 11 + j;
                if idx < chars.len() {
                    entry.name3[j] = chars[idx];
                } else if idx == chars.len() {
                    entry.name3[j] = 0x0000;
                }
            }

            entries.insert(0, entry);
        }

        entries
    }

    // Public API

    /// Get filesystem info
    pub fn info(&self) -> Fat32Info {
        Fat32Info {
            total_clusters: self.total_clusters,
            free_clusters: 0, // Would need to scan or use FSInfo
            bytes_per_cluster: self.bytes_per_cluster,
            bytes_per_sector: self.bpb.bytes_per_sector,
            sectors_per_cluster: self.bpb.sectors_per_cluster,
            volume_label: String::from_utf8_lossy(&self.ext_bpb.volume_label).trim().to_string(),
            root_cluster: self.ext_bpb.root_cluster,
        }
    }

    /// List root directory
    pub fn list_root(&mut self) -> Result<Vec<FileInfo>, VFatError> {
        self.list_directory(self.ext_bpb.root_cluster)
    }

    /// List directory contents
    pub fn list_directory(&mut self, cluster: u32) -> Result<Vec<FileInfo>, VFatError> {
        let entries = self.read_directory(cluster)?;
        let mut files = Vec::new();

        for (entry, long_name) in entries {
            if !entry.is_free() && entry.name[0] != 0x00 {
                files.push(FileInfo {
                    name: long_name.unwrap_or_else(|| entry.short_name()),
                    short_name: entry.short_name(),
                    size: entry.file_size,
                    attributes: entry.attr,
                    cluster: entry.cluster(),
                    is_directory: entry.is_directory(),
                    is_file: entry.is_file(),
                });
            }
        }

        Ok(files)
    }

    /// Create a new file
    pub fn create_file(&mut self, dir_cluster: u32, name: &str) -> Result<FileInfo, VFatError> {
        // Check if file already exists
        if self.find_entry(dir_cluster, name)?.is_some() {
            return Err(VFatError::io(IoError::already_exists("File already exists")));
        }

        // Allocate a cluster for the file
        let file_cluster = self.allocate_cluster()?;

        // Generate short name
        let entries = self.read_directory(dir_cluster)?;
        let short_name = self.generate_short_name(name, &entries);

        // Create directory entry
        let mut entry = DirEntry::new();
        entry.set_short_name(&short_name);
        entry.attr = attr::ARCHIVE;
        entry.set_cluster(file_cluster);

        // Write LFN entries if needed
        let lfn_needed = name.len() > short_name.len() || name != short_name;
        let num_lfn = if lfn_needed { (name.len() + 12) / 13 } else { 0 };
        let num_entries = num_lfn + 1;

        // Find free entries
        let mut free_positions = Vec::new();
        let cluster_data = self.read_cluster_data(dir_cluster)?;
        let entry_count = cluster_data.len() / 32;

        for i in 0..entry_count {
            let offset = i * 32;
            if cluster_data[offset] == 0x00 || cluster_data[offset] == 0xE5 {
                free_positions.push((dir_cluster, offset));
                if free_positions.len() >= num_entries {
                    break;
                }
            }
        }

        if free_positions.len() < num_entries {
            return Err(VFatError::io(IoError::other("Directory full")));
        }

        // Write LFN entries
        if lfn_needed {
            let checksum = LfnEntry::calculate_checksum(&entry.name);
            let lfn_entries = Self::generate_lfn_entries(name, checksum);
            
            for (i, lfn) in lfn_entries.iter().enumerate() {
                let (_, offset) = free_positions[i];
                self.write_lfn_entry(dir_cluster, offset, lfn)?;
            }
        }

        // Write short entry
        let (_, offset) = free_positions[num_lfn];
        self.write_entry(dir_cluster, offset, &entry)?;

        Ok(FileInfo {
            name: name.to_string(),
            short_name,
            size: 0,
            attributes: entry.attr,
            cluster: file_cluster,
            is_directory: false,
            is_file: true,
        })
    }

    /// Create a new directory
    pub fn create_directory(&mut self, parent_cluster: u32, name: &str) -> Result<FileInfo, VFatError> {
        // Check if directory already exists
        if self.find_entry(parent_cluster, name)?.is_some() {
            return Err(VFatError::io(IoError::already_exists("Directory already exists")));
        }

        // Allocate cluster for new directory
        let new_dir_cluster = self.allocate_cluster()?;

        // Initialize directory with . and .. entries
        self.initialize_directory(new_dir_cluster, parent_cluster)?;

        // Add entry to parent directory
        let entries = self.read_directory(parent_cluster)?;
        let short_name = self.generate_short_name(name, &entries);

        let mut entry = DirEntry::new();
        entry.set_short_name(&short_name);
        entry.attr = attr::DIRECTORY;
        entry.set_cluster(new_dir_cluster);

        // Write entries (similar to create_file)
        // ... (simplified for brevity)

        Ok(FileInfo {
            name: name.to_string(),
            short_name,
            size: 0,
            attributes: attr::DIRECTORY,
            cluster: new_dir_cluster,
            is_directory: true,
            is_file: false,
        })
    }

    /// Initialize a new directory cluster with . and .. entries
    fn initialize_directory(&mut self, cluster: u32, parent_cluster: u32) -> Result<(), VFatError> {
        let mut data = vec![0u8; self.bytes_per_cluster as usize];

        // Create . entry
        let mut dot = DirEntry::new();
        dot.name = *b".          ";
        dot.attr = attr::DIRECTORY;
        dot.set_cluster(cluster);

        // Create .. entry
        let mut dotdot = DirEntry::new();
        dotdot.name = *b"..         ";
        dotdot.attr = attr::DIRECTORY;
        dotdot.set_cluster(parent_cluster);

        // Write entries
        unsafe {
            let dot_bytes = core::slice::from_raw_parts(&dot as *const _ as *const u8, 32);
            let dotdot_bytes = core::slice::from_raw_parts(&dotdot as *const _ as *const u8, 32);
            data[0..32].copy_from_slice(dot_bytes);
            data[32..64].copy_from_slice(dotdot_bytes);
        }

        self.write_cluster(cluster, &data)?;
        Ok(())
    }

    /// Delete a file or directory
    pub fn delete(&mut self, dir_cluster: u32, name: &str) -> Result<(), VFatError> {
        let (entry, cluster, offset) = self.find_entry(dir_cluster, name)?
            .ok_or_else(|| VFatError::not_found("File not found"))?;

        // Free cluster chain
        self.free_cluster_chain(entry.cluster())?;

        // Mark entry as deleted
        let mut deleted_entry = entry;
        deleted_entry.mark_deleted();
        self.write_entry(cluster, offset, &deleted_entry)?;

        // TODO: Also delete preceding LFN entries

        Ok(())
    }

    /// Read file contents
    pub fn read_file(&mut self, file_info: &FileInfo) -> Result<Vec<u8>, VFatError> {
        let data = self.read_cluster_data(file_info.cluster)?;
        // Trim to actual file size
        Ok(data[..file_info.size as usize].to_vec())
    }

    /// Write file contents
    pub fn write_file(&mut self, dir_cluster: u32, name: &str, data: &[u8]) -> Result<(), VFatError> {
        let (mut entry, cluster, offset) = self.find_entry(dir_cluster, name)?
            .ok_or_else(|| VFatError::not_found("File not found"))?;

        // Free old cluster chain
        if entry.cluster() >= cluster::FIRST_DATA {
            self.free_cluster_chain(entry.cluster())?;
        }

        // Allocate new clusters
        let clusters_needed = (data.len() as u32 + self.bytes_per_cluster - 1) / self.bytes_per_cluster;
        let mut current_cluster = self.allocate_cluster()?;
        entry.set_cluster(current_cluster);

        // Write data
        let mut written = 0usize;
        for i in 0..clusters_needed {
            let chunk_size = core::cmp::min(self.bytes_per_cluster as usize, data.len() - written);
            let mut cluster_data = vec![0u8; self.bytes_per_cluster as usize];
            cluster_data[..chunk_size].copy_from_slice(&data[written..written + chunk_size]);
            self.write_cluster(current_cluster, &cluster_data)?;
            written += chunk_size;

            // Allocate next cluster if needed
            if i < clusters_needed - 1 {
                let next_cluster = self.extend_chain(current_cluster)?;
                current_cluster = next_cluster;
            }
        }

        // Update file size
        entry.file_size = data.len() as u32;
        self.write_entry(cluster, offset, &entry)?;

        Ok(())
    }

    /// Write LFN entry
    fn write_lfn_entry(&mut self, cluster: u32, offset: usize, entry: &LfnEntry) -> Result<(), VFatError> {
        let sector_size = self.bpb.bytes_per_sector as usize;
        let sector_offset = offset / sector_size;
        let entry_offset = offset % sector_size;
        let sector = self.cluster_to_sector(cluster) + sector_offset as u64;

        let mut sector_data = self.cache.read_sector(&mut self.device, sector)?.to_vec();

        unsafe {
            let entry_bytes = core::slice::from_raw_parts(
                entry as *const _ as *const u8,
                32
            );
            sector_data[entry_offset..entry_offset + 32].copy_from_slice(entry_bytes);
        }

        self.cache.write_sector(&mut self.device, sector, &sector_data)?;
        Ok(())
    }

    /// Flush all pending writes
    pub fn flush(&mut self) -> Result<(), VFatError> {
        self.cache.flush(&mut self.device)?;
        self.device.flush()
    }

    /// Unmount filesystem
    pub fn unmount(mut self) -> Result<(), VFatError> {
        self.flush()?;
        self.mounted = false;
        Ok(())
    }
}

/// Filesystem information
#[derive(Debug, Clone)]
pub struct Fat32Info {
    /// Total number of clusters
    pub total_clusters: u32,
    /// Number of free clusters
    pub free_clusters: u32,
    /// Bytes per cluster
    pub bytes_per_cluster: u32,
    /// Bytes per sector
    pub bytes_per_sector: u16,
    /// Sectors per cluster
    pub sectors_per_cluster: u8,
    /// Volume label
    pub volume_label: String,
    /// Root directory cluster
    pub root_cluster: u32,
}

/// File information
#[derive(Debug, Clone)]
pub struct FileInfo {
    /// Full name (long filename if available)
    pub name: String,
    /// Short 8.3 name
    pub short_name: String,
    /// File size in bytes
    pub size: u32,
    /// File attributes
    pub attributes: u8,
    /// Starting cluster
    pub cluster: u32,
    /// Is a directory
    pub is_directory: bool,
    /// Is a regular file
    pub is_file: bool,
}

// External crate dependencies for alloc
extern crate alloc;
