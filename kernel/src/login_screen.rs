//! Pixel-Based Login Screen for WebbOS
//!
//! This module implements a graphical login screen using direct pixel drawing
//! to the VESA framebuffer. It appears after the boot triangle animation.

use alloc::string::String;
use spin::Mutex;
use lazy_static::lazy_static;

use crate::println;
use crate::drivers::vesa::{self, VesaDriver, colors};
use crate::desktop;

/// Color palette for login screen
mod palette {
    use super::vesa::colors;
    
    pub const fn rgb(r: u8, g: u8, b: u8) -> u32 {
        0xFF000000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
    }
    
    pub const BG_TOP: u32 = rgb(102, 126, 234);      // #667eea - Purple gradient top
    pub const BG_BOTTOM: u32 = rgb(118, 75, 162);    // #764ba2 - Purple gradient bottom
    pub const CARD_BG: u32 = colors::WHITE;
    pub const TEXT_PRIMARY: u32 = rgb(51, 51, 51);   // #333333
    pub const TEXT_SECONDARY: u32 = rgb(102, 102, 102); // #666666
    pub const INPUT_BORDER: u32 = rgb(224, 224, 224); // #e0e0e0
    pub const INPUT_BORDER_FOCUS: u32 = rgb(102, 126, 234); // #667eea
    pub const BUTTON_BG: u32 = rgb(102, 126, 234);   // #667eea
    pub const BUTTON_HOVER: u32 = rgb(86, 110, 218); // Slightly darker
    pub const BUTTON_TEXT: u32 = colors::WHITE;
    pub const HINT_BG: u32 = rgb(245, 245, 245);     // #f5f5f5
    pub const HINT_BORDER: u32 = rgb(224, 224, 224); // #e0e0e0
}

/// Login screen state
pub struct LoginScreen {
    visible: bool,
    username: String,
    password: String,
    focused_field: Field,
    cursor_position: usize,
    show_password: bool,
}

/// Focusable fields
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Field {
    Username,
    Password,
    Button,
}

impl LoginScreen {
    /// Create new login screen
    const fn new() -> Self {
        Self {
            visible: false,
            username: String::new(),
            password: String::new(),
            focused_field: Field::Username,
            cursor_position: 0,
            show_password: false,
        }
    }
    
    /// Show the login screen
    pub fn show(&mut self) {
        println!("[login_screen] Showing login screen");
        self.visible = true;
        self.username.clear();
        self.password.clear();
        self.focused_field = Field::Username;
        self.cursor_position = 0;
        self.draw();
    }
    
    /// Hide the login screen
    pub fn hide(&mut self) {
        self.visible = false;
    }
    
    /// Check if login screen is visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }
    
