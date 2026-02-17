//! Desktop Wallpaper Support
//!
//! Loads and displays wallpaper images from the filesystem.
//! Supports BMP and PPM formats with fallback to gradient.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::vec;
use crate::println;

/// Wallpaper image data
#[derive(Debug, Clone)]
pub struct Wallpaper {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u32>, // ARGB format
}

/// Wallpaper load error
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WallpaperError {
    NotFound,
    InvalidFormat,
    UnsupportedBpp,
    DecodeError,
}

impl Wallpaper {
    /// Create a new empty wallpaper
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; (width * height) as usize],
        }
    }

    /// Get pixel at (x, y)
    pub fn get_pixel(&self, x: u32, y: u32) -> u32 {
        if x >= self.width || y >= self.height {
            return 0;
        }
        self.pixels[(y * self.width + x) as usize]
    }

    /// Set pixel at (x, y)
    pub fn set_pixel(&mut self, x: u32, y: u32, color: u32) {
        if x < self.width && y < self.height {
            self.pixels[(y * self.width + x) as usize] = color;
        }
    }

    /// Load wallpaper from file data
    pub fn from_bytes(data: &[u8]) -> Result<Self, WallpaperError> {
        // Detect format by magic number
        if data.len() < 2 {
            return Err(WallpaperError::InvalidFormat);
        }

        // BMP: "BM" magic
        if data[0] == b'B' && data[1] == b'M' {
            return Self::decode_bmp(data);
        }
        
        // PPM: "P6" magic (binary PPM)
        if data[0] == b'P' && data[1] == b'6' {
            return Self::decode_ppm(data);
        }

        Err(WallpaperError::InvalidFormat)
    }

    /// Decode BMP image (24-bit and 32-bit supported)
    fn decode_bmp(data: &[u8]) -> Result<Self, WallpaperError> {
        if data.len() < 54 {
            return Err(WallpaperError::InvalidFormat);
        }

        // BMP header
        let file_size = u32::from_le_bytes([data[2], data[3], data[4], data[5]]) as usize;
        let data_offset = u32::from_le_bytes([data[10], data[11], data[12], data[13]]) as usize;
        
        // DIB header
        let header_size = u32::from_le_bytes([data[14], data[15], data[16], data[17]]) as usize;
        let width = i32::from_le_bytes([data[18], data[19], data[20], data[21]]) as u32;
        let height = i32::from_le_bytes([data[22], data[23], data[24], data[25]]) as u32;
        let bpp = u16::from_le_bytes([data[28], data[29]]) as u32;
        let compression = u32::from_le_bytes([data[30], data[31], data[32], data[33]]);

        if width == 0 || height == 0 {
            return Err(WallpaperError::InvalidFormat);
        }

        if compression != 0 {
            // Compressed BMP not supported
            return Err(WallpaperError::UnsupportedBpp);
        }

        let mut wallpaper = Self::new(width, height);
        let row_size = ((bpp * width + 31) / 32) * 4; // Padded to 4-byte boundary

        match bpp {
            24 => {
                for y in 0..height {
                    for x in 0..width {
                        let src_y = height - 1 - y; // BMP is stored bottom-up
                        let offset = data_offset + (src_y as usize * row_size as usize) + (x as usize * 3);
                        
                        if offset + 2 < data.len() {
                            let b = data[offset] as u32;
                            let g = data[offset + 1] as u32;
                            let r = data[offset + 2] as u32;
                            let color = 0xFF000000 | (r << 16) | (g << 8) | b;
                            wallpaper.set_pixel(x, y, color);
                        }
                    }
                }
            }
            32 => {
                for y in 0..height {
                    for x in 0..width {
                        let src_y = height - 1 - y; // BMP is stored bottom-up
                        let offset = data_offset + (src_y as usize * row_size as usize) + (x as usize * 4);
                        
                        if offset + 3 < data.len() {
                            let b = data[offset] as u32;
                            let g = data[offset + 1] as u32;
                            let r = data[offset + 2] as u32;
                            let a = data[offset + 3] as u32;
                            let color = (a << 24) | (r << 16) | (g << 8) | b;
                            wallpaper.set_pixel(x, y, color);
                        }
                    }
                }
            }
            _ => return Err(WallpaperError::UnsupportedBpp),
        }

        Ok(wallpaper)
    }

    /// Decode PPM image (P6 binary format)
    fn decode_ppm(data: &[u8]) -> Result<Self, WallpaperError> {
        // Parse PPM header
        let mut pos = 2; // Skip "P6"
        
        // Skip whitespace and comments
        while pos < data.len() {
            let c = data[pos];
            if c == b'#' {
                // Skip comment line
                while pos < data.len() && data[pos] != b'\n' {
                    pos += 1;
                }
            } else if c.is_ascii_whitespace() {
                pos += 1;
            } else {
                break;
            }
        }

        // Read width
        let mut width = 0u32;
        while pos < data.len() && data[pos].is_ascii_digit() {
            width = width * 10 + (data[pos] - b'0') as u32;
            pos += 1;
        }

        // Skip whitespace
        while pos < data.len() && data[pos].is_ascii_whitespace() {
            pos += 1;
        }

        // Read height
        let mut height = 0u32;
        while pos < data.len() && data[pos].is_ascii_digit() {
            height = height * 10 + (data[pos] - b'0') as u32;
            pos += 1;
        }

        // Skip whitespace
        while pos < data.len() && data[pos].is_ascii_whitespace() {
            pos += 1;
        }

        // Read max value (should be 255)
        let mut maxval = 0u32;
        while pos < data.len() && data[pos].is_ascii_digit() {
            maxval = maxval * 10 + (data[pos] - b'0') as u32;
            pos += 1;
        }

        // Skip single whitespace after maxval
        if pos < data.len() && data[pos].is_ascii_whitespace() {
            pos += 1;
        }

        if width == 0 || height == 0 || maxval != 255 {
            return Err(WallpaperError::InvalidFormat);
        }

        let mut wallpaper = Self::new(width, height);
        let pixel_data = &data[pos..];
        
        // Read RGB data
        for y in 0..height {
            for x in 0..width {
                let offset = ((y * width + x) * 3) as usize;
                if offset + 2 < pixel_data.len() {
                    let r = pixel_data[offset] as u32;
                    let g = pixel_data[offset + 1] as u32;
                    let b = pixel_data[offset + 2] as u32;
                    let color = 0xFF000000 | (r << 16) | (g << 8) | b;
                    wallpaper.set_pixel(x, y, color);
                }
            }
        }

        Ok(wallpaper)
    }

    /// Create a gradient wallpaper as fallback
    pub fn create_gradient(width: u32, height: u32, color1: u32, color2: u32) -> Self {
        let mut wallpaper = Self::new(width, height);

        let r1 = ((color1 >> 16) & 0xFF) as i32;
        let g1 = ((color1 >> 8) & 0xFF) as i32;
        let b1 = (color1 & 0xFF) as i32;

        let r2 = ((color2 >> 16) & 0xFF) as i32;
        let g2 = ((color2 >> 8) & 0xFF) as i32;
        let b2 = (color2 & 0xFF) as i32;

        for y in 0..height {
            let t = y as f32 / height as f32;
            let r = (r1 as f32 + (r2 - r1) as f32 * t) as u32;
            let g = (g1 as f32 + (g2 - g1) as f32 * t) as u32;
            let b = (b1 as f32 + (b2 - b1) as f32 * t) as u32;
            let color = 0xFF000000 | (r << 16) | (g << 8) | b;

            for x in 0..width {
                wallpaper.set_pixel(x, y, color);
            }
        }

        wallpaper
    }

    /// Resize wallpaper to fit screen (simple nearest neighbor)
    pub fn resize(&self, new_width: u32, new_height: u32) -> Self {
        let mut resized = Self::new(new_width, new_height);

        for y in 0..new_height {
            for x in 0..new_width {
                let src_x = (x * self.width) / new_width;
                let src_y = (y * self.height) / new_height;
                let color = self.get_pixel(src_x, src_y);
                resized.set_pixel(x, y, color);
            }
        }

        resized
    }

    /// Resize to fill screen while maintaining aspect ratio (crop)
    pub fn resize_cover(&self, screen_width: u32, screen_height: u32) -> Self {
        let img_ratio = self.width as f32 / self.height as f32;
        let screen_ratio = screen_width as f32 / screen_height as f32;

        let (new_width, new_height, offset_x, offset_y) = if img_ratio > screen_ratio {
            // Image is wider, crop sides
            let h = screen_height;
            let w = ((screen_height as f32) * img_ratio) as u32;
            let x = (w - screen_width) / 2;
            (w, h, x, 0)
        } else {
            // Image is taller, crop top/bottom
            let w = screen_width;
            let h = ((screen_width as f32) / img_ratio) as u32;
            let y = (h - screen_height) / 2;
            (w, h, 0, y)
        };

        let scaled = self.resize(new_width, new_height);
        
        // Crop to screen size
        let mut cropped = Self::new(screen_width, screen_height);
        for y in 0..screen_height {
            for x in 0..screen_width {
                let src_x = x + offset_x;
                let src_y = y + offset_y;
                if src_x < new_width && src_y < new_height {
                    let color = scaled.get_pixel(src_x, src_y);
                    cropped.set_pixel(x, y, color);
                }
            }
        }

        cropped
    }
}

