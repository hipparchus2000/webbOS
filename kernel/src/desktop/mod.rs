//! WebbOS Desktop Environment
//!
//! HTML-based desktop with window manager, taskbar, and applications.

use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;
use alloc::collections::BTreeMap;
use spin::Mutex;
use lazy_static::lazy_static;

use crate::println;
use crate::users::{self, User};

pub mod vesa_login;

/// Window ID
pub type WindowId = u32;

/// Application ID
pub type AppId = u32;

/// Window state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowState {
    Normal,
    Minimized,
    Maximized,
    Focused,
}

/// Window structure
#[derive(Debug, Clone)]
pub struct Window {
    pub id: WindowId,
    pub app_id: AppId,
    pub title: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub state: WindowState,
    pub z_index: u32,
    pub content: String, // HTML content
    pub icon: char, // Unicode icon
}

/// Application structure
#[derive(Debug, Clone)]
pub struct Application {
    pub id: AppId,
    pub name: String,
    pub title: String,
    pub icon: char,
    pub description: String,
    pub html_content: String,
    pub css_styles: String,
    pub js_scripts: String,
    pub singleton: bool, // Only one instance allowed
}

/// Desktop item (icon on desktop)
#[derive(Debug, Clone)]
pub struct DesktopItem {
    pub id: u32,
    pub name: String,
    pub icon: char,
    pub x: i32,
    pub y: i32,
    pub is_folder: bool,
    pub path: String,
}

/// Desktop manager
pub struct DesktopManager {
    windows: BTreeMap<WindowId, Window>,
    applications: BTreeMap<AppId, Application>,
    desktop_items: Vec<DesktopItem>,
    next_window_id: WindowId,
    next_app_id: AppId,
    next_item_id: u32,
    active_window: Option<WindowId>,
    wallpaper: String,
    current_user: Option<User>,
    show_login: bool,
    show_desktop: bool,
    screen_width: u32,
    screen_height: u32,
    taskbar_height: u32,
}

impl DesktopManager {
    /// Create new desktop manager
    fn new() -> Self {
        let mut manager = Self {
            windows: BTreeMap::new(),
            applications: BTreeMap::new(),
            desktop_items: Vec::new(),
            next_window_id: 1,
            next_app_id: 1,
            next_item_id: 1,
            active_window: None,
            wallpaper: String::from("/system/wallpapers/default.jpg"),
            current_user: None,
            show_login: true,
            show_desktop: false,
            screen_width: 1024,
            screen_height: 768,
            taskbar_height: 40,
        };
        
        // Register built-in applications
        manager.register_builtin_apps();
        
        // Create default desktop items
        manager.create_default_desktop_items();
        
        manager
    }
    
    /// Register built-in applications
    fn register_builtin_apps(&mut self) {
        // File Manager
        self.register_app(Application {
            id: 0, // Will be assigned
            name: String::from("filemanager"),
            title: String::from("File Manager"),
            icon: '📁',
            description: String::from("Browse and manage files"),
            html_content: get_filemanager_html(),
            css_styles: get_filemanager_css(),
            js_scripts: get_filemanager_js(),
            singleton: false,
        });
        
        // Notepad
        self.register_app(Application {
            id: 0,
            name: String::from("notepad"),
            title: String::from("Notepad"),
            icon: '📝',
            description: String::from("Simple text editor"),
            html_content: get_notepad_html(),
            css_styles: get_notepad_css(),
            js_scripts: get_notepad_js(),
            singleton: false,
        });
        
        // Paint
        self.register_app(Application {
            id: 0,
            name: String::from("paint"),
            title: String::from("Paint"),
            icon: '🎨',
            description: String::from("Draw and edit images"),
            html_content: get_paint_html(),
            css_styles: get_paint_css(),
            js_scripts: get_paint_js(),
            singleton: false,
        });
        
        // Task Manager
        self.register_app(Application {
            id: 0,
            name: String::from("taskmanager"),
            title: String::from("Task Manager"),
            icon: '📊',
            description: String::from("Manage running processes"),
            html_content: get_taskmanager_html(),
            css_styles: get_taskmanager_css(),
            js_scripts: get_taskmanager_js(),
            singleton: true,
        });
        
        // User Manager
        self.register_app(Application {
            id: 0,
            name: String::from("usermanager"),
            title: String::from("User Manager"),
            icon: '👥',
            description: String::from("Manage user accounts"),
            html_content: get_usermanager_html(),
            css_styles: get_usermanager_css(),
            js_scripts: get_usermanager_js(),
            singleton: true,
        });
        
        // Terminal
        self.register_app(Application {
            id: 0,
            name: String::from("terminal"),
            title: String::from("Terminal"),
            icon: '💻',
            description: String::from("Command line interface"),
            html_content: get_terminal_html(),
            css_styles: get_terminal_css(),
            js_scripts: get_terminal_js(),
            singleton: false,
        });
        
        // Web Browser
        self.register_app(Application {
            id: 0,
            name: String::from("browser"),
            title: String::from("WebbBrowser"),
            icon: '🌐',
            description: String::from("Browse the web"),
            html_content: get_browser_html(),
            css_styles: get_browser_css(),
            js_scripts: get_browser_js(),
            singleton: false,
        });
    }
    
