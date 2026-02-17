//! PWA App Store
//!
//! Simple app store for discovering and installing PWAs:
//! - Lists available apps from a repository
//! - Provides install buttons for apps
//! - Handles app updates

use super::{PwaApp, PwaManifest, PwaResult, PwaError, registry};
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::vec;
use alloc::format;
use alloc::collections::BTreeMap;

/// Default app store repository URL
const DEFAULT_REPOSITORY: &str = "https://apps.webbos.local";

/// Path to local app store
const APPSTORE_PATH: &str = "/system/appstore";

/// Available app info from repository
#[derive(Debug, Clone)]
pub struct StoreApp {
    /// App ID
    pub id: String,
    /// App name
    pub name: String,
    /// Description
    pub description: String,
    /// Author
    pub author: String,
    /// Version
    pub version: String,
    /// Icon URL
    pub icon_url: String,
    /// Download URL
    pub download_url: String,
    /// Size in bytes
    pub size: u64,
    /// Category
    pub category: String,
    /// Is installed
    pub is_installed: bool,
    /// Installed version (if installed)
    pub installed_version: Option<String>,
}

/// App Store
pub struct AppStore {
    /// Repository URL
    repository: String,
    /// Cached available apps
    available_apps: Vec<StoreApp>,
    /// Categories
    categories: Vec<String>,
    /// Last update timestamp
    last_update: u64,
}

impl AppStore {
    /// Create a new app store
    pub fn new() -> Self {
        let mut categories = Vec::new();
        categories.push("all".to_string());
        categories.push("productivity".to_string());
        categories.push("internet".to_string());
        categories.push("media".to_string());
        categories.push("system".to_string());
        categories.push("games".to_string());
        
        Self {
            repository: DEFAULT_REPOSITORY.to_string(),
            available_apps: Vec::new(),
            categories,
            last_update: 0,
        }
    }
    
    /// Initialize with default apps
    pub fn init(&mut self) {
        // Populate with built-in available apps
        self.available_apps = self.get_builtin_apps();
        self.update_install_status();
    }
    
    /// Refresh available apps from repository
    pub fn refresh(&mut self) -> PwaResult<()> {
        // In a full implementation, this would fetch from the network
        // For now, use built-in apps
        self.available_apps = self.get_builtin_apps();
        self.update_install_status();
        self.last_update = self.get_timestamp();
        
        Ok(())
    }
    
    /// Get built-in available apps
    fn get_builtin_apps(&self) -> Vec<StoreApp> {
        vec![
            StoreApp {
                id: "calculator".to_string(),
                name: "Calculator".to_string(),
                description: "Simple calculator for basic math operations".to_string(),
                author: "WebbOS".to_string(),
                version: "1.0.0".to_string(),
                icon_url: "/system/appstore/icons/calculator.png".to_string(),
                download_url: "/system/appstore/apps/calculator.zip".to_string(),
                size: 10240,
                category: "productivity".to_string(),
                is_installed: false,
                installed_version: None,
            },
            StoreApp {
                id: "notepad".to_string(),
                name: "Notepad".to_string(),
                description: "Simple text editor for taking notes".to_string(),
                author: "WebbOS".to_string(),
                version: "1.0.0".to_string(),
                icon_url: "/system/appstore/icons/notepad.png".to_string(),
                download_url: "/system/appstore/apps/notepad.zip".to_string(),
                size: 15360,
                category: "productivity".to_string(),
                is_installed: false,
                installed_version: None,
            },
            StoreApp {
                id: "paint".to_string(),
                name: "Paint".to_string(),
                description: "Draw and edit images".to_string(),
                author: "WebbOS".to_string(),
                version: "1.0.0".to_string(),
                icon_url: "/system/appstore/icons/paint.png".to_string(),
                download_url: "/system/appstore/apps/paint.zip".to_string(),
                size: 25600,
                category: "media".to_string(),
                is_installed: false,
                installed_version: None,
            },
            StoreApp {
                id: "music".to_string(),
                name: "Music Player".to_string(),
                description: "Play audio files and manage playlists".to_string(),
                author: "WebbOS".to_string(),
                version: "1.0.0".to_string(),
                icon_url: "/system/appstore/icons/music.png".to_string(),
                download_url: "/system/appstore/apps/music.zip".to_string(),
                size: 51200,
                category: "media".to_string(),
                is_installed: false,
                installed_version: None,
            },
            StoreApp {
                id: "weather".to_string(),
                name: "Weather".to_string(),
                description: "Local weather forecasts".to_string(),
                author: "WebbOS".to_string(),
                version: "1.0.0".to_string(),
                icon_url: "/system/appstore/icons/weather.png".to_string(),
                download_url: "/system/appstore/apps/weather.zip".to_string(),
                size: 30720,
                category: "internet".to_string(),
                is_installed: false,
                installed_version: None,
            },
            StoreApp {
                id: "todo".to_string(),
                name: "Todo List".to_string(),
                description: "Manage your tasks and to-do lists".to_string(),
                author: "WebbOS".to_string(),
                version: "1.0.0".to_string(),
                icon_url: "/system/appstore/icons/todo.png".to_string(),
                download_url: "/system/appstore/apps/todo.zip".to_string(),
                size: 20480,
                category: "productivity".to_string(),
                is_installed: false,
                installed_version: None,
            },
            StoreApp {
                id: "terminal".to_string(),
                name: "Terminal".to_string(),
                description: "Command line interface for WebbOS".to_string(),
                author: "WebbOS".to_string(),
                version: "1.0.0".to_string(),
                icon_url: "/system/appstore/icons/terminal.png".to_string(),
                download_url: "/system/appstore/apps/terminal.zip".to_string(),
                size: 40960,
                category: "system".to_string(),
                is_installed: false,
                installed_version: None,
            },
        ]
    }
    
