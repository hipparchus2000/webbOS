//! Graphical Desktop UI (macOS-style)
//!
//! Renders a desktop environment with:
//! - Menu bar at top
//! - Dock at bottom
//! - Desktop icons
//! - Mouse click detection

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::vec;
use alloc::format;
use spin::Mutex;
use lazy_static::lazy_static;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use crate::drivers::vesa::{self, VesaDriver, colors};
use crate::println;
use crate::browser;

/// macOS-style color palette
mod palette {
    pub const MENU_BAR_BG: u32 = 0xFFEFEFEF;      // Light gray
    pub const MENU_BAR_TEXT: u32 = 0xFF000000;    // Black
    pub const DOCK_BG: u32 = 0xCC1A1A1A;          // Semi-transparent dark (80% opacity)
    pub const DOCK_BORDER: u32 = 0x99000000;       // Semi-transparent black
    pub const ICON_BG: u32 = 0xFFFFFFFF;          // White
    pub const ICON_SELECTED: u32 = 0xFF0080FF;    // Blue
    pub const DESKTOP_BG: u32 = 0xFF2B5B84;       // Nice blue
    pub const WINDOW_BG: u32 = 0xFFFFFFFF;        // White
    pub const WINDOW_TITLE: u32 = 0xFFE0E0E0;     // Light gray
    pub const TEXT_BLACK: u32 = 0xFF000000;
    pub const TEXT_WHITE: u32 = 0xFFFFFFFF;
}

/// Desktop icon
#[derive(Debug, Clone)]
pub struct Icon {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub label: String,
    pub icon_char: char,
    pub icon_path: Option<String>, // Path to PNG icon file
    pub action: IconAction,
    pub is_folder: bool,           // For desktop icons
}

#[derive(Debug, Clone)]
pub enum IconAction {
    LaunchApp(String),      // Launch application by name
    OpenFolder(String),     // Open folder in file manager
    OpenHtmlFile(String),   // Open HTML file in browser
    None,
}

/// Cursor save-under buffer size (must be larger than cursor)
const CURSOR_SIZE: usize = 16;
const SAVE_BUFFER_SIZE: usize = CURSOR_SIZE * CURSOR_SIZE;

/// Dirty rectangle for optimized redraw
#[derive(Clone, Copy, Debug)]
pub struct DirtyRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl DirtyRect {
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }
    
    /// Expand to include another rectangle
    pub fn merge(&mut self, other: &DirtyRect) {
        let x1 = self.x.min(other.x);
        let y1 = self.y.min(other.y);
        let x2 = (self.x + self.width as i32).max(other.x + other.width as i32);
        let y2 = (self.y + self.height as i32).max(other.y + other.height as i32);
        self.x = x1;
        self.y = y1;
        self.width = (x2 - x1) as u32;
        self.height = (y2 - y1) as u32;
    }
    
    /// Check if this rectangle intersects with another
    pub fn intersects(&self, other: &DirtyRect) -> bool {
        !(self.x + self.width as i32 <= other.x ||
          other.x + other.width as i32 <= self.x ||
          self.y + self.height as i32 <= other.y ||
          other.y + other.height as i32 <= self.y)
    }
}

/// Desktop UI state
pub struct DesktopUI {
    menu_bar_height: u32,
    dock_height: u32,
    dock_icon_size: u32,
    dock_icons: Vec<Icon>,
    desktop_icons: Vec<Icon>,
    selected_icon: Option<usize>,
    mouse_x: i32,
    mouse_y: i32,
    old_mouse_x: i32,
    old_mouse_y: i32,
    browser_open: bool,
    // Save-under buffer for mouse cursor (stores pixels under cursor)
    save_buffer: [u32; SAVE_BUFFER_SIZE],
    save_buffer_valid: bool,
    save_buffer_x: i32,
    save_buffer_y: i32,
    // Dirty rectangle tracking for performance
    dirty_rects: Vec<DirtyRect>,
    full_redraw_needed: bool,
    screen_width: u32,
    screen_height: u32,
}

/// Browser window dimensions
const BROWSER_WIDTH: u32 = 1000;
const BROWSER_HEIGHT: u32 = 700;
const BROWSER_X: i32 = 140;
const BROWSER_Y: i32 = 50;

impl DesktopUI {
    pub fn new() -> Self {
        let mut ui = Self {
            menu_bar_height: 24,
            dock_height: 64,
            dock_icon_size: 48,
            dock_icons: Vec::new(),
            desktop_icons: Vec::new(),
            selected_icon: None,
            mouse_x: 640,
            mouse_y: 400,
            old_mouse_x: 640,
            old_mouse_y: 400,
            browser_open: false,
            save_buffer: [0; SAVE_BUFFER_SIZE],
            save_buffer_valid: false,
            save_buffer_x: 0,
            save_buffer_y: 0,
            dirty_rects: Vec::new(),
            full_redraw_needed: true, // Start with full redraw
            screen_width: 1280,
            screen_height: 800,
        };

        // Create dock icons (centered at bottom)
        ui.setup_dock_icons();
        ui.setup_desktop_icons();

        ui
    }
    