    /// Register an application
    fn register_app(&mut self, mut app: Application) {
        let id = self.next_app_id;
        self.next_app_id += 1;
        app.id = id;
        println!("[desktop] Registered app: {} ({})", app.name, app.title);
        self.applications.insert(id, app);
    }
    
    /// Create default desktop items
    fn create_default_desktop_items(&mut self) {
        self.desktop_items.push(DesktopItem {
            id: self.next_item_id,
            name: String::from("Home"),
            icon: '🏠',
            x: 20,
            y: 20,
            is_folder: true,
            path: String::from("/home"),
        });
        self.next_item_id += 1;
        
        self.desktop_items.push(DesktopItem {
            id: self.next_item_id,
            name: String::from("Documents"),
            icon: '📄',
            x: 20,
            y: 100,
            is_folder: true,
            path: String::from("/home/documents"),
        });
        self.next_item_id += 1;
        
        self.desktop_items.push(DesktopItem {
            id: self.next_item_id,
            name: String::from("Trash"),
            icon: '🗑',
            x: 20,
            y: 600,
            is_folder: true,
            path: String::from("/home/.trash"),
        });
        self.next_item_id += 1;
    }
    
    /// Launch an application
    pub fn launch_app(&mut self, app_id: AppId) -> Option<WindowId> {
        // Check if singleton app already running
        if let Some(app) = self.applications.get(&app_id) {
            if app.singleton {
                if let Some((id, _)) = self.windows.iter().find(|(_, w)| w.app_id == app_id) {
                    let existing_id = *id;
                    drop(id);
                    self.focus_window(existing_id);
                    return Some(existing_id);
                }
            }
            
            let window_id = self.next_window_id;
            self.next_window_id += 1;
            
            // Calculate window position (cascade)
            let offset = (self.windows.len() as i32 * 30) % 200;
            let x = 100 + offset;
            let y = 50 + offset;
            
            let window = Window {
                id: window_id,
                app_id,
                title: app.title.clone(),
                x,
                y,
                width: 800,
                height: 600,
                state: WindowState::Focused,
                z_index: self.windows.len() as u32 + 1,
                content: app.html_content.clone(),
                icon: app.icon,
            };
            
            println!("[desktop] Launched {} (window {})", app.name, window_id);
            self.windows.insert(window_id, window);
            self.active_window = Some(window_id);
            
            Some(window_id)
        } else {
            None
        }
    }
    
    /// Launch app by name
    pub fn launch_app_by_name(&mut self, name: &str) -> Option<WindowId> {
        if let Some((id, _)) = self.applications.iter().find(|(_, a)| a.name == name) {
            self.launch_app(*id)
        } else {
            None
        }
    }
    
    /// Close window
    pub fn close_window(&mut self, window_id: WindowId) -> bool {
        if self.windows.remove(&window_id).is_some() {
            if self.active_window == Some(window_id) {
                // Focus next window
                self.active_window = self.windows.keys().last().copied();
            }
            println!("[desktop] Closed window {}", window_id);
            true
        } else {
            false
        }
    }
    
    /// Focus window
    pub fn focus_window(&mut self, window_id: WindowId) {
        if self.windows.contains_key(&window_id) {
            let new_z = self.get_max_z_index() + 1;
            if let Some(window) = self.windows.get_mut(&window_id) {
                window.state = WindowState::Focused;
                window.z_index = new_z;
            }
            self.active_window = Some(window_id);
        }
    }
    
    /// Get max z-index
    fn get_max_z_index(&self) -> u32 {
        self.windows.values().map(|w| w.z_index).max().unwrap_or(0)
    }
    
    /// Minimize window
    pub fn minimize_window(&mut self, window_id: WindowId) {
        if let Some(window) = self.windows.get_mut(&window_id) {
            window.state = WindowState::Minimized;
        }
    }
    
    /// Maximize/restore window
    pub fn maximize_window(&mut self, window_id: WindowId) {
        if let Some(window) = self.windows.get_mut(&window_id) {
            window.state = match window.state {
                WindowState::Maximized => WindowState::Normal,
                _ => WindowState::Maximized,
            };
        }
    }
    
    /// Get all applications
    pub fn list_apps(&self) -> Vec<&Application> {
        self.applications.values().collect()
    }
    
    /// Get all windows
    pub fn list_windows(&self) -> Vec<&Window> {
        self.windows.values().collect()
    }
    
    /// Get desktop items
    pub fn list_desktop_items(&self) -> &[DesktopItem] {
        &self.desktop_items
    }
    
    /// Get active window
    pub fn active_window(&self) -> Option<&Window> {
        self.active_window.and_then(|id| self.windows.get(&id))
    }
    
    /// Login
    pub fn login(&mut self, username: &str, password: &str) -> bool {
        if let Some(session_id) = users::login(username, password) {
            self.current_user = users::current_user();
            self.show_login = false;
            self.show_desktop = true;
            println!("[desktop] Logged in as {}", username);
            true
        } else {
            false
        }
    }
    
    /// Logout
    pub fn logout(&mut self) {
        self.windows.clear();
        self.active_window = None;
        self.current_user = None;
        self.show_login = true;
        self.show_desktop = false;
        println!("[desktop] Logged out");
    }
    
    /// Check if showing login
    pub fn showing_login(&self) -> bool {
        self.show_login
    }
    
    /// Check if showing desktop
    pub fn showing_desktop(&self) -> bool {
        self.show_desktop
    }
    
