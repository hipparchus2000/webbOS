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
use spin::Mutex;
use lazy_static::lazy_static;
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
}

#[derive(Debug, Clone)]
pub enum IconAction {
    LaunchApp(String),      // Launch application by name
    OpenFolder(String),     // Open folder in file manager
    None,
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
        };

        // Create dock icons (centered at bottom)
        ui.setup_dock_icons();
        ui.setup_desktop_icons();

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

    /// Handle mouse click
    pub fn handle_click(&mut self, x: i32, y: i32) -> bool {
        // Check if clicking close button on browser
        if self.browser_open {
            let close_x = BROWSER_X + 12;
            let close_y = BROWSER_Y + 16;
            let dist = ((x - close_x) * (x - close_x) + (y - close_y) * (y - close_y)) as f32;
            if dist < 36.0 { // Within 6px radius
                println!("[desktop] Closing browser window");
                self.browser_open = false;
                return true; // Redraw needed
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
                            self.browser_open = true;
                            return true; // Redraw needed
                        } else if app_name == "appstore" {
                            println!("[desktop] App Store coming soon!");
                        } else if app_name == "filemanager" {
                            println!("[desktop] File Manager coming soon!");
                        }
                    }
                    _ => {}
                }
                return true; // Redraw needed
            }
        }

        // Check desktop icons
        for icon in &self.desktop_icons {
            if x >= icon.x && x < icon.x + icon.width as i32 &&
               y >= icon.y && y < icon.y + icon.height as i32 {
                println!("[desktop] Clicked desktop icon: {}", icon.label);
                return true; // Redraw needed
            }
        }

        false // No redraw needed
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

    let desktop = DESKTOP_UI.lock();
    desktop.draw(&mut driver);

    println!("[desktop_ui] Desktop drawn, ready for interaction");
}

/// Update mouse position and redraw only the cursor
pub fn update_mouse(x: i32, y: i32) {
    // Use simpler approach: redraw small area around old cursor, then draw new cursor
    let mut desktop = DESKTOP_UI.lock();
    let old_x = desktop.old_mouse_x;
    let old_y = desktop.old_mouse_y;

    let mut driver = vesa::driver().lock();
    if !driver.is_initialized() {
        return;
    }

    // Only update if mouse actually moved significantly (reduce updates)
    let dx = (x - old_x).abs();
    let dy = (y - old_y).abs();
    if dx < 2 && dy < 2 {
        return; // Ignore tiny movements
    }

    // Bounds check to prevent drawing outside screen
    let info = driver.info();
    if x < 0 || y < 0 || x >= info.width as i32 || y >= info.height as i32 {
        return; // Mouse coordinates out of bounds, skip update
    }

    // Update mouse position first
    desktop.update_mouse(x, y);

    // Simple approach: just redraw small areas
    // This is less efficient but more reliable than save/restore
    desktop.redraw_cursor_area(&mut driver, old_x, old_y);
    desktop.draw_mouse_cursor(&mut driver);
}

/// Handle mouse click
pub fn handle_click(x: i32, y: i32) {
    let mut desktop = DESKTOP_UI.lock();
    let needs_redraw = desktop.handle_click(x, y);

    if needs_redraw {
        let mut driver = vesa::driver().lock();
        if driver.is_initialized() {
            // Full redraw needed (window opened/closed)
            desktop.draw(&mut driver);
        }
    }
}

/// Check if desktop is active
pub fn is_active() -> bool {
    true // Desktop is always active after login
}
