//! WebbOS PWA (Progressive Web App) System
//!
//! This module provides PWA support for WebbOS, allowing apps to be:
//! - Installed from URLs or local filesystem
//! - Managed (listed, launched, uninstalled)
//! - Run in the browser with proper isolation
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────┐
//! │  PWA Registry                       │
//! │  - Manages installed PWAs           │
//! │  - Loads/saves from filesystem      │
//! ├─────────────────────────────────────┤
//! │  Manifest Parser                    │
//! │  - Parses manifest.json files       │
//! │  - Validates PWA metadata           │
//! ├─────────────────────────────────────┤
//! │  App Launcher                       │
//! │  - Launches PWAs in browser         │
//! │  - Handles app switching            │
//! ├─────────────────────────────────────┤
//! │  App Store                          │
//! │  - Lists available apps             │
//! │  - Installs from URLs               │
//! └─────────────────────────────────────┘
//! ```

pub mod manifest;
pub mod registry;
pub mod launcher;
pub mod appstore;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::format;
use spin::Mutex;
use lazy_static::lazy_static;

use crate::println;

/// PWA error types
#[derive(Debug, Clone, PartialEq)]
pub enum PwaError {
    /// Invalid manifest
    InvalidManifest(String),
    /// App not found
    AppNotFound(String),
    /// Already installed
    AlreadyInstalled(String),
    /// Installation failed
    InstallFailed(String),
    /// Filesystem error
    FilesystemError(String),
    /// Network error
    NetworkError(String),
    /// Launch failed
    LaunchFailed(String),
    /// Invalid parameter
    InvalidParameter(String),
    /// Registry error
    RegistryError(String),
}

impl PwaError {
    pub fn invalid_manifest(msg: &str) -> Self {
        PwaError::InvalidManifest(msg.to_string())
    }
    
    pub fn app_not_found(id: &str) -> Self {
        PwaError::AppNotFound(id.to_string())
    }
    
    pub fn already_installed(id: &str) -> Self {
        PwaError::AlreadyInstalled(id.to_string())
    }
    
    pub fn install_failed(msg: &str) -> Self {
        PwaError::InstallFailed(msg.to_string())
    }
    
    pub fn filesystem_error(msg: &str) -> Self {
        PwaError::FilesystemError(msg.to_string())
    }
    
    pub fn network_error(msg: &str) -> Self {
        PwaError::NetworkError(msg.to_string())
    }
    
    pub fn launch_failed(msg: &str) -> Self {
        PwaError::LaunchFailed(msg.to_string())
    }
    
    pub fn invalid_parameter(msg: &str) -> Self {
        PwaError::InvalidParameter(msg.to_string())
    }
    
    pub fn registry_error(msg: &str) -> Self {
        PwaError::RegistryError(msg.to_string())
    }
}

/// Result type for PWA operations
pub type PwaResult<T> = Result<T, PwaError>;

/// Display mode for PWAs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayMode {
    /// Standalone app (no browser UI)
    Standalone,
    /// Fullscreen (immersive)
    Fullscreen,
    /// Minimal UI
    MinimalUi,
    /// Browser UI (default)
    Browser,
}

impl DisplayMode {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "standalone" => DisplayMode::Standalone,
            "fullscreen" => DisplayMode::Fullscreen,
            "minimal-ui" | "minimalui" | "minimal" => DisplayMode::MinimalUi,
            "browser" => DisplayMode::Browser,
            _ => DisplayMode::Standalone,
        }
    }
    
    pub fn to_str(&self) -> &'static str {
        match self {
            DisplayMode::Standalone => "standalone",
            DisplayMode::Fullscreen => "fullscreen",
            DisplayMode::MinimalUi => "minimal-ui",
            DisplayMode::Browser => "browser",
        }
    }
}

