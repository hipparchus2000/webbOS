//! Graphical Desktop UI (macOS-style)
//!
//! Renders a desktop environment with:
//! - Menu bar at top
//! - Dock at bottom
//! - Desktop icons
//! - Mouse click detection
//! - Window management with damage tracking and double buffering

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::vec;
use alloc::format;
use spin::Mutex;
use lazy_static::lazy_static;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use crate::drivers::vesa::{self, VesaDriver, colors};
use crate::println;

/// Rectangle for damage tracking and hit testing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    /// Create a new rectangle
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }

    /// Check if a point is inside this rectangle
    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x && px < self.x + self.width as i32 &&
        py >= self.y && py < self.y + self.height as i32
    }

    /// Check if this rectangle intersects with another
    pub fn intersects(&self, other: &Rect) -> bool {
        self.x < other.x + other.width as i32 &&
        self.x + self.width as i32 > other.x &&
        self.y < other.y + other.height as i32 &&
        self.y + self.height as i32 > other.y
    }

    /// Get the intersection of two rectangles
    pub fn intersection(&self, other: &Rect) -> Option<Rect> {
        let x1 = self.x.max(other.x);
        let y1 = self.y.max(other.y);
        let x2 = (self.x + self.width as i32).min(other.x + other.width as i32);
        let y2 = (self.y + self.height as i32).min(other.y + other.height as i32);

        if x1 < x2 && y1 < y2 {
            Some(Rect::new(x1, y1, (x2 - x1) as u32, (y2 - y1) as u32))
        } else {
            None
        }
    }

    /// Union two rectangles (return bounds that contain both)
    pub fn union(&self, other: &Rect) -> Rect {
        let x1 = self.x.min(other.x);
        let y1 = self.y.min(other.y);
        let x2 = (self.x + self.width as i32).max(other.x + other.width as i32);
        let y2 = (self.y + self.height as i32).max(other.y + other.height as i32);

        Rect::new(x1, y1, (x2 - x1) as u32, (y2 - y1) as u32)
    }
}

/// Integer square root using binary search
fn integer_sqrt(n: i32) -> i32 {
    if n <= 0 {
        return 0;
    }
    if n < 4 {
        return 1;
    }
    
    let mut lo = 1i32;
    let mut hi = n / 2;
    
    while lo <= hi {
        let mid = lo + (hi - lo) / 2;
        let sq = mid.saturating_mul(mid);
        
        if sq == n {
            return mid;
        } else if sq < n {
            lo = mid + 1;
        } else {
            hi = mid - 1;
        }
    }
    
    hi // Return floor of sqrt
}

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
    pub const WINDOW_TITLE_ACTIVE: u32 = 0xFFE8E8E8;     // Light gray for active window
    pub const WINDOW_TITLE_INACTIVE: u32 = 0xFFF0F0F0;   // Slightly lighter for inactive
    pub const WINDOW_BORDER: u32 = 0xFF888888;    // Window border color
    pub const TEXT_BLACK: u32 = 0xFF000000;
    pub const TEXT_WHITE: u32 = 0xFFFFFFFF;
    pub const BUTTON_CLOSE: u32 = 0xFFFF5F56;     // Red close button
    pub const BUTTON_MINIMIZE: u32 = 0xFFFFBD2E;  // Yellow minimize button
    pub const BUTTON_MAXIMIZE: u32 = 0xFF27C93F;  // Green maximize button
    pub const URL_BAR_BG: u32 = 0xFFFFFFFF;       // White URL bar
    pub const URL_BAR_BORDER: u32 = 0xFFCCCCCC;   // URL bar border
    pub const URL_BAR_TEXT: u32 = 0xFF666666;     // URL text color
    pub const INPUT_CURSOR: u32 = 0xFF007AFF;     // Cursor color
    pub const RESIZE_HANDLE: u32 = 0xFF999999;    // Resize handle color
}

/// Window state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowState {
    Normal,
    Minimized,
    Maximized,
}

/// Window structure for desktop windows
#[derive(Debug, Clone)]
pub struct Window {
    pub id: u32,
    pub title: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub state: WindowState,
    pub is_active: bool,
    pub url: String,           // URL for browser windows
    pub is_browser: bool,      // Is this a browser window
    pub url_input_focused: bool, // Is URL input focused
    pub url_cursor_pos: usize,   // Cursor position in URL input
}

/// Title bar constants
const TITLE_BAR_HEIGHT: u32 = 32;
const BUTTON_SIZE: i32 = 12;
const BUTTON_SPACING: i32 = 8;
const RESIZE_BORDER: i32 = 8;

/// Desktop icon type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    Folder,
    Text,
    Image,
    Audio,
    Video,
    Archive,
    Pdf,
    Code,
    Executable,
    Unknown,
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
    pub file_type: FileType,       // Type of file for icon selection
    pub is_selected: bool,         // Selection state
}

#[derive(Debug, Clone)]
pub enum IconAction {
    LaunchApp(String),      // Launch application by name
    OpenFolder(String),     // Open folder in file manager
    OpenFile(String),       // Open file with default app
    None,
}

/// Cursor save-under buffer size (must be larger than cursor)
const CURSOR_SIZE: usize = 16;
const SAVE_BUFFER_SIZE: usize = CURSOR_SIZE * CURSOR_SIZE;

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
    desktop_folder_scanned: bool,
    // Save-under buffer for mouse cursor (stores pixels under cursor)
    save_buffer: [u32; SAVE_BUFFER_SIZE],
    save_buffer_valid: bool,
    save_buffer_x: i32,
    save_buffer_y: i32,
    
    // === NEW: Damage tracking ===
    /// Dirty regions that need redrawing
    dirty_regions: Vec<Rect>,
    /// Full redraw requested
    full_redraw_needed: bool,
    
    // === NEW: Double buffering ===
    /// Back buffer for double buffering
    back_buffer: Vec<u32>,
    /// Back buffer dimensions
    back_buffer_width: u32,
    back_buffer_height: u32,
    
    // === NEW: Window management ===
    /// Windows list (browser is now a proper window)
    windows: Vec<Window>,
    /// Active window ID (for focus)
    active_window_id: Option<u32>,
    /// Next window ID counter
    next_window_id: u32,
    /// Drag state
    is_dragging: bool,
    drag_window_id: Option<u32>,
    drag_start_x: i32,
    drag_start_y: i32,
    drag_window_start_x: i32,
    drag_window_start_y: i32,
    /// Resize state
    is_resizing: bool,
    resize_window_id: Option<u32>,
    resize_start_x: i32,
    resize_start_y: i32,
    resize_start_width: u32,
    resize_start_height: u32,
}

/// Default browser window dimensions
const BROWSER_DEFAULT_WIDTH: u32 = 1000;
const BROWSER_DEFAULT_HEIGHT: u32 = 700;
const BROWSER_DEFAULT_X: i32 = 140;
const BROWSER_DEFAULT_Y: i32 = 50;

