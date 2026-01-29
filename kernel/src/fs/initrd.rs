//! Initial RAM Disk (initrd)
//!
//! Simple RAM-based filesystem for early boot.

use alloc::vec::Vec;
use alloc::string::String;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use spin::Mutex;

use super::{FileSystem, INode, Metadata, FileType, Permissions, FsResult, FsError};

/// Inode data
struct InodeData {
    /// Inode number
    num: INode,
    /// Metadata
    metadata: Metadata,
    /// File data (for regular files)
    data: Vec<u8>,
    /// Directory entries (for directories)
    entries: BTreeMap<String, INode>,
}

/// Initial RAM Disk filesystem
pub struct InitRamFs {
    /// Filesystem name
    name: String,
    /// Inode table
    inodes: Mutex<BTreeMap<u64, InodeData>>,
    /// Next inode number
    next_inode: Mutex<u64>,
}

impl InitRamFs {
    /// Create a new empty initrd
    pub fn new(name: &str) -> Self {
        let mut fs = Self {
            name: name.to_string(),
            inodes: Mutex::new(BTreeMap::new()),
            next_inode: Mutex::new(1),
        };

        // Create root directory (inode 0)
        let root = InodeData {
            num: INode::new(0),
            metadata: Metadata::directory(),
            data: Vec::new(),
            entries: BTreeMap::new(),
        };

        fs.inodes.lock().insert(0, root);

        fs
    }

    /// Allocate a new inode number
    fn alloc_inode(&self) -> INode {
        let mut next = self.next_inode.lock();
        let num = *next;
        *next += 1;
        INode::new(num)
    }

    /// Create a file
    pub fn create_file(&self, path: &str, data: Vec<u8>) -> FsResult<()> {
        // Parse path
        let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        if parts.is_empty() {
            return Err(FsError::InvalidArgument);
        }

        let file_name = parts.last().unwrap();
        let parent_path = &parts[..parts.len() - 1];

        // Find parent directory
        let mut parent_inode = INode::new(0);
        for part in parent_path {
            let inodes = self.inodes.lock();
            let parent = inodes.get(&parent_inode.as_u64())
                .ok_or(FsError::NotFound)?;
            
            let child = parent.entries.get(*part)
                .ok_or(FsError::NotFound)?;
            
            parent_inode = *child;
        }

        // Create file inode
        let file_inode = self.alloc_inode();
        let file_data = InodeData {
            num: file_inode,
            metadata: Metadata::file(data.len() as u64),
            data,
            entries: BTreeMap::new(),
        };

        // Add to parent directory
        {
            let mut inodes = self.inodes.lock();
            inodes.insert(file_inode.as_u64(), file_data);
            
            if let Some(parent) = inodes.get_mut(&parent_inode.as_u64()) {
                parent.entries.insert(file_name.to_string(), file_inode);
            }
        }

        Ok(())
    }

    /// Create a directory
    pub fn create_dir(&self, path: &str) -> FsResult<()> {
        // Parse path
        let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        if parts.is_empty() {
            return Err(FsError::InvalidArgument);
        }

        let dir_name = parts.last().unwrap();
        let parent_path = &parts[..parts.len() - 1];

        // Find parent directory
        let mut parent_inode = INode::new(0);
        for part in parent_path {
            let inodes = self.inodes.lock();
            let parent = inodes.get(&parent_inode.as_u64())
                .ok_or(FsError::NotFound)?;
            
            let child = parent.entries.get(*part)
                .ok_or(FsError::NotFound)?;
            
            parent_inode = *child;
        }

        // Create directory inode
        let dir_inode = self.alloc_inode();
        let dir_data = InodeData {
            num: dir_inode,
            metadata: Metadata::directory(),
            data: Vec::new(),
            entries: BTreeMap::new(),
        };

        // Add to parent directory
        {
            let mut inodes = self.inodes.lock();
            inodes.insert(dir_inode.as_u64(), dir_data);
            
            if let Some(parent) = inodes.get_mut(&parent_inode.as_u64()) {
                parent.entries.insert(dir_name.to_string(), dir_inode);
            }
        }

        Ok(())
    }

    /// Read file contents
    pub fn read_file(&self, path: &str) -> FsResult<Vec<u8>> {
        let inode = self.lookup_path(path)?;
        
        let inodes = self.inodes.lock();
        let data = inodes.get(&inode.as_u64())
            .ok_or(FsError::NotFound)?;
        
        Ok(data.data.clone())
    }

    /// Lookup path
    fn lookup_path(&self, path: &str) -> FsResult<INode> {
        let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        
        let mut current = INode::new(0); // Start at root
        
        for part in &parts {
            let inodes = self.inodes.lock();
            let inode_data = inodes.get(&current.as_u64())
                .ok_or(FsError::NotFound)?;
            
            current = *inode_data.entries.get(*part)
                .ok_or(FsError::NotFound)?;
        }
        
        Ok(current)
    }
}

impl FileSystem for InitRamFs {
    fn name(&self) -> &str {
        &self.name
    }

    fn root(&self) -> INode {
        INode::new(0)
    }