    /// Get current user
    pub fn current_user(&self) -> Option<&User> {
        self.current_user.as_ref()
    }
    
    /// Generate full HTML page
    pub fn generate_html(&self) -> String {
        if self.show_login {
            generate_login_page()
        } else {
            generate_desktop_page(self)
        }
    }
}

/// Global desktop manager
lazy_static! {
    static ref DESKTOP_MANAGER: Mutex<DesktopManager> = Mutex::new(DesktopManager::new());
}

/// Initialize desktop environment
pub fn init() {
    println!("[desktop] Initializing desktop environment...");
    
    let manager = DESKTOP_MANAGER.lock();
    println!("[desktop] {} applications registered", manager.applications.len());
    println!("[desktop] {} desktop items", manager.desktop_items.len());
    
    // Show login screen
    println!("[desktop] Showing login screen");
}

/// Launch application by name
pub fn launch_app(name: &str) -> Option<WindowId> {
    DESKTOP_MANAGER.lock().launch_app_by_name(name)
}

/// Close window
pub fn close_window(window_id: WindowId) -> bool {
    DESKTOP_MANAGER.lock().close_window(window_id)
}

/// Login
pub fn login(username: &str, password: &str) -> bool {
    DESKTOP_MANAGER.lock().login(username, password)
}

/// Logout
pub fn logout() {
    DESKTOP_MANAGER.lock().logout();
}

/// Get current user
pub fn current_user() -> Option<User> {
    DESKTOP_MANAGER.lock().current_user.clone()
}

/// Generate HTML page
pub fn generate_html() -> String {
    DESKTOP_MANAGER.lock().generate_html()
}

/// List all applications
pub fn list_apps() -> Vec<Application> {
    DESKTOP_MANAGER.lock().list_apps().into_iter().cloned().collect()
}

/// Print desktop info
pub fn print_info() {
    let manager = DESKTOP_MANAGER.lock();
    
    println!("\nDesktop Environment:");
    println!("  Resolution: {}x{}", manager.screen_width, manager.screen_height);
    println!("  Applications: {}", manager.applications.len());
    println!("  Windows open: {}", manager.windows.len());
    println!("  Desktop items: {}", manager.desktop_items.len());
    
    if let Some(user) = &manager.current_user {
        println!("  Current user: {} ({})", user.username,
            if user.is_admin { "admin" } else { "user" });
    } else {
        println!("  Current user: none (login screen)");
    }
}

// HTML/CSS/JS for applications will be in separate files
// For now, include them as functions returning strings

fn generate_login_page() -> String {
    String::from(r#"<!DOCTYPE html>
<html>
<head>
    <title>WebbOS Login</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
        }
        .login-container {
            background: white;
            padding: 40px;
            border-radius: 16px;
            box-shadow: 0 20px 60px rgba(0,0,0,0.3);
            width: 360px;
            text-align: center;
        }
        .logo {
            font-size: 64px;
            margin-bottom: 20px;
        }
        h1 {
            color: #333;
            margin-bottom: 8px;
            font-size: 24px;
        }
        .subtitle {
            color: #666;
            margin-bottom: 30px;
            font-size: 14px;
        }
        .input-group {
            margin-bottom: 16px;
            text-align: left;
        }
        label {
            display: block;
            margin-bottom: 6px;
            color: #555;
            font-size: 14px;
            font-weight: 500;
        }
        input {
            width: 100%;
            padding: 12px 16px;
            border: 2px solid #e0e0e0;
            border-radius: 8px;
            font-size: 16px;
            transition: border-color 0.2s;
        }
        input:focus {
            outline: none;
            border-color: #667eea;
        }
        button {
            width: 100%;
            padding: 14px;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            border: none;
            border-radius: 8px;
            font-size: 16px;
            font-weight: 600;
            cursor: pointer;
            transition: transform 0.2s, box-shadow 0.2s;
        }
        button:hover {
            transform: translateY(-2px);
            box-shadow: 0 8px 20px rgba(102, 126, 234, 0.4);
        }
        .hint {
            margin-top: 20px;
            padding: 12px;
            background: #f5f5f5;
            border-radius: 6px;
            font-size: 12px;
            color: #666;
        }
        .hint code {
            background: #e0e0e0;
            padding: 2px 6px;
            border-radius: 3px;
            font-family: monospace;
        }
    </style>
</head>
<body>
    <div class="login-container">
        <div class="logo">🌐</div>
        <h1>Welcome to WebbOS</h1>
        <p class="subtitle">Web Browser Operating System</p>
        <form id="loginForm">
            <div class="input-group">
                <label for="username">Username</label>
                <input type="text" id="username" name="username" placeholder="Enter username" required>
            </div>
            <div class="input-group">
                <label for="password">Password</label>
                <input type="password" id="password" name="password" placeholder="Enter password" required>
            </div>
            <button type="submit">Sign In</button>
        </form>
        <div class="hint">
            Default accounts:<br>
            Admin: <code>admin</code> / <code>admin</code><br>
            User: <code>user</code> / <code>user</code>
        </div>
    </div>
    <script>
        document.getElementById('loginForm').addEventListener('submit', function(e) {
            e.preventDefault();
            const username = document.getElementById('username').value;
            const password = document.getElementById('password').value;
            // Send login request to kernel
            window.parent.postMessage({ type: 'login', username, password }, '*');
        });
    </script>
</body>
</html>"#)
}

