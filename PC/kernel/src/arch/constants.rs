//! x86_64 hardware address constants

/// VGA text mode buffer address
pub const VGA_TEXT_BUFFER_ADDR: usize = 0xB8000;

/// VGA text buffer size (80x25 = 2000 characters, 2 bytes each)
pub const VGA_TEXT_BUFFER_SIZE: usize = 4000;

/// Screen dimensions for text mode
pub const VGA_WIDTH: usize = 80;
pub const VGA_HEIGHT: usize = 25;

/// Higher half kernel base address
pub const KERNEL_BASE: usize = 0xFFFF800000000000;

/// Framebuffer virtual address (when mapped at 2GB in higher half)
pub const FRAMEBUFFER_VIRT_BASE: usize = 0xFFFF800080000000;

/// Framebuffer default physical address (QEMU VESA at 2GB)
pub const FRAMEBUFFER_PHYS_ADDR: usize = 0x80000000;

/// Default framebuffer dimensions
pub const FRAMEBUFFER_DEFAULT_WIDTH: u32 = 1280;
pub const FRAMEBUFFER_DEFAULT_HEIGHT: u32 = 800;
pub const FRAMEBUFFER_DEFAULT_BPP: u8 = 32;

/// Default kernel stack top (virtual)
pub const KERNEL_STACK_TOP: usize = 0xFFFF800000500000;
