//! Network drivers
//!
//! VirtIO network device driver implementation.

use crate::println;

pub mod virtio_net;

/// Initialize network drivers
pub fn init() {
    println!("[net/drivers] Initializing network drivers...");

    // Try to initialize VirtIO net
    virtio_net::init();

    println!("[net/drivers] Network drivers initialized");
}
