//! Icon Cache System
//!
//! Caches loaded PNG icons for efficient reuse.

use alloc::vec::Vec;
use alloc::string::{String, ToString};
use alloc::collections::BTreeMap;
use alloc::boxed::Box;
use spin::Mutex;
use lazy_static::lazy_static;

use crate::graphics::png::{PngImage, load_png_from_file, decode_png};
use crate::println;

/// Cached icon data
#[derive(Clone)]
pub struct CachedIcon {
    pub width: u32,
    pub height: u32,
    pub rgba_data: Vec<u8>,
}

impl CachedIcon {
    /// Create from PNG image
    pub fn from_png(png: &PngImage) -> Self {
        Self {
            width: png.width,
            height: png.height,
            rgba_data: png.rgba_data.clone(),
        }
    }

    /// Create from raw RGBA data
    pub fn from_rgba(width: u32, height: u32, data: Vec<u8>) -> Self {
        Self {
            width,
            height,
            rgba_data: data,
        }
    }
}

/// Icon cache
pub struct IconCache {
    cache: BTreeMap<String, CachedIcon>,
    max_size: usize,
    current_size: usize,
}

impl IconCache {
    /// Create new icon cache
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: BTreeMap::new(),
            max_size,
            current_size: 0,
        }
    }

    /// Get icon from cache or load from filesystem
    pub fn get_icon(&mut self, path: &str) -> Option<CachedIcon> {
        // Check cache first
        if let Some(icon) = self.cache.get(path) {
            return Some(icon.clone());
        }

        // Try to load from filesystem
        match load_png_from_file(path) {
            Ok(png) => {
                let icon = CachedIcon::from_png(&png);
                
                // Add to cache if there's room
                let icon_size = icon.rgba_data.len();
                if self.current_size + icon_size <= self.max_size {
                    self.cache.insert(path.to_string(), icon.clone());
                    self.current_size += icon_size;
                }
                
                Some(icon)
            }
            Err(e) => {
                println!("[icon_cache] Failed to load icon {}: {:?}", path, e);
                None
            }
        }
    }

    /// Get icon from embedded data (for built-in icons)
    pub fn get_embedded_icon(&mut self, name: &str, rgba_data: &[u8], width: u32, height: u32) -> CachedIcon {
        // Check cache first
        if let Some(icon) = self.cache.get(name) {
            return icon.clone();
        }

        // Create from embedded data
        let icon = CachedIcon::from_rgba(width, height, rgba_data.to_vec());
        
        // Add to cache
        let icon_size = icon.rgba_data.len();
        if self.current_size + icon_size <= self.max_size {
            self.cache.insert(name.to_string(), icon.clone());
            self.current_size += icon_size;
        }
        
        icon
    }

    /// Preload an icon into cache
    pub fn preload(&mut self, path: &str) -> bool {
        if self.cache.contains_key(path) {
            return true;
        }

        match load_png_from_file(path) {
            Ok(png) => {
                let icon = CachedIcon::from_png(&png);
                let icon_size = icon.rgba_data.len();
                
                if self.current_size + icon_size <= self.max_size {
                    self.cache.insert(path.to_string(), icon);
                    self.current_size += icon_size;
                    true
                } else {
                    false
                }
            }
            Err(_) => false,
        }
    }

    /// Clear cache
    pub fn clear(&mut self) {
        self.cache.clear();
        self.current_size = 0;
    }

    /// Get cache stats
    pub fn stats(&self) -> (usize, usize) {
        (self.cache.len(), self.current_size)
    }
}

/// Global icon cache (1MB max)
lazy_static! {
    static ref ICON_CACHE: Mutex<IconCache> = Mutex::new(IconCache::new(1024 * 1024));
}

/// Get icon from cache or filesystem
pub fn get_icon(path: &str) -> Option<CachedIcon> {
    ICON_CACHE.lock().get_icon(path)
}

/// Get icon from embedded data
pub fn get_embedded_icon(name: &str, rgba_data: &[u8], width: u32, height: u32) -> CachedIcon {
    ICON_CACHE.lock().get_embedded_icon(name, rgba_data, width, height)
}

/// Preload icon into cache
pub fn preload_icon(path: &str) -> bool {
    ICON_CACHE.lock().preload(path)
}

/// Clear icon cache
pub fn clear_cache() {
    ICON_CACHE.lock().clear();
}

/// Get cache stats (entries, bytes)
pub fn cache_stats() -> (usize, usize) {
    ICON_CACHE.lock().stats()
}

/// Initialize icon cache and preload common icons
pub fn init() {
    println!("[icon_cache] Initializing...");
    
    // Preload common icons
    let common_icons = [
        "system/icons/globe_icon_64.png",
        "system/icons/filemanager_icon_64.png",
        "system/icons/folder_icon_64.png",
        "system/icons/file_icon_64.png",
    ];
    
    let mut loaded = 0;
    for path in &common_icons {
        if preload_icon(path) {
            loaded += 1;
        }
    }
    
    let (entries, bytes) = cache_stats();
    println!("[icon_cache] Loaded {}/{} icons, {} entries, {} bytes", 
        loaded, common_icons.len(), entries, bytes);
}

/// Try to decode PNG from bytes (for testing)
pub fn decode_icon_bytes(data: &[u8]) -> Option<CachedIcon> {
    match decode_png(data) {
        Ok(png) => Some(CachedIcon::from_png(&png)),
        Err(e) => {
            println!("[icon_cache] PNG decode failed: {:?}", e);
            None
        }
    }
}
