//! PWA Registry
//!
//! Manages the collection of installed PWAs:
//! - Loads apps from filesystem (/system/apps/)
//! - Persists registry to filesystem
//! - Handles install/uninstall operations

use super::{PwaApp, PwaManifest, PwaResult, PwaError};
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::format;
use alloc::collections::BTreeMap;

/// Path to system apps directory
const SYSTEM_APPS_PATH: &str = "/system/apps";

/// Path to registry file
const REGISTRY_PATH: &str = "/system/pwa_registry.json";

/// PWA Registry
pub struct PwaRegistry {
    /// Installed apps (key: app_id)
    apps: BTreeMap<String, PwaApp>,
    /// Registry loaded flag
    loaded: bool,
    /// Last app ID counter (for generating IDs)
    next_id: u32,
}

impl PwaRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            apps: BTreeMap::new(),
            loaded: false,
            next_id: 1,
        }
    }
    
    /// Load registry from filesystem
    pub fn load_from_filesystem(&mut self) -> PwaResult<()> {
        if self.loaded {
            return Ok(());
        }
        
        // Scan /system/apps/ directory for installed PWAs
        self.scan_apps_directory()?;
        
        // Also try to load persisted registry for metadata
        self.load_registry_file().ok();
        
        self.loaded = true;
        Ok(())
    }
    
    /// Scan the apps directory for installed PWAs
    fn scan_apps_directory(&mut self) -> PwaResult<()> {
        // In a full implementation, this would use the VFS to read /system/apps/
        // For now, we create the directory structure in memory
        
        // List of expected system apps
        let system_apps = [
            "calculator",
            "notepad", 
            "settings",
            "browser",
        ];
        
        for app_id in system_apps.iter() {
            let app_path = format!("{}/{}", SYSTEM_APPS_PATH, app_id);
            
            // Try to load manifest for this app
            match self.load_app_manifest(&app_path, app_id) {
                Ok(app) => {
                    self.apps.insert(app_id.to_string(), app);
                }
                Err(e) => {
                    // Create a default app entry if manifest not found
                    let app = self.create_default_app(app_id);
                    self.apps.insert(app_id.to_string(), app);
                }
            }
        }
        
        Ok(())
    }
    
    /// Load a single app's manifest
    fn load_app_manifest(&self, app_path: &str, app_id: &str) -> PwaResult<PwaApp> {
        let manifest_path = format!("{}/manifest.json", app_path);
        
        // Read manifest file (using filesystem when available)
        // For now, create default manifests
        let manifest = self.create_default_manifest(app_id)?;
        
        let mut app = PwaApp::new(app_id, manifest);
        app.app_path = app_path.to_string();
        app.is_system = true;
        app.install_date = self.get_timestamp();
        
        Ok(app)
    }
    
    /// Create default manifest for a system app
    fn create_default_manifest(&self, app_id: &str) -> PwaResult<PwaManifest> {
        let (name, description, icon) = match app_id {
            "calculator" => ("Calculator", "Simple calculator for basic math", "icon.png"),
            "notepad" => ("Notepad", "Simple text editor for notes", "icon.png"),
            "settings" => ("Settings", "System settings and configuration", "icon.png"),
            "browser" => ("Browser", "Web browser for surfing the internet", "icon.png"),
            "filemanager" => ("File Manager", "Browse and manage files", "icon.png"),
            "taskmanager" => ("Task Manager", "Monitor system processes", "icon.png"),
            _ => (app_id, "A WebbOS application", "icon.png"),
        };
        
        let mut manifest = PwaManifest::new(name);
        manifest.short_name = name.to_string();
        manifest.description = description.to_string();
        manifest.icon = Some(icon.to_string());
        manifest.is_system = true;
        
        Ok(manifest)
    }
    
    /// Create a default app entry
    fn create_default_app(&self, app_id: &str) -> PwaApp {
        let manifest = self.create_default_manifest(app_id).unwrap_or_else(|_| {
            let mut m = PwaManifest::new(app_id);
            m.is_system = true;
            m
        });
        
        let mut app = PwaApp::new(app_id, manifest);
        app.app_path = format!("{}/{}", SYSTEM_APPS_PATH, app_id);
        app.is_system = true;
        app.install_date = self.get_timestamp();
        
        app
    }
    
    /// Load registry metadata from file
    fn load_registry_file(&mut self) -> PwaResult<()> {
        // This would read /system/pwa_registry.json when filesystem is fully available
        // For now, we just use the scanned apps
        Ok(())
    }
    
    /// Save registry to filesystem
    pub fn save_registry(&self) -> PwaResult<()> {
        // This would write to /system/pwa_registry.json when filesystem is fully available
        // For now, manifests are stored with each app
        Ok(())
    }
    
    /// Get an app by ID
    pub fn get_app(&self, id: &str) -> Option<PwaApp> {
        self.apps.get(id).cloned()
    }
    
    /// List all installed apps
    pub fn list_apps(&self) -> Vec<PwaApp> {
        self.apps.values().cloned().collect()
    }
    
    /// Get app count
    pub fn app_count(&self) -> usize {
        self.apps.len()
    }
    
    /// Register a new app
    pub fn register_app(&mut self, app: PwaApp) -> PwaResult<()> {
        if self.apps.contains_key(&app.id) {
            return Err(PwaError::already_installed(&app.id));
        }
        
        self.apps.insert(app.id.clone(), app);
        self.save_registry()?;
        
        Ok(())
    }
    
    /// Uninstall an app
    pub fn uninstall(&mut self, app_id: &str) -> PwaResult<()> {
        let app = self.apps.get(app_id)
            .ok_or_else(|| PwaError::app_not_found(app_id))?;
        
        if app.is_system {
            return Err(PwaError::invalid_parameter("Cannot uninstall system apps"));
        }
        
        // Remove app directory from filesystem
        // self.remove_app_directory(&app.app_path)?;
        
        // Remove from registry
        self.apps.remove(app_id);
        self.save_registry()?;
        
        Ok(())
    }
    
    /// Install a PWA from a URL
    pub fn install_from_url(&mut self, url: &str, app_id: Option<&str>) -> PwaResult<PwaApp> {
        // Generate app ID from URL if not provided
        let id = match app_id {
            Some(id) => id.to_string(),
            None => self.generate_app_id_from_url(url),
        };
        
        // Check if already installed
        if self.apps.contains_key(&id) {
            return Err(PwaError::already_installed(&id));
        }
        
        // Download and install (when network is fully available)
        // For now, create a placeholder
        let mut app = self.create_default_app(&id);
        app.source_url = Some(url.to_string());
        app.is_system = false;
        
        // Save to filesystem
        self.install_app_files(&app)?;
        
        // Register
        self.register_app(app.clone())?;
        
        Ok(app)
    }
    
    /// Install a PWA from local filesystem
    pub fn install_from_filesystem(&mut self, path: &str, app_id: &str) -> PwaResult<PwaApp> {
        // Check if already installed
        if self.apps.contains_key(app_id) {
            return Err(PwaError::already_installed(app_id));
        }
        
        // Load manifest from source path
        let manifest_path = format!("{}/manifest.json", path);
        let manifest = self.load_manifest_from_path(&manifest_path)?;
        
        let mut app = PwaApp::new(app_id, manifest);
        app.app_path = format!("{}/{}", SYSTEM_APPS_PATH, app_id);
        app.is_system = false;
        app.install_date = self.get_timestamp();
        
        // Copy files to system apps directory
        self.copy_app_files(path, &app.app_path)?;
        
        // Register
        self.register_app(app.clone())?;
        
        Ok(app)
    }
    
    /// Load manifest from a path
    fn load_manifest_from_path(&self, path: &str) -> PwaResult<PwaManifest> {
        // This would read the file using the VFS when available
        // For now, return an error
        Err(PwaError::filesystem_error("Filesystem not fully available"))
    }
    
    /// Copy app files from source to destination
    fn copy_app_files(&self, source: &str, dest: &str) -> PwaResult<()> {
        // This would use the VFS when available
        // For now, just succeed
        Ok(())
    }
    
    /// Install app files to the system directory
    fn install_app_files(&self, app: &PwaApp) -> PwaResult<()> {
        // Create app directory
        // Write manifest.json
        // Copy/write app files
        Ok(())
    }
    
    /// Generate an app ID from a URL
    fn generate_app_id_from_url(&self, url: &str) -> String {
        // Extract domain/path and sanitize
        let sanitized: String = url
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .take(32)
            .collect();
        
        if sanitized.is_empty() {
            format!("app{}", self.next_id)
        } else {
            sanitized.to_lowercase()
        }
    }
    
    /// Get current timestamp
    fn get_timestamp(&self) -> u64 {
        // Use timer ticks when available
        // For now, return 0
        0
    }
    
    /// Create default system apps
    pub fn create_default_apps(&mut self) {
        let default_apps = [
            ("calculator", "Calculator", "Basic calculator"),
            ("notepad", "Notepad", "Simple text editor"),
            ("settings", "Settings", "System settings"),
            ("browser", "Browser", "Web browser"),
        ];
        
        for (id, name, desc) in default_apps.iter() {
            if !self.apps.contains_key(*id) {
                let mut manifest = PwaManifest::new(name);
                manifest.short_name = name.to_string();
                manifest.description = desc.to_string();
                manifest.is_system = true;
                
                let mut app = PwaApp::new(id, manifest);
                app.is_system = true;
                app.app_path = format!("{}/{}", SYSTEM_APPS_PATH, id);
                
                self.apps.insert(id.to_string(), app);
            }
        }
        
        // Save the registry
        let _ = self.save_registry();
    }
    
    /// Update an existing app
    pub fn update_app(&mut self, app: PwaApp) -> PwaResult<()> {
        if !self.apps.contains_key(&app.id) {
            return Err(PwaError::app_not_found(&app.id));
        }
        
        self.apps.insert(app.id.clone(), app);
        self.save_registry()?;
        
        Ok(())
    }
    
    /// Check if an app is installed
    pub fn is_installed(&self, app_id: &str) -> bool {
        self.apps.contains_key(app_id)
    }
    
    /// Search apps by name
    pub fn search(&self, query: &str) -> Vec<PwaApp> {
        let query_lower = query.to_lowercase();
        self.apps
            .values()
            .filter(|app| {
                app.manifest.name.to_lowercase().contains(&query_lower) ||
                app.manifest.description.to_lowercase().contains(&query_lower) ||
                app.id.to_lowercase().contains(&query_lower)
            })
            .cloned()
            .collect()
    }
    
    /// Get apps by category (based on app type)
    pub fn get_apps_by_category(&self, category: &str) -> Vec<PwaApp> {
        // Simple categorization based on app ID/name
        self.apps
            .values()
            .filter(|app| {
                let name_lower = app.manifest.name.to_lowercase();
                match category {
                    "productivity" => {
                        name_lower.contains("note") || 
                        name_lower.contains("calc") ||
                        name_lower.contains("file")
                    }
                    "system" => {
                        name_lower.contains("setting") ||
                        name_lower.contains("task") ||
                        app.is_system
                    }
                    "internet" => {
                        name_lower.contains("browser") ||
                        name_lower.contains("web") ||
                        name_lower.contains("mail")
                    }
                    "media" => {
                        name_lower.contains("music") ||
                        name_lower.contains("video") ||
                        name_lower.contains("photo")
                    }
                    _ => true, // "all" category
                }
            })
            .cloned()
            .collect()
    }
}

impl Default for PwaRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// Global registry instance
use spin::Mutex;
use lazy_static::lazy_static;

lazy_static! {
    static ref REGISTRY: Mutex<PwaRegistry> = Mutex::new(PwaRegistry::new());
}

/// Get access to the global registry
pub fn registry() -> spin::MutexGuard<'static, PwaRegistry> {
    REGISTRY.lock()
}

/// Get an app by ID from the global registry
pub fn get_app(id: &str) -> Option<PwaApp> {
    REGISTRY.lock().get_app(id)
}
