//! EXT2 Filesystem
//!
//! Implementation of the Second Extended Filesystem.

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use alloc::boxed::Box;
use spin::Mutex;

use crate::fs::{FileSystem, FileType, Metadata, Permissions, INode, FsResult, FsError};
use crate::storage::{BlockDevice, StorageError};
use crate::println;

/// EXT2 superblock (located at offset 1024)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Superblock {
    pub inodes_count: u32,
    pub blocks_count: u32,
    pub r_blocks_count: u32,
    pub free_blocks_count: u32,
    pub free_inodes_count: u32,
    pub first_data_block: u32,
    pub log_block_size: u32,
    pub log_frag_size: u32,
    pub blocks_per_group: u32,
    pub frags_per_group: u32,
    pub inodes_per_group: u32,
    pub mtime: u32,
    pub wtime: u32,
    pub mnt_count: u16,
    pub max_mnt_count: u16,
    pub magic: u16,
    pub state: u16,
    pub errors: u16,
    pub minor_rev_level: u16,
    pub lastcheck: u32,
    pub checkinterval: u32,
    pub creator_os: u32,
    pub rev_level: u32,
    pub def_resuid: u16,
    pub def_resgid: u16,
    // Extended fields for rev_level >= 1
    pub first_ino: u32,
    pub inode_size: u16,
    pub block_group_nr: u16,
    pub feature_compat: u32,
    pub feature_incompat: u32,
    pub feature_ro_compat: u32,
    pub uuid: [u8; 16],
    pub volume_name: [u8; 16],
    pub last_mounted: [u8; 64],
    pub algo_bitmap: u32,
}

/// EXT2 magic number
const EXT2_MAGIC: u16 = 0xEF53;

/// Block group descriptor
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GroupDescriptor {
    pub block_bitmap: u32,
    pub inode_bitmap: u32,
    pub inode_table: u32,
    pub free_blocks_count: u16,
    pub free_inodes_count: u16,
    pub used_dirs_count: u16,
    pub pad: u16,
    pub reserved: [u32; 3],
}

/// Inode structure
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Inode {
    pub mode: u16,
    pub uid: u16,
    pub size: u32,
    pub atime: u32,
    pub ctime: u32,
    pub mtime: u32,
    pub dtime: u32,
    pub gid: u16,
    pub links_count: u16,
    pub blocks: u32,
    pub flags: u32,
    pub osd1: u32,
    pub block: [u32; 15],
    pub generation: u32,
    pub file_acl: u32,
    pub dir_acl: u32,
    pub faddr: u32,
    pub osd2: [u32; 3],
}

/// Directory entry
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DirEntry {
    pub inode: u32,
    pub rec_len: u16,
    pub name_len: u8,
    pub file_type: u8,
    // Name follows (up to 255 bytes)
}

/// File types for directory entries
const EXT2_FT_UNKNOWN: u8 = 0;
const EXT2_FT_REG_FILE: u8 = 1;
const EXT2_FT_DIR: u8 = 2;
const EXT2_FT_CHRDEV: u8 = 3;
const EXT2_FT_BLKDEV: u8 = 4;
const EXT2_FT_FIFO: u8 = 5;
const EXT2_FT_SOCK: u8 = 6;
const EXT2_FT_SYMLINK: u8 = 7;

/// Inode mode bits
const S_IFREG: u16 = 0x8000;  // Regular file
const S_IFDIR: u16 = 0x4000;  // Directory
const S_IFCHR: u16 = 0x2000;  // Character device
const S_IFBLK: u16 = 0x6000;  // Block device
const S_IFIFO: u16 = 0x1000;  // FIFO
const S_IFLNK: u16 = 0xA000;  // Symbolic link
const S_IFSOCK: u16 = 0xC000; // Socket

const S_IRUSR: u16 = 0x0100;  // User read
const S_IWUSR: u16 = 0x0080;  // User write
const S_IXUSR: u16 = 0x0040;  // User execute
const S_IRGRP: u16 = 0x0020;  // Group read
const S_IWGRP: u16 = 0x0010;  // Group write
const S_IXGRP: u16 = 0x0008;  // Group execute
const S_IROTH: u16 = 0x0004;  // Other read
const S_IWOTH: u16 = 0x0002;  // Other write
const S_IXOTH: u16 = 0x0001;  // Other execute

/// EXT2 filesystem instance
pub struct Ext2Fs {
    device: Box<dyn BlockDevice>,
    superblock: Superblock,
    block_size: u32,
    groups_count: u32,
    group_descriptors: Vec<GroupDescriptor>,
}