/// Desktop icon layout constants
const DESKTOP_ICON_WIDTH: u32 = 64;
const DESKTOP_ICON_HEIGHT: u32 = 80;
const DESKTOP_ICON_SPACING_X: i32 = 20;
const DESKTOP_ICON_SPACING_Y: i32 = 20;
const DESKTOP_START_X: i32 = 40;
const DESKTOP_START_Y: i32 = 60;
const DESKTOP_ICONS_PER_COLUMN: u32 = 8;

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
            desktop_folder_scanned: false,
            save_buffer: [0; SAVE_BUFFER_SIZE],
            save_buffer_valid: false,
            save_buffer_x: 0,
            save_buffer_y: 0,
            
            // Damage tracking
            dirty_regions: Vec::new(),
            full_redraw_needed: true,
            
            // Double buffering (will be initialized on first draw)
            back_buffer: Vec::new(),
            back_buffer_width: 0,
            back_buffer_height: 0,
            
            // Window management
            windows: Vec::new(),
            active_window_id: None,
            next_window_id: 1,
            is_dragging: false,
            drag_window_id: None,
            drag_start_x: 0,
            drag_start_y: 0,
            drag_window_start_x: 0,
            drag_window_start_y: 0,
            is_resizing: false,
            resize_window_id: None,
            resize_start_x: 0,
            resize_start_y: 0,
            resize_start_width: 0,
            resize_start_height: 0,
        };

        // Create dock icons (centered at bottom)
        ui.setup_dock_icons();
        ui.setup_default_desktop_icons();

        ui
    }

    // === NEW: Damage tracking methods ===

    /// Mark a region as dirty (needs redrawing)
    pub fn request_redraw(&mut self, rect: Rect) {
        // Merge with existing regions if possible, or add new
        let mut merged = false;
        
        // Try to merge with existing dirty regions
        for existing in &mut self.dirty_regions {
            // Simple merge: if rectangles overlap significantly, expand to union
            if existing.intersects(&rect) || Self::should_merge_static(existing, &rect) {
                *existing = existing.union(&rect);
                merged = true;
                break;
            }
        }
        
        if !merged {
            self.dirty_regions.push(rect);
        }
        
        // Limit number of dirty regions to prevent explosion
        if self.dirty_regions.len() > 50 {
            // Merge all into one big region
            let mut combined = self.dirty_regions[0];
            for region in &self.dirty_regions[1..] {
                combined = combined.union(region);
            }
            self.dirty_regions.clear();
            self.dirty_regions.push(combined);
        }
    }

    /// Request full screen redraw
    pub fn request_full_redraw(&mut self) {
        self.full_redraw_needed = true;
        self.dirty_regions.clear();
    }

    /// Check if two rectangles should be merged (close to each other)
    fn should_merge(&self, a: &Rect, b: &Rect) -> bool {
        Self::should_merge_static(a, b)
    }
    
    /// Static version to avoid self borrow issues
    fn should_merge_static(a: &Rect, b: &Rect) -> bool {
        const MERGE_THRESHOLD: i32 = 50; // Pixels
        
        let expanded_a = Rect::new(
            a.x - MERGE_THRESHOLD,
            a.y - MERGE_THRESHOLD,
            a.width + MERGE_THRESHOLD as u32 * 2,
            a.height + MERGE_THRESHOLD as u32 * 2,
        );
        
        expanded_a.intersects(b)
    }

    /// Clear all dirty regions
    fn clear_dirty_regions(&mut self) {
        self.dirty_regions.clear();
        self.full_redraw_needed = false;
    }

    // === NEW: Double buffering methods ===

    /// Initialize back buffer with given dimensions
    fn init_back_buffer(&mut self, width: u32, height: u32) {
        if self.back_buffer_width != width || self.back_buffer_height != height {
            let size = (width * height) as usize;
            self.back_buffer.resize(size, 0);
            self.back_buffer_width = width;
            self.back_buffer_height = height;
        }
    }

    /// Get pixel from back buffer
    fn get_back_buffer_pixel(&self, x: u32, y: u32) -> u32 {
        if x < self.back_buffer_width && y < self.back_buffer_height {
            let idx = (y * self.back_buffer_width + x) as usize;
            self.back_buffer.get(idx).copied().unwrap_or(0)
        } else {
            0
        }
    }

    /// Set pixel in back buffer
    fn set_back_buffer_pixel(&mut self, x: u32, y: u32, color: u32) {
        if x < self.back_buffer_width && y < self.back_buffer_height {
            let idx = (y * self.back_buffer_width + x) as usize;
            if let Some(pixel) = self.back_buffer.get_mut(idx) {
                *pixel = color;
            }
        }
    }

    /// Clear back buffer with color
    fn clear_back_buffer(&mut self, color: u32) {
        for pixel in &mut self.back_buffer {
            *pixel = color;
        }
    }

    /// Fill rectangle in back buffer
    fn fill_rect_back_buffer(&mut self, x: i32, y: i32, w: u32, h: u32, color: u32) {
        let x0 = x.max(0) as u32;
        let y0 = y.max(0) as u32;
        let x1 = ((x as u32) + w).min(self.back_buffer_width);
        let y1 = ((y as u32) + h).min(self.back_buffer_height);

        for py in y0..y1 {
            for px in x0..x1 {
                self.set_back_buffer_pixel(px, py, color);
            }
        }
    }

    /// Swap buffers - blit back buffer to screen
    pub fn swap_buffers(&mut self, driver: &mut VesaDriver) {
        let info = driver.info();
        let screen_w = info.width;
        let screen_h = info.height;

        // Blit entire back buffer to screen
        // Optimization: only blit dirty regions
        if self.full_redraw_needed || self.dirty_regions.is_empty() {
            // Full blit
            for y in 0..screen_h.min(self.back_buffer_height) {
                for x in 0..screen_w.min(self.back_buffer_width) {
                    let color = self.get_back_buffer_pixel(x, y);
                    driver.set_pixel(x, y, color);
                }
            }
        } else {
            // Partial blit - only dirty regions
            for region in &self.dirty_regions {
                let x0 = region.x.max(0) as u32;
                let y0 = region.y.max(0) as u32;
                let x1 = (region.x + region.width as i32).min(screen_w as i32).max(0) as u32;
                let y1 = (region.y + region.height as i32).min(screen_h as i32).max(0) as u32;

                for y in y0..y1 {
                    for x in x0..x1 {
                        let color = self.get_back_buffer_pixel(x, y);
                        driver.set_pixel(x, y, color);
                    }
                }
            }
        }
    }

    /// Present the frame - swap buffers and clear dirty regions
    pub fn present(&mut self, driver: &mut VesaDriver) {
        self.swap_buffers(driver);
        self.clear_dirty_regions();
    }

    // === NEW: Window management methods ===

    /// Create a new browser window
    pub fn create_browser_window(&mut self) -> u32 {
        let id = self.next_window_id;
        self.next_window_id += 1;

        // Calculate position (cascade from default)
        let offset = ((self.windows.len() as i32 * 30) % 200);
        let x = BROWSER_DEFAULT_X + offset;
        let y = BROWSER_DEFAULT_Y + offset;

        let window = Window {
            id,
            title: String::from("WebbOS Browser"),
            x,
            y,
            width: BROWSER_DEFAULT_WIDTH,
            height: BROWSER_DEFAULT_HEIGHT,
            state: WindowState::Normal,
            is_active: true,
            url: String::from("https://webbos.local"),
            is_browser: true,
            url_input_focused: false,
            url_cursor_pos: 0,
        };

        // Deactivate other windows
        for w in &mut self.windows {
            w.is_active = false;
        }

        self.windows.push(window);
        self.active_window_id = Some(id);

        // Request redraw of new window area
        self.request_redraw(Rect::new(x, y, BROWSER_DEFAULT_WIDTH, BROWSER_DEFAULT_HEIGHT));

        id
    }

    /// Close a window by ID
    pub fn close_window(&mut self, window_id: u32) -> bool {
        if let Some(idx) = self.windows.iter().position(|w| w.id == window_id) {
            let window = &self.windows[idx];
            // Mark window area as dirty for redraw
            self.request_redraw(Rect::new(window.x, window.y, window.width, window.height));
            
            self.windows.remove(idx);
            
            // Update active window
            if self.active_window_id == Some(window_id) {
                self.active_window_id = self.windows.last().map(|w| w.id);
                if let Some(new_active) = self.active_window_id {
                    if let Some(w) = self.windows.iter_mut().find(|w| w.id == new_active) {
                        w.is_active = true;
                    }
                }
            }
            
            // Need full redraw to show desktop behind closed window
            self.request_full_redraw();
            true
        } else {
            false
        }
    }

    /// Find window at position
    fn find_window_at(&self, x: i32, y: i32) -> Option<&Window> {
        // Search from top to bottom (last in list is on top)
        self.windows.iter().rev().find(|w| {
            if w.state == WindowState::Minimized {
                return false;
            }
            x >= w.x && x < w.x + w.width as i32 &&
            y >= w.y && y < w.y + w.height as i32
        })
    }

    /// Find window at position (mutable)
    fn find_window_at_mut(&mut self, x: i32, y: i32) -> Option<&mut Window> {
        self.windows.iter_mut().rev().find(|w| {
            if w.state == WindowState::Minimized {
                return false;
            }
            x >= w.x && x < w.x + w.width as i32 &&
            y >= w.y && y < w.y + w.height as i32
        })
    }

    /// Focus a window
    fn focus_window(&mut self, window_id: u32) {
        // Deactivate all windows
        for w in &mut self.windows {
            w.is_active = false;
        }
        
        // Activate target window and move to front
        if let Some(idx) = self.windows.iter().position(|w| w.id == window_id) {
            let mut window = self.windows.remove(idx);
            window.is_active = true;
            self.windows.push(window);
            self.active_window_id = Some(window_id);
            
            // Request redraw of window areas
            if let Some(w) = self.windows.iter().find(|w| w.id == window_id) {
                self.request_redraw(Rect::new(w.x, w.y, w.width, w.height));
            }
        }
    }

    /// Check if point is on window title bar
    fn is_on_title_bar(&self, window: &Window, x: i32, y: i32) -> bool {
        x >= window.x && x < window.x + window.width as i32 &&
        y >= window.y && y < window.y + TITLE_BAR_HEIGHT as i32
    }

    /// Check if point is on window resize handle
    fn is_on_resize_handle(&self, window: &Window, x: i32, y: i32) -> bool {
        if window.state == WindowState::Maximized {
            return false;
        }
        let handle_x = window.x + window.width as i32 - RESIZE_BORDER;
        let handle_y = window.y + window.height as i32 - RESIZE_BORDER;
        x >= handle_x && x < window.x + window.width as i32 &&
        y >= handle_y && y < window.y + window.height as i32
    }

    /// Check if point is on window border
    fn is_on_border(&self, window: &Window, x: i32, y: i32) -> bool {
        if window.state == WindowState::Maximized {
            return false;
        }
        let outer = Rect::new(window.x, window.y, window.width, window.height);
        let inner = Rect::new(
            window.x + RESIZE_BORDER,
            window.y + TITLE_BAR_HEIGHT as i32,
            window.width.saturating_sub(RESIZE_BORDER as u32 * 2),
            window.height.saturating_sub((TITLE_BAR_HEIGHT + RESIZE_BORDER as u32) as u32),
        );
        
        outer.contains(x, y) && !inner.contains(x, y)
    }

    /// Check if point is on close button
    fn is_on_close_button(&self, window: &Window, x: i32, y: i32) -> bool {
        let btn_x = window.x + 12;
        let btn_y = window.y + 16;
        let dist_sq = (x - btn_x) * (x - btn_x) + (y - btn_y) * (y - btn_y);
        dist_sq < 36 // Within 6px radius
    }

    /// Check if point is on minimize button
    fn is_on_minimize_button(&self, window: &Window, x: i32, y: i32) -> bool {
        let btn_x = window.x + 32;
        let btn_y = window.y + 16;
        let dist_sq = (x - btn_x) * (x - btn_x) + (y - btn_y) * (y - btn_y);
        dist_sq < 36
    }

    /// Check if point is on maximize button
    fn is_on_maximize_button(&self, window: &Window, x: i32, y: i32) -> bool {
        let btn_x = window.x + 52;
        let btn_y = window.y + 16;
        let dist_sq = (x - btn_x) * (x - btn_x) + (y - btn_y) * (y - btn_y);
        dist_sq < 36
    }

    /// Check if point is on URL bar (for browser windows)
    fn is_on_url_bar(&self, window: &Window, x: i32, y: i32) -> bool {
        if !window.is_browser {
            return false;
        }
        let url_x = window.x + 80;
        let url_y = window.y + 40;
        let url_w = window.width.saturating_sub(160);
        x >= url_x && x < url_x + url_w as i32 &&
        y >= url_y && y < url_y + 28
    }

    /// Check if point is on Go button
    fn is_on_go_button(&self, window: &Window, x: i32, y: i32) -> bool {
        if !window.is_browser {
            return false;
        }
        let btn_x = window.x + window.width as i32 - 75;
        let btn_y = window.y + 40;
        x >= btn_x && x < btn_x + 50 &&
        y >= btn_y && y < btn_y + 28
    }

    // === NEW: Window drag and resize handling ===

    /// Start dragging a window
    pub fn handle_window_drag_start(&mut self, x: i32, y: i32) -> bool {
        // Extract window data first to avoid borrow issues
        let window_data = self.find_window_at(x, y).map(|window| {
            let is_on_close = self.is_on_close_button(window, x, y);
            let is_on_minimize = self.is_on_minimize_button(window, x, y);
            let is_on_maximize = self.is_on_maximize_button(window, x, y);
            let is_on_resize = self.is_on_resize_handle(window, x, y);
            let is_on_title = self.is_on_title_bar(window, x, y);
            let is_on_url = self.is_on_url_bar(window, x, y);
            
            (window.id, window.x, window.y, window.width, window.height, 
             window.is_browser, is_on_close, is_on_minimize, is_on_maximize,
             is_on_resize, is_on_title, is_on_url)
        });
        
        if let Some((window_id, win_x, win_y, win_width, win_height, 
                     is_browser, is_on_close, is_on_minimize, is_on_maximize,
                     is_on_resize, is_on_title, is_on_url)) = window_data {
            
            // Check if clicking on controls
            if is_on_close || is_on_minimize || is_on_maximize {
                return false; // Let click handler deal with buttons
            }
            
            // Check if on resize handle
            if is_on_resize {
                self.is_resizing = true;
                self.resize_window_id = Some(window_id);
                self.resize_start_x = x;
                self.resize_start_y = y;
                self.resize_start_width = win_width;
                self.resize_start_height = win_height;
                self.focus_window(window_id);
                return true;
            }
            
            // Check if on title bar (for dragging)
            if is_on_title {
                self.is_dragging = true;
                self.drag_window_id = Some(window_id);
                self.drag_start_x = x;
                self.drag_start_y = y;
                self.drag_window_start_x = win_x;
                self.drag_window_start_y = win_y;
                self.focus_window(window_id);
                return true;
            }
            
            // Clicking inside window - just focus it
            self.focus_window(window_id);
            
            // Check URL bar focus for browser windows
            if is_browser && is_on_url {
                let window_pos: Option<(i32, i32, u32, u32)> = self.windows.iter_mut()
                    .find(|w| w.id == window_id)
                    .map(|w| {
                        w.url_input_focused = true;
                        (w.x, w.y, w.width, w.height)
                    });
                if let Some((x, y, width, _height)) = window_pos {
                    self.request_redraw(Rect::new(x, y + 40, width, 28));
                }
            } else if is_browser {
                // Clicked elsewhere - unfocus URL bar
                let window_pos: Option<(i32, i32, u32, u32)> = self.windows.iter_mut()
                    .find(|w| w.id == window_id)
                    .filter(|w| w.url_input_focused)
                    .map(|w| {
                        w.url_input_focused = false;
                        (w.x, w.y, w.width, w.height)
                    });
                if let Some((x, y, width, _height)) = window_pos {
                    self.request_redraw(Rect::new(x, y + 40, width, 28));
                }
            }
            
            return true;
        }
        false
    }

    /// Handle window dragging
    pub fn handle_window_drag(&mut self, x: i32, y: i32) {
        if self.is_dragging {
            if let Some(window_id) = self.drag_window_id {
                let dx = x - self.drag_start_x;
                let dy = y - self.drag_start_y;
                
                let new_x = self.drag_window_start_x + dx;
                let new_y = self.drag_window_start_y + dy;
                
                // Extract window data first
                let window_data: Option<(i32, i32, u32, u32)> = 
                    self.windows.iter().find(|w| w.id == window_id)
                        .map(|w| (w.x, w.y, w.width, w.height));
                
                if let Some((old_x, old_y, width, height)) = window_data {
                    // Mark old position as dirty
                    self.request_redraw(Rect::new(old_x, old_y, width, height));
                    
                    // Update position
                    if let Some(window) = self.windows.iter_mut().find(|w| w.id == window_id) {
                        window.x = new_x;
                        window.y = new_y;
                    }
                    
                    // Mark new position as dirty
                    self.request_redraw(Rect::new(new_x, new_y, width, height));
                }
            }
        } else if self.is_resizing {
            if let Some(window_id) = self.resize_window_id {
                let dx = x - self.resize_start_x;
                let dy = y - self.resize_start_y;
                
                let new_width = (self.resize_start_width as i32 + dx).max(300) as u32;
                let new_height = (self.resize_start_height as i32 + dy).max(200) as u32;
                
                // Extract window data first
                let window_data: Option<(i32, i32, u32, u32)> = 
                    self.windows.iter().find(|w| w.id == window_id)
                        .map(|w| (w.x, w.y, w.width, w.height));
                
                if let Some((win_x, win_y, old_width, old_height)) = window_data {
                    // Mark old position as dirty
                    self.request_redraw(Rect::new(win_x, win_y, old_width, old_height));
                    
                    // Update size
                    if let Some(window) = self.windows.iter_mut().find(|w| w.id == window_id) {
                        window.width = new_width;
                        window.height = new_height;
                    }
                    
                    // Mark new position as dirty
                    self.request_redraw(Rect::new(win_x, win_y, new_width, new_height));
                }
            }
        }
    }

    /// Stop dragging/resizing
    pub fn handle_window_drag_end(&mut self) {
        self.is_dragging = false;
        self.drag_window_id = None;
        self.is_resizing = false;
        self.resize_window_id = None;
    }

    /// Handle window resize (resize handle drag)
    pub fn handle_window_resize(&mut self, x: i32, y: i32) {
        // This is handled by handle_window_drag when is_resizing is true
        self.handle_window_drag(x, y);
    }

    /// Update mouse position
    pub fn update_mouse(&mut self, x: i32, y: i32) {
        self.old_mouse_x = self.mouse_x;
        self.old_mouse_y = self.mouse_y;
        // Trust that coordinates from mouse driver are already clamped
        self.mouse_x = x;
        self.mouse_y = y;
    }

    /// Get mouse position
    pub fn mouse_position(&self) -> (i32, i32) {
        (self.mouse_x, self.mouse_y)
    }

    /// Scan the /Desktop folder and create icons for files
    pub fn scan_desktop_folder(&mut self) {
        if self.desktop_folder_scanned {
            return;
        }

        println!("[desktop_ui] Scanning /Desktop folder for icons...");

        // Read the /Desktop directory from filesystem
        let entries = self.read_desktop_directory();

        match entries {
            Ok(files) => {
                println!("[desktop_ui] Found {} files in /Desktop", files.len());

                // Clear existing non-default icons
                self.desktop_icons.retain(|icon| {
                    matches!(icon.action, IconAction::OpenFolder(_) | IconAction::OpenFile(_))
                });

                // Add icons for each file/folder
                for (idx, entry) in files.iter().enumerate() {
                    let icon = self.create_desktop_icon_for_entry(entry, idx);
                    self.desktop_icons.push(icon);
                }

                self.desktop_folder_scanned = true;
                println!("[desktop_ui] Desktop folder scanned successfully");
                
                // Request redraw of desktop area
                self.request_redraw(Rect::new(0, self.menu_bar_height as i32, 1280, 800 - self.menu_bar_height - self.dock_height));
            }
            Err(e) => {
                println!("[desktop_ui] Could not scan /Desktop: {:?}", e);
            }
        }
    }

    /// Read the /Desktop directory from filesystem
    fn read_desktop_directory(&self) -> Result<Vec<DirEntryInfo>, ()> {
        // When VFS is available, this should call the filesystem
        // For now, return empty to use default icons
        // TODO: Integrate with fs::vfs::readdir("/Desktop")
        Err(())
    }

    /// Create a desktop icon for a directory entry
    fn create_desktop_icon_for_entry(&self, entry: &DirEntryInfo, index: usize) -> Icon {
        // Calculate position in a grid layout on the right side
        let col = (index as u32) / DESKTOP_ICONS_PER_COLUMN;
        let row = (index as u32) % DESKTOP_ICONS_PER_COLUMN;

        let x = DESKTOP_START_X + (col as i32 * (DESKTOP_ICON_WIDTH as i32 + DESKTOP_ICON_SPACING_X));
        let y = DESKTOP_START_Y + (row as i32 * (DESKTOP_ICON_HEIGHT as i32 + DESKTOP_ICON_SPACING_Y));

        let file_type = Self::get_file_type(&entry.name, entry.is_dir);
        let icon_char = Self::get_icon_char_for_type(file_type);
        let icon_path = Self::get_icon_path_for_type(file_type);

        let action = if entry.is_dir {
            IconAction::OpenFolder(format!("/Desktop/{}", entry.name))
        } else {
            IconAction::OpenFile(format!("/Desktop/{}", entry.name))
        };

        Icon {
            x,
            y,
            width: DESKTOP_ICON_WIDTH,
            height: DESKTOP_ICON_HEIGHT,
            label: entry.name.clone(),
            icon_char,
            icon_path,
            action,
            is_folder: entry.is_dir,
            file_type,
            is_selected: false,
        }
    }

    /// Determine file type from name and extension
    fn get_file_type(name: &str, is_dir: bool) -> FileType {
        if is_dir {
            return FileType::Folder;
        }

        let extension = name.split('.').last().map(|s| s.to_lowercase());

        match extension.as_deref() {
            Some("txt") | Some("md") | Some("doc") | Some("docx") => FileType::Text,
            Some("jpg") | Some("jpeg") | Some("png") | Some("gif") | Some("bmp") => FileType::Image,
            Some("mp3") | Some("wav") | Some("ogg") | Some("flac") => FileType::Audio,
            Some("mp4") | Some("avi") | Some("mkv") | Some("mov") => FileType::Video,
            Some("zip") | Some("rar") | Some("7z") | Some("gz") => FileType::Archive,
            Some("pdf") => FileType::Pdf,
            Some("rs") | Some("c") | Some("cpp") | Some("h") | Some("py") | 
            Some("js") | Some("html") | Some("css") | Some("java") | Some("go") => FileType::Code,
            Some("exe") | Some("bin") | Some("sh") => FileType::Executable,
            _ => FileType::Unknown,
        }
    }

    /// Get icon character for file type
    fn get_icon_char_for_type(file_type: FileType) -> char {
        match file_type {
            FileType::Folder => '📁',
            FileType::Text => '📄',
            FileType::Image => '🖼',
            FileType::Audio => '🎵',
            FileType::Video => '🎬',
            FileType::Archive => '📦',
            FileType::Pdf => '📕',
            FileType::Code => '💻',
            FileType::Executable => '⚙',
            FileType::Unknown => '📃',
        }
    }

    /// Get icon path for file type
    fn get_icon_path_for_type(file_type: FileType) -> Option<String> {
        match file_type {
            FileType::Folder => Some("system/icons/folder_icon_64.png".to_string()),
            FileType::Text => Some("system/icons/text_icon_64.png".to_string()),
            FileType::Image => Some("system/icons/image_icon_64.png".to_string()),
            FileType::Code => Some("system/icons/code_icon_64.png".to_string()),
            _ => Some("system/icons/file_icon_64.png".to_string()),
        }
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
                file_type: FileType::Executable,
                is_selected: false,
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
                file_type: FileType::Executable,
                is_selected: false,
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
                file_type: FileType::Executable,
                is_selected: false,
            },
        ];
    }

    fn setup_default_desktop_icons(&mut self) {
        // Desktop icons on the right side - these are the default icons
        // FAT32 files will be added to the left side
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
                file_type: FileType::Folder,
                is_selected: false,
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
                file_type: FileType::Folder,
                is_selected: false,
            },
        ];
    }

    /// Draw the entire desktop (full redraw)
    pub fn draw(&mut self, driver: &mut VesaDriver) {
        let info = driver.info();
        let screen_w = info.width;
        let screen_h = info.height;

        // Initialize back buffer if needed
        self.init_back_buffer(screen_w, screen_h);

        // Clear back buffer
        self.clear_back_buffer(palette::DESKTOP_BG);

        // Draw all components to back buffer
        self.draw_desktop_background_to_back_buffer(screen_w, screen_h);
        self.draw_menu_bar_to_back_buffer(screen_w);
        
        // Draw desktop icons (clone to avoid borrow issues)
        let icons_copy = self.desktop_icons.clone();
        for icon in &icons_copy {
            self.draw_desktop_icon_to_back_buffer(icon);
        }

        // Draw all windows (in order, so later windows are on top)
        let windows_copy = self.windows.clone();
        for window in &windows_copy {
            self.draw_window_to_back_buffer(window);
        }

        // Draw dock
        self.draw_dock_to_back_buffer(screen_w, screen_h);

        // Swap buffers to screen
        self.swap_buffers(driver);
        
        // Mark full redraw as done
        self.full_redraw_needed = false;
        self.dirty_regions.clear();
    }

    /// Draw only damaged regions (partial redraw)
    pub fn draw_partial(&mut self, driver: &mut VesaDriver) {
        if self.full_redraw_needed {
            self.draw(driver);
            return;
        }

        let info = driver.info();
        let screen_w = info.width;
        let screen_h = info.height;

        // Initialize back buffer if needed
        self.init_back_buffer(screen_w, screen_h);

        // For each dirty region, redraw the affected components
        let regions: Vec<Rect> = self.dirty_regions.clone();
        
        for region in &regions {
            // Redraw desktop background for this region
            self.draw_desktop_background_region_to_back_buffer(region, screen_w, screen_h);
            
            // Redraw desktop icons that intersect this region (clone to avoid borrow issues)
            let icons_copy = self.desktop_icons.clone();
            for icon in &icons_copy {
                let icon_rect = Rect::new(icon.x, icon.y, icon.width, icon.height);
                if icon_rect.intersects(region) {
                    self.draw_desktop_icon_to_back_buffer(icon);
                }
            }
            
            // Redraw windows that intersect this region
            let windows_copy = self.windows.clone();
            for window in &windows_copy {
                let window_rect = Rect::new(window.x, window.y, window.width, window.height);
                if window_rect.intersects(region) {
                    self.draw_window_to_back_buffer(window);
                }
            }
            
            // Redraw dock if it intersects this region
            let dock_width = (self.dock_icon_size + 16) * self.dock_icons.len() as u32 + 16;
            let dock_x = (screen_w - dock_width) / 2;
            let dock_y = screen_h - self.dock_height - 8;
            let dock_rect = Rect::new(dock_x as i32, dock_y as i32, dock_width, self.dock_height);
            if dock_rect.intersects(region) {
                self.draw_dock_to_back_buffer(screen_w, screen_h);
            }
            
            // Redraw menu bar if it intersects this region
            let menu_rect = Rect::new(0, 0, screen_w, self.menu_bar_height);
            if menu_rect.intersects(region) {
                self.draw_menu_bar_to_back_buffer(screen_w);
            }
        }

        // Swap only dirty regions to screen
        self.swap_buffers(driver);
        self.clear_dirty_regions();
    }

    /// Check if there are pending redraws
    pub fn has_pending_redraws(&self) -> bool {
        self.full_redraw_needed || !self.dirty_regions.is_empty()
    }

    /// Render a partial region directly (for external use)
    pub fn render_partial(&mut self, rect: Rect, driver: &mut VesaDriver) {
        self.request_redraw(rect);
        self.draw_partial(driver);
    }

    // === Back buffer drawing methods ===

    fn draw_desktop_background_to_back_buffer(&mut self, screen_w: u32, screen_h: u32) {
        // Desktop background is already cleared, just draw icons area if needed
        // This could be expanded to support wallpaper
    }

    fn draw_desktop_background_region_to_back_buffer(&mut self, region: &Rect, screen_w: u32, screen_h: u32) {
        let x0 = region.x.max(0) as u32;
        let y0 = region.y.max(0) as u32;
        let x1 = (region.x + region.width as i32).min(screen_w as i32).max(0) as u32;
        let y1 = (region.y + region.height as i32).min(screen_h as i32).max(0) as u32;

        for y in y0..y1 {
            for x in x0..x1 {
                self.set_back_buffer_pixel(x, y, palette::DESKTOP_BG);
            }
        }
    }

    fn draw_window_to_back_buffer(&mut self, window: &Window) {
        if window.state == WindowState::Minimized {
            return;
        }

        let x = window.x;
        let y = window.y;
        let w = window.width;
        let h = window.height;

        // Shadow
        self.fill_rect_back_buffer(x + 4, y + 4, w, h, 0x80000000);

        // Window background
        self.fill_rect_back_buffer(x, y, w, h, palette::WINDOW_BG);

        // Title bar
        let title_color = if window.is_active {
            palette::WINDOW_TITLE_ACTIVE
        } else {
            palette::WINDOW_TITLE_INACTIVE
        };
        self.fill_rect_back_buffer(x, y, w, TITLE_BAR_HEIGHT, title_color);

        // Window border
        self.draw_window_border_to_back_buffer(x, y, w, h, window.is_active);

        // Title text
        self.draw_text_to_back_buffer(&window.title, x + 40, y + 10, palette::TEXT_BLACK, 1);

        // Traffic light buttons
        self.draw_window_buttons_to_back_buffer(x, y);

        // Window content based on type
        if window.is_browser {
            self.draw_browser_content_to_back_buffer(window);
        }

        // Resize handle (if not maximized)
        if window.state != WindowState::Maximized {
            self.draw_resize_handle_to_back_buffer(x, y, w, h);
        }
    }

    fn draw_window_border_to_back_buffer(&mut self, x: i32, y: i32, w: u32, h: u32, is_active: bool) {
        let border_color = if is_active {
            palette::WINDOW_BORDER
        } else {
            0xFFAAAAAA
        };

        // Top
        self.draw_hline_to_back_buffer(x, y, w, border_color);
        // Bottom
        self.draw_hline_to_back_buffer(x, y + h as i32 - 1, w, border_color);
        // Left
        self.draw_vline_to_back_buffer(x, y, h, border_color);
        // Right
        self.draw_vline_to_back_buffer(x + w as i32 - 1, y, h, border_color);
    }

    fn draw_window_buttons_to_back_buffer(&mut self, x: i32, y: i32) {
        // Close (red)
        self.fill_circle_to_back_buffer(x + 12, y + 16, 6, palette::BUTTON_CLOSE);
        // Minimize (yellow)
        self.fill_circle_to_back_buffer(x + 32, y + 16, 6, palette::BUTTON_MINIMIZE);
        // Maximize (green)
        self.fill_circle_to_back_buffer(x + 52, y + 16, 6, palette::BUTTON_MAXIMIZE);
    }

    fn draw_resize_handle_to_back_buffer(&mut self, x: i32, y: i32, w: u32, h: u32) {
        let handle_x = x + w as i32 - RESIZE_BORDER;
        let handle_y = y + h as i32 - RESIZE_BORDER;
        
        // Draw small triangle pattern for resize handle
        for i in 0..RESIZE_BORDER {
            self.draw_hline_to_back_buffer(
                handle_x + i,
                handle_y + i,
                (RESIZE_BORDER - i) as u32,
                palette::RESIZE_HANDLE
            );
        }
    }

    fn draw_browser_content_to_back_buffer(&mut self, window: &Window) {
        let x = window.x;
        let y = window.y;
        let w = window.width;

        // URL bar background
        let url_x = x + 80;
        let url_y = y + 40;
        let url_w = w.saturating_sub(160);
        
        // URL bar with focus indicator
        let url_bg_color = if window.url_input_focused {
            0xFFF0F8FF // Light blue when focused
        } else {
            palette::URL_BAR_BG
        };
        
        self.fill_rect_back_buffer(url_x, url_y, url_w, 28, url_bg_color);
        self.draw_rect_to_back_buffer(url_x, url_y, url_w, 28, palette::URL_BAR_BORDER);
        
        // URL text
        self.draw_text_to_back_buffer(&window.url, url_x + 10, url_y + 8, palette::URL_BAR_TEXT, 1);
        
        // Draw cursor if URL bar is focused
        if window.url_input_focused {
            let cursor_x = url_x + 10 + (window.url_cursor_pos as i32 * 8);
            self.draw_vline_to_back_buffer(cursor_x, url_y + 4, 20, palette::INPUT_CURSOR);
        }

        // Go button
        let btn_x = x + w as i32 - 75;
        let btn_y = y + 40;
        self.fill_rect_back_buffer(btn_x, btn_y, 50, 28, 0xFF007AFF);
        self.draw_rect_to_back_buffer(btn_x, btn_y, 50, 28, 0xFF0055AA);
        self.draw_text_to_back_buffer("Go", btn_x + 18, btn_y + 8, palette::TEXT_WHITE, 1);

        // Navigation buttons
        self.fill_rect_back_buffer(x + 10, y + 40, 30, 28, 0xFFE0E0E0);
        self.draw_rect_to_back_buffer(x + 10, y + 40, 30, 28, palette::URL_BAR_BORDER);
        self.draw_text_to_back_buffer("<", x + 20, y + 48, palette::TEXT_BLACK, 1);

        self.fill_rect_back_buffer(x + 45, y + 40, 30, 28, 0xFFE0E0E0);
        self.draw_rect_to_back_buffer(x + 45, y + 40, 30, 28, palette::URL_BAR_BORDER);
        self.draw_text_to_back_buffer(">", x + 55, y + 48, palette::TEXT_BLACK, 1);

        // Content area with welcome message
        let content_y = y + 80;
        self.draw_text_to_back_buffer("Welcome to WebbOS Browser!", x + 40, content_y, palette::TEXT_BLACK, 2);
        self.draw_text_to_back_buffer("A minimal web browser built into the OS", x + 40, content_y + 40, 0xFF666666, 1);

        // Demo content
        self.draw_text_to_back_buffer("Features:", x + 40, content_y + 80, palette::TEXT_BLACK, 1);
        self.draw_text_to_back_buffer("- HTML5 parsing engine", x + 60, content_y + 100, 0xFF666666, 1);
        self.draw_text_to_back_buffer("- CSS3 styling support", x + 60, content_y + 120, 0xFF666666, 1);
        self.draw_text_to_back_buffer("- JavaScript interpreter", x + 60, content_y + 140, 0xFF666666, 1);
        self.draw_text_to_back_buffer("- WebAssembly runtime", x + 60, content_y + 160, 0xFF666666, 1);
    }

    // === Low-level back buffer drawing primitives ===

    fn draw_hline_to_back_buffer(&mut self, x: i32, y: i32, w: u32, color: u32) {
        if y < 0 || y >= self.back_buffer_height as i32 {
            return;
        }
        let x0 = x.max(0) as u32;
        let x1 = ((x as u32) + w).min(self.back_buffer_width);
        
        for px in x0..x1 {
            self.set_back_buffer_pixel(px, y as u32, color);
        }
    }

    fn draw_vline_to_back_buffer(&mut self, x: i32, y: i32, h: u32, color: u32) {
        if x < 0 || x >= self.back_buffer_width as i32 {
            return;
        }
        let y0 = y.max(0) as u32;
        let y1 = ((y as u32) + h).min(self.back_buffer_height);
        
        for py in y0..y1 {
            self.set_back_buffer_pixel(x as u32, py, color);
        }
    }

    fn draw_rect_to_back_buffer(&mut self, x: i32, y: i32, w: u32, h: u32, color: u32) {
        self.draw_hline_to_back_buffer(x, y, w, color);
        self.draw_hline_to_back_buffer(x, y + h as i32 - 1, w, color);
        self.draw_vline_to_back_buffer(x, y, h, color);
        self.draw_vline_to_back_buffer(x + w as i32 - 1, y, h, color);
    }

    fn fill_circle_to_back_buffer(&mut self, cx: i32, cy: i32, r: i32, color: u32) {
        for dy in -r..=r {
            // Integer-based square root approximation
            let r_sq = r * r;
            let dy_sq = dy * dy;
            let dx_sq = r_sq - dy_sq;
            let dx = integer_sqrt(dx_sq);
            self.draw_hline_to_back_buffer(cx - dx, cy + dy, (dx * 2 + 1) as u32, color);
        }
    }

    fn draw_char_to_back_buffer(&mut self, ch: char, x: i32, y: i32, color: u32, scale: u32) {
        let bitmap = get_char_bitmap(ch);
        for row in 0..8usize {
            for col in 0..8usize {
                if bitmap[row] & (1 << (7 - col)) != 0 {
                    for sy in 0..scale {
                        for sx in 0..scale {
                            let px = (x as u32) + (col as u32) * scale + sx;
                            let py = (y as u32) + (row as u32) * scale + sy;
                            if px < self.back_buffer_width && py < self.back_buffer_height {
                                self.set_back_buffer_pixel(px, py, color);
                            }
                        }
                    }
                }
            }
        }
    }

    fn draw_text_to_back_buffer(&mut self, text: &str, x: i32, y: i32, color: u32, scale: u32) {
        let mut cx = x;
        for ch in text.chars() {
            self.draw_char_to_back_buffer(ch, cx, y, color, scale);
            cx += (8 * scale) as i32;
        }
    }

    // === Mouse cursor methods ===

    fn draw_mouse_cursor_to_back_buffer(&mut self) {
        let cursor_data: &[(i32, i32)] = &[
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

        for &(ox, oy) in cursor_data {
            let px = self.mouse_x + ox;
            let py = self.mouse_y + oy;

            if px >= 0 && px < self.back_buffer_width as i32 && py >= 0 && py < self.back_buffer_height as i32 {
                self.set_back_buffer_pixel(px as u32, py as u32, 0xFF000000);
                if ox > 0 && oy > 0 && ox < 8 && oy < 9 {
                    self.set_back_buffer_pixel(px as u32, py as u32, 0xFFFFFFFF);
                }
            }
        }
    }

    /// Save the pixels under the cursor to the save buffer
    fn save_under_cursor(&mut self, driver: &VesaDriver) {
        let info = driver.info();
        let screen_w = info.width as i32;
        let screen_h = info.height as i32;
        
        if self.mouse_x < 0 || self.mouse_y < 0 || 
           self.mouse_x >= screen_w || self.mouse_y >= screen_h {
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
        let cursor_data: &[(i32, i32)] = &[
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

        let (screen_w, screen_h) = {
            let info = driver.info();
            (info.width, info.height)
        };

        for &(ox, oy) in cursor_data {
            let px = self.mouse_x + ox;
            let py = self.mouse_y + oy;

            if px >= 0 && px < screen_w as i32 && py >= 0 && py < screen_h as i32 {
                driver.set_pixel(px as u32, py as u32, 0xFF000000);
                if ox > 0 && oy > 0 && ox < 8 && oy < 9 {
                    driver.set_pixel(px as u32, py as u32, 0xFFFFFFFF);
                }
            }
        }
    }

    fn draw_menu_bar_to_back_buffer(&mut self, screen_w: u32) {
        // Menu bar background
        self.fill_rect_back_buffer(0, 0, screen_w, self.menu_bar_height, palette::MENU_BAR_BG);

        // Apple logo (W for Webb)
        self.draw_text_to_back_buffer("W", 10, 6, palette::MENU_BAR_TEXT, 1);

        // System info (right side)
        let time_str = "12:00";
        self.draw_text_to_back_buffer(time_str, (screen_w - 60) as i32, 6, palette::MENU_BAR_TEXT, 1);
    }

    fn draw_dock_to_back_buffer(&mut self, screen_w: u32, screen_h: u32) {
        let dock_width = (self.dock_icon_size + 16) * self.dock_icons.len() as u32 + 16;
        let dock_x = (screen_w - dock_width) / 2;
        let dock_y = screen_h - self.dock_height - 8;

        // Dock background
        self.fill_rect_back_buffer(
            dock_x as i32,
            dock_y as i32,
            dock_width,
            self.dock_height,
            palette::DOCK_BG
        );

        // Dock border
        self.draw_rect_to_back_buffer(
            dock_x as i32,
            dock_y as i32,
            dock_width,
            self.dock_height,
            palette::DOCK_BORDER
        );

        // Draw dock icons (clone to avoid borrow issues)
        let icons_copy = self.dock_icons.clone();
        for icon in &icons_copy {
            self.draw_dock_icon_to_back_buffer(icon);
        }
    }

    fn draw_dock_icon_to_back_buffer(&mut self, icon: &Icon) {
        // Icon background
        self.fill_rect_back_buffer(
            icon.x,
            icon.y,
            icon.width,
            icon.height,
            palette::ICON_BG
        );

        // Icon border
        self.draw_rect_to_back_buffer(
            icon.x,
            icon.y,
            icon.width,
            icon.height,
            palette::DOCK_BORDER
        );

        // Try to load and draw PNG icon from filesystem first
        let mut icon_drawn = false;
        
        if let Some(ref path) = icon.icon_path {
            if let Some(cached) = crate::desktop::icon_cache::get_icon(path) {
                self.draw_rgba_icon_to_back_buffer(icon.x, icon.y, &cached.rgba_data, cached.width, cached.height);
                icon_drawn = true;
            } else {
                icon_drawn = self.draw_embedded_icon_to_back_buffer(icon, path);
            }
        }

        // Fallback to character display if no icon was drawn
        if !icon_drawn {
            let char_x = icon.x + (icon.width as i32 / 2) - 8;
            let char_y = icon.y + (icon.height as i32 / 2) - 8;
            self.draw_char_to_back_buffer(icon.icon_char, char_x, char_y, palette::TEXT_BLACK, 2);
        }

        // Label (below icon)
        let label_x = icon.x + (icon.width as i32 / 2) - ((icon.label.len() as i32 * 4));
        self.draw_text_to_back_buffer(&icon.label, label_x, icon.y + icon.height as i32 + 4, palette::TEXT_WHITE, 1);
    }

    /// Draw embedded icon based on path
    fn draw_embedded_icon_to_back_buffer(&mut self, icon: &Icon, path: &str) -> bool {
        use crate::desktop::embedded_icons;
        
        if path.contains("globe") {
            self.draw_rgba_icon_to_back_buffer(icon.x, icon.y,
                embedded_icons::GLOBE_ICON_DATA,
                embedded_icons::GLOBE_ICON_WIDTH,
                embedded_icons::GLOBE_ICON_HEIGHT);
            true
        } else if path.contains("filemanager") {
            self.draw_rgba_icon_to_back_buffer(icon.x, icon.y,
                embedded_icons::FILEMANAGER_ICON_DATA,
                embedded_icons::FILEMANAGER_ICON_WIDTH,
                embedded_icons::FILEMANAGER_ICON_HEIGHT);
            true
        } else if path.contains("folder") {
            self.draw_rgba_icon_to_back_buffer(icon.x, icon.y,
                embedded_icons::FOLDER_ICON_DATA,
                embedded_icons::FOLDER_ICON_WIDTH,
                embedded_icons::FOLDER_ICON_HEIGHT);
            true
        } else {
            false
        }
    }

    /// Draw an RGBA icon from embedded data
    fn draw_rgba_icon_to_back_buffer(&mut self, x: i32, y: i32, data: &[u8], width: u32, height: u32) {
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
                    let color = ((r as u32) << 16) | ((g as u32) << 8) | (b as u32) | 0xFF000000;
                    let dst_x = x + px as i32;
                    let dst_y = y + py as i32;
                    if dst_x >= 0 && dst_x < self.back_buffer_width as i32 &&
                       dst_y >= 0 && dst_y < self.back_buffer_height as i32 {
                        self.set_back_buffer_pixel(dst_x as u32, dst_y as u32, color);
                    }
                }
            }
        }
    }

    fn draw_desktop_icon_to_back_buffer(&mut self, icon: &Icon) {
        // Draw selection highlight if selected
        if icon.is_selected {
            self.fill_rect_back_buffer(
                icon.x - 4,
                icon.y - 4,
                icon.width + 8,
                icon.height + 8,
                palette::ICON_SELECTED
            );
        }

        // Icon background
        self.fill_rect_back_buffer(
            icon.x,
            icon.y,
            icon.width,
            icon.height - 16,
            palette::ICON_BG
        );

        // Try to load and draw PNG icon
        let mut icon_drawn = false;
        
        if let Some(ref path) = icon.icon_path {
            if let Some(cached) = crate::desktop::icon_cache::get_icon(path) {
                self.draw_rgba_icon_to_back_buffer(icon.x, icon.y, &cached.rgba_data, cached.width, cached.height);
                icon_drawn = true;
            } else if path.contains("folder") {
                self.draw_rgba_icon_to_back_buffer(icon.x, icon.y,
                    crate::desktop::embedded_icons::FOLDER_ICON_DATA,
                    crate::desktop::embedded_icons::FOLDER_ICON_WIDTH,
                    crate::desktop::embedded_icons::FOLDER_ICON_HEIGHT);
                icon_drawn = true;
            }
        }

        // Fallback to character display
        if !icon_drawn {
            let char_x = icon.x + (icon.width as i32 / 2) - 16;
            let char_y = icon.y + 12;
            self.draw_char_to_back_buffer(icon.icon_char, char_x, char_y, palette::TEXT_BLACK, 4);
        }

        // Label
        let max_label_len = 10;
        let label = if icon.label.len() > max_label_len {
            format!("{}...", &icon.label[..max_label_len-3])
        } else {
            icon.label.clone()
        };
        
        let label_x = icon.x + (icon.width as i32 / 2) - ((label.len() as i32 * 4));
        self.draw_text_to_back_buffer(&label, label_x, icon.y + icon.height as i32 - 12, palette::TEXT_WHITE, 1);
    }

    /// Handle mouse click (single click selects, double click opens)
    pub fn handle_click(&mut self, x: i32, y: i32) -> bool {
        // First check if clicking on a window
        if let Some(window) = self.find_window_at(x, y) {
            let window_id = window.id;
            
            // Check close button
            if self.is_on_close_button(window, x, y) {
                println!("[desktop] Closing window {}", window_id);
                self.close_window(window_id);
                return true;
            }
            
            // Check minimize button
            if self.is_on_minimize_button(window, x, y) {
                println!("[desktop] Minimizing window {}", window_id);
                let window_pos: Option<(i32, i32, u32, u32)> = self.windows.iter_mut()
                    .find(|w| w.id == window_id)
                    .map(|w| {
                        w.state = match w.state {
                            WindowState::Minimized => WindowState::Normal,
                            _ => WindowState::Minimized,
                        };
                        (w.x, w.y, w.width, w.height)
                    });
                if let Some((x, y, width, height)) = window_pos {
                    self.request_redraw(Rect::new(x, y, width, height));
                }
                return true;
            }
            
            // Check maximize button
            if self.is_on_maximize_button(window, x, y) {
                println!("[desktop] Maximizing/restoring window {}", window_id);
                let window_pos: Option<(i32, i32, u32, u32)> = self.windows.iter_mut()
                    .find(|w| w.id == window_id)
                    .map(|w| {
                        w.state = match w.state {
                            WindowState::Maximized => WindowState::Normal,
                            _ => WindowState::Maximized,
                        };
                        (w.x, w.y, w.width, w.height)
                    });
                if let Some((x, y, width, height)) = window_pos {
                    self.request_redraw(Rect::new(x, y, width, height));
                }
                return true;
            }
            
            // Check Go button for browser
            if window.is_browser && self.is_on_go_button(window, x, y) {
                let url = window.url.clone();
                let window_id = window.id;
                println!("[desktop] Browser navigate to: {}", url);
                
                // Get window position for redraw
                let (win_x, win_y, win_width) = if let Some(w) = self.windows.iter().find(|w| w.id == window_id) {
                    (w.x, w.y, w.width)
                } else {
                    (0, 0, 0)
                };
                
                // Update window title to show loading
                if let Some(w) = self.windows.iter_mut().find(|w| w.id == window_id) {
                    w.title = String::from("Loading...");
                }
                self.request_redraw(Rect::new(win_x, win_y, win_width, TITLE_BAR_HEIGHT as u32));
                
                // Trigger browser navigation
                match crate::browser::navigate(&url) {
                    Ok(_) => {
                        println!("[desktop] Navigation successful");
                        // Update window title after successful navigation
                        let new_title = crate::browser::get_title();
                        if let Some(w) = self.windows.iter_mut().find(|w| w.id == window_id) {
                            w.title = if new_title.is_empty() {
                                String::from("WebbOS Browser")
                            } else {
                                new_title
                            };
                        }
                        self.request_redraw(Rect::new(win_x, win_y, win_width, TITLE_BAR_HEIGHT as u32));
                    }
                    Err(e) => {
                        println!("[desktop] Navigation failed: {:?}", e);
                        // Show error in title
                        if let Some(w) = self.windows.iter_mut().find(|w| w.id == window_id) {
                            w.title = String::from("Error loading page");
                        }
                        self.request_redraw(Rect::new(win_x, win_y, win_width, TITLE_BAR_HEIGHT as u32));
                    }
                }
                return true;
            }
        }
        
        // Check dock icons
        for icon in &self.dock_icons {
            if x >= icon.x && x < icon.x + icon.width as i32 &&
               y >= icon.y && y < icon.y + icon.height as i32 {
                println!("[desktop] Clicked dock icon: {}", icon.label);
                match &icon.action {
                    IconAction::LaunchApp(app_name) => {
                        if app_name == "browser" {
                            println!("[desktop] Opening browser window");
                            self.create_browser_window();
                            return true;
                        } else if app_name == "appstore" {
                            println!("[desktop] App Store coming soon!");
                        } else if app_name == "filemanager" {
                            println!("[desktop] Opening file manager");
                        }
                    }
                    _ => {}
                }
                return true;
            }
        }
        
        // Check desktop icons
        let clicked_idx = self.desktop_icons.iter().position(|icon| {
            x >= icon.x && x < icon.x + icon.width as i32 &&
            y >= icon.y && y < icon.y + icon.height as i32
        });
        
        if let Some(idx) = clicked_idx {
            println!("[desktop] Selected icon: {}", self.desktop_icons[idx].label);
            
            if let Some(prev_idx) = self.selected_icon {
                if prev_idx != idx && prev_idx < self.desktop_icons.len() {
                    let prev_icon = &self.desktop_icons[prev_idx];
                    self.request_redraw(Rect::new(
                        prev_icon.x - 4, prev_icon.y - 4,
                        prev_icon.width + 8, prev_icon.height + 8
                    ));
                    self.desktop_icons[prev_idx].is_selected = false;
                }
            }
            
            self.desktop_icons[idx].is_selected = true;
            let icon = &self.desktop_icons[idx];
            self.request_redraw(Rect::new(
                icon.x - 4, icon.y - 4,
                icon.width + 8, icon.height + 8
            ));
            self.selected_icon = Some(idx);
            
            return true;
        }
        
        // Clicked elsewhere - clear selection
        if let Some(prev_idx) = self.selected_icon {
            if prev_idx < self.desktop_icons.len() {
                let prev_icon = &self.desktop_icons[prev_idx];
                self.request_redraw(Rect::new(
                    prev_icon.x - 4, prev_icon.y - 4,
                    prev_icon.width + 8, prev_icon.height + 8
                ));
                self.desktop_icons[prev_idx].is_selected = false;
            }
            self.selected_icon = None;
            return true;
        }
        
        false
    }
    
    /// Handle double-click
    pub fn handle_double_click(&mut self, x: i32, y: i32) -> bool {
        if let Some(idx) = self.selected_icon {
            if idx < self.desktop_icons.len() {
                let icon = &self.desktop_icons[idx];
                
                if x >= icon.x && x < icon.x + icon.width as i32 &&
                   y >= icon.y && y < icon.y + icon.height as i32 {
                    
                    match &icon.action {
                        IconAction::OpenFolder(path) => {
                            println!("[desktop] Opening folder: {}", path);
                            return true;
                        }
                        IconAction::OpenFile(path) => {
                            println!("[desktop] Opening file: {}", path);
                            return true;
                        }
                        IconAction::LaunchApp(app_name) => {
                            println!("[desktop] Launching app: {}", app_name);
                            if app_name == "browser" {
                                self.create_browser_window();
                            }
                            return true;
                        }
                        _ => {}
                    }
                }
            }
        }
        
        self.handle_click(x, y)
    }

    /// Get the currently selected icon index
    pub fn selected_icon(&self) -> Option<usize> {
        self.selected_icon
    }

    /// Get mutable reference to desktop icons
    pub fn desktop_icons_mut(&mut self) -> &mut Vec<Icon> {
        &mut self.desktop_icons
    }

    /// Get reference to desktop icons
    pub fn desktop_icons(&self) -> &[Icon] {
        &self.desktop_icons
    }

    /// Check if browser is open (for backwards compatibility)
    pub fn browser_open(&self) -> bool {
        self.windows.iter().any(|w| w.is_browser && w.state != WindowState::Minimized)
    }

    /// Set URL for active browser window
    pub fn set_browser_url(&mut self, url: &str) {
        if let Some(window_id) = self.active_window_id {
            let window_pos: Option<(i32, i32, u32, u32)> = self.windows.iter_mut()
                .find(|w| w.id == window_id && w.is_browser)
                .map(|w| {
                    w.url = url.to_string();
                    w.url_cursor_pos = url.len();
                    (w.x, w.y, w.width, w.height)
                });
            // Request redraw of URL bar area
            if let Some((x, y, width, _height)) = window_pos {
                self.request_redraw(Rect::new(x, y + 40, width, 28));
            }
        }
    }

    /// Handle keyboard input for URL bar
    pub fn handle_url_input(&mut self, ch: char) {
        if let Some(window_id) = self.active_window_id {
            let mut should_navigate = false;
            let mut url_to_navigate = String::new();
            
            // First, handle character input
            let needs_redraw: Option<(i32, i32, u32, u32)> = self.windows.iter_mut()
                .find(|w| w.id == window_id && w.is_browser && w.url_input_focused)
                .and_then(|w| {
                    if ch == '\n' || ch == '\r' {
                        // Enter pressed - trigger navigation
                        should_navigate = true;
                        url_to_navigate = w.url.clone();
                        println!("[desktop] Navigate to: {}", w.url);
                        Some((w.x, w.y, w.width, w.height))
                    } else if ch == '\x08' { // Backspace
                        if w.url_cursor_pos > 0 {
                            w.url_cursor_pos -= 1;
                            w.url.remove(w.url_cursor_pos);
                            Some((w.x, w.y, w.width, w.height))
                        } else {
                            None
                        }
                    } else if ch.is_ascii_graphic() || ch == ' ' || ch == '.' || ch == '/' || ch == ':' {
                        w.url.insert(w.url_cursor_pos, ch);
                        w.url_cursor_pos += 1;
                        Some((w.x, w.y, w.width, w.height))
                    } else {
                        None
                    }
                });
            
            if let Some((x, y, width, _height)) = needs_redraw {
                self.request_redraw(Rect::new(x, y + 40, width, 28));
            }
            
            // Perform navigation after releasing the borrow
            if should_navigate && !url_to_navigate.is_empty() {
                // Get window position first
                let (win_x, win_y, win_width) = if let Some(w) = self.windows.iter().find(|w| w.id == window_id) {
                    (w.x, w.y, w.width)
                } else {
                    (0, 0, 0)
                };
                
                // Update window title to show loading
                if let Some(w) = self.windows.iter_mut().find(|w| w.id == window_id) {
                    w.title = String::from("Loading...");
                }
                self.request_redraw(Rect::new(win_x, win_y, win_width, TITLE_BAR_HEIGHT as u32));
                
                // Navigate using browser module
                match crate::browser::navigate(&url_to_navigate) {
                    Ok(_) => {
                        println!("[desktop] Navigation successful");
                        // Get new title
                        let new_title = crate::browser::get_title();
                        // Update window title after successful navigation
                        if let Some(w) = self.windows.iter_mut().find(|w| w.id == window_id) {
                            w.title = if new_title.is_empty() {
                                String::from("WebbOS Browser")
                            } else {
                                new_title
                            };
                        }
                        self.request_redraw(Rect::new(win_x, win_y, win_width, TITLE_BAR_HEIGHT as u32));
                    }
                    Err(e) => {
                        println!("[desktop] Navigation failed: {:?}", e);
                        // Show error in title
                        if let Some(w) = self.windows.iter_mut().find(|w| w.id == window_id) {
                            w.title = String::from("Error loading page");
                        }
                        self.request_redraw(Rect::new(win_x, win_y, win_width, TITLE_BAR_HEIGHT as u32));
                    }
                }
            }
        }
    }
}

