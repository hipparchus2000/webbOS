//! Rendering Engine
//!
//! Renders the layout tree to a framebuffer.

use alloc::vec;
use alloc::vec::Vec;

use crate::browser::BrowserError;
use crate::browser::layout::{LayoutBox, LayoutTree, BoxType, Color, TextAlign, FontWeight};
use crate::println;

/// Framebuffer for rendering
pub struct Framebuffer {
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Pixel data (RGBA)
    pub data: Vec<u32>,
}

impl Framebuffer {
    /// Create new framebuffer
    pub fn new(width: u32, height: u32) -> Self {
        let size = (width * height) as usize;
        Self {
            width,
            height,
            data: vec![0xFFFFFFFF; size], // White background
        }
    }

    /// Clear framebuffer
    pub fn clear(&mut self, color: u32) {
        for pixel in &mut self.data {
            *pixel = color;
        }
    }

    /// Set pixel
    pub fn set_pixel(&mut self, x: i32, y: i32, color: u32) {
        if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 {
            let idx = (y as u32 * self.width + x as u32) as usize;
            self.data[idx] = color;
        }
    }

    /// Get pixel
    pub fn get_pixel(&self, x: i32, y: i32) -> u32 {
        if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 {
            let idx = (y as u32 * self.width + x as u32) as usize;
            self.data[idx]
        } else {
            0
        }
    }

    /// Fill rectangle
    pub fn fill_rect(&mut self, x: i32, y: i32, width: u32, height: u32, color: u32) {
        for dy in 0..height as i32 {
            for dx in 0..width as i32 {
                self.set_pixel(x + dx, y + dy, color);
            }
        }
    }

    /// Draw rectangle outline
    pub fn draw_rect(&mut self, x: i32, y: i32, width: u32, height: u32, color: u32) {
        for dx in 0..width as i32 {
            self.set_pixel(x + dx, y, color);
            self.set_pixel(x + dx, y + height as i32 - 1, color);
        }
        for dy in 0..height as i32 {
            self.set_pixel(x, y + dy, color);
            self.set_pixel(x + width as i32 - 1, y + dy, color);
        }
    }

