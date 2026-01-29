//! FAT32 Filesystem
//!
//! Implementation of the FAT32 filesystem.

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use alloc::boxed::Box;

use crate::fs::{FileSystem, FileType, Metadata, Permissions, INode, FsResult, FsError};
use crate::storage::BlockDevice;
use crate::println;

/// FAT32 boot sector
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BootSector {
    pub jmp: [u8; 3],
    pub oem: [u8; 8],
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub reserved_sectors: u16,
    pub fat_count: u8,
    pub root_entries: u16,
    pub total_sectors_16: u16,
    pub media_type: u8,
    pub sectors_per_fat_16: u16,
    pub sectors_per_track: u16,
    pub head_count: u16,
    pub hidden_sectors: u32,
    pub total_sectors_32: u32,
    // FAT32 specific fields
    pub sectors_per_fat_32: u32,
    pub ext_flags: u16,
    pub fs_version: u16,
    pub root_cluster: u32,
    pub fs_info_sector: u16,
    pub backup_boot_sector: u16,
    pub reserved: [u8; 12],
    pub drive_num: u8,
    pub reserved1: u8,
    pub boot_sig: u8,
    pub volume_id: u32,
    pub volume_label: [u8; 11],
    pub fs_type: [u8; 8],
}

/// Directory entry (32 bytes)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DirEntry {
    pub name: [u8; 11],
    pub attrs: u8,
    pub reserved: u8,
    pub create_time_tenths: u8,
    pub create_time: u16,
    pub create_date: u16,
    pub access_date: u16,
    pub cluster_high: u16,
    pub modify_time: u16,
    pub modify_date: u16,
    pub cluster_low: u16,
    pub size: u32,
}

/// Long file name entry
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LfnEntry {
    pub order: u8,
    pub name1: [u16; 5],
    pub attrs: u8,
    pub entry_type: u8,
    pub checksum: u8,
    pub name2: [u16; 6],
    pub reserved: u16,
    pub name3: [u16; 2],
}

/// File attributes
const ATTR_READ_ONLY: u8 = 0x01;
const ATTR_HIDDEN: u8 = 0x02;
const ATTR_SYSTEM: u8 = 0x04;
const ATTR_VOLUME_ID: u8 = 0x08;
const ATTR_DIRECTORY: u8 = 0x10;
const ATTR_ARCHIVE: u8 = 0x20;
const ATTR_LFN: u8 = 0x0F;

/// FAT special values
const FAT_ENTRY_FREE: u32 = 0x00000000;
const FAT_ENTRY_RESERVED: u32 = 0x00000001;
const FAT_ENTRY_MIN: u32 = 0x00000002;
const FAT_ENTRY_MAX: u32 = 0x0FFFFFF6;
const FAT_ENTRY_BAD: u32 = 0x0FFFFFF7;
const FAT_ENTRY_EOF: u32 = 0x0FFFFFFF;

/// FAT32 filesystem instance
pub struct Fat32Fs {
    device: Box<dyn BlockDevice>,
    boot_sector: BootSector,
    bytes_per_sector: u16,
    sectors_per_cluster: u8,
    bytes_per_cluster: u32,
    reserved_sectors: u16,
    fat_count: u8,
    sectors_per_fat: u32,
    root_cluster: u32,
    data_start_sector: u32,
    fat: Vec<u32>,
}