/// Directory entry info for FAT32 integration
#[derive(Debug, Clone)]
pub struct DirEntryInfo {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
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
        
        // Scan desktop folder on first show
        desktop.scan_desktop_folder();
        
        // Initial full draw
        desktop.draw(&mut driver);
        
        // Initial save of area under cursor
        desktop.save_under_cursor(&mut driver);
        desktop.draw_mouse_cursor(&mut driver);
        
        println!("[desktop_ui] Desktop drawn, ready for interaction");
    }
}

/// Scan the /Desktop folder from FAT32 filesystem
pub fn scan_desktop_folder() {
    DESKTOP_UI.lock().scan_desktop_folder();
}

/// Update mouse position and redraw only the cursor
pub fn update_mouse(x: i32, y: i32) {
    static UPDATE_COUNT: AtomicU32 = AtomicU32::new(0);
    static LAST_PRINT: AtomicU64 = AtomicU64::new(0);
    static TRACE_COUNT: AtomicU32 = AtomicU32::new(0);
    
    let trace = TRACE_COUNT.fetch_add(1, Ordering::Relaxed);
    let should_trace = trace < 20;
    
    if should_trace {
        crate::println!("[mouse-trace] update_mouse({},{}) start", x, y);
    }
    
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
    
    let dx = (x - desktop.mouse_x).abs();
    let dy = (y - desktop.mouse_y).abs();
    if dx < 1 && dy < 1 {
        if should_trace {
            crate::println!("[mouse-trace] movement too small, returning");
        }
        return;
    }

    let info = driver.info();
    if x < 0 || y < 0 || x >= info.width as i32 || y >= info.height as i32 {
        if should_trace {
            crate::println!("[mouse-trace] out of bounds, returning");
        }
        return;
    }

    let count = UPDATE_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    let current = crate::arch::interrupts::get_timer_ticks();
    let last = LAST_PRINT.load(Ordering::Relaxed);
    
    #[cfg(target_arch = "aarch64")]
    let _ = (count, current, last);
    
    if current > last + 500 {
        if count > 0 {
            crate::println!("[desktop] Mouse updates: {}, pos: ({},{})", 
                count, x, y);
        }
        UPDATE_COUNT.store(0, Ordering::Relaxed);
        LAST_PRINT.store(current, Ordering::Relaxed);
    }

    // Handle window dragging if active
    if desktop.is_dragging || desktop.is_resizing {
        desktop.handle_window_drag(x, y);
    }

    if should_trace {
        crate::println!("[mouse-trace] restoring under cursor...");
    }
    desktop.restore_under_cursor(&mut driver);
    
    if should_trace {
        crate::println!("[mouse-trace] updating position...");
    }
    desktop.update_mouse(x, y);
    
    if should_trace {
        crate::println!("[mouse-trace] saving under cursor...");
    }
    desktop.save_under_cursor(&mut driver);
    
    if should_trace {
        crate::println!("[mouse-trace] drawing cursor...");
    }
    desktop.draw_mouse_cursor(&mut driver);
    
    if should_trace {
        crate::println!("[mouse-trace] update_mouse done");
    }
}

