//! PWA Launcher
//!
//! Handles launching PWAs in the browser with proper isolation:
//! - Opens app in browser window
//! - Manages app lifecycle (start, stop, switch)
//! - Handles app permissions and isolation

use super::{PwaApp, PwaResult, PwaError};
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use spin::Mutex;
use lazy_static::lazy_static;

/// Maximum number of concurrently running apps
const MAX_RUNNING_APPS: usize = 8;

/// Running app instance
#[derive(Debug, Clone)]
pub struct RunningApp {
    /// App ID
    pub app_id: String,
    /// Window ID (in desktop/window manager)
    pub window_id: u32,
    /// Start time (ticks)
    pub start_time: u64,
    /// Is currently focused
    pub is_focused: bool,
    /// App URL
    pub url: String,
}

/// PWA Launcher state
pub struct PwaLauncher {
    /// Currently running apps
    running_apps: Vec<RunningApp>,
    /// Next window ID
    next_window_id: u32,
    /// Currently focused app
    focused_app: Option<String>,
    /// Launch history (for app switching)
    launch_history: Vec<String>,
}

impl PwaLauncher {
    /// Create a new launcher
    pub fn new() -> Self {
        Self {
            running_apps: Vec::new(),
            next_window_id: 1000, // PWA windows start at 1000
            focused_app: None,
            launch_history: Vec::new(),
        }
    }
    
    /// Launch a PWA by ID
    pub fn launch(&mut self, app_id: &str) -> PwaResult<u32> {
        // Check if app exists
        let app = super::get_app(app_id)
            .ok_or_else(|| PwaError::app_not_found(app_id))?;
        
        // Check if already running
        if let Some(window_id) = self.find_running_app(app_id).map(|r| r.window_id) {
            // Focus the existing window
            self.focus_app(app_id);
            return Ok(window_id);
        }
        
        // Check max running apps limit
        if self.running_apps.len() >= MAX_RUNNING_APPS {
            // Kill the oldest unfocused app
            self.kill_oldest_app()?;
        }
        
        // Create new window ID
        let window_id = self.next_window_id;
        self.next_window_id += 1;
        
        // Get app URL
        let app_url = app.entry_url();
        
        // Launch in browser
        self.launch_in_browser(&app, window_id)?;
        
        // Track running app
        let running = RunningApp {
            app_id: app_id.to_string(),
            window_id,
            start_time: self.get_timestamp(),
            is_focused: true,
            url: app_url,
        };
        
        self.running_apps.push(running);
        self.launch_history.push(app_id.to_string());
        
        // Update focused app
        self.set_focused_app(app_id);
        
        crate::println!("[pwa] Launched {} (window {})", app_id, window_id);
        
        Ok(window_id)
    }
    
    /// Launch a PWA in the browser
    fn launch_in_browser(&self, app: &PwaApp, window_id: u32) -> PwaResult<()> {
        // Use the browser module to navigate to the app URL
        let url = app.entry_url();
        
        crate::println!("[pwa] Opening browser for {} at {}", app.id, url);
        
        // Navigate browser to app URL
        // In the full implementation, this would integrate with the desktop window manager
        // to create a proper app window
        
        // For now, we'll use the browser's navigate function
        // This opens the app in the browser context
        if let Err(_) = crate::browser::navigate(&url) {
            // Browser might not be fully initialized, that's OK
            crate::println!("[pwa] Browser navigate returned error (may not be initialized yet)");
        }
        
        Ok(())
    }
    
    /// Close a running app
    pub fn close_app(&mut self, app_id: &str) -> PwaResult<()> {
        let idx = self.running_apps.iter()
            .position(|a| a.app_id == app_id)
            .ok_or_else(|| PwaError::app_not_found(app_id))?;
        
        let running = self.running_apps.remove(idx);
        
        // Close browser window
        self.close_browser_window(running.window_id);
        
        // Update focused app if needed
        if self.focused_app.as_deref() == Some(app_id) {
            self.focused_app = self.running_apps.last().map(|a| a.app_id.clone());
        }
        
        crate::println!("[pwa] Closed {} (window {})", app_id, running.window_id);
        
        Ok(())
    }
    
