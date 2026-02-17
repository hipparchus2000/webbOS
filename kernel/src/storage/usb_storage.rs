//! USB Storage Integration
//!
//! Provides integration between USB mass storage devices and the storage subsystem.
//! Handles detection, registration, and auto-mounting of USB storage devices.

use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::format;
use alloc::vec;
use spin::Mutex;

use crate::println;
use crate::drivers::usb::mass_storage::{UsbMassStorage, UsbMassStorageBlockDevice, UsbMassStorageVfsBlockDevice};
use crate::storage::{BlockDevice, StorageError, register_device as register_block_device};
use crate::fs::fat32::Fat32Filesystem;
use crate::fs::vfs::{Vfs, VfsOperations};

/// Information about a registered USB storage device
#[derive(Debug, Clone)]
pub struct UsbStorageInfo {
    /// Device index in the storage subsystem
    pub storage_index: usize,
    /// Device model/name
    pub model: String,
    /// Total capacity in bytes
    pub capacity: u64,
    /// Sector size in bytes
    pub sector_size: u32,
    /// Whether the device is currently mounted
    pub mounted: bool,
    /// Mount point (if mounted)
    pub mount_point: Option<String>,
}

/// USB storage manager
pub struct UsbStorageManager {
    /// List of registered USB storage devices
    devices: Vec<Arc<Mutex<UsbMassStorage>>>,
    /// Device information
    device_info: Vec<UsbStorageInfo>,
    /// VFS instances for mounted filesystems (index matches devices)
    mounted_vfs: Vec<Option<Vfs<UsbMassStorageVfsBlockDevice>>>,
}

impl UsbStorageManager {
    /// Create a new USB storage manager
    pub const fn new() -> Self {
        Self {
            devices: Vec::new(),
            device_info: Vec::new(),
            mounted_vfs: Vec::new(),
        }
    }

    /// Register a new USB mass storage device
    /// 
    /// This function:
    /// 1. Adds the device to the internal list
    /// 2. Registers it as a block device with the storage subsystem
    /// 3. Attempts to auto-mount a FAT32 filesystem
    pub fn register_device(&mut self, device: Arc<Mutex<UsbMassStorage>>) -> Result<usize, StorageError> {
        let storage_index = self.devices.len();
        
        // Get device info
        let (model, capacity, sector_size) = {
            let dev = device.lock();
            (
                dev.model.clone(),
                dev.capacity_sectors * dev.sector_size as u64,
                dev.sector_size,
            )
        };

        println!("[usb-storage] Registering device {}: {} ({} MB)",
            storage_index,
            model,
            capacity / (1024 * 1024)
        );

        // Create block device wrapper for storage subsystem
        let block_device = UsbMassStorageBlockDevice::new(device.clone(), storage_index);
        
        // Register with storage subsystem
        register_block_device(Box::new(block_device));

        // Add to internal tracking
        self.devices.push(device.clone());
        
        let info = UsbStorageInfo {
            storage_index,
            model: model.clone(),
            capacity,
            sector_size,
            mounted: false,
            mount_point: None,
        };
        self.device_info.push(info);
        self.mounted_vfs.push(None);

        // Attempt to auto-mount
        self.try_mount(storage_index);

        Ok(storage_index)
    }

    /// Try to mount a filesystem on the specified USB storage device
    fn try_mount(&mut self, device_index: usize) -> bool {
        if device_index >= self.devices.len() {
            return false;
        }

        // Check if already mounted
        if self.device_info[device_index].mounted {
            return true;
        }

        let device = self.devices[device_index].clone();

        // Create VFS block device wrapper
        let vfs_block_dev = UsbMassStorageVfsBlockDevice::new(device);

        // Try to mount FAT32 filesystem
        match Fat32Filesystem::mount(vfs_block_dev) {
            Ok(fs) => {
                println!("[usb-storage] Mounted FAT32 on device {}", device_index);
                
                let vfs = Vfs::new(fs);
                self.mounted_vfs[device_index] = Some(vfs);
                
                let mount_point = format!("/usb{}", device_index);
                self.device_info[device_index].mounted = true;
                self.device_info[device_index].mount_point = Some(mount_point.clone());
                
                // Update global storage status
                usb_storage_mounted(device_index, true);
                
                println!("[usb-storage] USB device {} mounted at {}", device_index, mount_point);
                true
            }
            Err(e) => {
                println!("[usb-storage] Failed to mount FAT32 on device {}: {:?}", device_index, e);
                
                self.device_info[device_index].mounted = false;
                usb_storage_mounted(device_index, false);
                false
            }
        }
    }

