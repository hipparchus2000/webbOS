//! Virtual File System (VFS) Layer
//!
//! Provides a unified interface for multiple filesystem implementations,
//! file descriptor management, and system call integration.

use crate::error::{VFatError, IoError};
use crate::fs::block::BlockDevice;
use crate::fs::fat32::{Fat32Filesystem, FileInfo};
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// Maximum number of open files
pub const MAX_OPEN_FILES: usize = 256;

/// File descriptor type
pub type Fd = i32;

/// VFS file handle
#[derive(Debug, Clone)]
pub struct FileHandle {
    /// File descriptor
    pub fd: Fd,
    /// Path to file
    pub path: String,
    /// Current position in file
    pub position: u64,
    /// Open flags
    pub flags: OpenFlags,
    /// File size at open time
    pub size: u64,
    /// Starting cluster
    pub cluster: u32,
    /// Parent directory cluster
    pub parent_cluster: u32,
    /// Entry offset in parent directory
    pub entry_offset: usize,
}

/// Open flags
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OpenFlags {
    /// Read access
    pub read: bool,
    /// Write access
    pub write: bool,
    /// Create if doesn't exist
    pub create: bool,
    /// Truncate existing file
    pub truncate: bool,
    /// Append mode
    pub append: bool,
    /// Exclusive creation
    pub exclusive: bool,
}

impl OpenFlags {
    /// Create flags from mode string (e.g., "r", "w", "a", "r+", "w+")
    pub fn from_mode(mode: &str) -> Result<Self, VFatError> {
        match mode {
            "r" => Ok(Self { read: true, write: false, create: false, truncate: false, append: false, exclusive: false }),
            "w" => Ok(Self { read: false, write: true, create: true, truncate: true, append: false, exclusive: false }),
            "a" => Ok(Self { read: false, write: true, create: true, truncate: false, append: true, exclusive: false }),
            "r+" => Ok(Self { read: true, write: true, create: false, truncate: false, append: false, exclusive: false }),
            "w+" => Ok(Self { read: true, write: true, create: true, truncate: true, append: false, exclusive: false }),
            "a+" => Ok(Self { read: true, write: true, create: true, truncate: false, append: true, exclusive: false }),
            "x" => Ok(Self { read: false, write: true, create: true, truncate: false, append: false, exclusive: true }),
            "x+" => Ok(Self { read: true, write: true, create: true, truncate: false, append: false, exclusive: true }),
            _ => Err(VFatError::InvalidParameter(format!("Invalid open mode: {}", mode))),
        }
    }
}

/// VFS operations trait
pub trait VfsOperations {
    /// Open a file
    fn open(&mut self, path: &str, flags: OpenFlags) -> Result<Fd, VFatError>;
    
    /// Close a file
    fn close(&mut self, fd: Fd) -> Result<(), VFatError>;
    
    /// Read from file
    fn read(&mut self, fd: Fd, buffer: &mut [u8]) -> Result<usize, VFatError>;
    
    /// Write to file
    fn write(&mut self, fd: Fd, buffer: &[u8]) -> Result<usize, VFatError>;
    
    /// Seek to position
    fn seek(&mut self, fd: Fd, position: SeekFrom) -> Result<u64, VFatError>;
    
    /// Get file metadata
    fn stat(&mut self, path: &str) -> Result<FileStat, VFatError>;
    
    /// Create directory
    fn mkdir(&mut self, path: &str) -> Result<(), VFatError>;
    
    /// Remove file or directory
    fn remove(&mut self, path: &str) -> Result<(), VFatError>;
    
    /// List directory contents
    fn readdir(&mut self, path: &str) -> Result<Vec<DirEntry>, VFatError>;
    
    /// Flush pending writes
    fn sync(&mut self) -> Result<(), VFatError>;
}

/// Seek position
#[derive(Debug, Clone, Copy)]
pub enum SeekFrom {
    /// Start of file + offset
    Start(u64),
    /// Current position + offset
    Current(i64),
    /// End of file + offset
    End(i64),
}

/// File metadata
#[derive(Debug, Clone)]
pub struct FileStat {
    /// File name
    pub name: String,
    /// Full path
    pub path: String,
    /// File size in bytes
    pub size: u64,
    /// Is a directory
    pub is_dir: bool,
    /// Is a regular file
    pub is_file: bool,
    /// Creation timestamp
    pub created: u64,
    /// Last modification timestamp
    pub modified: u64,
    /// Last access timestamp
    pub accessed: u64,
}