/// PWA manifest structure
#[derive(Debug, Clone)]
pub struct PwaManifest {
    /// App name (full)
    pub name: String,
    /// Short name (for limited space)
    pub short_name: String,
    /// App description
    pub description: String,
    /// Entry point URL (relative to app root)
    pub start_url: String,
    /// Display mode
    pub display: DisplayMode,
    /// Background color (ARGB)
    pub background_color: u32,
    /// Theme color (ARGB)
    pub theme_color: u32,
    /// Icon path (relative to app root)
    pub icon: Option<String>,
    /// App version
    pub version: String,
    /// App author
    pub author: Option<String>,
    /// Required permissions
    pub permissions: Vec<String>,
    /// Is a system app (pre-installed)
    pub is_system: bool,
}

impl PwaManifest {
    /// Create a new default manifest
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            short_name: name.to_string(),
            description: String::new(),
            start_url: String::from("index.html"),
            display: DisplayMode::Standalone,
            background_color: 0xFFFFFFFF, // White
            theme_color: 0xFF000000,      // Black
            icon: None,
            version: String::from("1.0.0"),
            author: None,
            permissions: Vec::new(),
            is_system: false,
        }
    }
    
    /// Get the best icon path (or default)
    pub fn icon_path(&self) -> &str {
        self.icon.as_deref().unwrap_or("icon.png")
    }
    
    /// Get display title (short name preferred)
    pub fn display_title(&self) -> &str {
        if self.short_name.is_empty() {
            &self.name
        } else {
            &self.short_name
        }
    }
}

/// PWA app information
#[derive(Debug, Clone)]
pub struct PwaApp {
    /// Unique app ID (directory name)
    pub id: String,
    /// App manifest
    pub manifest: PwaManifest,
    /// Installation timestamp (Unix epoch)
    pub install_date: u64,
    /// Source URL (if installed from web)
    pub source_url: Option<String>,
    /// App directory path
    pub app_path: String,
    /// Is system app (pre-installed)
    pub is_system: bool,
    /// Is currently running
    pub is_running: bool,
}

impl PwaApp {
    /// Create a new PWA app
    pub fn new(id: &str, manifest: PwaManifest) -> Self {
        Self {
            id: id.to_string(),
            manifest,
            install_date: 0, // Will be set by registry
            source_url: None,
            app_path: format!("/system/apps/{}", id),
            is_system: false,
            is_running: false,
        }
    }
    
    /// Get the full URL to the app's entry point
    pub fn entry_url(&self) -> String {
        format!("file://{}/{}", self.app_path, self.manifest.start_url)
    }
    
    /// Get the full URL to the app's icon
    pub fn icon_url(&self) -> String {
        format!("file://{}/{}", self.app_path, self.manifest.icon_path())
    }
    
    /// Get the app's display icon (emoji or character)
    pub fn display_icon(&self) -> char {
        // Map app names to appropriate icons (using ASCII characters for no_std compatibility)
        let name_lower = self.manifest.name.to_lowercase();
        match name_lower.as_str() {
            n if n.contains("calc") => '#',
            n if n.contains("note") || n.contains("text") || n.contains("edit") => 'N',
            n if n.contains("paint") || n.contains("draw") || n.contains("art") => 'P',
            n if n.contains("file") || n.contains("folder") => 'F',
            n if n.contains("setting") || n.contains("config") => 'S',
            n if n.contains("browser") || n.contains("web") => 'W',
            n if n.contains("mail") || n.contains("email") => '@',
            n if n.contains("music") || n.contains("audio") || n.contains("sound") => 'M',
            n if n.contains("video") || n.contains("movie") => 'V',
            n if n.contains("photo") || n.contains("image") || n.contains("pic") => 'I',
            n if n.contains("game") || n.contains("play") => 'G',
            n if n.contains("chat") || n.contains("message") => 'C',
            n if n.contains("calendar") || n.contains("date") => 'D',
            n if n.contains("clock") || n.contains("time") => 'T',
            n if n.contains("weather") => '*',
            n if n.contains("map") || n.contains("nav") => '&',
            n if n.contains("shop") || n.contains("store") => '$',
            n if n.contains("terminal") || n.contains("console") || n.contains("shell") => '>',
            n if n.contains("code") || n.contains("dev") || n.contains("program") => '{',
            _ => 'A', // Default app icon
        }
    }
}