impl Fat32Fs {
    /// Create new FAT32 filesystem from block device
    pub fn new(device: Box<dyn BlockDevice>) -> FsResult<Self> {
        // Read boot sector
        let mut boot_data = [0u8; 512];
        device.read_blocks(0, 1, &mut boot_data)
            .map_err(|_| FsError::IoError)?;

        let boot_sector = unsafe {
            core::ptr::read(boot_data.as_ptr() as *const BootSector)
        };

        // Verify FAT32 signature
        if boot_sector.boot_sig != 0x29 {
            return Err(FsError::InvalidFilesystem);
        }

        let bytes_per_sector = boot_sector.bytes_per_sector;
        let sectors_per_cluster = boot_sector.sectors_per_cluster;
        let bytes_per_cluster = (bytes_per_sector as u32) * (sectors_per_cluster as u32);
        
        // Calculate FAT size
        let sectors_per_fat = boot_sector.sectors_per_fat_32;
        
        // Calculate data start sector
        let data_start_sector = boot_sector.reserved_sectors as u32 + 
                               (boot_sector.fat_count as u32 * sectors_per_fat);

        println!("[fat32] Mounting FAT32 filesystem");
        println!("  Volume: {}", 
            core::str::from_utf8(&boot_sector.volume_label).unwrap_or("Unknown").trim());
        println!("  Bytes per sector: {}", bytes_per_sector);
        println!("  Sectors per cluster: {}", sectors_per_cluster);
        println!("  Total sectors: {}", 
            if boot_sector.total_sectors_32 != 0 { 
                boot_sector.total_sectors_32 
            } else { 
                boot_sector.total_sectors_16 as u32 
            });
        println!("  Root cluster: {}", boot_sector.root_cluster);

        // Read FAT into memory
        let fat_entries = (sectors_per_fat as usize * bytes_per_sector as usize) / 4;
        let mut fat = Vec::with_capacity(fat_entries);
        
        let mut fat_buffer = vec![0u8; (sectors_per_fat as usize * bytes_per_sector as usize)];
        let fat_start = boot_sector.reserved_sectors as u64;
        device.read_blocks(fat_start, sectors_per_fat as usize, &mut fat_buffer)
            .map_err(|_| FsError::IoError)?;

        for i in 0..fat_entries {
            let entry = unsafe {
                core::ptr::read_unaligned(fat_buffer.as_ptr().add(i * 4) as *const u32)
            } & 0x0FFFFFFF;
            fat.push(entry);
        }

        Ok(Self {
            device,
            boot_sector,
            bytes_per_sector,
            sectors_per_cluster,
            bytes_per_cluster,
            reserved_sectors: boot_sector.reserved_sectors,
            fat_count: boot_sector.fat_count,
            sectors_per_fat,
            root_cluster: boot_sector.root_cluster,
            data_start_sector,
            fat,
        })
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

    /// Get next cluster from FAT
    fn next_cluster(&self, cluster: u32) -> Option<u32> {
        let entry = self.fat.get(cluster as usize)?;
        
        if *entry >= FAT_ENTRY_MIN && *entry <= FAT_ENTRY_MAX {
            Some(*entry)
        } else {
            None
        }
    }

    /// Read file data from clusters
    fn read_clusters(&self, start_cluster: u32, offset: u64, buf: &mut [u8]) -> FsResult<usize> {
        let mut current_cluster = start_cluster;
        let mut cluster_offset = (offset / self.bytes_per_cluster as u64) as u32;
        let mut byte_offset = (offset % self.bytes_per_cluster as u64) as usize;
        let mut bytes_read = 0;
        let mut buf_offset = 0;
        
        // Skip to the right cluster
        for _ in 0..cluster_offset {
            match self.next_cluster(current_cluster) {
                Some(next) => current_cluster = next,
                None => return Ok(0),
            }
        }

        let mut cluster_data = vec![0u8; self.bytes_per_cluster as usize];
        
        // Read first (possibly partial) cluster
        if byte_offset > 0 || buf.len() < self.bytes_per_cluster as usize {
            self.read_cluster(current_cluster, &mut cluster_data)?;
            let to_copy = buf.len().min(cluster_data.len() - byte_offset);
            buf[..to_copy].copy_from_slice(&cluster_data[byte_offset..byte_offset + to_copy]);
            bytes_read += to_copy;
            buf_offset += to_copy;
            
            // Move to next cluster
            match self.next_cluster(current_cluster) {
                Some(next) => current_cluster = next,
                None => return Ok(bytes_read),
            }
        }

        // Read full clusters
        while buf_offset + self.bytes_per_cluster as usize <= buf.len() {
            self.read_cluster(current_cluster, &mut buf[buf_offset..buf_offset + self.bytes_per_cluster as usize])?;
            bytes_read += self.bytes_per_cluster as usize;
            buf_offset += self.bytes_per_cluster as usize;
            
            match self.next_cluster(current_cluster) {
                Some(next) => current_cluster = next,
                None => return Ok(bytes_read),
            }
        }

        // Read last (possibly partial) cluster
        let remaining = buf.len() - buf_offset;
        if remaining > 0 {
            self.read_cluster(current_cluster, &mut cluster_data)?;
            buf[buf_offset..].copy_from_slice(&cluster_data[..remaining]);
            bytes_read += remaining;
        }

        Ok(bytes_read)
    }

    /// Parse directory entries
    fn read_dir_entries(&self, cluster: u32) -> FsResult<Vec<(String, DirEntry)>> {
        let mut entries = Vec::new();
        let mut cluster_data = vec![0u8; self.bytes_per_cluster as usize];
        let mut current_cluster = cluster;
        let mut lfn_buffer: Vec<u16> = Vec::new();

        loop {
            self.read_cluster(current_cluster, &mut cluster_data)?;

            let entry_count = cluster_data.len() / 32;
            for i in 0..entry_count {
                let entry_offset = i * 32;
                let first_byte = cluster_data[entry_offset];

                // End of directory
                if first_byte == 0x00 {
                    return Ok(entries);
                }

                // Deleted entry
                if first_byte == 0xE5 {
                    lfn_buffer.clear();
                    continue;
                }

                let attrs = cluster_data[entry_offset + 11];

                // Long file name entry
                if attrs == ATTR_LFN {
                    let lfn: &LfnEntry = unsafe {
                        &*(cluster_data.as_ptr().add(entry_offset) as *const LfnEntry)
                    };
                    
                    // Extract name parts
                    if lfn.order & 0x40 != 0 {
                        lfn_buffer.clear();
                    }
                    
                    for j in (0..5).rev() {
                        if lfn.name1[j] != 0 && lfn.name1[j] != 0xFFFF {
                            lfn_buffer.insert(0, lfn.name1[j]);
                        }
                    }
                    for j in (0..6).rev() {
                        if lfn.name2[j] != 0 && lfn.name2[j] != 0xFFFF {
                            lfn_buffer.insert(0, lfn.name2[j]);
                        }
                    }
                    for j in (0..2).rev() {
                        if lfn.name3[j] != 0 && lfn.name3[j] != 0xFFFF {
                            lfn_buffer.insert(0, lfn.name3[j]);
                        }
                    }
                    
                    continue;
                }

                // Regular 8.3 entry
                let entry = unsafe {
                    *(cluster_data.as_ptr().add(entry_offset) as *const DirEntry)
                };

                // Skip volume label and special entries
                if attrs & ATTR_VOLUME_ID != 0 {
                    lfn_buffer.clear();
                    continue;
                }

                // Get filename
                let name = if !lfn_buffer.is_empty() {
                    // Convert UTF-16 to String
                    let mut name = String::new();
                    for c in &lfn_buffer {
                        if *c < 0x80 {
                            name.push(*c as u8 as char);
                        } else {
                            name.push('?');
                        }
                    }
                    lfn_buffer.clear();
                    name
                } else {
                    // 8.3 format
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
                };

                if !name.is_empty() && name != "." && name != ".." {
                    entries.push((name, entry));
                }
                
                lfn_buffer.clear();
            }

            // Next cluster
            match self.next_cluster(current_cluster) {
                Some(next) => current_cluster = next,
                None => break,
            }
        }

        Ok(entries)
    }

    /// Find entry in directory
    fn find_entry(&self, cluster: u32, name: &str) -> FsResult<DirEntry> {
        let entries = self.read_dir_entries(cluster)?;
        
        let lower_name = name.to_ascii_lowercase();
        
        for (entry_name, entry) in entries {
            if entry_name.to_ascii_lowercase() == lower_name {
                return Ok(entry);
            }
        }

        Err(FsError::NotFound)
    }

    /// Lookup path
    fn lookup(&self, path: &str) -> FsResult<DirEntry> {
        let components: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        let num_components = components.len();
        
        let mut current_cluster = self.root_cluster;

        for (idx, component) in components.iter().enumerate() {
            let entry = self.find_entry(current_cluster, component)?;
            
            if entry.attrs & ATTR_DIRECTORY != 0 {
                current_cluster = ((entry.cluster_high as u32) << 16) | (entry.cluster_low as u32);
            } else {
                // Last component must be the file
                if idx == num_components - 1 {
                    return Ok(entry);
                }
                return Err(FsError::NotDirectory);
            }
        }

        // Return directory entry for root or last directory
        Ok(DirEntry {
            name: [b' '; 11],
            attrs: ATTR_DIRECTORY,
            reserved: 0,
            create_time_tenths: 0,
            create_time: 0,
            create_date: 0,
            access_date: 0,
            cluster_high: ((current_cluster >> 16) & 0xFFFF) as u16,
            modify_time: 0,
            modify_date: 0,
            cluster_low: (current_cluster & 0xFFFF) as u16,
            size: 0,
        })
    }

    /// Entry to cluster number
    fn entry_to_cluster(entry: &DirEntry) -> u32 {
        ((entry.cluster_high as u32) << 16) | (entry.cluster_low as u32)
    }

    /// Convert FAT attributes to FileType
    fn attrs_to_file_type(attrs: u8) -> FileType {
        if attrs & ATTR_DIRECTORY != 0 {
            FileType::Directory
        } else {
            FileType::Regular
        }
    }
}

impl FileSystem for Fat32Fs {
    fn name(&self) -> &str {
        "fat32"
    }