/// Directory entry
#[derive(Debug, Clone)]
pub struct DirEntry {
    /// Entry name
    pub name: String,
    /// Is a directory
    pub is_dir: bool,
    /// Is a regular file
    pub is_file: bool,
    /// File size
    pub size: u64,
}

/// VFS implementation for FAT32
pub struct Vfs<B: BlockDevice> {
    /// Underlying FAT32 filesystem
    fs: Fat32Filesystem<B>,
    /// Open file handles
    handles: BTreeMap<Fd, FileHandle>,
    /// Next file descriptor
    next_fd: Fd,
    /// Current directory (cluster)
    current_dir: u32,
}

impl<B: BlockDevice> Vfs<B> {
    /// Create a new VFS instance
    pub fn new(fs: Fat32Filesystem<B>) -> Self {
        let root_cluster = fs.info().root_cluster;
        Self {
            fs,
            handles: BTreeMap::new(),
            next_fd: 3, // 0=stdin, 1=stdout, 2=stderr
            current_dir: root_cluster,
        }
    }

    /// Parse path and return components
    fn parse_path<'a>(&self, path: &'a str) -> Vec<&'a str> {
        path.split('/')
            .filter(|s| !s.is_empty() && *s != ".")
            .collect()
    }

    /// Resolve path to directory cluster and entry name
    fn resolve_path(&mut self, path: &str) -> Result<(u32, String), VFatError> {
        let is_absolute = path.starts_with('/');
        let components: Vec<&str> = path.split('/')
            .filter(|s| !s.is_empty() && *s != ".")
            .collect();
        
        if components.is_empty() || path == "/" {
            return Ok((self.fs.info().root_cluster, String::new()));
        }

        let mut current_cluster = if is_absolute {
            self.fs.info().root_cluster
        } else {
            self.current_dir
        };

        // Navigate through path components (excluding the last one which is the target)
        if components.len() > 1 {
            for i in 0..components.len() - 1 {
                let name = components[i];
                
                if name == ".." {
                    // Handle parent directory
                    // In a real implementation, we'd track parent cluster
                    continue;
                }

                // Find directory entry
                let entries = self.fs.list_directory(current_cluster)?;
                let found = entries.iter()
                    .find(|e| e.is_directory && e.name.eq_ignore_ascii_case(name));

                match found {
                    Some(entry) => current_cluster = entry.cluster,
                    None => return Err(VFatError::io(IoError::not_found(&format!("Directory not found: {}", name)))),
                }
            }
        }

        let file_name = components.last().unwrap().to_string();
        Ok((current_cluster, file_name))
    }

    /// Find file entry by path
    fn find_file(&mut self, path: &str) -> Result<(FileInfo, u32, usize), VFatError> {
        let (dir_cluster, file_name) = self.resolve_path(path)?;
        
        if file_name.is_empty() {
            // Path is a directory
            return Err(VFatError::io(IoError::invalid_input("Path is a directory")));
        }

        let entries = self.fs.list_directory(dir_cluster)?;
        let found = entries.iter()
            .find(|e| e.name.eq_ignore_ascii_case(&file_name));

        match found {
            Some(entry) => {
                // Find entry offset (simplified - would need actual offset tracking)
                Ok((entry.clone(), dir_cluster, 0))
            }
            None => Err(VFatError::io(IoError::not_found(&format!("File not found: {}", file_name)))),
        }
    }
}

impl<B: BlockDevice> VfsOperations for Vfs<B> {
    fn open(&mut self, path: &str, flags: OpenFlags) -> Result<Fd, VFatError> {
        let (dir_cluster, file_name) = self.resolve_path(path)?;

        if file_name.is_empty() {
            return Err(VFatError::InvalidParameter("Invalid file path".to_string()));
        }

        // Check if file exists
        let entries = self.fs.list_directory(dir_cluster)?;
        let existing = entries.iter()
            .find(|e| e.is_file && e.name.eq_ignore_ascii_case(&file_name));

        let (file_info, entry_offset) = if let Some(entry) = existing {
            if flags.exclusive {
                return Err(VFatError::io(IoError::already_exists("File already exists")));
            }

            if flags.truncate {
                // Truncate file
                self.fs.write_file(dir_cluster, &file_name, &[])?;
            }

            (entry.clone(), 0) // Would need actual offset
        } else {
            if !flags.create {
                return Err(VFatError::io(IoError::not_found("File not found")));
            }

            // Create new file
            let info = self.fs.create_file(dir_cluster, &file_name)?;
            (info, 0)
        };

        // Allocate file descriptor
        let fd = self.next_fd;
        self.next_fd += 1;

        let position = if flags.append {
            file_info.size as u64
        } else {
            0
        };

        let handle = FileHandle {
            fd,
            path: path.to_string(),
            position,
            flags,
            size: file_info.size as u64,
            cluster: file_info.cluster,
            parent_cluster: dir_cluster,
            entry_offset,
        };

        self.handles.insert(fd, handle);
        Ok(fd)
    }