/// Handle mouse button down (for drag start)
pub fn handle_mouse_down(x: i32, y: i32) {
    let mut desktop = DESKTOP_UI.lock();
    desktop.handle_window_drag_start(x, y);
}

/// Handle mouse button up (for drag end)
pub fn handle_mouse_up(_x: i32, _y: i32) {
    let mut desktop = DESKTOP_UI.lock();
    desktop.handle_window_drag_end();
}

/// Handle mouse click (single)
pub fn handle_click(x: i32, y: i32) {
    let mut driver = vesa::driver().lock();
    if !driver.is_initialized() {
        return;
    }
    
    let mut desktop = DESKTOP_UI.lock();
    let needs_redraw = desktop.handle_click(x, y);

    if needs_redraw {
        if desktop.has_pending_redraws() {
            desktop.draw_partial(&mut driver);
        } else {
            desktop.draw(&mut driver);
        }
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
        if desktop.has_pending_redraws() {
            desktop.draw_partial(&mut driver);
        } else {
            desktop.draw(&mut driver);
        }
    }
}

/// Get the currently selected icon index
pub fn selected_icon() -> Option<usize> {
    DESKTOP_UI.lock().selected_icon()
}

/// Check if desktop is active
pub fn is_active() -> bool {
    true
}

/// Create a browser window
pub fn create_browser_window() -> u32 {
    DESKTOP_UI.lock().create_browser_window()
}