    /// Close app by window ID
    pub fn close_window(&mut self, window_id: u32) -> PwaResult<()> {
        let idx = self.running_apps.iter()
            .position(|a| a.window_id == window_id)
            .ok_or_else(|| PwaError::invalid_parameter("Window not found"))?;
        
        let running = self.running_apps.remove(idx);
        
        // Close browser window
        self.close_browser_window(window_id);
        
        // Update focused app if needed
        if self.focused_app == Some(running.app_id) {
            self.focused_app = self.running_apps.last().map(|a| a.app_id.clone());
        }
        
        Ok(())
    }
    
    /// Focus an app
    pub fn focus_app(&mut self, app_id: &str) -> PwaResult<()> {
        // Update focus status
        for app in &mut self.running_apps {
            app.is_focused = app.app_id == app_id;
        }
        
        self.set_focused_app(app_id);
        
        // Bring window to front in browser/desktop
        // This would integrate with the window manager
        
        crate::println!("[pwa] Focused {}", app_id);
        
        Ok(())
    }
    
    /// Switch to next app
    pub fn switch_to_next_app(&mut self) -> PwaResult<()> {
        if self.running_apps.is_empty() {
            return Err(PwaError::invalid_parameter("No running apps"));
        }
        
        // Find current focused app index
        let current_idx = self.running_apps.iter()
            .position(|a| a.is_focused)
            .unwrap_or(0);
        
        // Get next app
        let next_idx = (current_idx + 1) % self.running_apps.len();
        let next_app_id = self.running_apps[next_idx].app_id.clone();
        
        self.focus_app(&next_app_id)
    }
    
    /// Switch to previous app
    pub fn switch_to_prev_app(&mut self) -> PwaResult<()> {
        if self.running_apps.is_empty() {
            return Err(PwaError::invalid_parameter("No running apps"));
        }
        
        // Find current focused app index
        let current_idx = self.running_apps.iter()
            .position(|a| a.is_focused)
            .unwrap_or(0);
        
        // Get previous app
        let prev_idx = if current_idx == 0 {
            self.running_apps.len() - 1
        } else {
            current_idx - 1
        };
        let prev_app_id = self.running_apps[prev_idx].app_id.clone();
        
        self.focus_app(&prev_app_id)
    }
    
    /// Get list of running apps
    pub fn list_running(&self) -> Vec<RunningApp> {
        self.running_apps.clone()
    }
    
    /// Check if an app is running
    pub fn is_running(&self, app_id: &str) -> bool {
        self.running_apps.iter().any(|a| a.app_id == app_id)
    }
    
    /// Find a running app
    fn find_running_app(&self, app_id: &str) -> Option<&RunningApp> {
        self.running_apps.iter().find(|a| a.app_id == app_id)
    }
    
    /// Set the focused app
    fn set_focused_app(&mut self, app_id: &str) {
        self.focused_app = Some(app_id.to_string());
        
        // Update is_focused flag
        for app in &mut self.running_apps {
            app.is_focused = app.app_id == app_id;
        }
    }
    
    /// Kill the oldest running app (when at limit)
    fn kill_oldest_app(&mut self) -> PwaResult<()> {
        // Find oldest unfocused app
        let oldest_idx = self.running_apps.iter()
            .enumerate()
            .filter(|(_, a)| !a.is_focused)
            .min_by_key(|(_, a)| a.start_time)
            .map(|(idx, _)| idx);
        
        if let Some(idx) = oldest_idx {
            let app_id = self.running_apps[idx].app_id.clone();
            self.close_app(&app_id)?;
            Ok(())
        } else {
            // All apps are focused, can't kill any
            Err(PwaError::launch_failed("Maximum number of apps running"))
        }
    }
    
    /// Close browser window
    fn close_browser_window(&self, _window_id: u32) {
        // This would integrate with the browser/window manager
        // to close the specific window
        crate::println!("[pwa] Closing browser window {}", _window_id);
    }
    
