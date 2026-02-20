//! Network drivers
//!
//! VirtIO network device driver implementation.

use crate::println;

pub mod virtio_net;

/// Initialize network drivers
pub fn init() {
    println!("[net/drivers] Initializing network drivers...");

    // Try to initialize VirtIO net (x86_64 only)
    #[cfg(target_arch = "x86_64")]
    virtio_net::init();
    
    #[cfg(not(target_arch = "x86_64"))]
    println!("[net/drivers] VirtIO net not available on this platform");

    println!("[net/drivers] Network drivers initialized");
}