    /// Draw the entire login screen
    pub fn draw(&self) {
        if !self.visible {
            return;
        }
        
        println!("[login_screen] Drawing login screen...");
        
        println!("[login_screen] Acquiring VESA driver lock...");
        let mut driver = vesa::driver().lock();
        println!("[login_screen] VESA driver lock acquired");
        
        if !driver.is_initialized() {
            println!("[login_screen] VESA driver not initialized!");
            return;
        }
        
        let info = driver.info();
        let screen_w = info.width as i32;
        let screen_h = info.height as i32;
        
        println!("[login_screen] Screen size: {}x{}", screen_w, screen_h);
        
        // Simple test - clear screen to blue first
        println!("[login_screen] Clearing screen...");
        driver.clear(palette::BG_TOP);
        println!("[login_screen] Screen cleared");
        
        // Draw a simple white rectangle in center as test
        let rect_w = 400u32;
        let rect_h = 300u32;
        let rect_x = (screen_w - rect_w as i32) / 2;
        let rect_y = (screen_h - rect_h as i32) / 2;
        
        driver.fill_rect(rect_x, rect_y, rect_w, rect_h, palette::CARD_BG);
        driver.draw_rect(rect_x, rect_y, rect_w, rect_h, palette::INPUT_BORDER);
        
        // Draw title
        let title = "WebbOS Login";
        let title_x = rect_x + 20;
        let title_y = rect_y + 30;
        driver.draw_text(title, title_x, title_y, palette::TEXT_PRIMARY, 2);
        
        // Draw instructions
        let instr = "Use: admin/admin or user/user";
        driver.draw_text(instr, rect_x + 20, rect_y + 80, palette::TEXT_SECONDARY, 1);
        
        // Input field dimensions
        let input_x = rect_x + 120;
        let input_w = 240;
        let input_h = 24;
        
        // Draw username field
        driver.draw_text("Username:", rect_x + 20, rect_y + 130, palette::TEXT_PRIMARY, 1);
        let username_bg = if self.focused_field == Field::Username { palette::INPUT_BORDER_FOCUS } else { palette::INPUT_BORDER };
        driver.fill_rect(input_x, rect_y + 125, input_w, input_h, colors::WHITE);
        driver.draw_rect(input_x, rect_y + 125, input_w, input_h, username_bg);
        driver.draw_text(&self.username, input_x + 8, rect_y + 132, palette::TEXT_PRIMARY, 1);
        // Draw cursor if focused
        if self.focused_field == Field::Username {
            let cursor_x = input_x + 8 + (self.username.len() as i32 * 8);
            driver.fill_rect(cursor_x, rect_y + 128, 2, 18, palette::TEXT_PRIMARY);
        }
        
        // Draw password field
        driver.draw_text("Password:", rect_x + 20, rect_y + 170, palette::TEXT_PRIMARY, 1);
        let password_bg = if self.focused_field == Field::Password { palette::INPUT_BORDER_FOCUS } else { palette::INPUT_BORDER };
        driver.fill_rect(input_x, rect_y + 165, input_w, input_h, colors::WHITE);
        driver.draw_rect(input_x, rect_y + 165, input_w, input_h, password_bg);
        let stars: alloc::string::String = (0..self.password.len()).map(|_| '*').collect();
        driver.draw_text(&stars, input_x + 8, rect_y + 172, palette::TEXT_PRIMARY, 1);
        // Draw cursor if focused
        if self.focused_field == Field::Password {
            let cursor_x = input_x + 8 + (self.password.len() as i32 * 8);
            driver.fill_rect(cursor_x, rect_y + 168, 2, 18, palette::TEXT_PRIMARY);
        }
        
        // Draw Login button
        let btn_x = rect_x + 150;
        let btn_y = rect_y + 210;
        let btn_w = 100;
        let btn_h = 32;
        let btn_color = if self.focused_field == Field::Button { palette::BUTTON_HOVER } else { palette::BUTTON_BG };
        driver.fill_rect(btn_x, btn_y, btn_w, btn_h, btn_color);
        driver.draw_rect(btn_x, btn_y, btn_w, btn_h, palette::INPUT_BORDER);
        driver.draw_text("LOGIN", btn_x + 25, btn_y + 10, palette::BUTTON_TEXT, 1);
        
        // Draw hint box
        driver.fill_rect(rect_x + 20, rect_y + 260, 360, 24, palette::HINT_BG);
        driver.draw_rect(rect_x + 20, rect_y + 260, 360, 24, palette::HINT_BORDER);
        driver.draw_text("Tab: switch field  |  Enter: submit", rect_x + 30, rect_y + 267, palette::TEXT_SECONDARY, 1);
        
        println!("[login_screen] Drawing complete");
    }
    
    /// Draw gradient background (horizontal lines with interpolated colors)
    fn draw_gradient_background(&self, driver: &mut VesaDriver, screen_w: i32, screen_h: i32) {
        for y in 0..screen_h {
            let t = y as f32 / screen_h as f32;
            let color = self.interpolate_color(palette::BG_TOP, palette::BG_BOTTOM, t);
            driver.hline(0, y, screen_w as u32, color);
        }
    }
    