    fn close(&mut self, fd: Fd) -> Result<(), VFatError> {
        if fd < 0 || fd >= MAX_OPEN_FILES as i32 {
            return Err(VFatError::InvalidParameter("Invalid file descriptor".to_string()));
        }

        if let Some(handle) = self.handles.remove(&fd) {
            // Flush if write mode
            if handle.flags.write {
                self.fs.flush()?;
            }
            Ok(())
        } else {
            Err(VFatError::InvalidParameter("File not open".to_string()))
        }
    }

    fn read(&mut self, fd: Fd, buffer: &mut [u8]) -> Result<usize, VFatError> {
        let handle = self.handles.get(&fd)
            .ok_or_else(|| VFatError::InvalidParameter("Invalid file descriptor".to_string()))?
            .clone();

        if !handle.flags.read {
            return Err(VFatError::io(IoError::permission_denied("File not open for reading")));
        }

        // Read file data
        let data = self.fs.read_file(&FileInfo {
            name: handle.path.clone(),
            short_name: String::new(),
            size: handle.size as u32,
            attributes: 0,
            cluster: handle.cluster,
            is_directory: false,
            is_file: true,
        })?;

        // Copy to buffer from current position
        let start = handle.position as usize;
        let available = data.len().saturating_sub(start);
        let to_read = buffer.len().min(available);

        if to_read > 0 {
            buffer[..to_read].copy_from_slice(&data[start..start + to_read]);
        }

        // Update position
        if let Some(h) = self.handles.get_mut(&fd) {
            h.position += to_read as u64;
        }

        Ok(to_read)
    }

    fn write(&mut self, fd: Fd, buffer: &[u8]) -> Result<usize, VFatError> {
        let handle = self.handles.get(&fd)
            .ok_or_else(|| VFatError::InvalidParameter("Invalid file descriptor".to_string()))?
            .clone();

        if !handle.flags.write {
            return Err(VFatError::io(IoError::permission_denied("File not open for writing")));
        }

        // Read existing data
        let mut data = if handle.size > 0 {
            self.fs.read_file(&FileInfo {
                name: handle.path.clone(),
                short_name: String::new(),
                size: handle.size as u32,
                attributes: 0,
                cluster: handle.cluster,
                is_directory: false,
                is_file: true,
            })?
        } else {
            Vec::new()
        };

        // Expand if necessary
        let end_pos = handle.position as usize + buffer.len();
        if end_pos > data.len() {
            data.resize(end_pos, 0);
        }

        // Write at position
        let start = handle.position as usize;
        data[start..start + buffer.len()].copy_from_slice(buffer);

        // Write back
        let path = handle.path.clone();
        let (dir_cluster, file_name) = self.resolve_path(&path)?;
        self.fs.write_file(dir_cluster, &file_name, &data)?;

        // Update handle
        if let Some(h) = self.handles.get_mut(&fd) {
            h.position += buffer.len() as u64;
            h.size = data.len() as u64;
        }

        Ok(buffer.len())
    }

    fn seek(&mut self, fd: Fd, position: SeekFrom) -> Result<u64, VFatError> {
        let handle = self.handles.get(&fd)
            .ok_or_else(|| VFatError::InvalidParameter("Invalid file descriptor".to_string()))?;

        let new_position = match position {
            SeekFrom::Start(pos) => pos,
            SeekFrom::Current(offset) => {
                let current = handle.position as i64;
                let new = current + offset;
                if new < 0 {
                    return Err(VFatError::InvalidParameter("Seek before start of file".to_string()));
                }
                new as u64
            }
            SeekFrom::End(offset) => {
                let size = handle.size as i64;
                let new = size + offset;
                if new < 0 {
                    return Err(VFatError::InvalidParameter("Seek before start of file".to_string()));
                }
                new as u64
            }
        };

        if let Some(h) = self.handles.get_mut(&fd) {
            h.position = new_position;
        }

        Ok(new_position)
    }

