//! Rendering Engine
//!
//! Renders the layout tree to a framebuffer with animation support.

#![allow(dead_code)]

use alloc::vec;
use alloc::vec::Vec;

use crate::browser::BrowserError;
use crate::browser::css::{KeyframesRule, Value as CssValue};
use crate::browser::layout::{LayoutBox, LayoutTree, BoxType, Transform};
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

    /// Fill rectangle with alpha blending
    pub fn fill_rect_alpha(&mut self, x: i32, y: i32, width: u32, height: u32, color: u32, alpha: f32) {
        let alpha = alpha.clamp(0.0, 1.0);
        if alpha >= 1.0 {
            self.fill_rect(x, y, width, height, color);
            return;
        }
        if alpha <= 0.0 {
            return;
        }

        for dy in 0..height as i32 {
            for dx in 0..width as i32 {
                let px = x + dx;
                let py = y + dy;
                let bg = self.get_pixel(px, py);
                let blended = blend_colors(color, bg, alpha);
                self.set_pixel(px, py, blended);
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

/// Blend foreground color over background color with given alpha
fn blend_colors(fg: u32, bg: u32, alpha: f32) -> u32 {
    let fg_a = ((fg >> 24) & 0xFF) as f32 * alpha;
    let fg_r = ((fg >> 16) & 0xFF) as f32 * alpha;
    let fg_g = ((fg >> 8) & 0xFF) as f32 * alpha;
    let fg_b = (fg & 0xFF) as f32 * alpha;

    let bg_a = ((bg >> 24) & 0xFF) as f32 * (1.0 - alpha);
    let bg_r = ((bg >> 16) & 0xFF) as f32 * (1.0 - alpha);
    let bg_g = ((bg >> 8) & 0xFF) as f32 * (1.0 - alpha);
    let bg_b = (bg & 0xFF) as f32 * (1.0 - alpha);

    let a = (fg_a + bg_a) as u32;
    let r = (fg_r + bg_r) as u32;
    let g = (fg_g + bg_g) as u32;
    let b = (fg_b + bg_b) as u32;

    (a << 24) | (r << 16) | (g << 8) | b
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
    /// Current time in milliseconds (for animations)
    pub current_time: u64,
    /// Keyframes from stylesheet
    pub keyframes: Vec<KeyframesRule>,
}

impl RenderContext {
    /// Create new render context (without allocating framebuffer)
    pub fn new() -> Self {
        Self {
            framebuffer: None,
            layout_tree: None,
            viewport_width: 800,
            viewport_height: 600,
            current_time: 0,
            keyframes: Vec::new(),
        }
    }
    
    /// Initialize framebuffer when needed
    pub fn init_framebuffer(&mut self, width: u32, height: u32) {
        self.viewport_width = width;
        self.viewport_height = height;
        self.framebuffer = Some(Framebuffer::new(width, height));
    }

    /// Set current time for animations
    pub fn set_time(&mut self, time_ms: u64) {
        self.current_time = time_ms;
    }

    /// Update time (increment by delta)
    pub fn update_time(&mut self, delta_ms: u64) {
        self.current_time += delta_ms;
    }
}

/// Render layout tree to framebuffer (without animations)
pub fn render(layout_tree: &LayoutTree, framebuffer: &mut Framebuffer) -> Result<(), BrowserError> {
    // Clear background
    framebuffer.clear(0xFFFFFFFF); // White

    // Render root box without animations
    render_box(&layout_tree.root, framebuffer, 0.0, 0.0, 0, &[])?;

    Ok(())
}

/// Render layout tree with animations
pub fn render_with_animations(
    layout_tree: &LayoutTree,
    framebuffer: &mut Framebuffer,
    current_time: u64,
    keyframes: &[KeyframesRule],
) -> Result<(), BrowserError> {
    // Clear background
    framebuffer.clear(0xFFFFFFFF); // White

    // Render root box with animations
    render_box(&layout_tree.root, framebuffer, 0.0, 0.0, current_time, keyframes)?;

    Ok(())
}

/// Render a layout box
fn render_box(
    layout_box: &LayoutBox,
    framebuffer: &mut Framebuffer,
    offset_x: f32,
    offset_y: f32,
    current_time: u64,
    keyframes: &[KeyframesRule],
) -> Result<(), BrowserError> {
    if layout_box.box_type == BoxType::None {
        return Ok(());
    }

    // Get animated transform if available
    let animated_transform = if !keyframes.is_empty() {
        get_animated_transform(layout_box, current_time, keyframes)
    } else {
        layout_box.styles.transform.clone()
    };

    // Get animated opacity
    let animated_opacity = if !keyframes.is_empty() {
        get_animated_opacity(layout_box, current_time, keyframes)
    } else {
        layout_box.styles.opacity
    };

    // Apply transform if present
    let (x, y) = if let Some(ref transform) = animated_transform {
        let (tx, ty) = match transform {
            Transform::Translate(dx, dy) => (*dx, *dy),
            _ => (0.0, 0.0),
        };
        ((layout_box.x + offset_x + tx) as i32, (layout_box.y + offset_y + ty) as i32)
    } else {
        ((layout_box.x + offset_x) as i32, (layout_box.y + offset_y) as i32)
    };
    
    let width = layout_box.width as u32;
    let height = layout_box.height as u32;

    // Skip rendering if fully transparent
    if animated_opacity <= 0.0 {
        // Still render children in case they have higher opacity
        for child in &layout_box.children {
            render_box(child, framebuffer, layout_box.x + offset_x, layout_box.y + offset_y, current_time, keyframes)?;
        }
        return Ok(());
    }

    // Draw box shadow first (if present)
    if let Some(ref shadows) = layout_box.styles.box_shadow {
        for shadow in shadows {
            let sx = x + shadow.offset_x as i32;
            let sy = y + shadow.offset_y as i32;
            let sw = width + (shadow.spread_radius * 2.0) as u32;
            let sh = height + (shadow.spread_radius * 2.0) as u32;
            let sc = shadow.color.to_u32();
            
            // Simple shadow (no blur for now)
            framebuffer.fill_rect(sx, sy, sw, sh, sc);
        }
    }

    // Draw background (gradient or solid color)
    if let Some(ref gradient) = layout_box.styles.background_gradient {
        draw_gradient(framebuffer, x, y, width, height, gradient);
    } else if let Some(ref bg_color) = layout_box.styles.background_color {
        let color = bg_color.to_u32();
        
        // Draw with border radius if present
        if let Some(ref radius) = layout_box.styles.border_radius {
            fill_rounded_rect(framebuffer, x, y, width, height, radius, color, animated_opacity);
        } else {
            if animated_opacity < 1.0 {
                framebuffer.fill_rect_alpha(x, y, width, height, color, animated_opacity);
            } else {
                framebuffer.fill_rect(x, y, width, height, color);
            }
        }
    }

    // Draw border
    let border_color = 0xFF000000;
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
            .map(|c| c.to_u32())
            .unwrap_or(0xFF000000);
        
        render_text(framebuffer, text, text_x, text_y, layout_box.styles.font_size, text_color, animated_opacity);
    }

    // Render children
    for child in &layout_box.children {
        render_box(child, framebuffer, layout_box.x + offset_x, layout_box.y + offset_y, current_time, keyframes)?;
    }

    Ok(())
}

/// Draw a linear gradient
fn draw_gradient(framebuffer: &mut Framebuffer, x: i32, y: i32, width: u32, height: u32, gradient: &crate::browser::layout::LinearGradient) {
    let stops = &gradient.stops;
    if stops.len() < 2 {
        return;
    }
    
    // Simple horizontal gradient (ignore angle for now)
    for row in 0..height {
        let t = row as f32 / height as f32;
        
        // Find which two stops we're between
        let mut color = stops[0].color.to_u32();
        for i in 0..stops.len() - 1 {
            if t >= stops[i].position && t <= stops[i + 1].position {
                let local_t = (t - stops[i].position) / (stops[i + 1].position - stops[i].position);
                color = interpolate_color(&stops[i].color, &stops[i + 1].color, local_t);
                break;
            }
        }
        
        // Draw horizontal line with this color
        for col in 0..width {
            framebuffer.set_pixel(x + col as i32, y + row as i32, color);
        }
    }
}

/// Interpolate between two colors
fn interpolate_color(c1: &crate::browser::layout::Color, c2: &crate::browser::layout::Color, t: f32) -> u32 {
    let r = (c1.r as f32 * (1.0 - t) + c2.r as f32 * t) as u8;
    let g = (c1.g as f32 * (1.0 - t) + c2.g as f32 * t) as u8;
    let b = (c1.b as f32 * (1.0 - t) + c2.b as f32 * t) as u8;
    let a = (c1.a as f32 * (1.0 - t) + c2.a as f32 * t) as u8;
    ((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

/// Fill a rounded rectangle
fn fill_rounded_rect(
    framebuffer: &mut Framebuffer,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    radius: &crate::browser::layout::BorderRadius,
    color: u32,
    opacity: f32,
) {
    let r_tl = radius.top_left as i32;
    let r_tr = radius.top_right as i32;
    let r_br = radius.bottom_right as i32;
    let r_bl = radius.bottom_left as i32;
    
    for row in 0..height as i32 {
        for col in 0..width as i32 {
            let px = x + col;
            let py = y + row;
            
            // Check if pixel is inside the rounded rectangle
            let mut inside = true;
            
            // Top-left corner
            if col < r_tl && row < r_tl {
                let dx = r_tl - col;
                let dy = r_tl - row;
                if dx * dx + dy * dy > r_tl * r_tl {
                    inside = false;
                }
            }
            // Top-right corner
            else if col >= (width as i32 - r_tr) && row < r_tr {
                let dx = col - (width as i32 - r_tr);
                let dy = r_tr - row;
                if dx * dx + dy * dy > r_tr * r_tr {
                    inside = false;
                }
            }
            // Bottom-right corner
            else if col >= (width as i32 - r_br) && row >= (height as i32 - r_br) {
                let dx = col - (width as i32 - r_br);
                let dy = row - (height as i32 - r_br);
                if dx * dx + dy * dy > r_br * r_br {
                    inside = false;
                }
            }
            // Bottom-left corner
            else if col < r_bl && row >= (height as i32 - r_bl) {
                let dx = r_bl - col;
                let dy = row - (height as i32 - r_bl);
                if dx * dx + dy * dy > r_bl * r_bl {
                    inside = false;
                }
            }
            
            if inside {
                if opacity < 1.0 {
                    let bg = framebuffer.get_pixel(px, py);
                    let blended = blend_colors(color, bg, opacity);
                    framebuffer.set_pixel(px, py, blended);
                } else {
                    framebuffer.set_pixel(px, py, color);
                }
            }
        }
    }
}

/// Render text (simplified bitmap font)
fn render_text(framebuffer: &mut Framebuffer, text: &str, x: i32, y: i32, font_size: f32, color: u32, opacity: f32) {
    let char_width = (font_size * 0.6) as i32;
    let char_height = (font_size * 1.2) as i32;
    
    for (i, ch) in text.chars().enumerate() {
        let char_x = x + (i as i32 * char_width);
        render_char(framebuffer, ch, char_x, y, char_width, char_height, color, opacity);
    }
}

/// Render a single character (simplified)
fn render_char(framebuffer: &mut Framebuffer, ch: char, x: i32, y: i32, width: i32, height: i32, color: u32, opacity: f32) {
    // Simple block representation of characters
    // In a real implementation, this would use a proper font atlas
    
    match ch {
        ' ' => {}
        '!' => {
            fill_rect_with_opacity(framebuffer, x + width / 2 - 1, y, 2, (height * 2 / 3) as u32, color, opacity);
            fill_rect_with_opacity(framebuffer, x + width / 2 - 1, y + height - 3, 2, 3, color, opacity);
        }
        '.' => {
            fill_rect_with_opacity(framebuffer, x + width / 2 - 1, y + height - 4, 2, 2, color, opacity);
        }
        ',' => {
            fill_rect_with_opacity(framebuffer, x + width / 2 - 1, y + height - 4, 2, 2, color, opacity);
            fill_rect_with_opacity(framebuffer, x + width / 2, y + height - 2, 2, 2, color, opacity);
        }
        '-' => {
            fill_rect_with_opacity(framebuffer, x, y + height / 2, width as u32, 2, color, opacity);
        }
        '_' => {
            fill_rect_with_opacity(framebuffer, x, y + height - 2, width as u32, 2, color, opacity);
        }
        '/' => {
            draw_line_with_opacity(framebuffer, x, y + height, x + width, y, color, opacity);
        }
        '0'..='9' | 'A'..='Z' | 'a'..='z' => {
            // Draw a simple filled rectangle for letters/digits
            fill_rect_with_opacity(framebuffer, x + 1, y + 1, (width - 2) as u32, (height - 2) as u32, color, opacity);
        }
        _ => {
            // Default: small rectangle
            fill_rect_with_opacity(framebuffer, x + width / 4, y + height / 4, (width / 2) as u32, (height / 2) as u32, color, opacity);
        }
    }
}

/// Fill rectangle with opacity
fn fill_rect_with_opacity(framebuffer: &mut Framebuffer, x: i32, y: i32, width: u32, height: u32, color: u32, opacity: f32) {
    if opacity >= 1.0 {
        framebuffer.fill_rect(x, y, width, height, color);
    } else if opacity > 0.0 {
        framebuffer.fill_rect_alpha(x, y, width, height, color, opacity);
    }
}

/// Draw line with opacity
fn draw_line_with_opacity(framebuffer: &mut Framebuffer, x0: i32, y0: i32, x1: i32, y1: i32, color: u32, opacity: f32) {
    if opacity >= 1.0 {
        framebuffer.draw_line(x0, y0, x1, y1, color);
        return;
    }
    if opacity <= 0.0 {
        return;
    }

    let dx = (x1 - x0).abs();
    let dy = (y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx - dy;
    let mut x = x0;
    let mut y = y0;

    loop {
        let bg = framebuffer.get_pixel(x, y);
        let blended = blend_colors(color, bg, opacity);
        framebuffer.set_pixel(x, y, blended);

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

/// Convert RGB to u32 color
fn rgb_to_u32(r: u8, g: u8, b: u8) -> u32 {
    0xFF000000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

/// Get animated transform based on current time and keyframes
fn get_animated_transform(
    layout_box: &LayoutBox,
    current_time: u64,
    keyframes: &[KeyframesRule],
) -> Option<Transform> {
    let anim_state = layout_box.animation_state.as_ref()?;
    let anim = layout_box.styles.animation.as_ref()?;
    
    // Find the keyframes rule
    let keyframes_rule = keyframes.iter().find(|k| k.name == anim.name)?;
    
    // Calculate progress
    let progress = anim_state.progress(
        current_time,
        (anim.duration * 1000.0) as u64,
        anim.iteration_count,
    )?;
    
    let progress = anim.timing_function.apply(progress);
    
    // Apply direction
    let adjusted_progress = match anim.direction {
        crate::browser::layout::AnimationDirection::Normal => progress,
        crate::browser::layout::AnimationDirection::Reverse => 1.0 - progress,
        crate::browser::layout::AnimationDirection::Alternate => progress, // Simplified
        crate::browser::layout::AnimationDirection::AlternateReverse => 1.0 - progress, // Simplified
    };
    
    // Find the two keyframes we're between
    let percentage = (adjusted_progress * 100.0) as u8;
    
    let mut prev_pct: Option<u8> = None;
    let mut next_pct: Option<u8> = None;
    
    for (&pct, _) in &keyframes_rule.keyframes {
        if pct <= percentage {
            prev_pct = Some(pct);
        }
        if pct >= percentage && next_pct.is_none() {
            next_pct = Some(pct);
            break;
        }
    }
    
    let prev_pct = prev_pct?;
    let next_pct = next_pct.unwrap_or(prev_pct);
    
    // Get transform values from keyframes
    let prev_transform = get_transform_from_keyframe(keyframes_rule, prev_pct);
    let next_transform = get_transform_from_keyframe(keyframes_rule, next_pct);
    
    // Calculate local progress between keyframes
    let local_progress = if next_pct == prev_pct {
        0.0
    } else {
        (percentage - prev_pct) as f32 / (next_pct - prev_pct) as f32
    };
    
    // Interpolate
    match (prev_transform, next_transform) {
        (Some(prev), Some(next)) => Some(prev.interpolate(&next, local_progress)),
        (Some(prev), None) => Some(prev),
        (None, Some(next)) => Some(next),
        (None, None) => layout_box.styles.transform.clone(),
    }
}

/// Get transform from a keyframe at a specific percentage
fn get_transform_from_keyframe(keyframes_rule: &KeyframesRule, percentage: u8) -> Option<Transform> {
    let declarations = keyframes_rule.keyframes.get(&percentage)?;
    
    for decl in declarations {
        if decl.property == "transform" {
            match &decl.value {
                CssValue::Transform(t) => {
                    return Transform::from_css(t);
                }
                _ => {}
            }
        }
    }
    
    None
}

/// Get animated opacity
fn get_animated_opacity(
    layout_box: &LayoutBox,
    current_time: u64,
    keyframes: &[KeyframesRule],
) -> f32 {
    let base_opacity = layout_box.styles.opacity;
    
    let anim_state = match layout_box.animation_state.as_ref() {
        Some(s) => s,
        None => return base_opacity,
    };
    
    let anim = match layout_box.styles.animation.as_ref() {
        Some(a) => a,
        None => return base_opacity,
    };
    
    // Find the keyframes rule
    let keyframes_rule = match keyframes.iter().find(|k| k.name == anim.name) {
        Some(k) => k,
        None => return base_opacity,
    };
    
    // Calculate progress
    let progress = match anim_state.progress(
        current_time,
        (anim.duration * 1000.0) as u64,
        anim.iteration_count,
    ) {
        Some(p) => p,
        None => return base_opacity,
    };
    
    let progress = anim.timing_function.apply(progress);
    
    // Check if keyframes define opacity
    let percentage = (progress * 100.0) as u8;
    
    let mut prev_pct: Option<u8> = None;
    let mut next_pct: Option<u8> = None;
    
    for (&pct, _) in &keyframes_rule.keyframes {
        if pct <= percentage {
            prev_pct = Some(pct);
        }
        if pct >= percentage && next_pct.is_none() {
            next_pct = Some(pct);
            break;
        }
    }
    
    if let (Some(prev), Some(next)) = (prev_pct, next_pct) {
        let prev_opacity = get_opacity_from_keyframe(keyframes_rule, prev).unwrap_or(base_opacity);
        let next_opacity = get_opacity_from_keyframe(keyframes_rule, next).unwrap_or(base_opacity);
        
        if next == prev {
            return prev_opacity;
        }
        
        let local_progress = (percentage - prev) as f32 / (next - prev) as f32;
        return prev_opacity + (next_opacity - prev_opacity) * local_progress;
    }
    
    base_opacity
}

/// Get opacity from a keyframe at a specific percentage
fn get_opacity_from_keyframe(keyframes_rule: &KeyframesRule, percentage: u8) -> Option<f32> {
    let declarations = keyframes_rule.keyframes.get(&percentage)?;
    
    for decl in declarations {
        if decl.property == "opacity" {
            match &decl.value {
                CssValue::Number(n) => return Some(*n),
                _ => {}
            }
        }
    }
    
    None
}

/// Initialize render engine
pub fn init() {
    println!("[render] Rendering engine initialized with animation support");
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
