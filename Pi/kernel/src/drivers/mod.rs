//! Device drivers
//!
//! Hardware-specific drivers for various devices.

pub mod timer;
pub mod pci;
pub mod storage;
pub mod input;
pub mod usb;

// Raspberry Pi specific drivers
pub mod mailbox;
pub mod display;
pub mod sdio;
pub mod wifi;

// VESA compatibility layer - re-exports display with VESA-compatible API
// This allows existing x86_64 code to work with minimal changes
pub mod vesa {
    //! VESA compatibility layer for Raspberry Pi
    //! 
    //! This module re-exports the Pi display driver with a VESA-compatible API
    //! to minimize changes needed when porting code from x86_64.
    
    pub use crate::drivers::display::pi_framebuffer::{
        driver,
        clear,
        set_pixel,
        fill_rect,
        draw_text,
        print_info,
        colors,
        PiFramebuffer as VesaDriver,
        FramebufferInfo,
    };
    
    use crate::drivers::display::pi_framebuffer;
    
    /// Initialize VESA-compatible framebuffer
    /// 
    /// This function initializes the Pi framebuffer through the mailbox interface.
    pub fn init(width: u32, height: u32, bpp: u8) {
        crate::println!("[vesa] Initializing Pi framebuffer (VESA compatible mode)...");
        pi_framebuffer::init(width, height, bpp);
    }
    
    /// Initialize with pitch and virtual address (compatibility)
    /// 
    /// On Pi, the framebuffer is allocated by the GPU, so phys_addr and virt_addr
    /// are ignored. The mailbox interface is used instead.
    pub fn init_with_pitch(width: u32, height: u32, bpp: u8, _pitch: u32, _phys_addr: u64, _virt_addr: u64) {
        crate::println!("[vesa] Initializing Pi framebuffer (VESA compatible mode)...");
        pi_framebuffer::init(width, height, bpp);
    }
    
    /// Initialize with virtual address (compatibility)
    pub fn init_with_virt_addr(width: u32, height: u32, bpp: u8, _phys_addr: u64, _virt_addr: u64) {
        pi_framebuffer::init(width, height, bpp);
    }
}

use crate::println;

/// Initialize all drivers
pub fn init() {
    println!("[drivers] Initializing device drivers...");
    
    // Initialize mailbox early (needed for GPU communication on Pi)
    mailbox::init();
    
    timer::init();
    pci::init();
    // Storage drivers initialized separately after PCI enumeration
    
    // Initialize USB subsystem (for Raspberry Pi)
    usb::init();
    
    println!("[drivers] Device drivers initialized");
}

/// Initialize Raspberry Pi specific drivers (SDIO, WiFi)
/// 
/// # Arguments
/// * `pi4` - Set to true for Raspberry Pi 4, false for Pi 3
pub fn init_pi_drivers(pi4: bool) {
    println!("[drivers] Initializing Raspberry Pi specific drivers (Pi 4: {})...", pi4);
    
    // Initialize WiFi (which will also initialize SDIO if needed)
    wifi::init(pi4);
    
    println!("[drivers] Raspberry Pi drivers initialized");
}

/// Driver error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverError {
    /// Success
    Success = 0,
    /// Device not found
    NotFound = 1,
    /// Initialization failed
    InitFailed = 2,
    /// IO error
    IoError = 3,
    /// Unsupported operation
    Unsupported = 4,
    /// Timeout
    Timeout = 5,
    /// Unknown error
    Unknown = 255,
}

/// Result type for driver operations
pub type DriverResult<T> = Result<T, DriverError>;
