//! Virtual File System (VFS)
//!
//! Provides a unified interface for different filesystem implementations.

use alloc::sync::Arc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use spin::Mutex;
use lazy_static::lazy_static;
use crate::println;

/// File permissions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Permissions {
    pub owner_read: bool,
    pub owner_write: bool,
    pub owner_execute: bool,
    pub group_read: bool,
    pub group_write: bool,
    pub group_execute: bool,
    pub other_read: bool,
    pub other_write: bool,
    pub other_execute: bool,
}

impl Permissions {
    /// Default permissions (rw-r--r--)
    pub const fn default() -> Self {
        Self {
            owner_read: true,
            owner_write: true,
            owner_execute: false,
            group_read: true,
            group_write: false,
            group_execute: false,
            other_read: true,
            other_write: false,
            other_execute: false,
        }
    }

    /// Convert to mode bits
    pub fn to_mode(&self) -> u16 {
        let mut mode = 0;
        if self.owner_read { mode |= 0o400; }
        if self.owner_write { mode |= 0o200; }
        if self.owner_execute { mode |= 0o100; }
        if self.group_read { mode |= 0o040; }
        if self.group_write { mode |= 0o020; }
        if self.group_execute { mode |= 0o010; }
        if self.other_read { mode |= 0o004; }
        if self.other_write { mode |= 0o002; }
        if self.other_execute { mode |= 0o001; }
        mode
    }
}

/// File type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    /// Regular file
    Regular,
    /// Directory
    Directory,
    /// Character device
    CharDevice,
    /// Block device
    BlockDevice,
    /// FIFO/Named pipe
    Fifo,
    /// Symbolic link
    Symlink,
    /// Socket
    Socket,
    /// Unknown
    Unknown,
}

/// File metadata
#[derive(Debug, Clone)]
pub struct Metadata {
    /// File type
    pub file_type: FileType,
    /// File size in bytes
    pub size: u64,
    /// Permissions
    pub permissions: Permissions,
    /// Creation time (timestamp)
    pub created: u64,
    /// Modification time (timestamp)
    pub modified: u64,
    /// Access time (timestamp)
    pub accessed: u64,
    /// Owner user ID
    pub uid: u32,
    /// Owner group ID
    pub gid: u32,
    /// Number of hard links
    pub nlink: u32,
    /// Block size
    pub block_size: u32,
    /// Number of blocks
    pub blocks: u64,
}

impl Metadata {
    /// Create metadata for a directory
    pub fn directory() -> Self {
        Self {
            file_type: FileType::Directory,
            size: 0,
            permissions: Permissions::default(),
            created: 0,
            modified: 0,
            accessed: 0,
            uid: 0,
            gid: 0,
            nlink: 2,
            block_size: 4096,
            blocks: 0,
        }
    }

    /// Create metadata for a regular file
    pub fn file(size: u64) -> Self {
        Self {
            file_type: FileType::Regular,
            size,
            permissions: Permissions::default(),
            created: 0,
            modified: 0,
            accessed: 0,
            uid: 0,
            gid: 0,
            nlink: 1,
            block_size: 4096,
            blocks: (size + 4095) / 4096,
        }
    }
}

/// Directory entry
#[derive(Debug, Clone)]
pub struct DirEntry {
    /// Entry name
    pub name: String,
    /// Entry metadata
    pub metadata: Metadata,
    /// Inode number
    pub inode: u64,
}

/// File system error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsError {
    /// Success
    Success = 0,
    /// Permission denied
    PermissionDenied = 1,
    /// File not found
    NotFound = 2,
    /// File already exists
    AlreadyExists = 3,
    /// Not a directory
    NotDirectory = 4,
    /// Is a directory
    IsDirectory = 5,
    /// Invalid argument
    InvalidArgument = 6,
    /// Too many open files
    TooManyOpenFiles = 7,
    /// Out of memory
    OutOfMemory = 8,
    /// IO error
    IoError = 9,
    /// Not implemented
    NotImplemented = 10,
    /// Invalid filesystem
    InvalidFilesystem = 11,
    /// Read only
    ReadOnly = 12,
    /// Unknown error
    Unknown = 255,
}

impl FsError {
    /// Check if error is success
    pub fn is_ok(self) -> bool {
        matches!(self, FsError::Success)
    }

    /// Check if error is an error
    pub fn is_err(self) -> bool {
        !self.is_ok()
    }
}

/// Result type for filesystem operations
pub type FsResult<T> = Result<T, FsError>;

/// File trait - represents an open file
pub trait File: Send + Sync {
    /// Read bytes from file
    fn read(&self, buf: &mut [u8]) -> FsResult<usize>;
    /// Write bytes to file
    fn write(&self, buf: &[u8]) -> FsResult<usize>;
    /// Seek to position
    fn seek(&self, pos: SeekFrom) -> FsResult<u64>;
    /// Get file metadata
    fn metadata(&self) -> FsResult<Metadata>;
    /// Set file metadata
    fn set_metadata(&self, metadata: &Metadata) -> FsResult<()>;
    /// Sync file to disk
    fn sync(&self) -> FsResult<()>;
}

/// Directory trait
pub trait Directory: Send + Sync {
    /// Read directory entries
    fn read_dir(&self) -> FsResult<Vec<DirEntry>>;
    /// Lookup entry by name
    fn lookup(&self, name: &str) -> FsResult<INode>;
    /// Create file
    fn create_file(&self, name: &str, permissions: Permissions) -> FsResult<INode>;
    /// Create directory
    fn create_dir(&self, name: &str, permissions: Permissions) -> FsResult<INode>;
    /// Remove entry
    fn remove(&self, name: &str) -> FsResult<()>;
}