    /// Mark a region as dirty (needs redraw)
    pub fn mark_dirty(&mut self, x: i32, y: i32, width: u32, height: u32) {
        // Clamp to screen bounds
        let x = x.max(0);
        let y = y.max(0);
        let width = width.min(self.screen_width - x as u32);
        let height = height.min(self.screen_height - y as u32);
        
        if width == 0 || height == 0 {
            return;
        }
        
        let new_rect = DirtyRect::new(x, y, width, height);
        
        // Check if this can be merged with an existing dirty rect
        for rect in &mut self.dirty_rects {
            if rect.intersects(&new_rect) || 
               (rect.x.abs_diff(new_rect.x) < 50 && rect.y.abs_diff(new_rect.y) < 50) {
                rect.merge(&new_rect);
                return;
            }
        }
        
        // Add as new dirty rect (limit to avoid too many)
        if self.dirty_rects.len() < 10 {
            self.dirty_rects.push(new_rect);
        } else {
            // Too many dirty rects, just do a full redraw
            self.full_redraw_needed = true;
            self.dirty_rects.clear();
        }
    }
    
    /// Mark the entire screen for redraw
    pub fn mark_full_redraw(&mut self) {
        self.full_redraw_needed = true;
        self.dirty_rects.clear();
    }
    
    /// Mark dirty region for mouse movement (old and new positions)
    pub fn mark_mouse_dirty(&mut self) {
        // Mark old position (to restore background)
        self.mark_dirty(
            self.old_mouse_x - 2, 
            self.old_mouse_y - 2, 
            CURSOR_SIZE as u32 + 4, 
            CURSOR_SIZE as u32 + 4
        );
        // Mark new position (to draw cursor)
        self.mark_dirty(
            self.mouse_x - 2, 
            self.mouse_y - 2, 
            CURSOR_SIZE as u32 + 4, 
            CURSOR_SIZE as u32 + 4
        );
    }

    /// Update mouse position
    pub fn update_mouse(&mut self, x: i32, y: i32) {
        self.old_mouse_x = self.mouse_x;
        self.old_mouse_y = self.mouse_y;
        // Trust that coordinates from mouse driver are already clamped
        self.mouse_x = x;
        self.mouse_y = y;
        // Mark mouse region as dirty for efficient redraw
        self.mark_mouse_dirty();
    }

    /// Get mouse position
    pub fn mouse_position(&self) -> (i32, i32) {
        (self.mouse_x, self.mouse_y)
    }

    fn setup_dock_icons(&mut self) {
        // Calculate dock position (centered at bottom)
        let screen_width = 1280; // Default, will be updated on draw
        let dock_width = (self.dock_icon_size + 16) * 3; // 3 icons with padding
        let dock_x = (screen_width - dock_width) / 2;
        let dock_y = 800 - self.dock_height - 8; // 8px from bottom

        self.dock_icons = vec![
            Icon {
                x: dock_x as i32 + 8,
                y: dock_y as i32 + 8,
                width: self.dock_icon_size,
                height: self.dock_icon_size,
                label: "Browser".to_string(),
                icon_char: 'B',
                icon_path: Some("system/icons/globe_icon_64.png".to_string()),
                action: IconAction::LaunchApp("browser".to_string()),
                is_folder: false,
            },
            Icon {
                x: dock_x as i32 + 8 + 64,
                y: dock_y as i32 + 8,
                width: self.dock_icon_size,
                height: self.dock_icon_size,
                label: "App Store".to_string(),
                icon_char: 'A',
                icon_path: None, // No icon for app store yet
                action: IconAction::LaunchApp("appstore".to_string()),
                is_folder: false,
            },
            Icon {
                x: dock_x as i32 + 8 + 128,
                y: dock_y as i32 + 8,
                width: self.dock_icon_size,
                height: self.dock_icon_size,
                label: "Files".to_string(),
                icon_char: 'F',
                icon_path: Some("system/icons/filemanager_icon_64.png".to_string()),
                action: IconAction::LaunchApp("filemanager".to_string()),
                is_folder: false,
            },
        ];
    }

    fn setup_desktop_icons(&mut self) {
        // Desktop icons on the right side
        self.desktop_icons = vec![
            Icon {
                x: 1120,
                y: 40,
                width: 64,
                height: 80,
                label: "Documents".to_string(),
                icon_char: 'D',
                icon_path: Some("system/icons/folder_icon_64.png".to_string()),
                action: IconAction::OpenFolder("/home/user/documents".to_string()),
                is_folder: true,
            },
            Icon {
                x: 1120,
                y: 140,
                width: 64,
                height: 80,
                label: "Downloads".to_string(),
                icon_char: 'L',
                icon_path: Some("system/icons/folder_icon_64.png".to_string()),
                action: IconAction::OpenFolder("/home/user/downloads".to_string()),
                is_folder: true,
            },
            Icon {
                x: 1120,
                y: 240,
                width: 64,
                height: 80,
                label: "Apps".to_string(),
                icon_char: 'A',
                icon_path: Some("system/icons/apps_icon_64.png".to_string()),
                action: IconAction::OpenFolder("/Apps".to_string()),
                is_folder: true,
            },
        ];
    }

