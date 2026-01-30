//! VESA/VBE Framebuffer Driver
//!
//! Graphics driver for VESA BIOS Extensions (VBE) providing
//! high-resolution framebuffer access for WebbOS desktop.

use core::ptr::{read_volatile, write_volatile};
use spin::Mutex;
use lazy_static::lazy_static;

use crate::println;
use crate::mm::phys_to_virt;
use webbos_shared::types::PhysAddr;

/// VBE 2.0+ Information Block
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct VbeInfoBlock {
    pub signature: [u8; 4],        // "VESA"
    pub version: u16,               // VBE version (0x0200 for 2.0)
    pub oem_string: u32,            // Far pointer to OEM string
    pub capabilities: u32,          // Capabilities flags
    pub video_modes: u32,           // Far pointer to video mode list
    pub total_memory: u16,          // Total memory in 64KB blocks
    // VBE 2.0+ fields
    pub oem_software_rev: u16,
    pub oem_vendor_name: u32,
    pub oem_product_name: u32,
    pub oem_product_rev: u32,
    pub reserved: [u8; 222],
    pub oem_data: [u8; 256],
}

/// VBE Mode Information Block
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct VbeModeInfo {
    // Mandatory information
    pub mode_attributes: u16,
    pub win_a_attributes: u8,
    pub win_b_attributes: u8,
    pub win_granularity: u16,
    pub win_size: u16,
    pub win_a_segment: u16,
    pub win_b_segment: u16,
    pub win_func_ptr: u32,
    pub bytes_per_scanline: u16,
    
    // Direct color fields
    pub x_resolution: u16,
    pub y_resolution: u16,
    pub x_char_size: u8,
    pub y_char_size: u8,
    pub number_of_planes: u8,
    pub bits_per_pixel: u8,
    pub number_of_banks: u8,
    pub memory_model: u8,
    pub bank_size: u8,
    pub number_of_image_pages: u8,
    pub reserved1: u8,
    
    // Direct color fields for direct/6 and YUV/7 memory models
    pub red_mask_size: u8,
    pub red_field_position: u8,
    pub green_mask_size: u8,
    pub green_field_position: u8,
    pub blue_mask_size: u8,
    pub blue_field_position: u8,
    pub rsvd_mask_size: u8,
    pub rsvd_field_position: u8,
    pub direct_color_mode_info: u8,
    
    // VBE 2.0+ fields
    pub phys_base_ptr: u32,         // Physical address for flat framebuffer
    pub reserved2: u32,
    pub reserved3: u16,
    
    // VBE 3.0+ fields
    pub linear_bytes_per_scanline: u16,
    pub bank_number_of_image_pages: u8,
    pub linear_number_of_image_pages: u8,
    pub linear_red_mask_size: u8,
    pub linear_red_field_position: u8,
    pub linear_green_mask_size: u8,
    pub linear_green_field_position: u8,
    pub linear_blue_mask_size: u8,
    pub linear_blue_field_position: u8,
    pub linear_rsvd_mask_size: u8,
    pub linear_rsvd_field_position: u8,
    pub max_pixel_clock: u32,
    
    pub reserved4: [u8; 190],
}

/// Framebuffer information
#[derive(Debug, Clone, Copy)]
pub struct FramebufferInfo {
    pub width: u32,
    pub height: u32,
    pub pitch: u32,       // Bytes per scanline
    pub bpp: u8,          // Bits per pixel
    pub bytes_per_pixel: u8,
    pub red_mask: u32,
    pub green_mask: u32,
    pub blue_mask: u32,
    pub phys_addr: u64,   // Physical address
    pub size: usize,      // Total size in bytes
}

/// VESA driver state
pub struct VesaDriver {
    pub initialized: bool,
    pub info: FramebufferInfo,
    pub fb_virt_addr: *mut u8,
}

unsafe impl Send for VesaDriver {}
unsafe impl Sync for VesaDriver {}

impl VesaDriver {
    /// Create uninitialized driver
    const fn new() -> Self {
        Self {
            initialized: false,
            info: FramebufferInfo {
                width: 0,
                height: 0,
                pitch: 0,
                bpp: 0,
                bytes_per_pixel: 0,
                red_mask: 0,
                green_mask: 0,
                blue_mask: 0,
                phys_addr: 0,
                size: 0,
            },
            fb_virt_addr: core::ptr::null_mut(),
        }
    }
    