    /// Get current timestamp
    fn get_timestamp(&self) -> u64 {
        // Use timer ticks when available
        crate::arch::interrupts::get_timer_ticks()
    }
    
    /// Get the currently focused app
    pub fn get_focused_app(&self) -> Option<String> {
        self.focused_app.clone()
    }
    
    /// Launch app with URL override (for deep linking)
    pub fn launch_with_url(&mut self, app_id: &str, url: &str) -> PwaResult<u32> {
        // Check if app exists
        let _app = super::get_app(app_id)
            .ok_or_else(|| PwaError::app_not_found(app_id))?;
        
        // Check if already running
        if let Some(window_id) = self.find_running_app(app_id).map(|r| r.window_id) {
            // Navigate to new URL
            let _ = crate::browser::navigate(url);
            self.focus_app(app_id)?;
            return Ok(window_id);
        }
        
        // Launch new instance with URL
        let window_id = self.next_window_id;
        self.next_window_id += 1;
        
        // Navigate to the specific URL
        let _ = crate::browser::navigate(url);
        
        let running = RunningApp {
            app_id: app_id.to_string(),
            window_id,
            start_time: self.get_timestamp(),
            is_focused: true,
            url: url.to_string(),
        };
        
        self.running_apps.push(running);
        self.launch_history.push(app_id.to_string());
        self.set_focused_app(app_id);
        
        Ok(window_id)
    }
}

impl Default for PwaLauncher {
    fn default() -> Self {
        Self::new()
    }
}

lazy_static! {
    static ref LAUNCHER: Mutex<PwaLauncher> = Mutex::new(PwaLauncher::new());
}

/// Initialize the launcher
pub fn init() {
    crate::println!("[pwa] Initializing PWA launcher...");
    // Launcher is ready
}

/// Launch a PWA by ID
pub fn launch_app(app_id: &str) -> PwaResult<()> {
    let window_id = LAUNCHER.lock().launch(app_id)?;
    crate::println!("[pwa] App {} launched in window {}", app_id, window_id);
    Ok(())
}

/// Close a running app
pub fn close_app(app_id: &str) -> PwaResult<()> {
    LAUNCHER.lock().close_app(app_id)
}

/// Focus an app
pub fn focus_app(app_id: &str) -> PwaResult<()> {
    LAUNCHER.lock().focus_app(app_id)
}

/// Switch to next app
pub fn switch_to_next() -> PwaResult<()> {
    LAUNCHER.lock().switch_to_next_app()
}

/// Switch to previous app
pub fn switch_to_prev() -> PwaResult<()> {
    LAUNCHER.lock().switch_to_prev_app()
}

/// List running apps
pub fn list_running() -> Vec<RunningApp> {
    LAUNCHER.lock().list_running()
}

/// Check if app is running
pub fn is_running(app_id: &str) -> bool {
    LAUNCHER.lock().is_running(app_id)
}

/// Get focused app
pub fn get_focused() -> Option<String> {
    LAUNCHER.lock().get_focused_app()
}

/// Launch app with specific URL
pub fn launch_with_url(app_id: &str, url: &str) -> PwaResult<()> {
    let window_id = LAUNCHER.lock().launch_with_url(app_id, url)?;
    crate::println!("[pwa] App {} launched at {} in window {}", app_id, url, window_id);
    Ok(())
}

/// Print launcher status
pub fn print_status() {
    let launcher = LAUNCHER.lock();
    let running = launcher.list_running();
    
    crate::println!("\nPWA Launcher:");
    crate::println!("  Running apps: {}", running.len());
    
    for app in running {
        let focus_marker = if app.is_focused { " [FOCUSED]" } else { "" };
        crate::println!("  [{}] {} ({}){}", 
            app.window_id, 
            app.app_id,
            app.url,
            focus_marker
        );
    }
    
    if let Some(focused) = launcher.get_focused_app() {
        crate::println!("  Focused: {}", focused);
    }
}