/// Global PWA registry
lazy_static! {
    static ref PWA_REGISTRY: Mutex<registry::PwaRegistry> = Mutex::new(registry::PwaRegistry::new());
}

/// Initialize the PWA subsystem
pub fn init() {
    println!("[pwa] Initializing PWA subsystem...");
    
    // Initialize registry
    {
        let mut registry = PWA_REGISTRY.lock();
        if let Err(e) = registry.load_from_filesystem() {
            println!("[pwa] Could not load registry: {:?}", e);
            println!("[pwa] Creating new registry with default apps");
            registry.create_default_apps();
        }
        
        let app_count = registry.app_count();
        println!("[pwa] Registry loaded with {} apps", app_count);
    }
    
    // Initialize subsystems
    launcher::init();
    appstore::init();
    
    println!("[pwa] PWA subsystem initialized");
}

/// Get the global registry
pub fn registry() -> spin::MutexGuard<'static, registry::PwaRegistry> {
    PWA_REGISTRY.lock()
}

/// List all installed PWAs
pub fn list_apps() -> Vec<PwaApp> {
    PWA_REGISTRY.lock().list_apps()
}

/// Get a specific PWA by ID
pub fn get_app(id: &str) -> Option<PwaApp> {
    PWA_REGISTRY.lock().get_app(id)
}

/// Install a PWA from a URL
pub fn install_from_url(url: &str, app_id: Option<&str>) -> PwaResult<PwaApp> {
    PWA_REGISTRY.lock().install_from_url(url, app_id)
}

/// Install a PWA from local filesystem
pub fn install_from_filesystem(path: &str, app_id: &str) -> PwaResult<PwaApp> {
    PWA_REGISTRY.lock().install_from_filesystem(path, app_id)
}

/// Uninstall a PWA
pub fn uninstall(app_id: &str) -> PwaResult<()> {
    PWA_REGISTRY.lock().uninstall(app_id)
}

/// Launch a PWA
pub fn launch_app(app_id: &str) -> PwaResult<()> {
    launcher::launch_app(app_id)
}

/// Get app count
pub fn app_count() -> usize {
    PWA_REGISTRY.lock().app_count()
}

/// Print PWA statistics
pub fn print_stats() {
    let registry = PWA_REGISTRY.lock();
    
    println!("\nPWA System:");
    println!("  Installed apps: {}", registry.app_count());
    
    for app in registry.list_apps() {
        let source = app.source_url.as_deref().unwrap_or("system");
        println!("  [{}] {} - v{} ({})", 
            app.id, 
            app.manifest.name,
            app.manifest.version,
            source
        );
    }
}

/// Create default system apps (if they don't exist)
pub fn ensure_system_apps() {
    let mut registry = PWA_REGISTRY.lock();
    
    // Define default system apps
    let default_apps = [
        ("calculator", "Calculator", "Simple calculator app", "🧮"),
        ("notepad", "Notepad", "Simple text editor", "📝"),
        ("settings", "Settings", "System settings", "⚙"),
    ];
    
    for (id, name, desc, _icon) in default_apps.iter() {
        if registry.get_app(id).is_none() {
            println!("[pwa] Creating default app: {}", id);
            
            let mut manifest = PwaManifest::new(name);
            manifest.short_name = name.to_string();
            manifest.description = desc.to_string();
            manifest.is_system = true;
            
            let mut app = PwaApp::new(id, manifest);
            app.is_system = true;
            app.app_path = format!("/system/apps/{}", id);
            
            // Try to add to registry
            if let Err(e) = registry.register_app(app) {
                println!("[pwa] Failed to register {}: {:?}", id, e);
            }
        }
    }
}
