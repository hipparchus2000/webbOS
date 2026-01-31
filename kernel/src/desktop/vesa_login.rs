//! VESA Login Screen
//!
//! A graphical login screen using the VESA framebuffer

use crate::drivers::vesa::{self, colors};
use alloc::string::String;
use crate::drivers::input;
use crate::println;

const KEY_ENTER: u16 = 0x1C; // Enter key scancode

/// Show login screen on VESA framebuffer
pub fn show_login_screen() -> Option<(u64, String)> {
    // Clear screen to dark blue
    vesa::clear(colors::rgb(0, 0, 64));
    
    // Get screen dimensions
    let info = vesa::info()?;
    let cx = (info.width / 2) as i32;
    let cy = (info.height / 2) as i32;
    
    // Draw title
    vesa::draw_text("WebbOS Login", cx - 120, cy - 100, colors::WHITE, 3);
    
    // Draw username prompt
    vesa::draw_text("Username:", cx - 150, cy - 20, colors::YELLOW, 2);
    
    // Draw password prompt
    vesa::draw_text("Password:", cx - 150, cy + 40, colors::YELLOW, 2);
    
    // Draw input boxes
    vesa::draw_rect(cx - 20, cy - 25, 200, 30, colors::WHITE);
    vesa::draw_rect(cx - 20, cy + 35, 200, 30, colors::WHITE);
    
    // Simple login - just wait for Enter key
    vesa::draw_text("Press ENTER to login as 'admin'", cx - 180, cy + 120, colors::LIGHT_GRAY, 1);
    
    // Wait for keypress
    loop {
        if let Some(key) = input::get_key() {
            if key.keycode == KEY_ENTER {
                return Some((1, String::from("admin")));
            }
        }
        
        // Small delay
        for _ in 0..100000 {
            unsafe { core::arch::asm!("nop") };
        }
    }
}

/// Draw a welcome message
pub fn show_welcome_message() {
    // Clear to dark green
    vesa::clear(colors::rgb(0, 64, 0));
    
    let info = vesa::info().unwrap();
    let cx = (info.width / 2) as i32;
    let cy = (info.height / 2) as i32;
    
    // Draw welcome text
    vesa::draw_text("Welcome to WebbOS!", cx - 200, cy - 50, colors::WHITE, 3);
    vesa::draw_text("Login successful", cx - 120, cy + 20, colors::GREEN, 2);
}

/// Draw a circle (another shape after login)
pub fn draw_post_login_shape() {
    let info = vesa::info().unwrap();
    let cx = (info.width / 2) as i32;
    let cy = (info.height / 2) as i32 + 100;
    
    // Draw a filled circle below the welcome message
    vesa::fill_circle(cx, cy, 60, colors::MAGENTA);
    vesa::draw_circle(cx, cy, 60, colors::WHITE);
    
    println!("[vesa] Post-login circle drawn at ({}, {})", cx, cy);
}

/// Simple text input for VESA (basic version)
pub fn read_line_vesa(_prompt: &str, _x: i32, _y: i32) -> String {
    // For now, just return admin - full text input would need more work
    String::from("admin")
}