    fn root(&self) -> INode {
        INode::new(self.root_cluster as u64)
    }

    fn read_metadata(&self, inode: INode) -> FsResult<Metadata> {
        // For FAT32, inode is the cluster number
        // We need to find a directory entry to get metadata
        // For root, use defaults
        
        let is_root = inode.as_u64() == self.root_cluster as u64;
        
        Ok(Metadata {
            file_type: if is_root { FileType::Directory } else { FileType::Regular },
            size: if is_root { 0 } else { self.bytes_per_cluster as u64 },
            permissions: Permissions {
                owner_read: true,
                owner_write: true,
                owner_execute: true,
                group_read: true,
                group_write: true,
                group_execute: true,
                other_read: true,
                other_write: true,
                other_execute: true,
            },
            created: 0,
            modified: 0,
            accessed: 0,
            uid: 0,
            gid: 0,
            nlink: 1,
            block_size: self.bytes_per_cluster,
            blocks: if is_root { 0 } else { 1 },
        })
    }

    fn write_metadata(&self, _inode: INode, _metadata: &Metadata) -> FsResult<()> {
        Err(FsError::ReadOnly)
    }

    fn read(&self, inode: INode, offset: u64, buf: &mut [u8]) -> FsResult<usize> {
        let cluster = inode.as_u64() as u32;
        self.read_clusters(cluster, offset, buf)
    }

