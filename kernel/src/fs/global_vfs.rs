//! Global VFS Instance
//!
//! Provides a global Virtual File System instance that uses the boot disk.
//! This allows the rest of the kernel to access files on the boot disk.

use alloc::vec::Vec;
use alloc::vec;
use alloc::string::{String, ToString};
use alloc::format;
use spin::Mutex;
use lazy_static::lazy_static;

use crate::fs::fat32::Fat32Filesystem;
use crate::fs::vfs::{Vfs, VfsOperations, OpenFlags};
use crate::fs::block::BlockDevice;
use crate::error::{VFatError, IoError};
use crate::println;

/// A block device that delegates to the storage subsystem
pub struct StorageBlockDevice {
    device_index: usize,
}

impl StorageBlockDevice {
    /// Create a new block device wrapper for storage device 0 (boot disk)
    pub fn new() -> Self {
        Self { device_index: 0 }
    }
}

impl BlockDevice for StorageBlockDevice {
    fn read_block(&mut self, block: u64, buffer: &mut [u8]) -> Result<(), VFatError> {
        crate::storage::read(self.device_index, block, 1, buffer)
            .map_err(|_| VFatError::Io(IoError::Other("Read failed".to_string())))
    }

    fn write_block(&mut self, block: u64, buffer: &[u8]) -> Result<(), VFatError> {
        crate::storage::write(self.device_index, block, 1, buffer)
            .map_err(|_| VFatError::Io(IoError::Other("Write failed".to_string())))
    }

    fn read_blocks(&mut self, start_block: u64, count: usize, buffer: &mut [u8]) -> Result<(), VFatError> {
        crate::storage::read(self.device_index, start_block, count, buffer)
            .map_err(|_| VFatError::Io(IoError::Other("Read blocks failed".to_string())))
    }

    fn write_blocks(&mut self, start_block: u64, count: usize, buffer: &[u8]) -> Result<(), VFatError> {
        crate::storage::write(self.device_index, start_block, count, buffer)
            .map_err(|_| VFatError::Io(IoError::Other("Write blocks failed".to_string())))
    }

    fn capacity(&self) -> u64 {
        crate::storage::device_block_count(self.device_index).unwrap_or(0)
    }

    fn block_size(&self) -> usize {
        512
    }

    fn flush(&mut self) -> Result<(), VFatError> {
        Ok(())
    }
}

// SAFETY: StorageBlockDevice is thread-safe as it only stores an index
unsafe impl Send for StorageBlockDevice {}
unsafe impl Sync for StorageBlockDevice {}

/// Global VFS error type
#[derive(Debug, Clone)]
pub enum GlobalVfsError {
    NotInitialized,
    VfsError(String),
    IoError(String),
}

/// Global VFS wrapper using storage block device
pub struct GlobalVfs {
    /// The underlying VFS instance (None if not initialized)
    /// Uses a block device that wraps the storage subsystem
    vfs: Option<Vfs<StorageBlockDevice>>,
    /// Whether the VFS is mounted and ready
    mounted: bool,
}

impl GlobalVfs {
    /// Create new uninitialized global VFS
    pub const fn new() -> Self {
        Self {
            vfs: None,
            mounted: false,
        }
    }

    /// Initialize the global VFS with the boot disk
    /// 
    /// # Safety
    /// This should only be called once during kernel initialization
    /// after the storage subsystem has been initialized.
    pub unsafe fn init(&mut self) -> Result<(), GlobalVfsError> {
        if self.mounted {
            return Ok(());
        }

        println!("[global_vfs] Initializing global VFS...");

        // Check if storage is available
        if crate::storage::device_count() == 0 {
            println!("[global_vfs] No storage devices available");
            return Err(GlobalVfsError::NotInitialized);
        }

        // Create block device wrapper
        let block_dev = StorageBlockDevice::new();

        // Mount FAT32 filesystem
        println!("[global_vfs] Mounting FAT32 filesystem...");
        let fat32 = match Fat32Filesystem::mount(block_dev) {
            Ok(fs) => fs,
            Err(e) => {
                println!("[global_vfs] Failed to mount FAT32: {:?}", e);
                return Err(GlobalVfsError::VfsError(format!("{:?}", e)));
            }
        };

        // Create VFS
        let vfs = Vfs::new(fat32);
        self.vfs = Some(vfs);
        self.mounted = true;

        println!("[global_vfs] Global VFS initialized successfully");
        Ok(())
    }

    /// Check if VFS is initialized and ready
    pub fn is_ready(&self) -> bool {
        self.mounted && self.vfs.is_some()
    }

    /// Read a file from the VFS
    pub fn read_file(&mut self, path: &str) -> Result<Vec<u8>, GlobalVfsError> {
        let vfs = self.vfs.as_mut().ok_or(GlobalVfsError::NotInitialized)?;

        // Open file for reading
        let fd = vfs.open(path, OpenFlags::from_mode("r")
            .map_err(|e| GlobalVfsError::VfsError(format!("{:?}", e)))?)
            .map_err(|e| GlobalVfsError::VfsError(format!("{:?}", e)))?;

        // Get file size
        let stat = vfs.stat(path)
            .map_err(|e| GlobalVfsError::VfsError(format!("{:?}", e)))?;
        
        let size = stat.size as usize;
        
        if size == 0 {
            vfs.close(fd).ok();
            return Ok(Vec::new());
        }

        // Read file data
        let mut data = vec![0u8; size];
        let mut total_read = 0;
        
        while total_read < size {
            let bytes_read = vfs.read(fd, &mut data[total_read..])
                .map_err(|e| GlobalVfsError::VfsError(format!("{:?}", e)))?;
            
            if bytes_read == 0 {
                break; // EOF
            }
            total_read += bytes_read;
        }

        // Close file
        vfs.close(fd).ok();

        // Resize to actual bytes read
        data.truncate(total_read);
        Ok(data)
    }