    /// Draw the desktop (optimized with dirty rectangles)
    pub fn draw(&mut self, driver: &mut VesaDriver) {
        let info = driver.info();
        let screen_w = info.width;
        let screen_h = info.height;
        
        // Update screen dimensions if changed
        self.screen_width = screen_w;
        self.screen_height = screen_h;

        if self.full_redraw_needed {
            // Full redraw - clear everything
            driver.clear(palette::DESKTOP_BG);
            self.draw_menu_bar(driver, screen_w);
            for icon in &self.desktop_icons {
                self.draw_desktop_icon(driver, icon);
            }
            if self.browser_open {
                self.draw_browser_window(driver);
            }
            self.draw_dock(driver, screen_w, screen_h);
            self.full_redraw_needed = false;
        } else if !self.dirty_rects.is_empty() {
            // Partial redraw - only redraw dirty regions
            // For simplicity, we'll redraw intersecting elements
            for rect in &self.dirty_rects {
                self.draw_region(driver, rect, screen_w, screen_h);
            }
        }
        
        // Always draw mouse cursor on top (but don't add to dirty rects)
        self.draw_mouse_cursor(driver);
        
        // Clear dirty rects after drawing
        self.dirty_rects.clear();
    }
    
    /// Draw a specific region of the screen
    fn draw_region(&self, driver: &mut VesaDriver, rect: &DirtyRect, screen_w: u32, screen_h: u32) {
        // Check if region intersects menu bar
        let menu_bar_rect = DirtyRect::new(0, 0, screen_w, self.menu_bar_height);
        if rect.intersects(&menu_bar_rect) {
            self.draw_menu_bar(driver, screen_w);
        }
        
        // Check if region intersects any desktop icon
        for icon in &self.desktop_icons {
            let icon_rect = DirtyRect::new(icon.x, icon.y, icon.width, icon.height);
            if rect.intersects(&icon_rect) {
                self.draw_desktop_icon(driver, icon);
            }
        }
        
        // Check if region intersects browser window
        if self.browser_open {
            let browser_rect = DirtyRect::new(BROWSER_X, BROWSER_Y, BROWSER_WIDTH, BROWSER_HEIGHT);
            if rect.intersects(&browser_rect) {
                self.draw_browser_window(driver);
            }
        }
        
        // Check if region intersects dock
        let dock_width = (self.dock_icon_size + 16) * self.dock_icons.len() as u32;
        let dock_x = (screen_w - dock_width) / 2;
        let dock_y = screen_h - self.dock_height - 8;
        let dock_rect = DirtyRect::new(dock_x as i32, dock_y as i32, dock_width + 16, self.dock_height + 16);
        if rect.intersects(&dock_rect) {
            // First clear the dock area to desktop background
            self.clear_region(driver, dock_rect.x, dock_rect.y, dock_rect.width, dock_rect.height, palette::DESKTOP_BG);
            self.draw_dock(driver, screen_w, screen_h);
        }
    }
    
    /// Clear a specific region to a color
    fn clear_region(&self, driver: &mut VesaDriver, x: i32, y: i32, width: u32, height: u32, color: u32) {
        let info = driver.info();
        let screen_w = info.width as i32;
        let screen_h = info.height as i32;
        
        let x0 = x.max(0);
        let y0 = y.max(0);
        let x1 = (x + width as i32).min(screen_w);
        let y1 = (y + height as i32).min(screen_h);
        
        for py in y0..y1 {
            for px in x0..x1 {
                driver.set_pixel(px as u32, py as u32, color);
            }
        }
    }

    /// Draw mouse cursor
    /// Save the pixels under the cursor to the save buffer
    fn save_under_cursor(&mut self, driver: &VesaDriver) {
        let info = driver.info();
        let screen_w = info.width as i32;
        let screen_h = info.height as i32;
        
        // Validate mouse position is within bounds before saving
        if self.mouse_x < 0 || self.mouse_y < 0 || 
           self.mouse_x >= screen_w || self.mouse_y >= screen_h {
            // Mouse out of bounds - mark buffer as invalid
            self.save_buffer_valid = false;
            return;
        }
        
        self.save_buffer_x = self.mouse_x;
        self.save_buffer_y = self.mouse_y;
        
        let mut idx = 0;
        for y in 0..CURSOR_SIZE as i32 {
            for x in 0..CURSOR_SIZE as i32 {
                let px = self.mouse_x + x;
                let py = self.mouse_y + y;
                
                if px >= 0 && px < screen_w && py >= 0 && py < screen_h {
                    self.save_buffer[idx] = driver.get_pixel(px as u32, py as u32);
                } else {
                    self.save_buffer[idx] = 0;
                }
                idx += 1;
            }
        }
        self.save_buffer_valid = true;
    }
    
    /// Restore the pixels from the save buffer (erase cursor)
    fn restore_under_cursor(&self, driver: &mut VesaDriver) {
        if !self.save_buffer_valid {
            return;
        }
        
        // Validate save buffer position is within bounds
        if self.save_buffer_x < 0 || self.save_buffer_y < 0 {
            return;
        }
        
        let info = driver.info();
        let screen_w = info.width as i32;
        let screen_h = info.height as i32;
        
        if self.save_buffer_x >= screen_w || self.save_buffer_y >= screen_h {
            return;
        }
        
        let mut idx = 0;
        for y in 0..CURSOR_SIZE as i32 {
            for x in 0..CURSOR_SIZE as i32 {
                let px = self.save_buffer_x + x;
                let py = self.save_buffer_y + y;
                
                if px >= 0 && px < screen_w && py >= 0 && py < screen_h {
                    driver.set_pixel(px as u32, py as u32, self.save_buffer[idx]);
                }
                idx += 1;
            }
        }
    }