    fn read_metadata(&self, inode: INode) -> FsResult<Metadata> {
        let inodes = self.inodes.lock();
        let data = inodes.get(&inode.as_u64())
            .ok_or(FsError::NotFound)?;
        Ok(data.metadata.clone())
    }

    fn write_metadata(&self, inode: INode, metadata: &Metadata) -> FsResult<()> {
        let mut inodes = self.inodes.lock();
        let data = inodes.get_mut(&inode.as_u64())
            .ok_or(FsError::NotFound)?;
        data.metadata = metadata.clone();
        Ok(())
    }

    fn read(&self, inode: INode, offset: u64, buf: &mut [u8]) -> FsResult<usize> {
        let inodes = self.inodes.lock();
        let data = inodes.get(&inode.as_u64())
            .ok_or(FsError::NotFound)?;
        
        let offset = offset as usize;
        let len = buf.len().min(data.data.len().saturating_sub(offset));
        
        buf[..len].copy_from_slice(&data.data[offset..offset + len]);
        Ok(len)
    }

    fn write(&self, inode: INode, offset: u64, buf: &[u8]) -> FsResult<usize> {
        let mut inodes = self.inodes.lock();
        let data = inodes.get_mut(&inode.as_u64())
            .ok_or(FsError::NotFound)?;
        
        let offset = offset as usize;
        let end = offset + buf.len();
        
        if end > data.data.len() {
            data.data.resize(end, 0);
        }
        
        data.data[offset..end].copy_from_slice(buf);
        data.metadata.size = data.data.len() as u64;
        
        Ok(buf.len())
    }

    fn lookup(&self, parent: INode, name: &str) -> FsResult<INode> {
        let inodes = self.inodes.lock();
        let parent_data = inodes.get(&parent.as_u64())
            .ok_or(FsError::NotFound)?;
        
        parent_data.entries.get(name)
            .copied()
            .ok_or(FsError::NotFound)
    }

    fn create(&self, parent: INode, name: &str, file_type: FileType) -> FsResult<INode> {
        let new_inode = self.alloc_inode();
        
        let metadata = match file_type {
            FileType::Directory => Metadata::directory(),
            FileType::Regular => Metadata::file(0),
            _ => return Err(FsError::NotImplemented),
        };

        let data = InodeData {
            num: new_inode,
            metadata,
            data: Vec::new(),
            entries: BTreeMap::new(),
        };

        {
            let mut inodes = self.inodes.lock();
            inodes.insert(new_inode.as_u64(), data);
            
            if let Some(parent_data) = inodes.get_mut(&parent.as_u64()) {
                parent_data.entries.insert(name.to_string(), new_inode);
            }
        }

        Ok(new_inode)
    }

    fn remove(&self, parent: INode, name: &str) -> FsResult<()> {
        let mut inodes = self.inodes.lock();
        let parent_data = inodes.get_mut(&parent.as_u64())
            .ok_or(FsError::NotFound)?;
        
        let inode = parent_data.entries.remove(name)
            .ok_or(FsError::NotFound)?;
        
        inodes.remove(&inode.as_u64());
        Ok(())
    }

    fn read_dir(&self, inode: INode) -> FsResult<Vec<(String, INode)>> {
        let inodes = self.inodes.lock();
        let data = inodes.get(&inode.as_u64())
            .ok_or(FsError::NotFound)?;
        
        let entries: Vec<(String, INode)> = data.entries
            .iter()
            .map(|(name, inode)| (name.clone(), *inode))
            .collect();
        
        Ok(entries)
    }
}

/// Create a basic initrd with essential directories
pub fn create_basic_initrd() -> Arc<InitRamFs> {
    let initrd = Arc::new(InitRamFs::new("initrd"));

    // Create essential directories
    let _ = initrd.create_dir("/bin");
    let _ = initrd.create_dir("/etc");
    let _ = initrd.create_dir("/tmp");
    let _ = initrd.create_dir("/dev");
    let _ = initrd.create_dir("/proc");
    let _ = initrd.create_dir("/var");
    let _ = initrd.create_dir("/home");

    // Create a welcome file
    let welcome = b"Welcome to WebbOS v0.1.0\n";
    let _ = initrd.create_file("/etc/welcome", welcome.to_vec());

    initrd
}

/// Print initrd contents
pub fn print_initrd(initrd: &InitRamFs) {
    fn print_dir(initrd: &InitRamFs, inode: INode, prefix: &str) {
        if let Ok(entries) = initrd.read_dir(inode) {
            for (name, child_inode) in entries {
                let path = format!("{}{}", prefix, name);
                
                if let Ok(metadata) = initrd.read_metadata(child_inode) {
                    let type_char = match metadata.file_type {
                        FileType::Directory => 'd',
                        FileType::Regular => '-',
                        _ => '?',
                    };
                    
                    println!("{}{} {} bytes", prefix, name, metadata.size);
                    
                    if metadata.file_type == FileType::Directory {
                        print_dir(initrd, child_inode, &format!("{}  ", prefix));
                    }
                }
            }
        }
    }

    println!("Initial RAM Disk contents:");
    print_dir(initrd, INode::new(0), "");
}
