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
}

impl DesktopUI {
    pub fn new() -> Self {
        let mut ui = Self {
            menu_bar_height: 24,
            dock_height: 64,
            dock_icon_size: 48,
            dock_icons: Vec::new(),
            desktop_icons: Vec::new(),
            selected_icon: None,
        };

        // Create dock icons (centered at bottom)
        ui.setup_dock_icons();
        ui.setup_desktop_icons();

        ui
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
                action: IconAction::LaunchApp("browser".to_string()),
            },
            Icon {
                x: dock_x as i32 + 8 + 64,
                y: dock_y as i32 + 8,
                width: self.dock_icon_size,
                height: self.dock_icon_size,
                label: "App Store".to_string(),
                icon_char: 'A',
                action: IconAction::LaunchApp("appstore".to_string()),
            },
            Icon {
                x: dock_x as i32 + 8 + 128,
                y: dock_y as i32 + 8,
                width: self.dock_icon_size,
                height: self.dock_icon_size,
                label: "Files".to_string(),
                icon_char: 'F',
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
                action: IconAction::OpenFolder("/home/user/documents".to_string()),
            },
            Icon {
                x: 1120,
                y: 140,
                width: 64,
                height: 80,
                label: "Downloads".to_string(),
                icon_char: 'L',
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

        // Draw dock
        self.draw_dock(driver, screen_w, screen_h);
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

        // Icon character (centered)
        let char_x = icon.x + (icon.width as i32 / 2) - 8;
        let char_y = icon.y + (icon.height as i32 / 2) - 8;
        driver.draw_char(icon.icon_char, char_x, char_y, palette::TEXT_BLACK, 2);

        // Label (below icon)
        let label_x = icon.x + (icon.width as i32 / 2) - ((icon.label.len() as i32 * 4));
        driver.draw_text(&icon.label, label_x, icon.y + icon.height as i32 + 4, palette::TEXT_WHITE, 1);
    }

    fn draw_desktop_icon(&self, driver: &mut VesaDriver, icon: &Icon) {
        // Icon background (slightly rounded)
        driver.fill_rect(
            icon.x,
            icon.y,
            icon.width,
            icon.height - 16, // Space for label
            palette::ICON_BG
        );

        // Icon character (centered)
        let char_x = icon.x + (icon.width as i32 / 2) - 16;
        let char_y = icon.y + 12;
        driver.draw_char(icon.icon_char, char_x, char_y, palette::TEXT_BLACK, 4);

        // Label (below icon)
        let label_x = icon.x + (icon.width as i32 / 2) - ((icon.label.len() as i32 * 4));
        driver.draw_text(&icon.label, label_x, icon.y + icon.height as i32 - 12, palette::TEXT_WHITE, 1);
    }

    /// Handle mouse click
    pub fn handle_click(&self, x: i32, y: i32) -> Option<IconAction> {
        // Check dock icons
        for icon in &self.dock_icons {
            if x >= icon.x && x < icon.x + icon.width as i32 &&
               y >= icon.y && y < icon.y + icon.height as i32 {
                println!("[desktop] Clicked dock icon: {}", icon.label);
                return Some(icon.action.clone());
            }
        }

        // Check desktop icons
        for icon in &self.desktop_icons {
            if x >= icon.x && x < icon.x + icon.width as i32 &&
               y >= icon.y && y < icon.y + icon.height as i32 {
                println!("[desktop] Clicked desktop icon: {}", icon.label);
                return Some(icon.action.clone());
            }
        }

        None
    }

    /// Show the desktop
    pub fn show() {
        println!("[desktop_ui] Showing graphical desktop");
        let mut driver = vesa::driver().lock();

        if !driver.is_initialized() {
            println!("[desktop_ui] VESA driver not initialized!");
            return;
        }

        let desktop = DesktopUI::new();
        desktop.draw(&mut driver);

        println!("[desktop_ui] Desktop drawn, ready for interaction");
    }
}