    fn draw_mouse_cursor(&self, driver: &mut VesaDriver) {
        // Simple arrow cursor (11x16 pixels)
        let cursor_data: &[(i32, i32)] = &[
            // Arrow shape (x, y) offsets
            (0,0),(0,1),(0,2),(0,3),(0,4),(0,5),(0,6),(0,7),(0,8),(0,9),(0,10),
            (1,0),(1,1),(1,2),(1,3),(1,4),(1,5),(1,6),(1,7),(1,8),(1,9),
            (2,0),(2,1),(2,2),(2,3),(2,4),(2,5),(2,6),(2,7),(2,8),
            (3,0),(3,1),(3,2),(3,3),(3,4),(3,5),(3,6),(3,7),
            (4,0),(4,1),(4,2),(4,3),(4,4),(4,5),(4,6),
            (5,0),(5,1),(5,2),(5,3),(5,4),(5,5),(5,6),(5,7),
            (6,0),(6,1),(6,2),(6,3),(6,4),(6,5),(6,6),(6,7),(6,8),
            (7,0),(7,1),(7,2),(7,3),(7,4),(7,5),(7,6),(7,7),(7,8),(7,9),
            (8,0),(8,1),(8,2),(8,3),(8,4),(8,5),(8,6),(8,7),(8,8),(8,9),(8,10),
        ];

        // Get screen dimensions once, outside the loop
        let (screen_w, screen_h) = {
            let info = driver.info();
            (info.width, info.height)
        };

        // Draw white cursor with black border
        for &(ox, oy) in cursor_data {
            let px = self.mouse_x + ox;
            let py = self.mouse_y + oy;

            // Bounds check
            if px >= 0 && px < screen_w as i32 && py >= 0 && py < screen_h as i32 {
                // Black border
                driver.set_pixel(px as u32, py as u32, 0xFF000000);
                // White fill (slightly inside)
                if ox > 0 && oy > 0 && ox < 8 && oy < 9 {
                    driver.set_pixel(px as u32, py as u32, 0xFFFFFFFF);
                }
            }
        }
    }

    /// Redraw the area where the old cursor was (to erase it)
    fn redraw_cursor_area(&self, driver: &mut VesaDriver, x: i32, y: i32) {
        // Redraw a small 20x20 rectangle where the cursor was
        let cursor_size = 20;

        // Copy values from info before the loop to avoid holding immutable borrow
        let (screen_w, screen_h) = {
            let info = driver.info();
            (info.width, info.height)
        };

        for cy in 0..cursor_size {
            for cx in 0..cursor_size {
                let px = x + cx - 2; // Offset to cover cursor properly
                let py = y + cy - 2;

                if px < 0 || py < 0 || px >= screen_w as i32 || py >= screen_h as i32 {
                    continue;
                }

                // Redraw this pixel based on what should be there
                let color = self.get_background_color(px, py, screen_w, screen_h);
                driver.set_pixel(px as u32, py as u32, color);
            }
        }
    }

    /// Get the color that should be at a given position (based on desktop layout)
    fn get_background_color(&self, x: i32, y: i32, screen_w: u32, screen_h: u32) -> u32 {
        // Bounds check first
        if x < 0 || y < 0 || x >= screen_w as i32 || y >= screen_h as i32 {
            return palette::DESKTOP_BG; // Return default for out of bounds
        }

        // Menu bar area
        if y < self.menu_bar_height as i32 {
            return palette::MENU_BAR_BG;
        }

        // Dock area - check for underflow
        if screen_h > self.dock_height + 8 {
            let dock_y = screen_h - self.dock_height - 8;
            if y >= dock_y as i32 {
                let dock_width = (self.dock_icon_size + 16) * self.dock_icons.len() as u32 + 16;

                // Check for underflow in dock_x calculation
                if screen_w > dock_width {
                    let dock_x = (screen_w - dock_width) / 2;

                    if x >= dock_x as i32 && x < (dock_x + dock_width) as i32 {
                        return palette::DOCK_BG;
                    }
                }
            }
        }

        // Check if inside browser window
        if self.browser_open {
            if x >= BROWSER_X && x < (BROWSER_X + BROWSER_WIDTH as i32) &&
               y >= BROWSER_Y && y < (BROWSER_Y + BROWSER_HEIGHT as i32) {
                // Inside browser - would need more detailed check
                return palette::WINDOW_BG;
            }
        }

        // Default desktop background
        palette::DESKTOP_BG
    }