    /// Initialize with boot-provided framebuffer info
    pub fn init(&mut self, width: u32, height: u32, bpp: u8, phys_addr: u64) {
        println!("[vesa] Initializing VESA framebuffer...");
        println!("[vesa] Resolution: {}x{} @ {}bpp", width, height, bpp);
        println!("[vesa] Physical address: 0x{:016x}", phys_addr);
        
        let bytes_per_pixel = (bpp + 7) / 8;
        let pitch = width * bytes_per_pixel as u32;
        let size = (pitch * height) as usize;
        
        // Calculate color masks based on bpp
        let (red_mask, green_mask, blue_mask) = match bpp {
            32 => (0x00FF0000, 0x0000FF00, 0x000000FF), // ARGB
            24 => (0x00FF0000, 0x0000FF00, 0x000000FF), // RGB
            16 => (0x0000F800, 0x000007E0, 0x0000001F), // RGB565
            15 => (0x00007C00, 0x000003E0, 0x0000001F), // RGB555
            _ => (0x00FF0000, 0x0000FF00, 0x000000FF),
        };
        
        self.info = FramebufferInfo {
            width,
            height,
            pitch,
            bpp,
            bytes_per_pixel,
            red_mask,
            green_mask,
            blue_mask,
            phys_addr,
            size,
        };
        
        // Map framebuffer into virtual memory
        self.fb_virt_addr = phys_to_virt(PhysAddr::new(phys_addr)).as_u64() as *mut u8;
        
        println!("[vesa] Virtual address: {:p}", self.fb_virt_addr);
        println!("[vesa] Framebuffer size: {} KB", size / 1024);
        
        // Clear framebuffer to black
        self.clear(0);
        
        self.initialized = true;
        println!("[vesa] Initialization complete");
    }
    
    /// Check if initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
    
    /// Get framebuffer info
    pub fn info(&self) -> &FramebufferInfo {
        &self.info
    }
    
    /// Clear framebuffer with color
    pub fn clear(&mut self, color: u32) {
        if !self.initialized {
            return;
        }
        
        let pixel = self.color_to_pixel(color);
        let count = (self.info.pitch * self.info.height) as usize / self.info.bytes_per_pixel as usize;
        
        unsafe {
            let fb = self.fb_virt_addr as *mut u32;
            for i in 0..count {
                write_volatile(fb.add(i), pixel);
            }
        }
    }
    
    /// Set pixel at (x, y) with color
    pub fn set_pixel(&mut self, x: u32, y: u32, color: u32) {
        if !self.initialized || x >= self.info.width || y >= self.info.height {
            return;
        }
        
        let offset = (y * self.info.pitch + x * self.info.bytes_per_pixel as u32) as usize;
        let pixel = self.color_to_pixel(color);
        
        unsafe {
            match self.info.bytes_per_pixel {
                4 => {
                    let ptr = self.fb_virt_addr.add(offset) as *mut u32;
                    write_volatile(ptr, pixel);
                }
                3 => {
                    let ptr = self.fb_virt_addr.add(offset);
                    write_volatile(ptr.add(0), ((pixel >> 0) & 0xFF) as u8);
                    write_volatile(ptr.add(1), ((pixel >> 8) & 0xFF) as u8);
                    write_volatile(ptr.add(2), ((pixel >> 16) & 0xFF) as u8);
                }
                2 => {
                    let ptr = self.fb_virt_addr.add(offset) as *mut u16;
                    write_volatile(ptr, pixel as u16);
                }
                _ => {}
            }
        }
    }
    
    /// Get pixel color at (x, y)
    pub fn get_pixel(&self, x: u32, y: u32) -> u32 {
        if !self.initialized || x >= self.info.width || y >= self.info.height {
            return 0;
        }
        
        let offset = (y * self.info.pitch + x * self.info.bytes_per_pixel as u32) as usize;
        
        unsafe {
            match self.info.bytes_per_pixel {
                4 => {
                    let ptr = self.fb_virt_addr.add(offset) as *const u32;
                    read_volatile(ptr)
                }
                3 => {
                    let ptr = self.fb_virt_addr.add(offset);
                    let b = read_volatile(ptr.add(0)) as u32;
                    let g = read_volatile(ptr.add(1)) as u32;
                    let r = read_volatile(ptr.add(2)) as u32;
                    (r << 16) | (g << 8) | b
                }
                2 => {
                    let ptr = self.fb_virt_addr.add(offset) as *const u16;
                    read_volatile(ptr) as u32
                }
                _ => 0,
            }
        }
    }
    
