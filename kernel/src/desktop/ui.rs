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
    browser_open: bool,
    desktop_folder_scanned: bool,
    // Save-under buffer for mouse cursor (stores pixels under cursor)
    save_buffer: [u32; SAVE_BUFFER_SIZE],
    save_buffer_valid: bool,
    save_buffer_x: i32,
    save_buffer_y: i32,
}

/// Browser window dimensions
const BROWSER_WIDTH: u32 = 1000;
const BROWSER_HEIGHT: u32 = 700;
const BROWSER_X: i32 = 140;
const BROWSER_Y: i32 = 50;

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
            browser_open: false,
            desktop_folder_scanned: false,
            save_buffer: [0; SAVE_BUFFER_SIZE],
            save_buffer_valid: false,
            save_buffer_x: 0,
            save_buffer_y: 0,
        };

        // Create dock icons (centered at bottom)
        ui.setup_dock_icons();
        ui.setup_default_desktop_icons();

        ui
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

    /// Draw the entire desktop
    pub fn draw(&self, driver: &mut VesaDriver) {
        let info = driver.info();
        let screen_w = info.width;
        let screen_h = info.height;

        // Draw desktop background
        driver.clear(palette::DESKTOP_BG);

        // Draw menu bar
        self.draw_menu_bar(driver, screen_w);

        // Draw desktop icons
        for icon in &self.desktop_icons {
            self.draw_desktop_icon(driver, icon);
        }

        // Draw browser window if open
        if self.browser_open {
            self.draw_browser_window(driver);
        }

        // Draw dock
        self.draw_dock(driver, screen_w, screen_h);

        // Draw mouse cursor (always on top)
        self.draw_mouse_cursor(driver);
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
        use crate::desktop::{embedded_icons, icon_cache};

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

        // Try to load and draw PNG icon from filesystem first
        let mut icon_drawn = false;
        
        if let Some(ref path) = icon.icon_path {
            // Try to load from icon cache
            if let Some(cached) = icon_cache::get_icon(path) {
                self.draw_rgba_icon(driver, icon.x, icon.y, &cached.rgba_data, cached.width, cached.height);
                icon_drawn = true;
            } else {
                // Fall back to embedded icons
                icon_drawn = self.draw_embedded_icon(driver, icon, path);
            }
        }

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

    /// Draw embedded icon based on path
    fn draw_embedded_icon(&self, driver: &mut VesaDriver, icon: &Icon, path: &str) -> bool {
        use crate::desktop::embedded_icons;
        
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
        } else if path.contains("folder") {
            self.draw_rgba_icon(driver, icon.x, icon.y,
                embedded_icons::FOLDER_ICON_DATA,
                embedded_icons::FOLDER_ICON_WIDTH,
                embedded_icons::FOLDER_ICON_HEIGHT);
            true
        } else {
            false
        }
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
        use crate::desktop::{embedded_icons, icon_cache};

        // Draw selection highlight if selected
        if icon.is_selected {
            driver.fill_rect(
                icon.x - 4,
                icon.y - 4,
                icon.width + 8,
                icon.height + 8,
                palette::ICON_SELECTED
            );
        }

        // Icon background (slightly rounded)
        driver.fill_rect(
            icon.x,
            icon.y,
            icon.width,
            icon.height - 16, // Space for label
            palette::ICON_BG
        );

        // Try to load and draw PNG icon from filesystem first
        let mut icon_drawn = false;
        
        if let Some(ref path) = icon.icon_path {
            // Try to load from icon cache
            if let Some(cached) = icon_cache::get_icon(path) {
                self.draw_rgba_icon(driver, icon.x, icon.y, &cached.rgba_data, cached.width, cached.height);
                icon_drawn = true;
            } else if path.contains("folder") {
                // Fall back to embedded folder icon
                self.draw_rgba_icon(driver, icon.x, icon.y,
                    embedded_icons::FOLDER_ICON_DATA,
                    embedded_icons::FOLDER_ICON_WIDTH,
                    embedded_icons::FOLDER_ICON_HEIGHT);
                icon_drawn = true;
            }
        }

        // Fallback to character display if no icon was drawn
        if !icon_drawn {
            let char_x = icon.x + (icon.width as i32 / 2) - 16;
            let char_y = icon.y + 12;
            driver.draw_char(icon.icon_char, char_x, char_y, palette::TEXT_BLACK, 4);
        }

        // Label (below icon) - truncate if too long
        let max_label_len = 10;
        let label = if icon.label.len() > max_label_len {
            format!("{}...", &icon.label[..max_label_len-3])
        } else {
            icon.label.clone()
        };
        
        let label_x = icon.x + (icon.width as i32 / 2) - ((label.len() as i32 * 4));
        driver.draw_text(&label, label_x, icon.y + icon.height as i32 - 12, palette::TEXT_WHITE, 1);
    }

    /// Handle mouse click (single click selects, double click opens)
    pub fn handle_click(&mut self, x: i32, y: i32) -> bool {
        // Check if clicking browser close button (when browser is open)
        if self.browser_open {
            let close_x = BROWSER_X + 12;
            let close_y = BROWSER_Y + 16;
            let dist_sq = (x - close_x) * (x - close_x) + (y - close_y) * (y - close_y);
            if dist_sq < 36 { // Within 6px radius
                println!("[desktop] Closing browser window");
                self.browser_open = false;
                return true; // Redraw needed
            }
        }
        
        // Check dock icons first (launch on single click)
        for icon in &self.dock_icons {
            if x >= icon.x && x < icon.x + icon.width as i32 &&
               y >= icon.y && y < icon.y + icon.height as i32 {
                println!("[desktop] Clicked dock icon: {}", icon.label);
                match &icon.action {
                    IconAction::LaunchApp(app_name) => {
                        if app_name == "browser" {
                            println!("[desktop] Opening browser window");
                            self.browser_open = true;  // Use UI's own browser flag
                            return true; // Redraw needed
                        } else if app_name == "appstore" {
                            println!("[desktop] App Store coming soon!");
                        } else if app_name == "filemanager" {
                            println!("[desktop] Opening file manager");
                            // TODO: Launch file manager app
                        }
                    }
                    _ => {}
                }
                return true; // Redraw needed
            }
        }
        
        // Check desktop icons (select on single click)
        // First, find which icon was clicked (if any)
        let clicked_idx = self.desktop_icons.iter().position(|icon| {
            x >= icon.x && x < icon.x + icon.width as i32 &&
            y >= icon.y && y < icon.y + icon.height as i32
        });
        
        if let Some(idx) = clicked_idx {
            println!("[desktop] Selected icon: {}", self.desktop_icons[idx].label);
            
            // Deselect previous
            if let Some(prev_idx) = self.selected_icon {
                if prev_idx != idx && prev_idx < self.desktop_icons.len() {
                    self.desktop_icons[prev_idx].is_selected = false;
                }
            }
            
            // Select this icon
            self.desktop_icons[idx].is_selected = true;
            self.selected_icon = Some(idx);
            
            return true; // Redraw needed
        }
        
        // Clicked elsewhere - clear selection
        if let Some(prev_idx) = self.selected_icon {
            if prev_idx < self.desktop_icons.len() {
                self.desktop_icons[prev_idx].is_selected = false;
            }
            self.selected_icon = None;
            return true;
        }
        false // No redraw needed
    }
    
    /// Handle double-click (opens the selected icon)
    pub fn handle_double_click(&mut self, x: i32, y: i32) -> bool {
        // Find icon at position
        if let Some(idx) = self.selected_icon {
            if idx < self.desktop_icons.len() {
                let icon = &self.desktop_icons[idx];
                
                // Check if still over the same icon
                if x >= icon.x && x < icon.x + icon.width as i32 &&
                   y >= icon.y && y < icon.y + icon.height as i32 {
                    
                    match &icon.action {
                        IconAction::OpenFolder(path) => {
                            println!("[desktop] Opening folder: {}", path);
                            // TODO: Launch file manager with this folder
                            return true;
                        }
                        IconAction::OpenFile(path) => {
                            println!("[desktop] Opening file: {}", path);
                            // TODO: Open file with appropriate application
                            return true;
                        }
                        IconAction::LaunchApp(app_name) => {
                            println!("[desktop] Launching app: {}", app_name);
                            if app_name == "browser" {
                                self.browser_open = true;
                            }
                            return true;
                        }
                        _ => {}
                    }
                }
            }
        }
        
        // If no icon handled it, try single click handler
        self.handle_click(x, y)
    }

    /// Get the currently selected icon index
    pub fn selected_icon(&self) -> Option<usize> {
        self.selected_icon
    }

    /// Get mutable reference to desktop icons (for external updates)
    pub fn desktop_icons_mut(&mut self) -> &mut Vec<Icon> {
        &mut self.desktop_icons
    }

    /// Get reference to desktop icons
    pub fn desktop_icons(&self) -> &[Icon] {
        &self.desktop_icons
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
    
    // Suppress unused warning on aarch64 where timer might not be fully implemented
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

/// Get the currently selected icon index
pub fn selected_icon() -> Option<usize> {
    DESKTOP_UI.lock().selected_icon()
}

/// Check if desktop is active
pub fn is_active() -> bool {
    true // Desktop is always active after login
}