    /// Draw browser window
    fn draw_browser_window(&self, driver: &mut VesaDriver) {
        // Window shadow (optional, simple darkening)
        driver.fill_rect(
            BROWSER_X + 4,
            BROWSER_Y + 4,
            BROWSER_WIDTH,
            BROWSER_HEIGHT,
            0x80000000 // Semi-transparent black
        );

        // Window background
        driver.fill_rect(
            BROWSER_X,
            BROWSER_Y,
            BROWSER_WIDTH,
            BROWSER_HEIGHT,
            palette::WINDOW_BG
        );

        // Title bar
        driver.fill_rect(
            BROWSER_X,
            BROWSER_Y,
            BROWSER_WIDTH,
            32,
            palette::WINDOW_TITLE
        );

        // Window border
        driver.draw_rect(
            BROWSER_X,
            BROWSER_Y,
            BROWSER_WIDTH,
            BROWSER_HEIGHT,
            0xFF888888
        );

        // Title text
        driver.draw_text("WebbOS Browser", BROWSER_X + 40, BROWSER_Y + 10, palette::TEXT_BLACK, 1);

        // Traffic light buttons (macOS style - left side)
        // Close (red)
        driver.fill_circle(BROWSER_X + 12, BROWSER_Y + 16, 6, 0xFFFF5F56);
        // Minimize (yellow)
        driver.fill_circle(BROWSER_X + 32, BROWSER_Y + 16, 6, 0xFFFFBD2E);
        // Maximize (green)
        driver.fill_circle(BROWSER_X + 52, BROWSER_Y + 16, 6, 0xFF27C93F);

        // Address bar
        driver.fill_rect(
            BROWSER_X + 80,
            BROWSER_Y + 40,
            BROWSER_WIDTH - 160,
            28,
            0xFFFFFFFF
        );
        driver.draw_rect(
            BROWSER_X + 80,
            BROWSER_Y + 40,
            BROWSER_WIDTH - 160,
            28,
            0xFFCCCCCC
        );
        driver.draw_text("https://webbos.local", BROWSER_X + 90, BROWSER_Y + 48, 0xFF666666, 1);

        // Content area with welcome message
        let content_y = BROWSER_Y + 80;
        driver.draw_text("Welcome to WebbOS Browser!", BROWSER_X + 40, content_y, palette::TEXT_BLACK, 2);
        driver.draw_text("A minimal web browser built into the OS", BROWSER_X + 40, content_y + 40, 0xFF666666, 1);

        // Some demo content
        driver.draw_text("Features:", BROWSER_X + 40, content_y + 80, palette::TEXT_BLACK, 1);
        driver.draw_text("- HTML5 parsing engine", BROWSER_X + 60, content_y + 100, 0xFF666666, 1);
        driver.draw_text("- CSS3 styling support", BROWSER_X + 60, content_y + 120, 0xFF666666, 1);
        driver.draw_text("- JavaScript interpreter", BROWSER_X + 60, content_y + 140, 0xFF666666, 1);
        driver.draw_text("- WebAssembly runtime", BROWSER_X + 60, content_y + 160, 0xFF666666, 1);

        // Navigation buttons
        driver.fill_rect(BROWSER_X + 10, BROWSER_Y + 40, 30, 28, 0xFFE0E0E0);
        driver.draw_text("<", BROWSER_X + 20, BROWSER_Y + 48, palette::TEXT_BLACK, 1);

        driver.fill_rect(BROWSER_X + 45, BROWSER_Y + 40, 30, 28, 0xFFE0E0E0);
        driver.draw_text(">", BROWSER_X + 55, BROWSER_Y + 48, palette::TEXT_BLACK, 1);
    }

    fn draw_menu_bar(&self, driver: &mut VesaDriver, screen_w: u32) {
        // Menu bar background
        driver.fill_rect(0, 0, screen_w, self.menu_bar_height, palette::MENU_BAR_BG);

        // Apple logo (W for Webb)
        driver.draw_text("W", 10, 6, palette::MENU_BAR_TEXT, 1);

        // System info (right side)
        let time_str = "12:00";
        driver.draw_text(time_str, (screen_w - 60) as i32, 6, palette::MENU_BAR_TEXT, 1);
    }

    fn draw_dock(&self, driver: &mut VesaDriver, screen_w: u32, screen_h: u32) {
        let dock_width = (self.dock_icon_size + 16) * self.dock_icons.len() as u32 + 16;
        let dock_x = (screen_w - dock_width) / 2;
        let dock_y = screen_h - self.dock_height - 8;

        // Dock background (rounded rectangle - simplified as rectangle)
        driver.fill_rect(
            dock_x as i32,
            dock_y as i32,
            dock_width,
            self.dock_height,
            palette::DOCK_BG
        );

        // Dock border
        driver.draw_rect(
            dock_x as i32,
            dock_y as i32,
            dock_width,
            self.dock_height,
            palette::DOCK_BORDER
        );

        // Draw dock icons
        for icon in &self.dock_icons {
            self.draw_dock_icon(driver, icon);
        }
    }