    /// Draw filled rectangle
    pub fn fill_rect(&mut self, x: i32, y: i32, w: u32, h: u32, color: u32) {
        let x0 = x.max(0) as u32;
        let y0 = y.max(0) as u32;
        let x1 = ((x as u32) + w).min(self.info.width);
        let y1 = ((y as u32) + h).min(self.info.height);
        
        for py in y0..y1 {
            for px in x0..x1 {
                self.set_pixel(px, py, color);
            }
        }
    }
    
    /// Draw horizontal line
    pub fn hline(&mut self, x: i32, y: i32, w: u32, color: u32) {
        if y < 0 || y >= self.info.height as i32 {
            return;
        }
        let x0 = x.max(0) as u32;
        let x1 = ((x as u32) + w).min(self.info.width);
        
        for px in x0..x1 {
            self.set_pixel(px, y as u32, color);
        }
    }
    
    /// Draw vertical line
    pub fn vline(&mut self, x: i32, y: i32, h: u32, color: u32) {
        if x < 0 || x >= self.info.width as i32 {
            return;
        }
        let y0 = y.max(0) as u32;
        let y1 = ((y as u32) + h).min(self.info.height);
        
        for py in y0..y1 {
            self.set_pixel(x as u32, py, color);
        }
    }
    
    /// Draw rectangle outline
    pub fn draw_rect(&mut self, x: i32, y: i32, w: u32, h: u32, color: u32) {
        self.hline(x, y, w, color);
        self.hline(x, y + h as i32 - 1, w, color);
        self.vline(x, y, h, color);
        self.vline(x + w as i32 - 1, y, h, color);
    }
    
    /// Draw line using Bresenham's algorithm
    pub fn draw_line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: u32) {
        let dx = (x1 - x0).abs();
        let dy = (y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx - dy;
        let mut x = x0;
        let mut y = y0;

        loop {
            self.set_pixel(x as u32, y as u32, color);
            
            if x == x1 && y == y1 {
                break;
            }
            
            let e2 = 2 * err;
            if e2 > -dy {
                err -= dy;
                x += sx;
            }
            if e2 < dx {
                err += dx;
                y += sy;
            }
        }
    }
    
    /// Draw circle using Bresenham's algorithm
    pub fn draw_circle(&mut self, cx: i32, cy: i32, r: i32, color: u32) {
        let mut x = r;
        let mut y = 0;
        let mut err = 0;

        while x >= y {
            self.set_pixel((cx + x) as u32, (cy + y) as u32, color);
            self.set_pixel((cx + y) as u32, (cy + x) as u32, color);
            self.set_pixel((cx - y) as u32, (cy + x) as u32, color);
            self.set_pixel((cx - x) as u32, (cy + y) as u32, color);
            self.set_pixel((cx - x) as u32, (cy - y) as u32, color);
            self.set_pixel((cx - y) as u32, (cy - x) as u32, color);
            self.set_pixel((cx + y) as u32, (cy - x) as u32, color);
            self.set_pixel((cx + x) as u32, (cy - y) as u32, color);

            y += 1;
            err += 1 + 2 * y;
            if 2 * (err - x) + 1 > 0 {
                x -= 1;
                err += 1 - 2 * x;
            }
        }
    }
    
    /// Fill circle
    pub fn fill_circle(&mut self, cx: i32, cy: i32, r: i32, color: u32) {
        for dy in -r..=r {
            let dx = integer_sqrt(r * r - dy * dy);
            self.draw_line(cx - dx, cy + dy, cx + dx, cy + dy, color);
        }
    }
    
    /// Draw character using 8x8 font
    pub fn draw_char(&mut self, ch: char, x: i32, y: i32, color: u32, scale: u32) {
        let bitmap = get_char_bitmap(ch);
        for row in 0..8usize {
            for col in 0..8usize {
                if bitmap[row] & (1 << (7 - col)) != 0 {
                    for sy in 0..scale {
                        for sx in 0..scale {
                            self.set_pixel(
                                (x as u32) + (col as u32) * scale + sx,
                                (y as u32) + (row as u32) * scale + sy,
                                color
                            );
                        }
                    }
                }
            }
        }
    }
    
    /// Draw text string
    pub fn draw_text(&mut self, text: &str, x: i32, y: i32, color: u32, scale: u32) {
        let mut cx = x;
        for ch in text.chars() {
            self.draw_char(ch, cx, y, color, scale);
            cx += (8 * scale) as i32;
        }
    }
    
    /// Blit buffer to screen (for double buffering)
    pub fn blit(&mut self, buffer: &[u32], x: u32, y: u32, w: u32, h: u32) {
        if !self.initialized {
            return;
        }
        
        for row in 0..h {
            for col in 0..w {
                let src_idx = (row * w + col) as usize;
                if src_idx < buffer.len() {
                    self.set_pixel(x + col, y + row, buffer[src_idx]);
                }
            }
        }
    }
    
    /// Convert RGB color to pixel value
    fn color_to_pixel(&self, color: u32) -> u32 {
        match self.info.bpp {
            32 => color,
            24 => color & 0x00FFFFFF,
            16 => {
                let r = ((color >> 16) & 0xFF) >> 3;
                let g = ((color >> 8) & 0xFF) >> 2;
                let b = (color & 0xFF) >> 3;
                (r << 11) | (g << 5) | b
            }
            _ => color,
        }
    }
}