    fn stat(&mut self, path: &str) -> Result<FileStat, VFatError> {
        let (file_info, _, _) = self.find_file(path)?;
        
        Ok(FileStat {
            name: file_info.name.clone(),
            path: path.to_string(),
            size: file_info.size as u64,
            is_dir: file_info.is_directory,
            is_file: file_info.is_file,
            created: 0, // Would extract from entry
            modified: 0,
            accessed: 0,
        })
    }

    fn mkdir(&mut self, path: &str) -> Result<(), VFatError> {
        let (parent_cluster, dir_name) = self.resolve_path(path)?;

        if dir_name.is_empty() {
            return Err(VFatError::InvalidParameter("Invalid directory path".to_string()));
        }

        self.fs.create_directory(parent_cluster, &dir_name)?;
        Ok(())
    }

    fn remove(&mut self, path: &str) -> Result<(), VFatError> {
        let (dir_cluster, name) = self.resolve_path(path)?;

        if name.is_empty() {
            return Err(VFatError::InvalidParameter("Invalid path".to_string()));
        }

        // Check if file is open
        for handle in self.handles.values() {
            if handle.path == path {
                return Err(VFatError::io(IoError::other("File is open")));
            }
        }

        self.fs.delete(dir_cluster, &name)?;
        Ok(())
    }

    fn readdir(&mut self, path: &str) -> Result<Vec<DirEntry>, VFatError> {
        let (dir_cluster, _) = self.resolve_path(path)?;
        let entries = self.fs.list_directory(dir_cluster)?;

        Ok(entries.iter()
            .filter(|e| !e.short_name.starts_with('.')) // Skip . and ..
            .map(|e| DirEntry {
                name: e.name.clone(),
                is_dir: e.is_directory,
                is_file: e.is_file,
                size: e.size as u64,
            })
            .collect())
    }

    fn sync(&mut self) -> Result<(), VFatError> {
        self.fs.flush()
    }
}

/// System call interface for file operations
pub struct FileSystemSyscall;

impl FileSystemSyscall {
    /// Open a file (syscall interface)
    pub fn sys_open<B: BlockDevice>(vfs: &mut Vfs<B>, path: &str, mode: &str) -> Result<Fd, VFatError> {
        let flags = OpenFlags::from_mode(mode)?;
        vfs.open(path, flags)
    }

    /// Close a file (syscall interface)
    pub fn sys_close<B: BlockDevice>(vfs: &mut Vfs<B>, fd: Fd) -> Result<(), VFatError> {
        vfs.close(fd)
    }

    /// Read from file (syscall interface)
    pub fn sys_read<B: BlockDevice>(vfs: &mut Vfs<B>, fd: Fd, buffer: &mut [u8]) -> Result<usize, VFatError> {
        vfs.read(fd, buffer)
    }

    /// Write to file (syscall interface)
    pub fn sys_write<B: BlockDevice>(vfs: &mut Vfs<B>, fd: Fd, buffer: &[u8]) -> Result<usize, VFatError> {
        vfs.write(fd, buffer)
    }

    /// Seek in file (syscall interface)
    pub fn sys_lseek<B: BlockDevice>(vfs: &mut Vfs<B>, fd: Fd, offset: i64, whence: u32) -> Result<u64, VFatError> {
        let from = match whence {
            0 => SeekFrom::Start(offset as u64),
            1 => SeekFrom::Current(offset),
            2 => SeekFrom::End(offset),
            _ => return Err(VFatError::InvalidParameter("Invalid whence".to_string())),
        };
        vfs.seek(fd, from)
    }

    /// Get file status (syscall interface)
    pub fn sys_fstat<B: BlockDevice>(vfs: &mut Vfs<B>, path: &str) -> Result<FileStat, VFatError> {
        vfs.stat(path)
    }

    /// Create directory (syscall interface)
    pub fn sys_mkdir<B: BlockDevice>(vfs: &mut Vfs<B>, path: &str) -> Result<(), VFatError> {
        vfs.mkdir(path)
    }

    /// Remove file/directory (syscall interface)
    pub fn sys_unlink<B: BlockDevice>(vfs: &mut Vfs<B>, path: &str) -> Result<(), VFatError> {
        vfs.remove(path)
    }

    /// Read directory (syscall interface)
    pub fn sys_getdents<B: BlockDevice>(vfs: &mut Vfs<B>, path: &str) -> Result<Vec<DirEntry>, VFatError> {
        vfs.readdir(path)
    }

    /// Sync filesystem (syscall interface)
    pub fn sys_sync<B: BlockDevice>(vfs: &mut Vfs<B>) -> Result<(), VFatError> {
        vfs.sync()
    }
}

// External crate dependencies
extern crate alloc;
