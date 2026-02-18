// Simple FAT32 file reader/writer for boot disk
// Delegates to global_vfs when available

use alloc::vec::Vec;
use alloc::string::String;
use alloc::format;
use crate::println;
use crate::fs::global_vfs;

/// Read a file from the boot disk
/// 
/// First tries the global VFS, falls back to stub if not available
pub fn read_file(path: &str) -> Option<Vec<u8>> {
    println!("[boot_disk] Attempting to read file: {}", path);

    // Try global VFS first
    if global_vfs::is_ready() {
        match global_vfs::read_file(path) {
            Ok(data) => {
                println!("[boot_disk] Read {} bytes from {}", data.len(), path);
                return Some(data);
            }
            Err(e) => {
                println!("[boot_disk] VFS read failed: {:?}", e);
                return None;
            }
        }
    }

    // VFS not available - this is expected during early boot
    println!("[boot_disk] VFS not ready, file read unavailable");
    None
}

/// Write a file to the boot disk
/// 
/// Uses global VFS when available
pub fn write_file(path: &str, data: &[u8]) -> Result<(), String> {
    println!("[boot_disk] Attempting to write file: {} ({} bytes)", path, data.len());

    // Try global VFS first
    if global_vfs::is_ready() {
        return global_vfs::write_file(path, data)
            .map_err(|e| format!("{:?}", e));
    }

    Err(String::from("VFS not initialized - cannot write files"))
}

/// Save data to the Desktop folder
/// 
/// This is a convenience function for apps to save files to the Desktop.
/// The file will be created in the `/Desktop` folder.
/// 
/// # Arguments
/// * `filename` - Name of the file to create
/// * `data` - File contents
/// 
/// # Returns
/// * `Ok(())` - File saved successfully
/// * `Err(String)` - Error message if save failed
pub fn save_to_desktop(filename: &str, data: &[u8]) -> Result<(), String> {
    // Sanitize filename
    let safe_name = sanitize_filename(filename);
    if safe_name.is_empty() {
        return Err(String::from("Invalid filename"));
    }
    
    // Try global VFS first
    if global_vfs::is_ready() {
        return global_vfs::save_to_desktop(&safe_name, data)
            .map_err(|e| format!("{:?}", e));
    }

    Err(String::from("VFS not initialized - cannot save files"))
}

/// Create a directory
pub fn create_directory(path: &str) -> Result<(), String> {
    println!("[boot_disk] Creating directory: {}", path);
    
    if global_vfs::is_ready() {
        return global_vfs::create_dir(path)
            .map_err(|e| format!("{:?}", e));
    }

    Err(String::from("VFS not initialized"))
}

/// Check if a file exists
pub fn file_exists(path: &str) -> bool {
    if global_vfs::is_ready() {
        return global_vfs::file_exists(path);
    }
    
    // VFS not available
    false
}

/// Sanitize a filename to prevent directory traversal attacks
fn sanitize_filename(name: &str) -> String {
    // Remove any path components, keep only the filename
    let name = name.replace("..", "_");
    let name = name.replace("/", "_");
    let name = name.replace("\\", "_");
    
    // Trim whitespace
    let name = name.trim();
    
    // Limit length
    if name.len() > 255 {
        String::from(&name[..255])
    } else {
        String::from(name)
    }
}

/// Initialize boot disk file operations
/// 
/// This should be called after the storage subsystem is initialized
pub unsafe fn init() {
    println!("[boot_disk] Initializing boot disk file operations...");
    
    // Initialize global VFS
    if let Err(e) = global_vfs::init() {
        println!("[boot_disk] Warning: Failed to initialize VFS: {:?}", e);
        println!("[boot_disk] File operations will be unavailable");
    } else {
        println!("[boot_disk] Boot disk file operations ready");
    }
}