    /// Interpolate between two colors
    fn interpolate_color(&self, c1: u32, c2: u32, t: f32) -> u32 {
        let r1 = ((c1 >> 16) & 0xFF) as f32;
        let g1 = ((c1 >> 8) & 0xFF) as f32;
        let b1 = (c1 & 0xFF) as f32;
        
        let r2 = ((c2 >> 16) & 0xFF) as f32;
        let g2 = ((c2 >> 8) & 0xFF) as f32;
        let b2 = (c2 & 0xFF) as f32;
        
        let r = (r1 + (r2 - r1) * t) as u32;
        let g = (g1 + (g2 - g1) * t) as u32;
        let b = (b1 + (b2 - b1) * t) as u32;
        
        0xFF000000 | (r << 16) | (g << 8) | b
    }
    
    /// Draw logo (simple globe/circle with lines)
    fn draw_logo(&self, driver: &mut VesaDriver, cx: i32, cy: i32) {
        let radius = 20i32;
        let color = palette::BUTTON_BG;
        
        // Draw circle outline
        driver.draw_circle(cx, cy, radius, color);
        
        // Draw horizontal line through center
        driver.hline(cx - radius, cy, (radius * 2) as u32 + 1, color);
        
        // Draw vertical line through center  
        driver.vline(cx, cy - radius, (radius * 2) as u32 + 1, color);
        
        // Draw curved lines to represent globe (simplified as smaller circles)
        driver.draw_circle(cx, cy, radius - 7, color);
    }
    
    /// Draw an input field
    fn draw_input_field(&self, driver: &mut VesaDriver, x: i32, y: i32, w: u32, h: u32, 
                        text: &str, focused: bool) {
        // Background
        driver.fill_rect(x, y, w, h, colors::WHITE);
        
        // Border
        let border_color = if focused { palette::INPUT_BORDER_FOCUS } else { palette::INPUT_BORDER };
        driver.draw_rect(x, y, w, h, border_color);
        
        // If focused, draw thicker border effect
        if focused {
            driver.draw_rect(x - 1, y - 1, w + 2, h + 2, border_color);
        }
        
        // Text (with padding)
        let text_x = x + 12;
        let text_y = y + (h as i32 - 8) / 2; // Vertically centered (8 = char height)
        driver.draw_text(text, text_x, text_y, palette::TEXT_PRIMARY, 1);
        
        // Draw cursor if focused
        if focused {
            let cursor_x = text_x + (text.len() as i32 * 8) + 2;
            driver.vline(cursor_x, text_y - 2, 12, palette::TEXT_PRIMARY);
        }
    }
    
    /// Draw a button
    fn draw_button(&self, driver: &mut VesaDriver, x: i32, y: i32, w: u32, h: u32,
                   text: &str, focused: bool) {
        // Button background
        let bg_color = if focused { palette::BUTTON_HOVER } else { palette::BUTTON_BG };
        driver.fill_rect(x, y, w, h, bg_color);
        
        // Button text (centered)
        let text_width = text.len() as i32 * 8 * 2; // scale 2
        let text_x = x + (w as i32 - text_width) / 2;
        let text_y = y + (h as i32 - 8 * 2) / 2; // Vertically centered
        driver.draw_text(text, text_x, text_y, palette::BUTTON_TEXT, 2);
        
        // Focus indicator
        if focused {
            driver.draw_rect(x - 2, y - 2, w + 4, h + 4, palette::BUTTON_BG);
        }
    }
    
    /// Draw hint box with default credentials
    fn draw_hint_box(&self, driver: &mut VesaDriver, x: i32, y: i32, w: u32, h: u32) {
        // Background
        driver.fill_rect(x, y, w, h, palette::HINT_BG);
        driver.draw_rect(x, y, w, h, palette::HINT_BORDER);
        
        // Title
        driver.draw_text("Default accounts:", x + 10, y + 10, palette::TEXT_SECONDARY, 1);
        
        // Admin line
        driver.draw_text("Admin: admin / admin", x + 10, y + 28, palette::TEXT_PRIMARY, 1);
        
        // User line
        driver.draw_text("User:  user / user", x + 10, y + 44, palette::TEXT_PRIMARY, 1);
    }
    