impl Ext2Fs {
    /// Create new EXT2 filesystem from block device
    pub fn new(device: Box<dyn BlockDevice>) -> FsResult<Self> {
        // Read superblock at offset 1024
        let mut superblock_data = [0u8; 1024];
        device.read_blocks(2, 2, &mut superblock_data)
            .map_err(|_| FsError::IoError)?;

        let superblock = unsafe {
            core::ptr::read(superblock_data.as_ptr() as *const Superblock)
        };

        // Verify magic number
        if superblock.magic != EXT2_MAGIC {
            return Err(FsError::InvalidFilesystem);
        }

        let block_size = 1024 << superblock.log_block_size;
        let blocks_per_group = superblock.blocks_per_group;
        let groups_count = (superblock.blocks_count + blocks_per_group - 1) / blocks_per_group;

        println!("[ext2] Mounting EXT2 filesystem");
        println!("  Block size: {} bytes", block_size);
        println!("  Total blocks: {}", superblock.blocks_count);
        println!("  Total inodes: {}", superblock.inodes_count);
        println!("  Block groups: {}", groups_count);

        // Read group descriptors
        let gd_block = if block_size == 1024 { 2 } else { 1 };
        let gd_size = core::mem::size_of::<GroupDescriptor>();
        let gds_per_block = block_size as usize / gd_size;
        let gd_blocks = (groups_count as usize + gds_per_block - 1) / gds_per_block;

        let mut group_descriptors = Vec::with_capacity(groups_count as usize);
        let mut gd_buffer = vec![0u8; gd_blocks * block_size as usize];
        
        device.read_blocks(gd_block as u64, gd_blocks, &mut gd_buffer)
            .map_err(|_| FsError::IoError)?;

        for i in 0..groups_count {
            let offset = i as usize * gd_size;
            let gd = unsafe {
                core::ptr::read(gd_buffer.as_ptr().add(offset) as *const GroupDescriptor)
            };
            group_descriptors.push(gd);
        }

        Ok(Self {
            device,
            superblock,
            block_size,
            groups_count,
            group_descriptors,
        })
    }

    /// Read block from device
    fn read_block(&self, block_num: u32, buf: &mut [u8]) -> FsResult<()> {
        let blocks_per_read = self.block_size as usize / self.device.block_size();
        let device_block = block_num as u64 * blocks_per_read as u64;
        
        self.device.read_blocks(device_block, blocks_per_read, buf)
            .map_err(|_| FsError::IoError)
    }

    /// Write block to device
    fn write_block(&self, block_num: u32, buf: &[u8]) -> FsResult<()> {
        let blocks_per_write = self.block_size as usize / self.device.block_size();
        let device_block = block_num as u64 * blocks_per_write as u64;
        
        self.device.write_blocks(device_block, blocks_per_write, buf)
            .map_err(|_| FsError::IoError)
    }

    /// Read inode from disk
    fn read_inode(&self, inode_num: u32) -> FsResult<Inode> {
        if inode_num == 0 || inode_num > self.superblock.inodes_count {
            return Err(FsError::NotFound);
        }

        let group = (inode_num - 1) / self.superblock.inodes_per_group;
        let index = (inode_num - 1) % self.superblock.inodes_per_group;

        let gd = &self.group_descriptors[group as usize];
        let inode_table_block = gd.inode_table;
        let inode_size = if self.superblock.rev_level >= 1 {
            self.superblock.inode_size as u32
        } else {
            128
        };

        let block_offset = (index * inode_size) / self.block_size;
        let byte_offset = (index * inode_size) % self.block_size;

        let mut block = vec![0u8; self.block_size as usize];
        self.read_block(inode_table_block + block_offset, &mut block)?;

        let inode = unsafe {
            core::ptr::read(block.as_ptr().add(byte_offset as usize) as *const Inode)
        };

        Ok(inode)
    }

    /// Read data from inode
    fn read_inode_data(&self, inode: &Inode, offset: u64, buf: &mut [u8]) -> FsResult<usize> {
        let file_size = inode.size as u64;
        if offset >= file_size {
            return Ok(0);
        }

        let to_read = ((file_size - offset) as usize).min(buf.len());
        let block_size = self.block_size as u64;
        let start_block = (offset / block_size) as u32;
        let block_offset = (offset % block_size) as usize;

        let mut bytes_read = 0;
        let mut current_block = start_block;
        let mut buf_offset = 0;
        let mut remaining = to_read;

        // Handle partial first block
        if block_offset > 0 || to_read < block_size as usize {
            let block_num = self.get_block_number(inode, current_block)?;
            let mut block_data = vec![0u8; self.block_size as usize];
            self.read_block(block_num, &mut block_data)?;

            let from_block = block_offset.min(block_data.len());
            let to_copy = remaining.min(block_data.len() - from_block);
            
            buf[..to_copy].copy_from_slice(&block_data[from_block..from_block + to_copy]);
            bytes_read += to_copy;
            remaining -= to_copy;
            buf_offset += to_copy;
            current_block += 1;
        }

        // Read full blocks
        while remaining >= block_size as usize {
            let block_num = self.get_block_number(inode, current_block)?;
            let mut block_data = vec![0u8; self.block_size as usize];
            self.read_block(block_num, &mut block_data)?;

            buf[buf_offset..buf_offset + block_size as usize]
                .copy_from_slice(&block_data);
            
            bytes_read += block_size as usize;
            remaining -= block_size as usize;
            buf_offset += block_size as usize;
            current_block += 1;
        }

        // Handle partial last block
        if remaining > 0 {
            let block_num = self.get_block_number(inode, current_block)?;
            let mut block_data = vec![0u8; self.block_size as usize];
            self.read_block(block_num, &mut block_data)?;

            buf[buf_offset..buf_offset + remaining]
                .copy_from_slice(&block_data[..remaining]);
            bytes_read += remaining;
        }

        Ok(bytes_read)
    }