/// Default wallpaper paths to try
const DEFAULT_WALLPAPER_PATHS: &[&str] = &[
    "/system/wallpapers/default.bmp",
    "/system/wallpapers/default.ppm",
    "/System/Wallpaper/default.bmp",
    "/System/Wallpaper/default.ppm",
];

/// Load wallpaper from filesystem
/// Returns Ok(Wallpaper) if loaded, Err if not found
#[cfg(feature = "filesystem")]
pub fn load_wallpaper() -> Result<Wallpaper, WallpaperError> {
    // Try to load from filesystem
    for path in DEFAULT_WALLPAPER_PATHS {
        match try_load_file(path) {
            Ok(data) => {
                match Wallpaper::from_bytes(&data) {
                    Ok(mut wallpaper) => {
                        println!("[wallpaper] Loaded wallpaper from {}", path);
                        // Resize to screen resolution (1280x800 default)
                        wallpaper = wallpaper.resize_cover(1280, 800);
                        return Ok(wallpaper);
                    }
                    Err(e) => {
                        println!("[wallpaper] Failed to decode {}: {:?}", path, e);
                    }
                }
            }
            Err(_) => {
                // File not found, try next
            }
        }
    }

    Err(WallpaperError::NotFound)
}

/// Load wallpaper (stub when filesystem not available)
#[cfg(not(feature = "filesystem"))]
pub fn load_wallpaper() -> Result<Wallpaper, WallpaperError> {
    Err(WallpaperError::NotFound)
}