    fn draw_dock_icon(&self, driver: &mut VesaDriver, icon: &Icon) {
        use crate::desktop::embedded_icons;

        // Icon background
        driver.fill_rect(
            icon.x,
            icon.y,
            icon.width,
            icon.height,
            palette::ICON_BG
        );

        // Icon border
        driver.draw_rect(
            icon.x,
            icon.y,
            icon.width,
            icon.height,
            palette::DOCK_BORDER
        );

        // Draw embedded icon if available based on icon_path
        let icon_drawn = if let Some(ref path) = icon.icon_path {
            if path.contains("globe") {
                self.draw_rgba_icon(driver, icon.x, icon.y,
                    embedded_icons::GLOBE_ICON_DATA,
                    embedded_icons::GLOBE_ICON_WIDTH,
                    embedded_icons::GLOBE_ICON_HEIGHT);
                true
            } else if path.contains("filemanager") {
                self.draw_rgba_icon(driver, icon.x, icon.y,
                    embedded_icons::FILEMANAGER_ICON_DATA,
                    embedded_icons::FILEMANAGER_ICON_WIDTH,
                    embedded_icons::FILEMANAGER_ICON_HEIGHT);
                true
            } else {
                false
            }
        } else {
            false
        };

        // Fallback to character display if no icon was drawn
        if !icon_drawn {
            let char_x = icon.x + (icon.width as i32 / 2) - 8;
            let char_y = icon.y + (icon.height as i32 / 2) - 8;
            driver.draw_char(icon.icon_char, char_x, char_y, palette::TEXT_BLACK, 2);
        }

        // Label (below icon)
        let label_x = icon.x + (icon.width as i32 / 2) - ((icon.label.len() as i32 * 4));
        driver.draw_text(&icon.label, label_x, icon.y + icon.height as i32 + 4, palette::TEXT_WHITE, 1);
    }

    /// Draw an RGBA icon from embedded data
    fn draw_rgba_icon(&self, driver: &mut VesaDriver, x: i32, y: i32, data: &[u8], width: u32, height: u32) {
        // Data is in RGBA format (4 bytes per pixel)
        for py in 0..height {
            for px in 0..width {
                let idx = ((py * width + px) * 4) as usize;
                if idx + 3 >= data.len() {
                    continue;
                }

                let r = data[idx];
                let g = data[idx + 1];
                let b = data[idx + 2];
                let a = data[idx + 3];

                // Simple alpha blending (alpha > 128 = opaque)
                if a > 128 {
                    let color = ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
                    driver.set_pixel((x + px as i32) as u32, (y + py as i32) as u32, color | 0xFF000000);
                }
            }
        }
    }

    fn draw_desktop_icon(&self, driver: &mut VesaDriver, icon: &Icon) {
        use crate::desktop::embedded_icons;

        // Icon background (slightly rounded)
        driver.fill_rect(
            icon.x,
            icon.y,
            icon.width,
            icon.height - 16, // Space for label
            palette::ICON_BG
        );

        // Draw embedded icon if available based on icon_path
        let icon_drawn = if let Some(ref path) = icon.icon_path {
            if path.contains("folder") {
                self.draw_rgba_icon(driver, icon.x, icon.y,
                    embedded_icons::FOLDER_ICON_DATA,
                    embedded_icons::FOLDER_ICON_WIDTH,
                    embedded_icons::FOLDER_ICON_HEIGHT);
                true
            } else {
                false
            }
        } else {
            false
        };

        // Fallback to character display if no icon was drawn
        if !icon_drawn {
            let char_x = icon.x + (icon.width as i32 / 2) - 16;
            let char_y = icon.y + 12;
            driver.draw_char(icon.icon_char, char_x, char_y, palette::TEXT_BLACK, 4);
        }

        // Label (below icon)
        let label_x = icon.x + (icon.width as i32 / 2) - ((icon.label.len() as i32 * 4));
        driver.draw_text(&icon.label, label_x, icon.y + icon.height as i32 - 12, palette::TEXT_WHITE, 1);
    }