    /// Update install status for all available apps
    fn update_install_status(&mut self) {
        let registry = super::registry();
        
        for app in &mut self.available_apps {
            if let Some(installed) = registry.get_app(&app.id) {
                app.is_installed = true;
                app.installed_version = Some(installed.manifest.version.clone());
            } else {
                app.is_installed = false;
                app.installed_version = None;
            }
        }
    }
    
    /// List available apps
    pub fn list_available(&self, category: Option<&str>) -> Vec<StoreApp> {
        match category {
            Some(cat) if cat != "all" => {
                self.available_apps.iter()
                    .filter(|a| a.category == cat)
                    .cloned()
                    .collect()
            }
            _ => self.available_apps.clone(),
        }
    }
    
    /// Search available apps
    pub fn search(&self, query: &str) -> Vec<StoreApp> {
        let query_lower = query.to_lowercase();
        self.available_apps.iter()
            .filter(|a| {
                a.name.to_lowercase().contains(&query_lower) ||
                a.description.to_lowercase().contains(&query_lower) ||
                a.id.to_lowercase().contains(&query_lower)
            })
            .cloned()
            .collect()
    }
    
    /// Get app details
    pub fn get_app(&self, app_id: &str) -> Option<StoreApp> {
        self.available_apps.iter()
            .find(|a| a.id == app_id)
            .cloned()
    }
    
    /// Install an app
    pub fn install(&mut self, app_id: &str) -> PwaResult<PwaApp> {
        // Find the app in available list
        let store_app = self.get_app(app_id)
            .ok_or_else(|| PwaError::app_not_found(app_id))?;
        
        if store_app.is_installed {
            return Err(PwaError::already_installed(app_id));
        }
        
        // Download and install
        let app = self.download_and_install(&store_app)?;
        
        // Update status
        self.update_install_status();
        
        crate::println!("[appstore] Installed {} v{}", app.manifest.name, app.manifest.version);
        
        Ok(app)
    }
    
    /// Download and install an app
    fn download_and_install(&self, store_app: &StoreApp) -> PwaResult<PwaApp> {
        // In a full implementation, this would:
        // 1. Download the app package from store_app.download_url
        // 2. Extract it to /system/apps/{app_id}/
        // 3. Parse the manifest
        // 4. Register the app
        
        // For now, create a default app
        let mut manifest = PwaManifest::new(&store_app.name);
        manifest.short_name = store_app.name.clone();
        manifest.description = store_app.description.clone();
        manifest.version = store_app.version.clone();
        manifest.author = Some(store_app.author.clone());
        
        let mut app = PwaApp::new(&store_app.id, manifest);
        app.source_url = Some(store_app.download_url.clone());
        app.app_path = format!("/system/apps/{}", store_app.id);
        app.is_system = false;
        
        // Register with registry
        super::registry().register_app(app.clone())?;
        
        Ok(app)
    }
    
