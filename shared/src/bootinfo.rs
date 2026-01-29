//! Boot Information Structure
//! 
//! Passed from bootloader to kernel at boot time.
//! This structure is placed at a known location in memory
//! and contains all information needed to initialize the kernel.

use crate::types::{MemoryRegion, PhysAddr, VirtAddr};

/// Magic number to identify valid boot info
pub const BOOTINFO_MAGIC: u64 = 0x1BAD_B002_0B0B_0055;

/// Boot protocol version
pub const BOOTINFO_VERSION: u32 = 1;

/// Boot information structure passed from bootloader to kernel
/// 
/// # Safety
/// This structure is placed in memory by the bootloader and
/// passed to the kernel. The kernel must not modify it until
/// it has copied any needed information.
#[derive(Debug)]
#[repr(C, align(8))]
pub struct BootInfo {
    /// Magic number (must be BOOTINFO_MAGIC)
    pub magic: u64,
    /// Boot protocol version
    pub version: u32,
    /// Reserved (padding)
    pub _reserved: u32,
    /// Physical address of memory map
    pub memory_map_addr: PhysAddr,
    /// Number of memory map entries
    pub memory_map_count: usize,
    /// Physical address of kernel
    pub kernel_addr: PhysAddr,
    /// Size of kernel in bytes
    pub kernel_size: u64,
    /// Virtual address where kernel is mapped
    pub kernel_virt_addr: VirtAddr,
    /// Framebuffer information
    pub framebuffer: FramebufferInfo,
    /// Physical address of RSDP (ACPI)
    pub rsdp_addr: Option<PhysAddr>,
    /// Command line string (null-terminated)
    pub cmdline: Option<PhysAddr>,
    /// Bootloader name string (null-terminated)
    pub bootloader_name: PhysAddr,
    /// Stack top address (virtual)
    pub stack_top: VirtAddr,
    /// Stack size
    pub stack_size: u64,
}

impl BootInfo {
    /// Verify that this boot info is valid
    pub fn verify(&self) -> bool {
        self.magic == BOOTINFO_MAGIC && self.version == BOOTINFO_VERSION
    }

    /// Get memory map as a slice
    /// 
    /// # Safety
    /// Caller must ensure the memory map is valid and accessible
    pub unsafe fn memory_map(&self) -> &[MemoryRegion] {
        if self.memory_map_count == 0 {
            return &[];
        }
        let ptr = self.memory_map_addr.as_ptr::<MemoryRegion>();
        core::slice::from_raw_parts(ptr, self.memory_map_count)
    }

    /// Get bootloader name as a string slice
    /// 
    /// # Safety
    /// Caller must ensure the string is valid UTF-8 and null-terminated
    pub unsafe fn bootloader_name(&self) -> &str {
        let ptr = self.bootloader_name.as_ptr::<u8>();
        let mut len = 0;
        while *ptr.add(len) != 0 {
            len += 1;
        }
        let slice = core::slice::from_raw_parts(ptr, len);
        core::str::from_utf8_unchecked(slice)
    }

    /// Get command line as a string slice
    /// 
    /// # Safety
    /// Caller must ensure the string is valid UTF-8 and null-terminated
    pub unsafe fn cmdline(&self) -> Option<&str> {
        let addr = self.cmdline?;
        let ptr = addr.as_ptr::<u8>();
        let mut len = 0;
        while *ptr.add(len) != 0 {
            len += 1;
        }
        let slice = core::slice::from_raw_parts(ptr, len);
        Some(core::str::from_utf8_unchecked(slice))
    }
}

/// Framebuffer information
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct FramebufferInfo {
    /// Physical address of framebuffer
    pub addr: PhysAddr,
    /// Virtual address of framebuffer (if mapped)
    pub virt_addr: Option<VirtAddr>,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Bits per pixel
    pub bpp: u32,
    /// Bytes per scanline (may include padding)
    pub pitch: u32,
    /// Pixel format
    pub format: PixelFormat,
}

impl FramebufferInfo {
    /// Check if a framebuffer is available
    pub fn is_valid(&self) -> bool {
        self.width > 0 && self.height > 0 && self.addr.as_u64() != 0
    }

    /// Calculate framebuffer size in bytes
    pub fn size(&self) -> usize {
        (self.height as usize) * (self.pitch as usize)
    }
}

impl Default for FramebufferInfo {
    fn default() -> Self {
        Self {
            addr: PhysAddr::new(0),
            virt_addr: None,
            width: 0,
            height: 0,
            bpp: 0,
            pitch: 0,
            format: PixelFormat::Rgb,
        }
    }
}

/// Pixel format for framebuffer
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum PixelFormat {
    /// RGB (8 bits per channel)
    Rgb = 1,
    /// BGR (8 bits per channel)
    Bgr = 2,
    /// Grayscale
    Grayscale = 3,
}

/// Boot info error types
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BootInfoError {
    InvalidMagic,
    InvalidVersion,
    NullPointer,
}

/// Result type for boot info operations
pub type BootInfoResult<T> = core::result::Result<T, BootInfoError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bootinfo_verify() {
        let bootinfo = BootInfo {
            magic: BOOTINFO_MAGIC,
            version: BOOTINFO_VERSION,
            _reserved: 0,
            memory_map_addr: PhysAddr::new(0),
            memory_map_count: 0,
            kernel_addr: PhysAddr::new(0),
            kernel_size: 0,
            kernel_virt_addr: VirtAddr::new(0),
            framebuffer: FramebufferInfo::default(),
            rsdp_addr: None,
            cmdline: None,
            bootloader_name: PhysAddr::new(0),
            stack_top: VirtAddr::new(0),
            stack_size: 0,
        };

        assert!(bootinfo.verify());
    }

    #[test]
    fn test_framebuffer_size() {
        let fb = FramebufferInfo {
            addr: PhysAddr::new(0x1000),
            virt_addr: None,
            width: 1920,
            height: 1080,
            bpp: 32,
            pitch: 1920 * 4,
            format: PixelFormat::Rgb,
        };

        assert_eq!(fb.size(), 1920 * 4 * 1080);
    }
}