/// Integer square root
fn integer_sqrt(n: i32) -> i32 {
    if n <= 0 {
        return 0;
    }
    let n = n as u32;
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x as i32
}

/// Get 8x8 bitmap for character
fn get_char_bitmap(ch: char) -> [u8; 8] {
    match ch {
        ' ' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        '!' => [0x18, 0x18, 0x18, 0x18, 0x18, 0x00, 0x18, 0x00],
        '0' => [0x3c, 0x66, 0x6e, 0x76, 0x66, 0x66, 0x3c, 0x00],
        '1' => [0x18, 0x18, 0x38, 0x18, 0x18, 0x18, 0x7e, 0x00],
        '2' => [0x3c, 0x66, 0x06, 0x0c, 0x30, 0x60, 0x7e, 0x00],
        '3' => [0x3c, 0x66, 0x06, 0x1c, 0x06, 0x66, 0x3c, 0x00],
        '4' => [0x06, 0x0e, 0x1e, 0x66, 0x7f, 0x06, 0x06, 0x00],
        '5' => [0x7e, 0x60, 0x7c, 0x06, 0x06, 0x66, 0x3c, 0x00],
        '6' => [0x3c, 0x66, 0x60, 0x7c, 0x66, 0x66, 0x3c, 0x00],
        '7' => [0x7e, 0x66, 0x0c, 0x18, 0x18, 0x18, 0x18, 0x00],
        '8' => [0x3c, 0x66, 0x66, 0x3c, 0x66, 0x66, 0x3c, 0x00],
        '9' => [0x3c, 0x66, 0x66, 0x3e, 0x06, 0x66, 0x3c, 0x00],
        'A' => [0x18, 0x3c, 0x66, 0x7e, 0x66, 0x66, 0x66, 0x00],
        'B' => [0x7c, 0x66, 0x66, 0x7c, 0x66, 0x66, 0x7c, 0x00],
        'C' => [0x3c, 0x66, 0x60, 0x60, 0x60, 0x66, 0x3c, 0x00],
        'D' => [0x78, 0x6c, 0x66, 0x66, 0x66, 0x6c, 0x78, 0x00],
        'E' => [0x7e, 0x60, 0x60, 0x78, 0x60, 0x60, 0x7e, 0x00],
        'F' => [0x7e, 0x60, 0x60, 0x78, 0x60, 0x60, 0x60, 0x00],
        'G' => [0x3c, 0x66, 0x60, 0x6e, 0x66, 0x66, 0x3c, 0x00],
        'H' => [0x66, 0x66, 0x66, 0x7e, 0x66, 0x66, 0x66, 0x00],
        'I' => [0x3c, 0x18, 0x18, 0x18, 0x18, 0x18, 0x3c, 0x00],
        'J' => [0x1e, 0x0c, 0x0c, 0x0c, 0x0c, 0x6c, 0x38, 0x00],
        'K' => [0x66, 0x6c, 0x78, 0x70, 0x78, 0x6c, 0x66, 0x00],
        'L' => [0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x7e, 0x00],
        'M' => [0x63, 0x77, 0x7f, 0x6b, 0x63, 0x63, 0x63, 0x00],
        'N' => [0x66, 0x76, 0x7e, 0x7e, 0x6e, 0x66, 0x66, 0x00],
        'O' => [0x3c, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3c, 0x00],
        'P' => [0x7c, 0x66, 0x66, 0x7c, 0x60, 0x60, 0x60, 0x00],
        'Q' => [0x3c, 0x66, 0x66, 0x66, 0x66, 0x3c, 0x0e, 0x00],
        'R' => [0x7c, 0x66, 0x66, 0x7c, 0x78, 0x6c, 0x66, 0x00],
        'S' => [0x3c, 0x66, 0x60, 0x3c, 0x06, 0x66, 0x3c, 0x00],
        'T' => [0x7e, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x00],
        'U' => [0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3c, 0x00],
        'V' => [0x66, 0x66, 0x66, 0x66, 0x66, 0x3c, 0x18, 0x00],
        'W' => [0x63, 0x63, 0x63, 0x6b, 0x7f, 0x77, 0x63, 0x00],
        'X' => [0x66, 0x66, 0x3c, 0x18, 0x3c, 0x66, 0x66, 0x00],
        'Y' => [0x66, 0x66, 0x66, 0x3c, 0x18, 0x18, 0x18, 0x00],
        'Z' => [0x7e, 0x06, 0x0c, 0x18, 0x30, 0x60, 0x7e, 0x00],
        _ => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    }
}

