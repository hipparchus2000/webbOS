//! Raspberry Pi Framebuffer Driver
//!
//! This driver uses the VideoCore mailbox property interface to allocate
//! and configure a framebuffer on Raspberry Pi.
//!
//! The framebuffer is allocated by the GPU and returned as a physical address
//! that needs to be mapped into the kernel's address space.

use core::ptr::{read_volatile, write_volatile};
use spin::Mutex;
use lazy_static::lazy_static;

use crate::println;
use crate::mm::phys_to_virt;
use crate::drivers::mailbox::{Mailbox, mailbox, tags};
use webbos_shared::types::PhysAddr;

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

/// Pi Framebuffer driver state
pub struct PiFramebuffer {
    pub initialized: bool,
    pub info: FramebufferInfo,
    pub fb_virt_addr: *mut u8,
}

unsafe impl Send for PiFramebuffer {}
unsafe impl Sync for PiFramebuffer {}

impl PiFramebuffer {
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
    
    /// Initialize the framebuffer using the mailbox interface
    /// 
    /// This sends a series of property messages to configure and allocate
    /// the framebuffer from the VideoCore GPU.
    pub fn init(&mut self, width: u32, height: u32, bpp: u8) -> bool {
        println!("[pi_fb] Initializing Pi framebuffer...");
        println!("[pi_fb] Requested: {}x{} @ {}bpp", width, height, bpp);
        
        let mb = mailbox().lock();
        
        if !mb.is_initialized() {
            println!("[pi_fb] Error: Mailbox not initialized");
            return false;
        }
        
        // Step 1: Set physical size (display size)
        if !self.set_physical_size(&mb, width, height) {
            println!("[pi_fb] Failed to set physical size");
            return false;
        }
        
        // Step 2: Set virtual size (buffer size, can be larger for panning)
        if !self.set_virtual_size(&mb, width, height) {
            println!("[pi_fb] Failed to set virtual size");
            return false;
        }
        
        // Step 3: Set depth (bits per pixel)
        if !self.set_depth(&mb, bpp) {
            println!("[pi_fb] Failed to set depth");
            return false;
        }
        
        // Step 4: Set pixel order (RGB = 1)
        if !self.set_pixel_order(&mb, 1) {
            println!("[pi_fb] Failed to set pixel order");
            return false;
        }
        
        // Step 5: Allocate the framebuffer
        let (fb_addr, fb_size) = match self.allocate_buffer(&mb) {
            Some((addr, size)) => (addr, size),
            None => {
                println!("[pi_fb] Failed to allocate framebuffer");
                return false;
            }
        };
        
        // Step 6: Get the pitch (bytes per scanline)
        let pitch = match self.get_pitch(&mb) {
            Some(p) => p,
            None => {
                println!("[pi_fb] Failed to get pitch, using calculated value");
                width * ((bpp as u32 + 7) / 8)
            }
        };
        
        let bytes_per_pixel = (bpp + 7) / 8;
        
        // Calculate color masks based on bpp and pixel order
        // For RGB order (pixel_order = 1):
        // - 32bpp: 0x00FF0000 = R, 0x0000FF00 = G, 0x000000FF = B
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
            phys_addr: fb_addr as u64,
            size: fb_size as usize,
        };
        
        // Map the framebuffer to virtual address space
        // On ARM, we use the kernel's memory mapping
        self.fb_virt_addr = phys_to_virt(PhysAddr::new(fb_addr as u64)).as_u64() as *mut u8;
        
        println!("[pi_fb] Framebuffer allocated:");
        println!("[pi_fb]   Physical address: 0x{:08X}", fb_addr);
        println!("[pi_fb]   Virtual address: {:p}", self.fb_virt_addr);
        println!("[pi_fb]   Size: {} KB", fb_size / 1024);
        println!("[pi_fb]   Resolution: {}x{}", width, height);
        println!("[pi_fb]   Pitch: {} bytes", pitch);
        println!("[pi_fb]   {} bits per pixel", bpp);
        
        // Clear framebuffer to black
        self.clear(0);
        