fn generate_desktop_page(manager: &DesktopManager) -> String {
    // Build taskbar items
    let mut taskbar_items = String::new();
    for window in manager.list_windows() {
        let active_class = if window.state == WindowState::Focused { "active" } else { "" };
        taskbar_items.push_str(&format!(
            r#"<div class="taskbar-item {}" data-window="{}">
                <span class="icon">{}</span>
                <span class="title">{}</span>
            </div>"#,
            active_class, window.id, window.icon, window.title
        ));
    }
    
    // Build desktop icons
    let mut desktop_icons = String::new();
    for item in manager.list_desktop_items() {
        desktop_icons.push_str(&format!(
            r#"<div class="desktop-icon" style="left: {}px; top: {}px;" data-path="{}">
                <div class="icon">{}</div>
                <div class="name">{}</div>
            </div>"#,
            item.x, item.y, item.path, item.icon, item.name
        ));
    }
    
    // Build application menu
    let mut app_menu_items = String::new();
    for app in manager.list_apps() {
        app_menu_items.push_str(&format!(
            r#"<div class="app-item" data-app="{}">
                <span class="icon">{}</span>
                <span class="name">{}</span>
                <span class="desc">{}</span>
            </div>"#,
            app.name, app.icon, app.title, app.description
        ));
    }
    
    format!(r#"<!DOCTYPE html>
<html>
<head>
    <title>WebbOS Desktop</title>
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: linear-gradient(135deg, #1a1a2e 0%, #16213e 100%);
            height: 100vh;
            overflow: hidden;
            user-select: none;
        }}
        
        /* Desktop */
        #desktop {{
            position: relative;
            width: 100%;
            height: calc(100vh - 40px);
            background-size: cover;
            background-position: center;
        }}
        
        .desktop-icon {{
            position: absolute;
            width: 80px;
            text-align: center;
            cursor: pointer;
            padding: 8px;
            border-radius: 8px;
            transition: background 0.2s;
        }}
        .desktop-icon:hover {{
            background: rgba(255,255,255,0.1);
        }}
        .desktop-icon .icon {{
            font-size: 48px;
            margin-bottom: 4px;
        }}
        .desktop-icon .name {{
            color: white;
            font-size: 12px;
            text-shadow: 0 1px 3px rgba(0,0,0,0.8);
            word-wrap: break-word;
        }}
        
        /* Taskbar */
        #taskbar {{
            position: fixed;
            bottom: 0;
            left: 0;
            right: 0;
            height: 40px;
            background: rgba(0,0,0,0.8);
            backdrop-filter: blur(10px);
            display: flex;
            align-items: center;
            padding: 0 8px;
            z-index: 10000;
        }}
        
        #start-btn {{
            display: flex;
            align-items: center;
            gap: 8px;
            padding: 6px 16px;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            border: none;
            border-radius: 4px;
            font-weight: 600;
            cursor: pointer;
            margin-right: 12px;
        }}
        
        #start-btn:hover {{
            transform: translateY(-1px);
        }}
        
        .taskbar-items {{
            flex: 1;
            display: flex;
            gap: 4px;
        }}
        
        .taskbar-item {{
            display: flex;
            align-items: center;
            gap: 8px;
            padding: 6px 12px;
            background: rgba(255,255,255,0.1);
            color: white;
            border-radius: 4px;
            cursor: pointer;
            font-size: 13px;
            max-width: 200px;
        }}
        
        .taskbar-item:hover {{
            background: rgba(255,255,255,0.2);
        }}
        
        .taskbar-item.active {{
            background: rgba(255,255,255,0.3);
        }}
        
        .taskbar-item .icon {{
            font-size: 16px;
        }}
        
        .taskbar-item .title {{
            white-space: nowrap;
            overflow: hidden;
            text-overflow: ellipsis;
        }}
        
        #clock {{
            color: white;
            font-size: 13px;
            padding: 0 12px;
        }}
        
        /* Start Menu */
        #start-menu {{
            position: fixed;
            bottom: 44px;
            left: 8px;
            width: 320px;
            background: rgba(0,0,0,0.9);
            backdrop-filter: blur(20px);
            border-radius: 12px;
            padding: 16px;
            display: none;
            z-index: 10001;
        }}
        
        #start-menu.show {{
            display: block;
        }}
        
        .app-item {{
            display: flex;
            align-items: center;
            gap: 12px;
            padding: 10px;
            color: white;
            border-radius: 8px;
            cursor: pointer;
        }}
        
        .app-item:hover {{
            background: rgba(255,255,255,0.1);
        }}
        
        .app-item .icon {{
            font-size: 24px;
        }}
        
        .app-item .name {{
            font-weight: 500;
        }}
        
        .app-item .desc {{
            margin-left: auto;
            font-size: 12px;
            color: #888;
        }}
        
        .menu-section {{
            margin-bottom: 12px;
            padding-bottom: 12px;
            border-bottom: 1px solid rgba(255,255,255,0.1);
        }}
        
        .menu-title {{
            color: #888;
            font-size: 11px;
            text-transform: uppercase;
            margin-bottom: 8px;
            padding-left: 10px;
        }}
        
        /* Windows */
        .window {{
            position: absolute;
            background: white;
            border-radius: 12px;
            box-shadow: 0 20px 60px rgba(0,0,0,0.3);
            overflow: hidden;
            display: flex;
            flex-direction: column;
            min-width: 400px;
            min-height: 300px;
        }}
        
        .window.minimized {{
            display: none;
        }}
        
        .window.maximized {{
            top: 0 !important;
            left: 0 !important;
            width: 100% !important;
            height: calc(100% - 40px) !important;
            border-radius: 0;
        }}
        
        .window-header {{
            display: flex;
            align-items: center;
            padding: 12px 16px;
            background: #f5f5f5;
            cursor: move;
        }}
        
        .window-icon {{
            font-size: 20px;
            margin-right: 10px;
        }}
        
        .window-title {{
            flex: 1;
            font-weight: 600;
            font-size: 14px;
        }}
        
        .window-controls {{
            display: flex;
            gap: 8px;
        }}
        
        .window-btn {{
            width: 28px;
            height: 28px;
            border: none;
            border-radius: 50%;
            cursor: pointer;
            font-size: 14px;
            display: flex;
            align-items: center;
            justify-content: center;
        }}
        
        .window-btn.minimize {{ background: #ffbd2e; }}
        .window-btn.maximize {{ background: #28c840; }}
        .window-btn.close {{ background: #ff5f57; }}
        
        .window-content {{
            flex: 1;
            overflow: auto;
            padding: 0;
        }}
        
        .window-content iframe {{
            width: 100%;
            height: 100%;
            border: none;
        }}
    </style>
</head>
<body>
    <div id="desktop">
        {}
    </div>
    
    <div id="taskbar">
        <button id="start-btn">🌐 WebbOS</button>
        <div class="taskbar-items">
            {}
        </div>
        <div id="clock">00:00</div>
    </div>
    
    <div id="start-menu">
        <div class="menu-section">
            <div class="menu-title">Applications</div>
            {}
        </div>
        <div class="menu-section">
            <div class="menu-title">System</div>
            <div class="app-item" data-action="settings">
                <span class="icon">⚙️</span>
                <span class="name">Settings</span>
            </div>
            <div class="app-item" data-action="logout">
                <span class="icon">🚪</span>
                <span class="name">Logout</span>
            </div>
        </div>
    </div>
    
    <script>
        // Start menu toggle
        const startBtn = document.getElementById('start-btn');
        const startMenu = document.getElementById('start-menu');
        
        startBtn.addEventListener('click', () => {{
            startMenu.classList.toggle('show');
        }});
        
        // Close start menu when clicking outside
        document.addEventListener('click', (e) => {{
            if (!startBtn.contains(e.target) && !startMenu.contains(e.target)) {{
                startMenu.classList.remove('show');
            }}
        }});
        
        // Launch apps from menu
        document.querySelectorAll('.app-item[data-app]').forEach(item => {{
            item.addEventListener('click', () => {{
                const app = item.dataset.app;
                window.parent.postMessage({{ type: 'launch', app }}, '*');
                startMenu.classList.remove('show');
            }});
        }});
        
        // System actions
        document.querySelectorAll('.app-item[data-action]').forEach(item => {{
            item.addEventListener('click', () => {{
                const action = item.dataset.action;
                if (action === 'logout') {{
                    window.parent.postMessage({{ type: 'logout' }}, '*');
                }}
                startMenu.classList.remove('show');
            }});
        }});
        
        // Clock update
        function updateClock() {{
            const now = new Date();
            document.getElementById('clock').textContent = 
                now.toLocaleTimeString([], {{ hour: '2-digit', minute: '2-digit' }});
        }}
        updateClock();
        setInterval(updateClock, 60000);
    </script>
</body>
</html>"#, desktop_icons, taskbar_items, app_menu_items)
}

// Application HTML/CSS/JS content will be in separate modules
fn get_filemanager_html() -> String {
    String::from(r#"<div class="filemanager">
    <div class="toolbar">
        <button onclick="goUp()">⬆️ Up</button>
        <span id="current-path">/home</span>
    </div>
    <div class="file-list" id="file-list">
        <!-- Files populated by JS -->
    </div>
</div>"#)
}

fn get_filemanager_css() -> String {
    String::from(r#"
.filemanager { height: 100%; display: flex; flex-direction: column; }
.toolbar { padding: 12px; background: #f0f0f0; border-bottom: 1px solid #ddd; display: flex; align-items: center; gap: 12px; }
.toolbar button { padding: 6px 12px; background: white; border: 1px solid #ccc; border-radius: 4px; cursor: pointer; }
.file-list { flex: 1; overflow: auto; padding: 12px; display: grid; grid-template-columns: repeat(auto-fill, minmax(120px, 1fr)); gap: 12px; }
.file-item { text-align: center; padding: 12px; border-radius: 8px; cursor: pointer; }
.file-item:hover { background: #f0f0f0; }
.file-item .icon { font-size: 48px; margin-bottom: 8px; }
.file-item .name { font-size: 12px; word-break: break-all; }
"#)
}

fn get_filemanager_js() -> String {
    String::from(r#"
let currentPath = '/home';
function goUp() {
    const parts = currentPath.split('/');
    parts.pop();
    currentPath = parts.join('/') || '/';
    loadFiles();
}
function loadFiles() {
    // Request file list from kernel
    window.parent.postMessage({ type: 'fs_list', path: currentPath }, '*');
}
// Listen for file list response
window.addEventListener('message', (e) => {
    if (e.data.type === 'fs_list_response') {
        renderFiles(e.data.files);
    }
});
function renderFiles(files) {
    const list = document.getElementById('file-list');
    list.innerHTML = files.map(f => `
        <div class="file-item" data-path="${f.path}">
            <div class="icon">${f.is_dir ? '📁' : '📄'}</div>
            <div class="name">${f.name}</div>
        </div>
    `).join('');
}
loadFiles();
"#)
}

fn get_notepad_html() -> String {
    String::from(r#"<div class="notepad">
    <div class="toolbar">
        <button onclick="newFile()">New</button>
        <button onclick="openFile()">Open</button>
        <button onclick="saveFile()">Save</button>
    </div>
    <textarea id="editor" placeholder="Type here..."></textarea>
</div>"#)
}

fn get_notepad_css() -> String {
    String::from(r#"
.notepad { height: 100%; display: flex; flex-direction: column; }
.toolbar { padding: 8px; background: #f0f0f0; border-bottom: 1px solid #ddd; display: flex; gap: 8px; }
.toolbar button { padding: 6px 16px; background: white; border: 1px solid #ccc; border-radius: 4px; cursor: pointer; }
.toolbar button:hover { background: #e0e0e0; }
#editor { flex: 1; border: none; padding: 16px; font-family: monospace; font-size: 14px; resize: none; outline: none; }
"#)
}

fn get_notepad_js() -> String {
    String::from(r#"
let currentFile = null;
function newFile() {
    document.getElementById('editor').value = '';
    currentFile = null;
    window.parent.postMessage({ type: 'window_title', title: 'Notepad' }, '*');
}
function openFile() {
    window.parent.postMessage({ type: 'dialog_open', filter: '.txt' }, '*');
}
function saveFile() {
    const content = document.getElementById('editor').value;
    if (currentFile) {
        window.parent.postMessage({ type: 'fs_write', path: currentFile, content }, '*');
    } else {
        window.parent.postMessage({ type: 'dialog_save', content }, '*');
    }
}
window.addEventListener('message', (e) => {
    if (e.data.type === 'file_opened') {
        document.getElementById('editor').value = e.data.content;
        currentFile = e.data.path;
        window.parent.postMessage({ type: 'window_title', title: 'Notepad - ' + e.data.name }, '*');
    }
});
"#)
}

fn get_paint_html() -> String {
    String::from(r##"<div class="paint">
    <div class="toolbar">
        <button onclick="setTool('pen')">✏️ Pen</button>
        <button onclick="setTool('eraser')">🧼 Eraser</button>
        <button onclick="clearCanvas()">🗑️ Clear</button>
        <input type="color" id="color" value="#000000" onchange="setColor(this.value)">
        <input type="range" id="size" min="1" max="50" value="5" onchange="setSize(this.value)">
        <button onclick="saveImage()">💾 Save</button>
    </div>
    <canvas id="canvas"></canvas>
</div>"##)
}

fn get_paint_css() -> String {
    String::from(r##"
.paint { height: 100%; display: flex; flex-direction: column; }
.toolbar { padding: 8px; background: #f0f0f0; border-bottom: 1px solid #ddd; display: flex; align-items: center; gap: 12px; }
.toolbar button { padding: 6px 12px; background: white; border: 1px solid #ccc; border-radius: 4px; cursor: pointer; }
#canvas { flex: 1; background: white; cursor: crosshair; }
"##)
}

fn get_paint_js() -> String {
    String::from(r##"
const canvas = document.getElementById('canvas');
const ctx = canvas.getContext('2d');
let drawing = false;
let tool = 'pen';
let color = '#000000';
let size = 5;
function resize() {
    canvas.width = canvas.parentElement.clientWidth;
    canvas.height = canvas.parentElement.clientHeight;
    ctx.lineCap = 'round';
    ctx.lineJoin = 'round';
}
window.addEventListener('resize', resize);
resize();
function startDraw(e) {
    drawing = true;
    draw(e);
}
function stopDraw() {
    drawing = false;
    ctx.beginPath();
}
function draw(e) {
    if (!drawing) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    ctx.lineWidth = size;
    ctx.strokeStyle = tool === 'eraser' ? '#ffffff' : color;
    ctx.lineTo(x, y);
    ctx.stroke();
    ctx.beginPath();
    ctx.moveTo(x, y);
}
canvas.addEventListener('mousedown', startDraw);
canvas.addEventListener('mouseup', stopDraw);
canvas.addEventListener('mousemove', draw);
canvas.addEventListener('mouseout', stopDraw);
function setTool(t) { tool = t; }
function setColor(c) { color = c; }
function setSize(s) { size = s; }
function clearCanvas() { ctx.clearRect(0, 0, canvas.width, canvas.height); }
function saveImage() {
    const dataUrl = canvas.toDataURL();
    window.parent.postMessage({ type: 'save_image', data: dataUrl }, '*');
}
"##)
}

fn get_taskmanager_html() -> String {
    String::from(r#"<div class="taskmanager">
    <div class="stats">
        <div class="stat">
            <div class="stat-value" id="cpu-usage">0%</div>
            <div class="stat-label">CPU Usage</div>
        </div>
        <div class="stat">
            <div class="stat-value" id="mem-usage">0 MB</div>
            <div class="stat-label">Memory Used</div>
        </div>
        <div class="stat">
            <div class="stat-value" id="proc-count">0</div>
            <div class="stat-label">Processes</div>
        </div>
    </div>
    <table class="process-table">
        <thead>
            <tr>
                <th>PID</th>
                <th>Name</th>
                <th>Status</th>
                <th>CPU</th>
                <th>Memory</th>
                <th>Action</th>
            </tr>
        </thead>
        <tbody id="process-list">
        </tbody>
    </table>
</div>"#)
}

fn get_taskmanager_css() -> String {
    String::from(r#"
.taskmanager { height: 100%; overflow: auto; }
.stats { display: flex; gap: 24px; padding: 20px; background: #f5f5f5; border-bottom: 1px solid #ddd; }
.stat { text-align: center; }
.stat-value { font-size: 32px; font-weight: bold; color: #667eea; }
.stat-label { font-size: 12px; color: #666; margin-top: 4px; }
.process-table { width: 100%; border-collapse: collapse; }
.process-table th, .process-table td { padding: 12px; text-align: left; border-bottom: 1px solid #eee; }
.process-table th { background: #f9f9f9; font-weight: 600; }
.process-table tr:hover { background: #f5f5f5; }
.process-table button { padding: 4px 12px; background: #ff5f57; color: white; border: none; border-radius: 4px; cursor: pointer; }
"#)
}

fn get_taskmanager_js() -> String {
    String::from(r#"
function updateStats() {
    window.parent.postMessage({ type: 'get_system_stats' }, '*');
}
function renderProcesses(processes) {
    const tbody = document.getElementById('process-list');
    tbody.innerHTML = processes.map(p => `
        <tr>
            <td>${p.pid}</td>
            <td>${p.name}</td>
            <td>${p.status}</td>
            <td>${p.cpu}%</td>
            <td>${p.memory} KB</td>
            <td><button onclick="killProcess(${p.pid})">End Task</button></td>
        </tr>
    `).join('');
    document.getElementById('proc-count').textContent = processes.length;
}
function killProcess(pid) {
    window.parent.postMessage({ type: 'kill_process', pid }, '*');
}
window.addEventListener('message', (e) => {
    if (e.data.type === 'system_stats') {
        document.getElementById('cpu-usage').textContent = e.data.cpu + '%';
        document.getElementById('mem-usage').textContent = e.data.memory + ' MB';
        renderProcesses(e.data.processes);
    }
});
setInterval(updateStats, 2000);
updateStats();
"#)
}

fn get_usermanager_html() -> String {
    String::from(r#"<div class="usermanager">
    <div class="header">
        <h2>User Accounts</h2>
        <button onclick="showAddUser()">+ Add User</button>
    </div>
    <table class="user-table">
        <thead>
            <tr>
                <th>ID</th>
                <th>Username</th>
                <th>Type</th>
                <th>Status</th>
                <th>Actions</th>
            </tr>
        </thead>
        <tbody id="user-list">
        </tbody>
    </table>
    <div id="add-user-dialog" class="dialog" style="display:none;">
        <h3>Add New User</h3>
        <input type="text" id="new-username" placeholder="Username">
        <input type="password" id="new-password" placeholder="Password">
        <label><input type="checkbox" id="new-is-admin"> Administrator</label>
        <div class="dialog-buttons">
            <button onclick="addUser()">Add</button>
            <button onclick="hideAddUser()">Cancel</button>
        </div>
    </div>
</div>"#)
}

fn get_usermanager_css() -> String {
    String::from(r#"
.usermanager { padding: 20px; }
.header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px; }
.header h2 { margin: 0; }
.header button { padding: 10px 20px; background: #667eea; color: white; border: none; border-radius: 8px; cursor: pointer; }
.user-table { width: 100%; border-collapse: collapse; }
.user-table th, .user-table td { padding: 12px; text-align: left; border-bottom: 1px solid #eee; }
.user-table th { background: #f9f9f9; font-weight: 600; }
.user-table tr:hover { background: #f5f5f5; }
.user-table button { padding: 4px 12px; margin-right: 8px; background: #667eea; color: white; border: none; border-radius: 4px; cursor: pointer; }
.user-table button.delete { background: #ff5f57; }
.dialog { position: fixed; top: 50%; left: 50%; transform: translate(-50%, -50%); background: white; padding: 24px; border-radius: 12px; box-shadow: 0 20px 60px rgba(0,0,0,0.3); z-index: 1000; }
.dialog input { display: block; width: 100%; padding: 10px; margin-bottom: 12px; border: 1px solid #ddd; border-radius: 6px; }
.dialog label { display: block; margin-bottom: 16px; }
.dialog-buttons { display: flex; gap: 12px; }
.dialog-buttons button { flex: 1; padding: 10px; border: none; border-radius: 6px; cursor: pointer; }
.dialog-buttons button:first-child { background: #667eea; color: white; }
.dialog-buttons button:last-child { background: #f0f0f0; }
"#)
}

fn get_usermanager_js() -> String {
    String::from(r#"
function loadUsers() {
    window.parent.postMessage({ type: 'list_users' }, '*');
}
function renderUsers(users) {
    const tbody = document.getElementById('user-list');
    tbody.innerHTML = users.map(u => `
        <tr>
            <td>${u.id}</td>
            <td>${u.username}</td>
            <td>${u.is_admin ? 'Administrator' : 'User'}</td>
            <td>${u.is_active ? 'Active' : 'Inactive'}</td>
            <td>
                <button onclick="toggleUser(${u.id}, ${!u.is_active})">${u.is_active ? 'Deactivate' : 'Activate'}</button>
                <button class="delete" onclick="deleteUser(${u.id})">Delete</button>
            </td>
        </tr>
    `).join('');
}
function showAddUser() {
    document.getElementById('add-user-dialog').style.display = 'block';
}
function hideAddUser() {
    document.getElementById('add-user-dialog').style.display = 'none';
}
function addUser() {
    const username = document.getElementById('new-username').value;
    const password = document.getElementById('new-password').value;
    const isAdmin = document.getElementById('new-is-admin').checked;
    if (username && password) {
        window.parent.postMessage({ type: 'add_user', username, password, is_admin: isAdmin }, '*');
        hideAddUser();
    }
}
function toggleUser(id, active) {
    window.parent.postMessage({ type: 'toggle_user', id, active }, '*');
}
function deleteUser(id) {
    if (confirm('Are you sure you want to delete this user?')) {
        window.parent.postMessage({ type: 'delete_user', id }, '*');
    }
}
window.addEventListener('message', (e) => {
    if (e.data.type === 'users_list') {
        renderUsers(e.data.users);
    }
});
loadUsers();
"#)
}

fn get_terminal_html() -> String {
    String::from(r#"<div class="terminal">
    <div id="output"></div>
    <div class="input-line">
        <span class="prompt">$</span>
        <input type="text" id="input" autofocus autocomplete="off">
    </div>
</div>"#)
}

fn get_terminal_css() -> String {
    String::from(r#"
.terminal { height: 100%; background: #1e1e1e; color: #d4d4d4; font-family: 'Consolas', monospace; font-size: 14px; padding: 12px; overflow-y: auto; display: flex; flex-direction: column; }
#output { flex: 1; white-space: pre-wrap; }
.input-line { display: flex; align-items: center; }
.prompt { color: #667eea; margin-right: 8px; }
#input { flex: 1; background: transparent; border: none; color: #d4d4d4; font-family: inherit; font-size: inherit; outline: none; }
"#)
}

fn get_terminal_js() -> String {
    String::from(r#"
const output = document.getElementById('output');
const input = document.getElementById('input');
const history = [];
let historyIndex = -1;
function println(text) {
    output.textContent += text + '\n';
    output.parentElement.scrollTop = output.parentElement.scrollHeight;
}
input.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
        const cmd = input.value.trim();
        if (cmd) {
            history.push(cmd);
            historyIndex = history.length;
            println('$ ' + cmd);
            window.parent.postMessage({ type: 'terminal_command', command: cmd }, '*');
        }
        input.value = '';
    } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        if (historyIndex > 0) {
            historyIndex--;
            input.value = history[historyIndex];
        }
    } else if (e.key === 'ArrowDown') {
        e.preventDefault();
        if (historyIndex < history.length - 1) {
            historyIndex++;
            input.value = history[historyIndex];
        } else {
            historyIndex = history.length;
            input.value = '';
        }
    }
});
window.addEventListener('message', (e) => {
    if (e.data.type === 'terminal_output') {
        println(e.data.text);
    }
});
window.parent.postMessage({ type: 'terminal_ready' }, '*');
"#)
}

fn get_browser_html() -> String {
    String::from(r#"<div class="browser">
    <div class="toolbar">
        <button onclick="goBack()">◀</button>
        <button onclick="goForward()">▶</button>
        <button onclick="reload()">↻</button>
        <input type="text" id="url-bar" placeholder="Enter URL...">
        <button onclick="navigate()">Go</button>
    </div>
    <iframe id="webview" sandbox="allow-scripts allow-same-origin"></iframe>
</div>"#)
}

fn get_browser_css() -> String {
    String::from(r#"
.browser { height: 100%; display: flex; flex-direction: column; }
.toolbar { padding: 8px; background: #f0f0f0; border-bottom: 1px solid #ddd; display: flex; gap: 8px; }
.toolbar button { padding: 6px 12px; background: white; border: 1px solid #ccc; border-radius: 4px; cursor: pointer; }
#url-bar { flex: 1; padding: 6px 12px; border: 1px solid #ccc; border-radius: 4px; }
#webview { flex: 1; border: none; }
"#)
}

fn get_browser_js() -> String {
    String::from(r#"
const urlBar = document.getElementById('url-bar');
const webview = document.getElementById('webview');
let history = [];
let historyPos = -1;
function navigate() {
    let url = urlBar.value;
    if (!url.match(/^https?:\/\//)) url = 'http://' + url;
    window.parent.postMessage({ type: 'browser_navigate', url }, '*');
}
function goBack() {
    if (historyPos > 0) {
        historyPos--;
        webview.src = history[historyPos];
    }
}
function goForward() {
    if (historyPos < history.length - 1) {
        historyPos++;
        webview.src = history[historyPos];
    }
}
function reload() {
    webview.contentWindow.location.reload();
}
urlBar.addEventListener('keypress', (e) => {
    if (e.key === 'Enter') navigate();
});
window.addEventListener('message', (e) => {
    if (e.data.type === 'browser_content') {
        webview.srcdoc = e.data.html;
        urlBar.value = e.data.url;
    }
});
"#)
}