/// Global VESA driver
lazy_static! {
    static ref VESA_DRIVER: Mutex<VesaDriver> = Mutex::new(VesaDriver::new());
}

/// Initialize VESA driver
pub fn init(width: u32, height: u32, bpp: u8, phys_addr: u64) {
    VESA_DRIVER.lock().init(width, height, bpp, phys_addr);
}

/// Get driver instance
pub fn driver() -> &'static Mutex<VesaDriver> {
    &VESA_DRIVER
}

/// Clear screen
pub fn clear(color: u32) {
    VESA_DRIVER.lock().clear(color);
}

/// Set pixel
pub fn set_pixel(x: u32, y: u32, color: u32) {
    VESA_DRIVER.lock().set_pixel(x, y, color);
}

/// Draw rectangle
pub fn fill_rect(x: i32, y: i32, w: u32, h: u32, color: u32) {
    VESA_DRIVER.lock().fill_rect(x, y, w, h, color);
}

/// Draw text
pub fn draw_text(text: &str, x: i32, y: i32, color: u32, scale: u32) {
    VESA_DRIVER.lock().draw_text(text, x, y, color, scale);
}

/// Print VESA info
pub fn print_info() {
    let driver = VESA_DRIVER.lock();
    if driver.is_initialized() {
        let info = driver.info();
        println!("VESA Framebuffer Info:");
        println!("  Resolution: {}x{}", info.width, info.height);
        println!("  Bits per pixel: {}", info.bpp);
        println!("  Bytes per pixel: {}", info.bytes_per_pixel);
        println!("  Pitch: {} bytes", info.pitch);
        println!("  Physical address: 0x{:016x}", info.phys_addr);
        println!("  Size: {} KB", info.size / 1024);
    } else {
        println!("VESA driver not initialized");
    }
}

/// Color utilities
pub mod colors {
    pub const fn rgb(r: u8, g: u8, b: u8) -> u32 {
        0xFF000000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
    }
    
    pub const BLACK: u32 = 0xFF000000;
    pub const WHITE: u32 = 0xFFFFFFFF;
    pub const RED: u32 = 0xFFFF0000;
    pub const GREEN: u32 = 0xFF00FF00;
    pub const BLUE: u32 = 0xFF0000FF;
    pub const YELLOW: u32 = 0xFFFFFF00;
    pub const CYAN: u32 = 0xFF00FFFF;
    pub const MAGENTA: u32 = 0xFFFF00FF;
    pub const GRAY: u32 = 0xFF808080;
    pub const DARK_GRAY: u32 = 0xFF404040;
    pub const LIGHT_GRAY: u32 = 0xFFC0C0C0;
}