    fn write(&self, _inode: INode, _offset: u64, _buf: &[u8]) -> FsResult<usize> {
        Err(FsError::ReadOnly)
    }

    fn lookup(&self, parent: INode, name: &str) -> FsResult<INode> {
        let parent_cluster = parent.as_u64() as u32;
        let entry = self.find_entry(parent_cluster, name)?;
        let cluster = Self::entry_to_cluster(&entry);
        Ok(INode::new(cluster as u64))
    }

    fn create(&self, _parent: INode, _name: &str, _file_type: FileType) -> FsResult<INode> {
        Err(FsError::ReadOnly)
    }

    fn remove(&self, _parent: INode, _name: &str) -> FsResult<()> {
        Err(FsError::ReadOnly)
    }

    fn read_dir(&self, inode: INode) -> FsResult<Vec<(String, INode)>> {
        let cluster = inode.as_u64() as u32;
        let entries = self.read_dir_entries(cluster)?;
        
        let mut result = Vec::with_capacity(entries.len());
        for (name, entry) in entries {
            let entry_cluster = Self::entry_to_cluster(&entry);
            result.push((name, INode::new(entry_cluster as u64)));
        }
        
        Ok(result)
    }
}

/// Mount FAT32 filesystem
pub fn mount(device: Box<dyn BlockDevice>) -> FsResult<Box<dyn FileSystem>> {
    let fs = Fat32Fs::new(device)?;
    Ok(Box::new(fs))
}

/// Initialize FAT32 filesystem driver
pub fn init() {
    println!("[fat32] FAT32 filesystem driver initialized");
}