    /// Write a file to the VFS
    pub fn write_file(&mut self, path: &str, data: &[u8]) -> Result<(), GlobalVfsError> {
        let vfs = self.vfs.as_mut().ok_or(GlobalVfsError::NotInitialized)?;

        // Open file for writing (create or truncate)
        let fd = vfs.open(path, OpenFlags::from_mode("w")
            .map_err(|e| GlobalVfsError::VfsError(format!("{:?}", e)))?)
            .map_err(|e| GlobalVfsError::VfsError(format!("{:?}", e)))?;

        // Write data
        let mut total_written = 0;
        while total_written < data.len() {
            let bytes_written = vfs.write(fd, &data[total_written..])
                .map_err(|e| GlobalVfsError::VfsError(format!("{:?}", e)))?;
            
            if bytes_written == 0 {
                return Err(GlobalVfsError::IoError("Write returned 0 bytes".to_string()));
            }
            total_written += bytes_written;
        }

        // Close file
        vfs.close(fd).ok();

        Ok(())
    }

    /// Check if a file exists
    pub fn file_exists(&mut self, path: &str) -> bool {
        let vfs = match self.vfs.as_mut() {
            Some(v) => v,
            None => return false,
        };

        vfs.stat(path).is_ok()
    }

    /// List directory entries
    pub fn read_dir(&mut self, path: &str) -> Result<Vec<DirEntry>, GlobalVfsError> {
        let vfs = self.vfs.as_mut().ok_or(GlobalVfsError::NotInitialized)?;

        // This is a simplified version - in the full implementation
        // we would use the VFS readdir functionality
        
        // For now, return an error indicating not implemented
        Err(GlobalVfsError::VfsError("read_dir not yet implemented".to_string()))
    }

    /// Create a directory
    pub fn create_dir(&mut self, path: &str) -> Result<(), GlobalVfsError> {
        let vfs = self.vfs.as_mut().ok_or(GlobalVfsError::NotInitialized)?;

        vfs.mkdir(path)
            .map_err(|e| GlobalVfsError::VfsError(format!("{:?}", e)))
    }

    /// Delete a file or directory
    pub fn remove(&mut self, path: &str) -> Result<(), GlobalVfsError> {
        let vfs = self.vfs.as_mut().ok_or(GlobalVfsError::NotInitialized)?;

        vfs.remove(path)
            .map_err(|e| GlobalVfsError::VfsError(format!("{:?}", e)))
    }
}

/// Directory entry
#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
}

/// Global VFS instance
static GLOBAL_VFS: Mutex<GlobalVfs> = Mutex::new(GlobalVfs::new());

/// Initialize the global VFS
/// 
/// # Safety
/// Must be called only once during kernel initialization
pub unsafe fn init() -> Result<(), GlobalVfsError> {
    GLOBAL_VFS.lock().init()
}

/// Check if global VFS is ready
pub fn is_ready() -> bool {
    GLOBAL_VFS.lock().is_ready()
}

/// Read a file from the global VFS
pub fn read_file(path: &str) -> Result<Vec<u8>, GlobalVfsError> {
    GLOBAL_VFS.lock().read_file(path)
}

/// Write a file to the global VFS
pub fn write_file(path: &str, data: &[u8]) -> Result<(), GlobalVfsError> {
    GLOBAL_VFS.lock().write_file(path, data)
}

/// Check if a file exists
pub fn file_exists(path: &str) -> bool {
    GLOBAL_VFS.lock().file_exists(path)
}

/// List directory entries
pub fn read_dir(path: &str) -> Result<Vec<DirEntry>, GlobalVfsError> {
    GLOBAL_VFS.lock().read_dir(path)
}

/// Create a directory
pub fn create_dir(path: &str) -> Result<(), GlobalVfsError> {
    GLOBAL_VFS.lock().create_dir(path)
}

/// Delete a file or directory
pub fn remove(path: &str) -> Result<(), GlobalVfsError> {
    GLOBAL_VFS.lock().remove(path)
}

/// Save a file to the Desktop folder
pub fn save_to_desktop(filename: &str, data: &[u8]) -> Result<(), GlobalVfsError> {
    // Sanitize filename
    let safe_name = sanitize_filename(filename);
    if safe_name.is_empty() {
        return Err(GlobalVfsError::IoError("Invalid filename".to_string()));
    }

    let path = format!("Desktop/{}", safe_name);
    write_file(&path, data)
}

/// Sanitize a filename
fn sanitize_filename(name: &str) -> String {
    let name = name.replace("..", "_");
    let name = name.replace("/", "_");
    let name = name.replace("\\", "_");
    let name = name.trim();
    
    if name.len() > 255 {
        String::from(&name[..255])
    } else {
        String::from(name)
    }
}

/// Print VFS status
pub fn print_status() {
    println!("Global VFS Status:");
    
    let vfs = GLOBAL_VFS.lock();
    if vfs.is_ready() {
        println!("  Status: Mounted and ready");
    } else {
        println!("  Status: Not initialized");
    }
}