/// Try to load a file from the filesystem
#[cfg(feature = "filesystem")]
fn try_load_file(path: &str) -> Result<Vec<u8>, ()> {
    use crate::fs::vfs::{Vfs, VfsOperations, OpenFlags};
    use crate::fs::block::VirtualBlockDevice;
    
    // This is a simplified version - in practice, you'd access the global VFS
    // For now, we return an error to trigger the fallback
    Err(())
}

/// Try to load a file (stub when filesystem not available)
#[cfg(not(feature = "filesystem"))]
fn try_load_file(path: &str) -> Result<Vec<u8>, ()> {
    Err(())
}

/// Get default gradient wallpaper
pub fn default_gradient(width: u32, height: u32) -> Wallpaper {
    // Nice blue-purple gradient
    let color1 = 0x1a1a2e; // Dark blue
    let color2 = 0x16213e; // Slightly lighter blue
    Wallpaper::create_gradient(width, height, color1, color2)
}

/// Initialize wallpaper and return it
pub fn init_wallpaper(screen_width: u32, screen_height: u32) -> Wallpaper {
    // Try to load from filesystem
    match load_wallpaper() {
        Ok(wallpaper) => wallpaper,
        Err(_) => {
            println!("[wallpaper] Using default gradient");
            default_gradient(screen_width, screen_height)
        }
    }
}