    /// Draw line (Bresenham)
    pub fn draw_line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: u32) {
        let dx = (x1 - x0).abs();
        let dy = (y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx - dy;
        let mut x = x0;
        let mut y = y0;

        loop {
            self.set_pixel(x, y, color);

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
}

/// Render context
pub struct RenderContext {
    /// Framebuffer (lazy init)
    pub framebuffer: Option<Framebuffer>,
    /// Layout tree
    pub layout_tree: Option<LayoutTree>,
    /// Viewport width
    pub viewport_width: u32,
    /// Viewport height
    pub viewport_height: u32,
}

impl RenderContext {
    /// Create new render context (without allocating framebuffer)
    pub fn new() -> Self {
        Self {
            framebuffer: None,
            layout_tree: None,
            viewport_width: 800,
            viewport_height: 600,
        }
    }
    
    /// Initialize framebuffer when needed
    pub fn init_framebuffer(&mut self, width: u32, height: u32) {
        self.viewport_width = width;
        self.viewport_height = height;
        self.framebuffer = Some(Framebuffer::new(width, height));
    }
}

/// Render layout tree to framebuffer
pub fn render(layout_tree: &LayoutTree, framebuffer: &mut Framebuffer) -> Result<(), BrowserError> {
    // Clear background
    framebuffer.clear(0xFFFFFFFF); // White

    // Render root box
    render_box(&layout_tree.root, framebuffer, 0.0, 0.0)?;

    Ok(())
}

/// Render a layout box
fn render_box(layout_box: &LayoutBox, framebuffer: &mut Framebuffer, offset_x: f32, offset_y: f32) -> Result<(), BrowserError> {
    if layout_box.box_type == BoxType::None {
        return Ok(());
    }

    let x = (layout_box.x + offset_x) as i32;
    let y = (layout_box.y + offset_y) as i32;
    let width = layout_box.width as u32;
    let height = layout_box.height as u32;

    // Draw background
    if let Some(ref bg_color) = layout_box.styles.background_color {
        let color = rgb_to_u32(bg_color.r, bg_color.g, bg_color.b);
        framebuffer.fill_rect(x, y, width, height, color);
    }

    // Draw border
    let border_color = rgb_to_u32(0, 0, 0);
    if layout_box.border.top > 0.0 {
        framebuffer.fill_rect(x, y, width, layout_box.border.top as u32, border_color);
    }
    if layout_box.border.bottom > 0.0 {
        framebuffer.fill_rect(x, y + height as i32 - layout_box.border.bottom as i32, width, layout_box.border.bottom as u32, border_color);
    }
    if layout_box.border.left > 0.0 {
        framebuffer.fill_rect(x, y, layout_box.border.left as u32, height, border_color);
    }
    if layout_box.border.right > 0.0 {
        framebuffer.fill_rect(x + width as i32 - layout_box.border.right as i32, y, layout_box.border.right as u32, height, border_color);
    }

    // Render text
    if let Some(ref text) = layout_box.text {
        let text_x = (layout_box.x + layout_box.padding.left + offset_x) as i32;
        let text_y = (layout_box.y + layout_box.padding.top + offset_y) as i32;
        let text_color = layout_box.styles.color.as_ref()
            .map(|c| rgb_to_u32(c.r, c.g, c.b))
            .unwrap_or(0xFF000000);
        
        render_text(framebuffer, text, text_x, text_y, layout_box.styles.font_size, text_color);
    }

    // Render children
    for child in &layout_box.children {
        render_box(child, framebuffer, layout_box.x + offset_x, layout_box.y + offset_y)?;
    }

    Ok(())
}

/// Render text (simplified bitmap font)
fn render_text(framebuffer: &mut Framebuffer, text: &str, x: i32, y: i32, font_size: f32, color: u32) {
    let char_width = (font_size * 0.6) as i32;
    let char_height = (font_size * 1.2) as i32;
    
    for (i, ch) in text.chars().enumerate() {
        let char_x = x + (i as i32 * char_width);
        render_char(framebuffer, ch, char_x, y, char_width, char_height, color);
    }
}

/// Render a single character (simplified)
fn render_char(framebuffer: &mut Framebuffer, ch: char, x: i32, y: i32, width: i32, height: i32, color: u32) {
    // Simple block representation of characters
    // In a real implementation, this would use a proper font atlas
    
    match ch {
        ' ' => {}
        '!' => {
            framebuffer.fill_rect(x + width / 2 - 1, y, 2, (height * 2 / 3) as u32, color);
            framebuffer.fill_rect(x + width / 2 - 1, y + height - 3, 2, 3, color);
        }
        '.' => {
            framebuffer.fill_rect(x + width / 2 - 1, y + height - 4, 2, 2, color);
        }
        ',' => {
            framebuffer.fill_rect(x + width / 2 - 1, y + height - 4, 2, 2, color);
            framebuffer.fill_rect(x + width / 2, y + height - 2, 2, 2, color);
        }
        '-' => {
            framebuffer.fill_rect(x, y + height / 2, width as u32, 2, color);
        }
        '_' => {
            framebuffer.fill_rect(x, y + height - 2, width as u32, 2, color);
        }
        '/' => {
            framebuffer.draw_line(x, y + height, x + width, y, color);
        }
        '0'..='9' | 'A'..='Z' | 'a'..='z' => {
            // Draw a simple filled rectangle for letters/digits
            framebuffer.fill_rect(x + 1, y + 1, (width - 2) as u32, (height - 2) as u32, color);
        }
        _ => {
            // Default: small rectangle
            framebuffer.fill_rect(x + width / 4, y + height / 4, (width / 2) as u32, (height / 2) as u32, color);
        }
    }
}

/// Convert RGB to u32 color
fn rgb_to_u32(r: u8, g: u8, b: u8) -> u32 {
    0xFF000000 | ((b as u32) << 16) | ((g as u32) << 8) | (r as u32)
}

/// Initialize render engine
pub fn init() {
    println!("[render] Rendering engine initialized");
}

/// Create a simple test pattern
pub fn test_pattern(framebuffer: &mut Framebuffer) {
    let width = framebuffer.width;
    let height = framebuffer.height;

    // Draw gradient
    for y in 0..height {
        let color = rgb_to_u32(
            ((y * 255) / height) as u8,
            0,
            (((height - y) * 255) / height) as u8,
        );
        framebuffer.fill_rect(0, y as i32, width, 1, color);
    }

    // Draw some shapes
    framebuffer.fill_rect(50, 50, 200, 100, 0xFFFF0000); // Red rect
    framebuffer.fill_rect(100, 100, 200, 100, 0xFF00FF00); // Green rect
    framebuffer.fill_rect(150, 150, 200, 100, 0xFF0000FF); // Blue rect

    // Draw circle approximation
    let cx = 600i32;
    let cy = 300i32;
    let radius = 100i32;
    // Use Bresenham's circle algorithm instead of trigonometry
    let mut x = radius;
    let mut y = 0i32;
    let mut err = 0i32;
    
    while x >= y {
        framebuffer.set_pixel(cx + x, cy + y, 0xFFFFFF00);
        framebuffer.set_pixel(cx + y, cy + x, 0xFFFFFF00);
        framebuffer.set_pixel(cx - y, cy + x, 0xFFFFFF00);
        framebuffer.set_pixel(cx - x, cy + y, 0xFFFFFF00);
        framebuffer.set_pixel(cx - x, cy - y, 0xFFFFFF00);
        framebuffer.set_pixel(cx - y, cy - x, 0xFFFFFF00);
        framebuffer.set_pixel(cx + y, cy - x, 0xFFFFFF00);
        framebuffer.set_pixel(cx + x, cy - y, 0xFFFFFF00);
        
        y += 1;
        err += 1 + 2 * y;
        if 2 * (err - x) + 1 > 0 {
            x -= 1;
            err += 1 - 2 * x;
        }
    }
}