    /// Unmount a USB storage device
    pub fn unmount(&mut self, device_index: usize) -> Result<(), StorageError> {
        if device_index >= self.devices.len() {
            return Err(StorageError::NotFound);
        }

        if !self.device_info[device_index].mounted {
            return Err(StorageError::InvalidArgument);
        }

        // Flush any pending writes
        if let Some(ref mut vfs) = self.mounted_vfs[device_index] {
            // The VFS doesn't have an explicit sync method,
            // but we could add one if needed
        }

        // Remove VFS instance
        self.mounted_vfs[device_index] = None;
        self.device_info[device_index].mounted = false;
        self.device_info[device_index].mount_point = None;

        // Update global storage status
        usb_storage_mounted(device_index, false);

        println!("[usb-storage] Device {} unmounted", device_index);
        Ok(())
    }

    /// Handle device disconnection
    pub fn disconnect(&mut self, device_index: usize) {
        if device_index >= self.devices.len() {
            return;
        }

        // Unmount if mounted
        if self.device_info[device_index].mounted {
            let _ = self.unmount(device_index);
        }

        // Mark device as disconnected
        self.device_info[device_index].mounted = false;
        
        println!("[usb-storage] Device {} disconnected", device_index);
    }

    /// Get device information
    pub fn device_info(&self, device_index: usize) -> Option<&UsbStorageInfo> {
        self.device_info.get(device_index)
    }

    /// Get list of all USB storage devices
    pub fn list_devices(&self) -> &[UsbStorageInfo] {
        &self.device_info
    }

    /// Get the VFS for a mounted device
    pub fn get_vfs(&mut self, device_index: usize) -> Option<&mut Vfs<UsbMassStorageVfsBlockDevice>> {
        if device_index < self.mounted_vfs.len() {
            self.mounted_vfs[device_index].as_mut()
        } else {
            None
        }
    }

    /// Check if a device is mounted
    pub fn is_mounted(&self, device_index: usize) -> bool {
        self.device_info
            .get(device_index)
            .map(|info| info.mounted)
            .unwrap_or(false)
    }

    /// Print USB storage status
    pub fn print_status(&self) {
        println!("USB Storage Devices:");
        
        if self.device_info.is_empty() {
            println!("  No USB storage devices connected");
            return;
        }

        println!("{:<4} {:<20} {:<12} {:<12} {}",
            "Idx", "Model", "Capacity", "Status", "Mount Point");
        println!("{}", "-".repeat(70));

        for info in &self.device_info {
            let capacity_mb = info.capacity / (1024 * 1024);
            let status = if info.mounted { "Mounted" } else { "Not mounted" };
            let mount_point = info.mount_point.as_deref().unwrap_or("-");
            
            println!("{:<4} {:<20} {:<12} {:<12} {}",
                info.storage_index,
                truncate_str(&info.model, 20),
                format!("{} MB", capacity_mb),
                status,
                mount_point
            );
        }
    }
}

/// Global USB storage manager
static USB_STORAGE_MANAGER: Mutex<UsbStorageManager> = Mutex::new(UsbStorageManager::new());

/// Initialize USB storage subsystem
pub fn init() {
    println!("[usb-storage] Initializing USB storage subsystem...");
    // The manager is lazily initialized via the static
    println!("[usb-storage] USB storage subsystem ready");
}

/// Register a USB mass storage device
/// 
/// Called by the USB mass storage driver when a new device is detected.
pub fn register_device(device: Arc<Mutex<UsbMassStorage>>) -> Result<usize, StorageError> {
    USB_STORAGE_MANAGER.lock().register_device(device)
}

/// Unmount a USB storage device
pub fn unmount(device_index: usize) -> Result<(), StorageError> {
    USB_STORAGE_MANAGER.lock().unmount(device_index)
}

/// Handle device disconnection
/// 
/// Called by the USB mass storage driver when a device is unplugged.
pub fn disconnect(device_index: usize) {
    USB_STORAGE_MANAGER.lock().disconnect(device_index)
}

/// Get device information
pub fn device_info(device_index: usize) -> Option<UsbStorageInfo> {
    USB_STORAGE_MANAGER
        .lock()
        .device_info(device_index)
        .cloned()
}

/// List all USB storage devices
pub fn list_devices() -> Vec<UsbStorageInfo> {
    USB_STORAGE_MANAGER.lock().list_devices().to_vec()
}

/// Print USB storage status
pub fn print_status() {
    USB_STORAGE_MANAGER.lock().print_status();
}

/// Check if a device is mounted
pub fn is_mounted(device_index: usize) -> bool {
    USB_STORAGE_MANAGER.lock().is_mounted(device_index)
}

/// Get the VFS for a mounted device (for filesystem operations)
pub fn get_vfs(device_index: usize) -> Option<Vfs<UsbMassStorageVfsBlockDevice>> {
    // Return an owned VFS instance by taking it out of the manager
    let mut manager = USB_STORAGE_MANAGER.lock();
    if device_index < manager.mounted_vfs.len() {
        // Take the VFS out, leaving None in its place
        // Caller must put it back when done
        manager.mounted_vfs[device_index].take()
    } else {
        None
    }
}