    /// Get physical block number from inode block index
    fn get_block_number(&self, inode: &Inode, index: u32) -> FsResult<u32> {
        let block_size = self.block_size;
        let ptrs_per_block = block_size / 4;

        if index < 12 {
            // Direct block
            Ok(inode.block[index as usize])
        } else if index < 12 + ptrs_per_block {
            // Single indirect
            let indirect_block = inode.block[12];
            self.read_indirect_block(indirect_block, index - 12)
        } else if index < 12 + ptrs_per_block + ptrs_per_block * ptrs_per_block {
            // Double indirect
            let indirect_block = inode.block[13];
            let idx = index - 12 - ptrs_per_block;
            let first_level = idx / ptrs_per_block;
            let second_level = idx % ptrs_per_block;
            
            let first_block = self.read_indirect_block(indirect_block, first_level)?;
            self.read_indirect_block(first_block, second_level)
        } else {
            // Triple indirect (simplified - not implemented)
            Err(FsError::NotImplemented)
        }
    }

    /// Read indirect block pointer
    fn read_indirect_block(&self, block: u32, index: u32) -> FsResult<u32> {
        let mut data = vec![0u8; self.block_size as usize];
        self.read_block(block, &mut data)?;

        let ptr = unsafe {
            core::ptr::read(data.as_ptr().add(index as usize * 4) as *const u32)
        };

        if ptr == 0 {
            Err(FsError::NotFound)
        } else {
            Ok(ptr)
        }
    }

    /// Find directory entry
    fn find_dirent(&self, dir_inode: &Inode, name: &str) -> FsResult<(u32, FileType)> {
        if dir_inode.mode & S_IFDIR == 0 {
            return Err(FsError::NotDirectory);
        }

        let file_size = dir_inode.size as usize;
        let mut offset = 0;
        let mut buffer = vec![0u8; self.block_size as usize];

        while offset < file_size {
            let bytes_read = self.read_inode_data(dir_inode, offset as u64, &mut buffer)?;
            if bytes_read == 0 {
                break;
            }

            let mut entry_offset = 0;
            while entry_offset < bytes_read {
                let entry: &DirEntry = unsafe {
                    &*(buffer.as_ptr().add(entry_offset) as *const DirEntry)
                };

                if entry.inode == 0 {
                    entry_offset += entry.rec_len as usize;
                    continue;
                }

                let name_len = entry.name_len as usize;
                let entry_name = unsafe {
                    core::str::from_utf8_unchecked(
                        core::slice::from_raw_parts(
                            buffer.as_ptr().add(entry_offset).add(8) as *const u8,
                            name_len
                        )
                    )
                };

                if entry_name.as_bytes() == name.as_bytes() {
                    let file_type = match entry.file_type {
                        EXT2_FT_REG_FILE => FileType::Regular,
                        EXT2_FT_DIR => FileType::Directory,
                        EXT2_FT_SYMLINK => FileType::Symlink,
                        _ => FileType::Regular,
                    };
                    return Ok((entry.inode, file_type));
                }

                entry_offset += entry.rec_len as usize;
            }

            offset += bytes_read;
        }

        Err(FsError::NotFound)
    }

    /// Lookup path
    fn lookup(&self, path: &str) -> FsResult<(u32, Inode)> {
        let components: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        
        let mut current_inode_num = 2; // Root inode
        let mut current_inode = self.read_inode(current_inode_num)?;

        for component in components {
            if current_inode.mode & S_IFDIR == 0 {
                return Err(FsError::NotDirectory);
            }

            let (inode_num, _) = self.find_dirent(&current_inode, component)?;
            current_inode_num = inode_num;
            current_inode = self.read_inode(inode_num)?;
        }

        Ok((current_inode_num, current_inode))
    }

