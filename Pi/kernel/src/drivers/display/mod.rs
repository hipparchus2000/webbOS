//! Display Drivers
//!
//! This module provides display drivers for the Raspberry Pi platform.
//! It wraps the Pi framebuffer driver to provide a compatible API
#![allow(dead_code)]

//! similar to the VESA driver used on x86_64.

pub mod pi_framebuffer;

// Re-export the Pi framebuffer types for compatibility

/// Initialize the display subsystem
/// 
/// This initializes the mailbox driver first, then allocates
/// the framebuffer through the VideoCore GPU.
pub fn init_display(width: u32, height: u32, bpp: u8) -> bool {
    crate::println!("[display] Initializing display subsystem...");
    
    // Initialize mailbox first (required for Pi framebuffer)
    crate::drivers::mailbox::init();
    
    // Initialize Pi framebuffer
    pi_framebuffer::init(width, height, bpp)
}

/// Check if display is initialized
pub fn is_initialized() -> bool {
    pi_framebuffer::driver().lock().is_initialized()
}

/// Compatibility type alias for code expecting VesaDriver
pub type VesaDriver = pi_framebuffer::PiFramebuffer;

/// Re-export colors from pi_framebuffer for VESA compatibility
pub mod vesa_colors {
    
}
