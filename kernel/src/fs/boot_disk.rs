// Simple FAT32 file reader for boot disk
// This is a minimal implementation to read/write files from the boot disk

use alloc::vec::Vec;
use alloc::string::{String, ToString};
use alloc::format;
use crate::println;

/// Read a file from the boot disk (stub for now)
/// TODO: Implement actual FAT32 reading from boot disk
pub fn read_file(path: &str) -> Option<Vec<u8>> {
    println!("[boot_disk] Attempting to read file: {}", path);

    // For now, return None - this needs proper implementation
    // Will need to:
    // 1. Access the boot disk (ATA/AHCI/NVMe)
    // 2. Read FAT32 structures
    // 3. Locate the file
    // 4. Read file data

    None
}

/// Write a file to the boot disk (stub for now)
/// 
/// # Arguments
/// * `path` - Path to the file (relative to disk root)
/// * `data` - File contents to write
/// 
/// # Returns
/// * `Ok(())` - File written successfully
/// * `Err(String)` - Error message if write failed
/// 
/// TODO: Implement actual FAT32 writing to boot disk
pub fn write_file(path: &str, data: &[u8]) -> Result<(), String> {
    println!("[boot_disk] Attempting to write file: {} ({} bytes)", path, data.len());

    // For now, return an error - this needs proper implementation
    // Will need to:
    // 1. Access the boot disk (ATA/AHCI/NVMe)
    // 2. Read FAT32 structures
    // 3. Find or create the file
    // 4. Allocate clusters if needed
    // 5. Write file data
    // 6. Update directory entry
    // 7. Update FAT tables

    Err(String::from("File writing not yet implemented - filesystem integration in progress"))
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
    
    let path = format!("Desktop/{}", safe_name);
    write_file(&path, data)
}

/// Create a directory (stub)
pub fn create_directory(path: &str) -> Result<(), String> {
    println!("[boot_disk] Creating directory: {}", path);
    Err(String::from("Directory creation not yet implemented"))
}

/// Check if a file exists (stub)
pub fn file_exists(path: &str) -> bool {
    // Try to read the file - if successful, it exists
    read_file(path).is_some()
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