        self.initialized = true;
        println!("[pi_fb] Initialization complete");
        true
    }
    
    /// Set physical display size
    fn set_physical_size(&self, mb: &Mailbox, width: u32, height: u32) -> bool {
        // Tag 0x48003: Set physical width/height
        let mut msg = [0u32; 8];
        msg[0] = 8 * 4;              // Buffer size
        msg[1] = 0;                  // Request code
        msg[2] = tags::SET_PHYSICAL_SIZE; // Tag
        msg[3] = 8;                  // Value buffer size
        msg[4] = 0;                  // Request
        msg[5] = width;              // Width
        msg[6] = height;             // Height
        msg[7] = 0;                  // End tag
        
        unsafe {
            mb.send_property_message(msg.as_mut_ptr())
        }
    }
    
    /// Set virtual buffer size
    fn set_virtual_size(&self, mb: &Mailbox, width: u32, height: u32) -> bool {
        // Tag 0x48004: Set virtual width/height
        let mut msg = [0u32; 8];
        msg[0] = 8 * 4;              // Buffer size
        msg[1] = 0;                  // Request code
        msg[2] = tags::SET_VIRTUAL_SIZE; // Tag
        msg[3] = 8;                  // Value buffer size
        msg[4] = 0;                  // Request
        msg[5] = width;              // Width
        msg[6] = height;             // Height
        msg[7] = 0;                  // End tag
        
        unsafe {
            mb.send_property_message(msg.as_mut_ptr())
        }
    }
    
    /// Set depth (bits per pixel)
    fn set_depth(&self, mb: &Mailbox, bpp: u8) -> bool {
        // Tag 0x48005: Set depth
        let mut msg = [0u32; 7];
        msg[0] = 7 * 4;              // Buffer size
        msg[1] = 0;                  // Request code
        msg[2] = tags::SET_DEPTH;    // Tag
        msg[3] = 4;                  // Value buffer size
        msg[4] = 0;                  // Request
        msg[5] = bpp as u32;         // Bits per pixel
        msg[6] = 0;                  // End tag
        
        unsafe {
            mb.send_property_message(msg.as_mut_ptr())
        }
    }
    
    /// Set pixel order (0 = BGR, 1 = RGB)
    fn set_pixel_order(&self, mb: &Mailbox, order: u32) -> bool {
        // Tag 0x48006: Set pixel order
        let mut msg = [0u32; 7];
        msg[0] = 7 * 4;              // Buffer size
        msg[1] = 0;                  // Request code
        msg[2] = tags::SET_PIXEL_ORDER; // Tag
        msg[3] = 4;                  // Value buffer size
        msg[4] = 0;                  // Request
        msg[5] = order;              // Pixel order
        msg[6] = 0;                  // End tag
        
        unsafe {
            mb.send_property_message(msg.as_mut_ptr())
        }
    }
    
    /// Allocate framebuffer buffer
    /// 
    /// Returns (physical address, size) on success
    fn allocate_buffer(&self, mb: &Mailbox) -> Option<(u32, u32)> {
        // Tag 0x40001: Allocate buffer
        let mut msg = [0u32; 8];
        msg[0] = 8 * 4;              // Buffer size
        msg[1] = 0;                  // Request code
        msg[2] = tags::ALLOCATE_FRAMEBUFFER; // Tag
        msg[3] = 8;                  // Value buffer size
        msg[4] = 0;                  // Request
        msg[5] = 0;                  // Alignment (0 = default)
        msg[6] = 0;                  // Response: address
        msg[7] = 0;                  // End tag
        
        unsafe {
            if mb.send_property_message(msg.as_mut_ptr()) {
                let addr = msg[5] & 0x3FFFFFFF; // Convert bus address to physical
                let size = msg[6];
                Some((addr, size))
            } else {
                None
            }
        }
    }
    
    /// Get pitch (bytes per scanline)
    fn get_pitch(&self, mb: &Mailbox) -> Option<u32> {
        // Tag 0x40008: Get pitch
        let mut msg = [0u32; 7];
        msg[0] = 7 * 4;              // Buffer size
        msg[1] = 0;                  // Request code
        msg[2] = tags::GET_PITCH;    // Tag
        msg[3] = 4;                  // Value buffer size
        msg[4] = 0;                  // Request
        msg[5] = 0;                  // Response: pitch
        msg[6] = 0;                  // End tag
        
        unsafe {
            if mb.send_property_message(msg.as_mut_ptr()) {
                Some(msg[5])
            } else {
                None
            }
        }
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
        
        // Use checked arithmetic to prevent overflow
        let y_offset = match (y as usize).checked_mul(self.info.pitch as usize) {
            Some(v) => v,
            None => return, // Overflow, bail out
        };
        
        let x_offset = match (x as usize).checked_mul(self.info.bytes_per_pixel as usize) {
            Some(v) => v,
            None => return, // Overflow, bail out
        };
        
        let offset = match y_offset.checked_add(x_offset) {
            Some(v) => v,
            None => return, // Overflow, bail out
        };
        
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
        
        // Use checked arithmetic to prevent overflow
        let y_offset = match (y as usize).checked_mul(self.info.pitch as usize) {
            Some(v) => v,
            None => return 0, // Overflow, bail out
        };
        
        let x_offset = match (x as usize).checked_mul(self.info.bytes_per_pixel as usize) {
            Some(v) => v,
            None => return 0, // Overflow, bail out
        };
        
        let offset = match y_offset.checked_add(x_offset) {
            Some(v) => v,
            None => return 0, // Overflow, bail out
        };
        
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
    
    /// Draw filled triangle using scanline fill
    pub fn fill_triangle(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, x3: i32, y3: i32, color: u32) {
        // Sort vertices by y-coordinate
        let mut v1 = (x1, y1);
        let mut v2 = (x2, y2);
        let mut v3 = (x3, y3);
        
        if v2.1 < v1.1 { core::mem::swap(&mut v1, &mut v2); }
        if v3.1 < v1.1 { core::mem::swap(&mut v1, &mut v3); }
        if v3.1 < v2.1 { core::mem::swap(&mut v2, &mut v3); }
        
        let interpolate = |y: i32, ya: i32, yb: i32, xa: i32, xb: i32| -> i32 {
            if ya == yb { xa } else { xa + (y - ya) * (xb - xa) / (yb - ya) }
        };
        
        for y in v1.1..=v3.1 {
            let (x_start, x_end) = if y < v2.1 {
                let x_a = interpolate(y, v1.1, v2.1, v1.0, v2.0);
                let x_b = interpolate(y, v1.1, v3.1, v1.0, v3.0);
                (x_a.min(x_b), x_a.max(x_b))
            } else {
                let x_a = interpolate(y, v2.1, v3.1, v2.0, v3.0);
                let x_b = interpolate(y, v1.1, v3.1, v1.0, v3.0);
                (x_a.min(x_b), x_a.max(x_b))
            };
            self.draw_line(x_start, y, x_end, y, color);
        }
    }
    
    /// Draw triangle outline
    pub fn draw_triangle(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, x3: i32, y3: i32, color: u32) {
        self.draw_line(x1, y1, x2, y2, color);
        self.draw_line(x2, y2, x3, y3, color);
        self.draw_line(x3, y3, x1, y1, color);
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
        '"' => [0x66, 0x66, 0x66, 0x00, 0x00, 0x00, 0x00, 0x00],
        '#' => [0x66, 0x66, 0xff, 0x66, 0xff, 0x66, 0x66, 0x00],
        '$' => [0x18, 0x3e, 0x60, 0x3c, 0x06, 0x7c, 0x18, 0x00],
        '%' => [0x62, 0x66, 0x0c, 0x18, 0x30, 0x66, 0x46, 0x00],
        '&' => [0x3c, 0x66, 0x3c, 0x38, 0x67, 0x66, 0x3f, 0x00],
        '\'' => [0x06, 0x0c, 0x18, 0x00, 0x00, 0x00, 0x00, 0x00],
        '(' => [0x0c, 0x18, 0x30, 0x30, 0x30, 0x18, 0x0c, 0x00],
        ')' => [0x30, 0x18, 0x0c, 0x0c, 0x0c, 0x18, 0x30, 0x00],
        '*' => [0x00, 0x66, 0x3c, 0xff, 0x3c, 0x66, 0x00, 0x00],
        '+' => [0x00, 0x18, 0x18, 0x7e, 0x18, 0x18, 0x00, 0x00],
        ',' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x30],
        '-' => [0x00, 0x00, 0x00, 0x7e, 0x00, 0x00, 0x00, 0x00],
        '.' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x00],
        '/' => [0x00, 0x03, 0x06, 0x0c, 0x18, 0x30, 0x60, 0x00],
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
        'a' => [0x00, 0x00, 0x3c, 0x06, 0x3e, 0x66, 0x3e, 0x00],
        'b' => [0x60, 0x60, 0x7c, 0x66, 0x66, 0x66, 0x7c, 0x00],
        'c' => [0x00, 0x00, 0x3c, 0x60, 0x60, 0x60, 0x3c, 0x00],
        'd' => [0x06, 0x06, 0x3e, 0x66, 0x66, 0x66, 0x3e, 0x00],
        'e' => [0x00, 0x00, 0x3c, 0x66, 0x7e, 0x60, 0x3c, 0x00],
        'f' => [0x0c, 0x18, 0x3c, 0x18, 0x18, 0x18, 0x18, 0x00],
        'g' => [0x00, 0x00, 0x3e, 0x66, 0x66, 0x3e, 0x06, 0x3c],
        'h' => [0x60, 0x60, 0x7c, 0x66, 0x66, 0x66, 0x66, 0x00],
        'i' => [0x18, 0x00, 0x38, 0x18, 0x18, 0x18, 0x3c, 0x00],
        'j' => [0x0c, 0x00, 0x0c, 0x0c, 0x0c, 0x6c, 0x38, 0x00],
        'k' => [0x60, 0x60, 0x66, 0x6c, 0x78, 0x6c, 0x66, 0x00],
        'l' => [0x38, 0x18, 0x18, 0x18, 0x18, 0x18, 0x3c, 0x00],
        'm' => [0x00, 0x00, 0xee, 0x7f, 0x6b, 0x6b, 0x63, 0x00],
        'n' => [0x00, 0x00, 0x7c, 0x66, 0x66, 0x66, 0x66, 0x00],
        'o' => [0x00, 0x00, 0x3c, 0x66, 0x66, 0x66, 0x3c, 0x00],
        'p' => [0x00, 0x00, 0x7c, 0x66, 0x66, 0x7c, 0x60, 0x60],
        'q' => [0x00, 0x00, 0x3e, 0x66, 0x66, 0x3e, 0x06, 0x06],
        'r' => [0x00, 0x00, 0x7c, 0x66, 0x60, 0x60, 0x60, 0x00],
        's' => [0x00, 0x00, 0x3e, 0x60, 0x3c, 0x06, 0x7c, 0x00],
        't' => [0x18, 0x18, 0x7e, 0x18, 0x18, 0x18, 0x0c, 0x00],
        'u' => [0x00, 0x00, 0x66, 0x66, 0x66, 0x66, 0x3e, 0x00],
        'v' => [0x00, 0x00, 0x66, 0x66, 0x66, 0x3c, 0x18, 0x00],
        'w' => [0x00, 0x00, 0x63, 0x6b, 0x7f, 0x77, 0x63, 0x00],
        'x' => [0x00, 0x00, 0x66, 0x3c, 0x18, 0x3c, 0x66, 0x00],
        'y' => [0x00, 0x00, 0x66, 0x66, 0x66, 0x3e, 0x0c, 0x78],
        'z' => [0x00, 0x00, 0x7e, 0x0c, 0x18, 0x30, 0x7e, 0x00],
        _ => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    }
}