/// Close a window
pub fn close_window(window_id: u32) -> bool {
    DESKTOP_UI.lock().close_window(window_id)
}

/// Set browser URL
pub fn set_browser_url(url: &str) {
    DESKTOP_UI.lock().set_browser_url(url);
}

/// Handle URL input
pub fn handle_url_input(ch: char) {
    DESKTOP_UI.lock().handle_url_input(ch);
}

/// Request redraw of a region
pub fn request_redraw(x: i32, y: i32, width: u32, height: u32) {
    DESKTOP_UI.lock().request_redraw(Rect::new(x, y, width, height));
}

/// Request full redraw
pub fn request_full_redraw() {
    DESKTOP_UI.lock().request_full_redraw();
}

/// Present frame (swap buffers)
pub fn present() {
    let mut driver = vesa::driver().lock();
    if !driver.is_initialized() {
        return;
    }
    
    let mut desktop = DESKTOP_UI.lock();
    desktop.present(&mut driver);
}

/// Check if browser is open (backwards compatibility)
pub fn browser_open() -> bool {
    DESKTOP_UI.lock().browser_open()
}

/// Get character bitmap for font rendering
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
        'w' => [0x00, 0x00, 0x63, 0x6b, 0x6b, 0x7f, 0x36, 0x00],
        'x' => [0x00, 0x00, 0x66, 0x3c, 0x18, 0x3c, 0x66, 0x00],
        'y' => [0x00, 0x00, 0x66, 0x66, 0x66, 0x3e, 0x0c, 0x78],
        'z' => [0x00, 0x00, 0x7e, 0x0c, 0x18, 0x30, 0x7e, 0x00],
        '*' => [0x00, 0x66, 0x3c, 0xff, 0x3c, 0x66, 0x00, 0x00],
        '•' => [0x00, 0x00, 0x00, 0x3c, 0x3c, 0x00, 0x00, 0x00],
        '.' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x00],
        ',' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x30],
        ':' => [0x00, 0x18, 0x18, 0x00, 0x18, 0x18, 0x00, 0x00],
        '/' => [0x00, 0x02, 0x06, 0x0c, 0x18, 0x30, 0x60, 0x00],
        '-' => [0x00, 0x00, 0x00, 0x7e, 0x00, 0x00, 0x00, 0x00],
        '_' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x7e, 0x00],
        '@' => [0x3c, 0x66, 0x6e, 0x6e, 0x60, 0x66, 0x3c, 0x00],
        ':' => [0x00, 0x00, 0x00, 0x18, 0x18, 0x00, 0x00, 0x00],
        '/' => [0x00, 0x02, 0x06, 0x0c, 0x18, 0x30, 0x60, 0x00],
        _ => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    }
}