/// Helper function to truncate a string to a maximum length
fn truncate_str(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len {
        s
    } else {
        &s[..max_len]
    }
}

/// Read a file from a mounted USB storage device
/// 
/// Convenience function to read a file from a USB device without
/// needing to access the VFS directly.
pub fn read_file(device_index: usize, path: &str) -> Result<Vec<u8>, StorageError> {
    use crate::fs::vfs::OpenFlags;
    
    let manager = &mut *USB_STORAGE_MANAGER.lock();
    
    let vfs = manager
        .get_vfs(device_index)
        .ok_or(StorageError::NotFound)?;

    // Open the file
    let fd = vfs
        .open(path, OpenFlags::from_mode("r").map_err(|_| StorageError::InvalidArgument)?)
        .map_err(|_| StorageError::IoError)?;

    // Get file size
    let stat = vfs.stat(path).map_err(|_| StorageError::IoError)?;
    let size = stat.size as usize;

    if size == 0 {
        vfs.close(fd).ok();
        return Ok(Vec::new());
    }

    // Read file data
    let mut data = vec![0u8; size];
    let mut total_read = 0;

    while total_read < size {
        let bytes_read = vfs
            .read(fd, &mut data[total_read..])
            .map_err(|_| StorageError::IoError)?;
        
        if bytes_read == 0 {
            break;
        }
        total_read += bytes_read;
    }

    // Close file
    vfs.close(fd).ok();

    // Truncate to actual bytes read
    data.truncate(total_read);
    Ok(data)
}

/// Write a file to a mounted USB storage device
/// 
/// Convenience function to write a file to a USB device.
pub fn write_file(device_index: usize, path: &str, data: &[u8]) -> Result<(), StorageError> {
    use crate::fs::vfs::OpenFlags;
    
    let manager = &mut *USB_STORAGE_MANAGER.lock();
    
    let vfs = manager
        .get_vfs(device_index)
        .ok_or(StorageError::NotFound)?;

    // Open the file for writing
    let fd = vfs
        .open(path, OpenFlags::from_mode("w").map_err(|_| StorageError::InvalidArgument)?)
        .map_err(|_| StorageError::IoError)?;

    // Write data
    let mut total_written = 0;
    while total_written < data.len() {
        let bytes_written = vfs
            .write(fd, &data[total_written..])
            .map_err(|_| StorageError::IoError)?;
        
        if bytes_written == 0 {
            return Err(StorageError::IoError);
        }
        total_written += bytes_written;
    }

    // Close file
    vfs.close(fd).ok();

    Ok(())
}

/// List directory contents on a mounted USB storage device
pub fn list_dir(device_index: usize, path: &str) -> Result<Vec<DirEntry>, StorageError> {
    use crate::fs::vfs::OpenFlags;
    
    let manager = &mut *USB_STORAGE_MANAGER.lock();
    
    let vfs = manager
        .get_vfs(device_index)
        .ok_or(StorageError::NotFound)?;

    // For now, return not implemented
    // Full implementation would require readdir support in VFS
    Err(StorageError::InvalidArgument)
}

/// Directory entry for USB storage
#[derive(Debug, Clone)]
pub struct DirEntry {
    /// Entry name
    pub name: String,
    /// Full path
    pub path: String,
    /// Whether this is a directory
    pub is_dir: bool,
    /// File size (0 for directories)
    pub size: u64,
}

/// Poll USB storage devices for changes
/// 
/// This should be called periodically to check for:
/// - New USB devices that need mounting
/// - Removed devices that need unmounting
pub fn poll() {
    // In a full implementation, this would:
    // 1. Check for new devices from the USB subsystem
    // 2. Attempt to mount unmounted devices
    // 3. Clean up disconnected devices
    
    // For now, the USB subsystem handles this via callbacks
}

// Legacy callback functions for backward compatibility

/// Called when a USB storage device is mounted or mount failed
/// 
/// This is called by the USB mass storage driver when a device is
/// detected and an attempt is made to auto-mount it.
pub fn usb_storage_mounted(device_index: usize, success: bool) {
    // Delegate to the manager's internal tracking
    // This function exists for backward compatibility
    let mut manager = USB_STORAGE_MANAGER.lock();
    
    if device_index < manager.device_info.len() {
        if success {
            println!("[usb-storage] USB storage device {} mounted successfully", device_index);
        } else {
            println!("[usb-storage] USB storage device {} mount failed", device_index);
        }
    }
}

/// Called when a USB storage device is disconnected
/// 
/// This is called by the USB mass storage driver when a device
/// is unplugged.
pub fn usb_storage_disconnected(device_index: usize) {
    disconnect(device_index);
}