/// Seek position
#[derive(Debug, Clone, Copy)]
pub enum SeekFrom {
    /// Start of file
    Start(u64),
    /// Current position
    Current(i64),
    /// End of file
    End(i64),
}

/// Inode number
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct INode(u64);

impl INode {
    /// Create new inode
    pub const fn new(num: u64) -> Self {
        Self(num)
    }

    /// Get inode number
    pub fn as_u64(self) -> u64 {
        self.0
    }
}

/// Filesystem trait
pub trait FileSystem: Send + Sync {
    /// Get filesystem name
    fn name(&self) -> &str;
    /// Get root inode
    fn root(&self) -> INode;
    /// Read inode metadata
    fn read_metadata(&self, inode: INode) -> FsResult<Metadata>;
    /// Write inode metadata
    fn write_metadata(&self, inode: INode, metadata: &Metadata) -> FsResult<()>;
    /// Read from file
    fn read(&self, inode: INode, offset: u64, buf: &mut [u8]) -> FsResult<usize>;
    /// Write to file
    fn write(&self, inode: INode, offset: u64, buf: &[u8]) -> FsResult<usize>;
    /// Lookup directory entry
    fn lookup(&self, parent: INode, name: &str) -> FsResult<INode>;
    /// Create directory entry
    fn create(&self, parent: INode, name: &str, file_type: FileType) -> FsResult<INode>;
    /// Remove directory entry
    fn remove(&self, parent: INode, name: &str) -> FsResult<()>;
    /// Read directory
    fn read_dir(&self, inode: INode) -> FsResult<Vec<(String, INode)>>;
}

/// Mount point
pub struct MountPoint {
    /// Mount path
    pub path: String,
    /// Filesystem
    pub fs: Arc<dyn FileSystem>,
}

lazy_static! {
    /// Global filesystem table
    static ref MOUNTS: Mutex<Vec<MountPoint>> = Mutex::new(Vec::new());
    static ref NEXT_FD: Mutex<u32> = Mutex::new(3); // Start after stdin/stdout/stderr
}

/// File type
pub mod ext2;
pub mod fat32;

/// Initialize VFS
pub fn init() {
    println!("[vfs] Initializing virtual file system...");

    // Initialize filesystem drivers
    ext2::init();
    fat32::init();

    println!("[vfs] VFS initialized");
}

/// Mount a filesystem
pub fn mount(path: &str, fs: Arc<dyn FileSystem>) -> FsResult<()> {
    let mut mounts = MOUNTS.lock();
    
    // Check if path is already mounted
    for mount in mounts.iter() {
        if mount.path == path {
            return Err(FsError::AlreadyExists);
        }
    }

    let fs_name = fs.name().to_string();
    mounts.push(MountPoint {
        path: path.to_string(),
        fs,
    });

    println!("[vfs] Mounted {} at {}", fs_name, path);
    Ok(())
}

/// Unmount a filesystem
pub fn unmount(path: &str) -> FsResult<()> {
    let mut mounts = MOUNTS.lock();
    
    let pos = mounts.iter()
        .position(|m| m.path == path)
        .ok_or(FsError::NotFound)?;

    mounts.remove(pos);
    println!("[vfs] Unmounted {}", path);
    Ok(())
}

/// Open a file
pub fn open(path: &str, _flags: OpenFlags) -> FsResult<FileHandle> {
    let mounts = MOUNTS.lock();
    
    // Find the filesystem that owns this path
    for mount in mounts.iter() {
        if path.starts_with(&mount.path) {
            let rel_path = &path[mount.path.len()..];
            // TODO: Resolve path and open file
            println!("[vfs] Opening {} on {}", rel_path, mount.fs.name());
            
            // Allocate file descriptor
            let mut next_fd = NEXT_FD.lock();
            let fd = *next_fd;
            *next_fd += 1;
            
            return Ok(FileHandle { fd });
        }
    }

    Err(FsError::NotFound)
}

/// File handle
#[derive(Debug, Clone, Copy)]
pub struct FileHandle {
    /// File descriptor
    fd: u32,
}

impl FileHandle {
    /// Get file descriptor number
    pub fn fd(&self) -> u32 {
        self.fd
    }
}

/// Open flags
#[derive(Debug, Clone, Copy)]
pub struct OpenFlags {
    /// Read only
    pub read: bool,
    /// Write only
    pub write: bool,
    /// Create if doesn't exist
    pub create: bool,
    /// Truncate if exists
    pub truncate: bool,
    /// Append mode
    pub append: bool,
}

impl OpenFlags {
    /// Read-only mode
    pub const RDONLY: Self = Self {
        read: true,
        write: false,
        create: false,
        truncate: false,
        append: false,
    };

    /// Write-only mode
    pub const WRONLY: Self = Self {
        read: false,
        write: true,
        create: false,
        truncate: false,
        append: false,
    };

    /// Read-write mode
    pub const RDWR: Self = Self {
        read: true,
        write: true,
        create: false,
        truncate: false,
        append: false,
    };
}

/// Print VFS statistics
pub fn print_stats() {
    let mounts = MOUNTS.lock();
    
    println!("VFS Statistics:");
    println!("  Mount points: {}", mounts.len());
    
    for mount in mounts.iter() {
        println!("    {} -> {} ({})", mount.path, mount.fs.name(), mount.fs.root().as_u64());
    }
}