    /// Uninstall an app
    pub fn uninstall(&mut self, app_id: &str) -> PwaResult<()> {
        super::registry().uninstall(app_id)?;
        self.update_install_status();
        
        crate::println!("[appstore] Uninstalled {}", app_id);
        
        Ok(())
    }
    
    /// Check for updates
    pub fn check_updates(&self) -> Vec<(String, String, String)> {
        // Returns list of (app_id, installed_version, available_version) for apps with updates
        let mut updates = Vec::new();
        
        let registry = super::registry();
        
        for store_app in &self.available_apps {
            if let Some(installed) = registry.get_app(&store_app.id) {
                if installed.manifest.version != store_app.version {
                    updates.push((
                        store_app.id.clone(),
                        installed.manifest.version.clone(),
                        store_app.version.clone(),
                    ));
                }
            }
        }
        
        updates
    }
    
    /// Update an app
    pub fn update(&mut self, app_id: &str) -> PwaResult<PwaApp> {
        // Uninstall old version
        self.uninstall(app_id)?;
        
        // Install new version
        self.install(app_id)
    }
    
    /// Get categories
    pub fn categories(&self) -> &[String] {
        &self.categories
    }
    
    /// Get current timestamp
    fn get_timestamp(&self) -> u64 {
        crate::arch::interrupts::get_timer_ticks()
    }
    