// Global Pi framebuffer driver
lazy_static! {
    static ref PI_FRAMEBUFFER: Mutex<PiFramebuffer> = Mutex::new(PiFramebuffer::new());
}

/// Initialize the Pi framebuffer driver
/// 
/// # Arguments
/// * `width` - Screen width in pixels
/// * `height` - Screen height in pixels
/// * `bpp` - Bits per pixel (usually 32)
pub fn init(width: u32, height: u32, bpp: u8) -> bool {
    PI_FRAMEBUFFER.lock().init(width, height, bpp)
}

/// Get the driver instance
pub fn driver() -> &'static Mutex<PiFramebuffer> {
    &PI_FRAMEBUFFER
}

/// Clear screen
pub fn clear(color: u32) {
    PI_FRAMEBUFFER.lock().clear(color);
}

/// Set pixel
pub fn set_pixel(x: u32, y: u32, color: u32) {
    PI_FRAMEBUFFER.lock().set_pixel(x, y, color);
}

/// Draw rectangle
pub fn fill_rect(x: i32, y: i32, w: u32, h: u32, color: u32) {
    PI_FRAMEBUFFER.lock().fill_rect(x, y, w, h, color);
}

/// Draw text
pub fn draw_text(text: &str, x: i32, y: i32, color: u32, scale: u32) {
    PI_FRAMEBUFFER.lock().draw_text(text, x, y, color, scale);
}

/// Print framebuffer info
pub fn print_info() {
    let driver = PI_FRAMEBUFFER.lock();
    if driver.is_initialized() {
        let info = driver.info();
        println!("Pi Framebuffer Info:");
        println!("  Resolution: {}x{}", info.width, info.height);
        println!("  Bits per pixel: {}", info.bpp);
        println!("  Bytes per pixel: {}", info.bytes_per_pixel);
        println!("  Pitch: {} bytes", info.pitch);
        println!("  Physical address: 0x{:016x}", info.phys_addr);
        println!("  Size: {} KB", info.size / 1024);
    } else {
        println!("Pi framebuffer driver not initialized");
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