    /// Convert inode mode to FileType
    fn mode_to_file_type(mode: u16) -> FileType {
        match mode & 0xF000 {
            S_IFREG => FileType::Regular,
            S_IFDIR => FileType::Directory,
            S_IFLNK => FileType::Symlink,
            _ => FileType::Regular,
        }
    }

    /// Convert inode mode to Permissions
    fn mode_to_permissions(mode: u16) -> Permissions {
        Permissions {
            owner_read: mode & S_IRUSR != 0,
            owner_write: mode & S_IWUSR != 0,
            owner_execute: mode & S_IXUSR != 0,
            group_read: mode & S_IRGRP != 0,
            group_write: mode & S_IWGRP != 0,
            group_execute: mode & S_IXGRP != 0,
            other_read: mode & S_IROTH != 0,
            other_write: mode & S_IWOTH != 0,
            other_execute: mode & S_IXOTH != 0,
        }
    }
}

impl FileSystem for Ext2Fs {
    fn name(&self) -> &str {
        "ext2"
    }

    fn root(&self) -> INode {
        INode::new(2) // Root inode
    }

    fn read_metadata(&self, inode: INode) -> FsResult<Metadata> {
        let ext_inode = self.read_inode(inode.as_u64() as u32)?;
        
        Ok(Metadata {
            file_type: Self::mode_to_file_type(ext_inode.mode),
            size: ext_inode.size as u64,
            permissions: Self::mode_to_permissions(ext_inode.mode),
            created: ext_inode.ctime as u64,
            modified: ext_inode.mtime as u64,
            accessed: ext_inode.atime as u64,
            uid: 0,
            gid: 0,
            nlink: ext_inode.links_count as u32,
            block_size: self.block_size,
            blocks: ext_inode.blocks as u64 / (self.block_size / 512) as u64,
        })
    }

    fn write_metadata(&self, _inode: INode, _metadata: &Metadata) -> FsResult<()> {
        Err(FsError::ReadOnly)
    }

    fn read(&self, inode: INode, offset: u64, buf: &mut [u8]) -> FsResult<usize> {
        let ext_inode = self.read_inode(inode.as_u64() as u32)?;
        
        if ext_inode.mode & S_IFREG == 0 && ext_inode.mode & S_IFLNK == 0 {
            return Err(FsError::InvalidArgument);
        }

        self.read_inode_data(&ext_inode, offset, buf)
    }

    fn write(&self, _inode: INode, _offset: u64, _buf: &[u8]) -> FsResult<usize> {
        // Read-only for now
        Err(FsError::ReadOnly)
    }

    fn lookup(&self, parent: INode, name: &str) -> FsResult<INode> {
        let parent_inode = self.read_inode(parent.as_u64() as u32)?;
        let (inode_num, _) = self.find_dirent(&parent_inode, name)?;
        Ok(INode::new(inode_num as u64))
    }

    fn create(&self, _parent: INode, _name: &str, _file_type: FileType) -> FsResult<INode> {
        Err(FsError::ReadOnly)
    }

    fn remove(&self, _parent: INode, _name: &str) -> FsResult<()> {
        Err(FsError::ReadOnly)
    }

    fn read_dir(&self, inode: INode) -> FsResult<Vec<(String, INode)>> {
        let dir_inode = self.read_inode(inode.as_u64() as u32)?;
        
        if dir_inode.mode & S_IFDIR == 0 {
            return Err(FsError::NotDirectory);
        }

        let file_size = dir_inode.size as usize;
        let mut entries = Vec::new();
        let mut offset = 0;
        let mut buffer = vec![0u8; self.block_size as usize];

        while offset < file_size {
            let bytes_read = self.read_inode_data(&dir_inode, offset as u64, &mut buffer)?;
            if bytes_read == 0 {
                break;
            }

            let mut entry_offset = 0;
            while entry_offset < bytes_read {
                let entry = unsafe {
                    &*(buffer.as_ptr().add(entry_offset) as *const DirEntry)
                };

                if entry.inode != 0 && entry.name_len > 0 {
                    let name = unsafe {
                        core::str::from_utf8_unchecked(
                            core::slice::from_raw_parts(
                                buffer.as_ptr().add(entry_offset + 8),
                                entry.name_len as usize
                            )
                        )
                    };

                    if name != "." && name != ".." {
                        entries.push((String::from(name), INode::new(entry.inode as u64)));
                    }
                }

                entry_offset += entry.rec_len as usize;
            }

            offset += bytes_read;
        }

        Ok(entries)
    }
}

/// Mount EXT2 filesystem
pub fn mount(device: Box<dyn BlockDevice>) -> FsResult<Box<dyn FileSystem>> {
    let fs = Ext2Fs::new(device)?;
    Ok(Box::new(fs))
}

/// Initialize EXT2 filesystem driver
pub fn init() {
    println!("[ext2] EXT2 filesystem driver initialized");
}