    /// Handle mouse click (now launches apps on single click)
    pub fn handle_click(&mut self, x: i32, y: i32) -> bool {
        // Check menu bar volume control area (right side)
        let screen_w = self.screen_width;
        if y >= 0 && y < self.menu_bar_height as i32 {
            // Volume area: from (screen_w - 130) to (screen_w - 65)
            let vol_x_start = (screen_w - 130) as i32;
            let vol_x_end = (screen_w - 65) as i32;
            if x >= vol_x_start && x < vol_x_end {
                // Audio not supported on PC platform
                println!("[desktop] Audio not supported on this platform");
                // Mark menu bar as dirty for redraw
                self.mark_dirty(0, 0, screen_w, self.menu_bar_height);
                return true;
            }
        }
        
        // Check if clicking browser close button (when browser is open)
        if self.browser_open {
            let close_x = BROWSER_X + 12;
            let close_y = BROWSER_Y + 16;
            let dist_sq = (x - close_x) * (x - close_x) + (y - close_y) * (y - close_y);
            if dist_sq < 36 { // Within 6px radius
                println!("[desktop] Closing browser window");
                self.browser_open = false;
                // Mark entire browser window area as dirty
                self.mark_dirty(BROWSER_X, BROWSER_Y, BROWSER_WIDTH, BROWSER_HEIGHT);
                return true; // Redraw needed
            }
        }
        
        // Check dock icons first (launch on single click)
        let mut file_manager_clicked = false;
        for icon in &self.dock_icons {
            if x >= icon.x && x < icon.x + icon.width as i32 &&
               y >= icon.y && y < icon.y + icon.height as i32 {
                println!("[desktop] Clicked dock icon: {}", icon.label);
                match &icon.action {
                    IconAction::LaunchApp(app_name) => {
                        if app_name == "browser" {
                            println!("[desktop] Opening browser window");
                            self.browser_open = true;
                            // Mark entire browser window area as dirty
                            self.mark_dirty(BROWSER_X, BROWSER_Y, BROWSER_WIDTH, BROWSER_HEIGHT);
                            return true;
                        } else if app_name == "appstore" {
                            println!("[desktop] App Store coming soon!");
                        } else if app_name == "filemanager" {
                            file_manager_clicked = true;
                        }
                    }
                    IconAction::OpenHtmlFile(path) => {
                        println!("[desktop] Opening HTML file: {}", path);
                        self.browser_open = true;
                        crate::desktop::launch_html(path);
                        self.mark_dirty(BROWSER_X, BROWSER_Y, BROWSER_WIDTH, BROWSER_HEIGHT);
                        return true;
                    }
                    _ => {}
                }
                // Mark dock icon as dirty
                self.mark_dirty(icon.x - 4, icon.y - 4, icon.width + 8, icon.height + 8);
                
                // Handle file manager click after loop to avoid borrow issues
                if file_manager_clicked {
                    println!("[desktop] Opening File Manager...");
                    self.open_file_manager_window("/");
                }
                return true;
            }
        }
        
        // Check desktop icons (select on single click)
        let mut clicked_icon_idx = None;
        for (idx, icon) in self.desktop_icons.iter().enumerate() {
            if x >= icon.x && x < icon.x + icon.width as i32 &&
               y >= icon.y && y < icon.y + icon.height as i32 {
                clicked_icon_idx = Some(idx);
                break;
            }
        }
        
        if let Some(idx) = clicked_icon_idx {
            // Copy all values we need before calling mark_dirty
            let (new_x, new_y, new_w, new_h, label) = {
                let icon = &self.desktop_icons[idx];
                (icon.x, icon.y, icon.width, icon.height, icon.label.clone())
            };
            println!("[desktop] Selected icon: {}", label);
            
            // Mark old selection as dirty (to remove highlight)
            if let Some(old_idx) = self.selected_icon {
                if old_idx != idx {
                    let (old_x, old_y, old_w, old_h) = {
                        let old_icon = &self.desktop_icons[old_idx];
                        (old_icon.x, old_icon.y, old_icon.width, old_icon.height)
                    };
                    self.mark_dirty(old_x - 4, old_y - 4, old_w + 8, old_h + 8);
                }
            }
            self.selected_icon = Some(idx);
            // Mark new selection as dirty
            self.mark_dirty(new_x - 4, new_y - 4, new_w + 8, new_h + 8);
            return true;
        }
        
        // Clicked elsewhere - clear selection
        if self.selected_icon.is_some() {
            let old_idx = self.selected_icon.unwrap();
            let (old_x, old_y, old_w, old_h) = {
                let old_icon = &self.desktop_icons[old_idx];
                (old_icon.x, old_icon.y, old_icon.width, old_icon.height)
            };
            self.mark_dirty(old_x - 4, old_y - 4, old_w + 8, old_h + 8);
            self.selected_icon = None;
            return true;
        }
        false // No redraw needed
    }
    
    pub fn handle_double_click(&mut self, x: i32, y: i32) -> bool {
        // Double-click now does the same as single-click for simplicity
        // In the future this could do something different (e.g., open properties)
        self.handle_click(x, y)
    }

}

/// Global desktop UI instance
lazy_static! {
    static ref DESKTOP_UI: Mutex<DesktopUI> = Mutex::new(DesktopUI::new());
}

/// Show the desktop
pub fn show() {
    println!("[desktop_ui] Showing graphical desktop");
    let mut driver = vesa::driver().lock();

    if !driver.is_initialized() {
        println!("[desktop_ui] VESA driver not initialized!");
        return;
    }

    {
        let mut desktop = DESKTOP_UI.lock();
        desktop.draw(&mut driver);
        
        // Initial save of area under cursor
        desktop.save_under_cursor(&mut driver);
        desktop.draw_mouse_cursor(&mut driver);
        
        println!("[desktop_ui] Desktop drawn, ready for interaction");
    }
}