    /// Handle keyboard input
    pub fn handle_key(&mut self, key: u8) -> LoginAction {
        if !self.visible {
            return LoginAction::None;
        }
        
        match key {
            b'\t' => {
                // Tab - move to next field
                println!("[login_screen] Tab pressed, switching field");
                self.focused_field = match self.focused_field {
                    Field::Username => Field::Password,
                    Field::Password => Field::Button,
                    Field::Button => Field::Username,
                };
                self.draw();
                LoginAction::None
            }
            b'\n' | b'\r' => {
                // Enter - submit or move to next
                println!("[login_screen] Enter pressed, focused: {:?}", self.focused_field);
                match self.focused_field {
                    Field::Username => {
                        self.focused_field = Field::Password;
                        self.draw();
                        LoginAction::None
                    }
                    Field::Password => {
                        self.focused_field = Field::Button;
                        self.draw();
                        LoginAction::None
                    }
                    Field::Button => {
                        self.attempt_login()
                    }
                }
            }
            27 => {
                // Escape - clear current field or cancel
                match self.focused_field {
                    Field::Username => {
                        self.username.clear();
                    }
                    Field::Password => {
                        self.password.clear();
                    }
                    Field::Button => {}
                }
                self.draw();
                LoginAction::None
            }
            8 | 127 => {
                // Backspace
                match self.focused_field {
                    Field::Username => {
                        if !self.username.is_empty() {
                            self.username.pop();
                        }
                    }
                    Field::Password => {
                        if !self.password.is_empty() {
                            self.password.pop();
                        }
                    }
                    Field::Button => {}
                }
                self.draw();
                LoginAction::None
            }
            c if c >= 32 && c <= 126 => {
                // Printable character
                println!("[login_screen] Key pressed: '{}'", c as char);
                match self.focused_field {
                    Field::Username => {
                        if self.username.len() < 32 {
                            self.username.push(c as char);
                            println!("[login_screen] Username now: '{}'", self.username);
                        }
                    }
                    Field::Password => {
                        if self.password.len() < 32 {
                            self.password.push(c as char);
                            println!("[login_screen] Password len now: {}", self.password.len());
                        }
                    }
                    Field::Button => {}
                }
                self.draw();
                LoginAction::None
            }
            _ => LoginAction::None
        }
    }
    
    /// Attempt login with current credentials
    fn attempt_login(&mut self) -> LoginAction {
        let username = self.username.clone();
        let password = self.password.clone();
        
        println!("[login_screen] Attempting login for user: {}", username);
        
        if desktop::login(&username, &password) {
            println!("[login_screen] Login successful!");
            self.hide();
            LoginAction::LoginSuccess
        } else {
            println!("[login_screen] Login failed!");
            // Clear password and focus back to it
            self.password.clear();
            self.focused_field = Field::Password;
            self.draw();
            LoginAction::LoginFailed
        }
    }
}

/// Actions the login screen can trigger
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginAction {
    None,
    LoginSuccess,
    LoginFailed,
}

/// Global login screen instance
lazy_static! {
    static ref LOGIN_SCREEN: Mutex<LoginScreen> = Mutex::new(LoginScreen::new());
}

/// Show the login screen
pub fn show() {
    LOGIN_SCREEN.lock().show();
}

/// Hide the login screen
pub fn hide() {
    LOGIN_SCREEN.lock().hide();
}

/// Check if login screen is visible
pub fn is_visible() -> bool {
    LOGIN_SCREEN.lock().is_visible()
}

/// Handle a key press
pub fn handle_key(key: u8) -> LoginAction {
    LOGIN_SCREEN.lock().handle_key(key)
}

/// Redraw the login screen
pub fn redraw() {
    LOGIN_SCREEN.lock().draw();
}

/// Initialize login screen module
pub fn init() {
    println!("[login_screen] Login screen module initialized");
}