    /// Generate HTML for app store page
    pub fn generate_html(&self) -> String {
        let mut html = String::from(r#"<!DOCTYPE html>
<html>
<head>
    <title>WebbOS App Store</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: linear-gradient(135deg, #1a1a2e 0%, #16213e 100%);
            min-height: 100vh;
            color: white;
            padding: 20px;
        }
        .header {
            text-align: center;
            padding: 20px 0;
        }
        .header h1 {
            font-size: 36px;
            margin-bottom: 10px;
        }
        .header p {
            color: #888;
        }
        .categories {
            display: flex;
            justify-content: center;
            gap: 10px;
            padding: 20px 0;
            flex-wrap: wrap;
        }
        .category-btn {
            padding: 8px 16px;
            background: rgba(255,255,255,0.1);
            border: 1px solid rgba(255,255,255,0.2);
            border-radius: 20px;
            color: white;
            cursor: pointer;
            transition: all 0.2s;
        }
        .category-btn:hover, .category-btn.active {
            background: #667eea;
            border-color: #667eea;
        }
        .apps-grid {
            display: grid;
            grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
            gap: 20px;
            padding: 20px;
            max-width: 1200px;
            margin: 0 auto;
        }
        .app-card {
            background: rgba(255,255,255,0.05);
            border-radius: 16px;
            padding: 20px;
            transition: transform 0.2s, box-shadow 0.2s;
        }
        .app-card:hover {
            transform: translateY(-4px);
            box-shadow: 0 10px 30px rgba(0,0,0,0.3);
        }
        .app-icon {
            width: 64px;
            height: 64px;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            border-radius: 16px;
            display: flex;
            align-items: center;
            justify-content: center;
            font-size: 32px;
            margin-bottom: 12px;
        }
        .app-name {
            font-size: 18px;
            font-weight: 600;
            margin-bottom: 4px;
        }
        .app-desc {
            font-size: 14px;
            color: #888;
            margin-bottom: 12px;
            line-height: 1.4;
        }
        .app-meta {
            display: flex;
            justify-content: space-between;
            align-items: center;
            font-size: 12px;
            color: #666;
            margin-bottom: 12px;
        }
        .install-btn {
            width: 100%;
            padding: 10px;
            background: #667eea;
            border: none;
            border-radius: 8px;
            color: white;
            font-weight: 500;
            cursor: pointer;
            transition: background 0.2s;
        }
        .install-btn:hover {
            background: #5a6fd6;
        }
        .install-btn.installed {
            background: #27c93f;
            cursor: default;
        }
        .install-btn.update {
            background: #ffbd2e;
        }
    </style>
</head>
<body>
    <div class="header">
        <h1>🛒 WebbOS App Store</h1>
        <p>Discover and install amazing apps for WebbOS</p>
    </div>
    
    <div class="categories">
        <button class="category-btn active" onclick="filterCategory('all')">All</button>
        <button class="category-btn" onclick="filterCategory('productivity')">Productivity</button>
        <button class="category-btn" onclick="filterCategory('internet')">Internet</button>
        <button class="category-btn" onclick="filterCategory('media')">Media</button>
        <button class="category-btn" onclick="filterCategory('system')">System</button>
        <button class="category-btn" onclick="filterCategory('games')">Games</button>
    </div>
    
    <div class="apps-grid">
"#);

        // Add app cards
        for app in &self.available_apps {
            let icon_char = self.get_icon_char(&app.id);
            let btn_class = if app.is_installed {
                if self.has_update(&app.id) {
                    "install-btn update"
                } else {
                    "install-btn installed"
                }
            } else {
                "install-btn"
            };
            
            let btn_text = if app.is_installed {
                if self.has_update(&app.id) {
                    "Update"
                } else {
                    "Installed"
                }
            } else {
                "Install"
            };
            
            html.push_str(&format!(r#"
        <div class="app-card" data-category="{}">
            <div class="app-icon">{}</div>
            <div class="app-name">{}</div>
            <div class="app-desc">{}</div>
            <div class="app-meta">
                <span>v{}</span>
                <span>{}</span>
            </div>
            <button class="{}" onclick="installApp('{}')" {}>{}</button>
        </div>
"#, 
                app.category,
                icon_char,
                app.name,
                app.description,
                app.version,
                app.author,
                btn_class,
                app.id,
                if app.is_installed && !self.has_update(&app.id) { "disabled" } else { "" },
                btn_text
            ));
        }
        
        html.push_str(r#"
    </div>
    
    <script>
        function filterCategory(cat) {
            // Update active button
            document.querySelectorAll('.category-btn').forEach(btn => {
                btn.classList.remove('active');
            });
            event.target.classList.add('active');
            
            // Filter apps
            document.querySelectorAll('.app-card').forEach(card => {
                if (cat === 'all' || card.dataset.category === cat) {
                    card.style.display = 'block';
                } else {
                    card.style.display = 'none';
                }
            });
        }
        
        function installApp(appId) {
            window.parent.postMessage({ type: 'installApp', appId: appId }, '*');
        }
    </script>
</body>
</html>
"#);
        
        html
    }
    
    /// Get icon character for app
    fn get_icon_char(&self, app_id: &str) -> char {
        match app_id {
            "calculator" => '🧮',
            "notepad" => '📝',
            "paint" => '🎨',
            "music" => '🎵',
            "weather" => '🌤',
            "todo" => '✓',
            "terminal" => '💻',
            _ => '📱',
        }
    }
    
    /// Check if app has update available
    fn has_update(&self, app_id: &str) -> bool {
        self.check_updates().iter().any(|(id, _, _)| id == app_id)
    }
}

impl Default for AppStore {
    fn default() -> Self {
        Self::new()
    }
}

// Global app store instance
use spin::Mutex;
use lazy_static::lazy_static;

lazy_static! {
    static ref APPSTORE: Mutex<AppStore> = Mutex::new(AppStore::new());
}

/// Initialize the app store
pub fn init() {
    crate::println!("[pwa] Initializing app store...");
    APPSTORE.lock().init();
    crate::println!("[pwa] App store initialized");
}

/// Get the app store HTML
pub fn get_html() -> String {
    APPSTORE.lock().generate_html()
}

/// List available apps
pub fn list_available(category: Option<&str>) -> Vec<StoreApp> {
    APPSTORE.lock().list_available(category)
}

/// Install an app
pub fn install(app_id: &str) -> PwaResult<PwaApp> {
    APPSTORE.lock().install(app_id)
}

/// Uninstall an app
pub fn uninstall(app_id: &str) -> PwaResult<()> {
    APPSTORE.lock().uninstall(app_id)
}

/// Search apps
pub fn search(query: &str) -> Vec<StoreApp> {
    APPSTORE.lock().search(query)
}

/// Check for updates
pub fn check_updates() -> Vec<(String, String, String)> {
    APPSTORE.lock().check_updates()
}

/// Update an app
pub fn update(app_id: &str) -> PwaResult<PwaApp> {
    APPSTORE.lock().update(app_id)
}

/// Refresh app list
pub fn refresh() -> PwaResult<()> {
    APPSTORE.lock().refresh()
}

/// Get app store categories
pub fn categories() -> Vec<String> {
    APPSTORE.lock().categories().to_vec()
}