/// Update mouse position and redraw only the cursor
pub fn update_mouse(x: i32, y: i32) {
    // Use atomics instead of static mut for thread safety
    static UPDATE_COUNT: AtomicU32 = AtomicU32::new(0);
    static LAST_PRINT: AtomicU64 = AtomicU64::new(0);
    static TRACE_COUNT: AtomicU32 = AtomicU32::new(0);
    
    // Trace first few calls to verify flow
    let trace = TRACE_COUNT.fetch_add(1, Ordering::Relaxed);
    let should_trace = trace < 20;
    
    if should_trace {
        crate::println!("[mouse-trace] update_mouse({},{}) start", x, y);
    }
    
    // CRITICAL: Lock ordering must match show() - driver first, then desktop!
    // This prevents deadlock with the event loop
    if should_trace {
        crate::println!("[mouse-trace] acquiring driver lock...");
    }
    let mut driver = vesa::driver().lock();
    if should_trace {
        crate::println!("[mouse-trace] driver lock acquired");
    }
    
    if !driver.is_initialized() {
        if should_trace {
            crate::println!("[mouse-trace] driver not initialized, returning");
        }
        return;
    }
    
    if should_trace {
        crate::println!("[mouse-trace] acquiring desktop lock...");
    }
    let mut desktop = DESKTOP_UI.lock();
    if should_trace {
        crate::println!("[mouse-trace] desktop lock acquired");
    }
    
    // Only update if mouse actually moved significantly (reduce updates)
    let dx = (x - desktop.mouse_x).abs();
    let dy = (y - desktop.mouse_y).abs();
    if dx < 1 && dy < 1 {
        if should_trace {
            crate::println!("[mouse-trace] movement too small, returning");
        }
        return; // Ignore tiny movements (reduced from 2 to 1)
    }

    // Bounds check to prevent drawing outside screen
    let info = driver.info();
    if x < 0 || y < 0 || x >= info.width as i32 || y >= info.height as i32 {
        if should_trace {
            crate::println!("[mouse-trace] out of bounds, returning");
        }
        return; // Mouse coordinates out of bounds, skip update
    }

    // Atomically update counters
    let count = UPDATE_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    let current = crate::arch::interrupts::get_timer_ticks();
    let last = LAST_PRINT.load(Ordering::Relaxed);
    
    if current > last + 500 {
        if count > 0 {
            crate::println!("[desktop] Mouse updates: {}, pos: ({},{})", 
                count, x, y);
        }
        UPDATE_COUNT.store(0, Ordering::Relaxed);
        LAST_PRINT.store(current, Ordering::Relaxed);
    }

    if should_trace {
        crate::println!("[mouse-trace] restoring under cursor...");
    }
    // Restore old position (erase cursor)
    desktop.restore_under_cursor(&mut driver);
    
    if should_trace {
        crate::println!("[mouse-trace] updating position...");
    }
    // Update mouse position
    desktop.update_mouse(x, y);
    
    if should_trace {
        crate::println!("[mouse-trace] saving under cursor...");
    }
    // Save new position and draw cursor
    desktop.save_under_cursor(&mut driver);
    
    if should_trace {
        crate::println!("[mouse-trace] drawing cursor...");
    }
    desktop.draw_mouse_cursor(&mut driver);
    
    if should_trace {
        crate::println!("[mouse-trace] update_mouse done");
    }
}

/// Handle mouse click (single)
pub fn handle_click(x: i32, y: i32) {
    // CRITICAL: Lock ordering must match show() - driver first, then desktop!
    let mut driver = vesa::driver().lock();
    if !driver.is_initialized() {
        return;
    }
    
    let mut desktop = DESKTOP_UI.lock();
    let needs_redraw = desktop.handle_click(x, y);

    if needs_redraw {
        desktop.draw(&mut driver);
    }
}

/// Handle mouse double-click
pub fn handle_double_click(x: i32, y: i32) {
    let mut driver = vesa::driver().lock();
    if !driver.is_initialized() {
        return;
    }
    
    let mut desktop = DESKTOP_UI.lock();
    let needs_redraw = desktop.handle_double_click(x, y);

    if needs_redraw {
        desktop.draw(&mut driver);
    }
}

/// Check if desktop is active
pub fn is_active() -> bool {
    true // Desktop is always active after login
}

/// Navigate browser to URL (called from HTML frontend via message)
pub fn browser_navigate(url: &str) {
    println!("[desktop] Browser navigate requested: {}", url);
    
    // Try to navigate the browser
    match crate::browser::navigate(url) {
        Ok(()) => {
            println!("[desktop] Browser navigation successful");
        }
        Err(e) => {
            println!("[desktop] Browser navigation failed: {:?}", e);
        }
    }
}

impl DesktopUI {
    /// Open file manager window showing files from filesystem
    fn open_file_manager_window(&mut self, path: &str) {
        println!("[desktop] Opening File Manager at: {}", path);
        
        // Scan directory for HTML files
        match crate::fs::read_dir(path) {
            Ok(entries) => {
                println!("[desktop] Found {} entries in {}", entries.len(), path);
                
                // Add HTML files as desktop icons dynamically
                let mut x_pos = 100;
                let mut y_pos = 400;
                
                for (name, is_dir) in entries {
                    if is_dir {
                        println!("[desktop]  [DIR]  {}", name);
                    } else if name.ends_with(".html") || name.ends_with(".htm") {
                        println!("[desktop]  [HTML] {}", name);
                        
                        // Add HTML file as a launchable icon
                        let full_path = format!("{}/{}", path, name);
                        self.desktop_icons.push(Icon {
                            x: x_pos,
                            y: y_pos,
                            width: 80,
                            height: 96,
                            label: name.clone(),
                            icon_char: '📄',
                            icon_path: Some("html".to_string()),
                            action: IconAction::OpenHtmlFile(full_path),
                            is_folder: false,
                        });
                        
                        // Position next icon
                        x_pos += 100;
                        if x_pos > 1000 {
                            x_pos = 100;
                            y_pos += 120;
                        }
                    } else {
                        println!("[desktop]  [FILE] {}", name);
                    }
                }
                
                // Mark area as dirty to show new icons
                self.mark_full_redraw();
            }
            Err(e) => {
                println!("[desktop] Failed to read directory {}: {:?}", path, e);
            }
        }
    }
}

/// Open file manager with given path (public API)
pub fn open_file_manager(path: &str) {
    println!("[desktop] Opening file manager at: {}", path);
    // This is called from external modules - actual UI update happens via message queue
}
