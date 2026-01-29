//! WebbOS Browser Engine
//!
//! A lightweight web browser engine for WebbOS.
//! Supports HTML, CSS, JavaScript, and WebAssembly.

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use spin::Mutex;
use lazy_static::lazy_static;

pub mod html;
pub mod css;
pub mod js;
pub mod wasm;
pub mod layout;
pub mod render;

use crate::println;

/// Browser configuration
pub struct BrowserConfig {
    /// User agent string
    pub user_agent: String,
    /// JavaScript enabled
    pub js_enabled: bool,
    /// WebAssembly enabled
    pub wasm_enabled: bool,
    /// Images enabled
    pub images_enabled: bool,
    /// CSS enabled
    pub css_enabled: bool,
    /// Default viewport width
    pub viewport_width: u32,
    /// Default viewport height
    pub viewport_height: u32,
}

impl BrowserConfig {
    /// Create default browser configuration
    pub fn default() -> Self {
        Self {
            user_agent: String::from("WebbOS/1.0 Browser"),
            js_enabled: true,
            wasm_enabled: true,
            images_enabled: true,
            css_enabled: true,
            viewport_width: 1024,
            viewport_height: 768,
        }
    }
}

/// Browser instance
pub struct Browser {
    /// Browser configuration
    pub config: BrowserConfig,
    /// Current document
    pub document: Option<html::Document>,
    /// Current URL
    pub current_url: String,
    /// Page title
    pub title: String,
    /// Render context
    pub render_context: render::RenderContext,
}

impl Browser {
    /// Create new browser instance
    pub fn new() -> Self {
        Self {
            config: BrowserConfig::default(),
            document: None,
            current_url: String::new(),
            title: String::from("New Tab"),
            render_context: render::RenderContext::new(),
        }
    }

    /// Navigate to URL
    pub fn navigate(&mut self, url: &str) -> Result<(), BrowserError> {
        println!("[browser] Navigating to: {}", url);
        
        // Parse URL
        let parsed_url = Url::parse(url)?;
        
        // Fetch resource
        let content = self.fetch(&parsed_url)?;
        
        // Parse based on content type
        match parsed_url.content_type() {
            ContentType::Html => {
                let document = html::parse(&content)?;
                self.document = Some(document);
                
                // Apply CSS if enabled
                if self.config.css_enabled {
                    self.apply_stylesheets()?;
                }
                
                // Execute JavaScript if enabled
                if self.config.js_enabled {
                    self.execute_scripts()?;
                }
                
                // Layout and render
                self.layout()?;
                self.render()?;
            }
            ContentType::Css => {
                // CSS file - not a document
            }
            ContentType::JavaScript => {
                // JS file - execute it
                if self.config.js_enabled {
                    js::execute(&content)?;
                }
            }
            ContentType::Wasm => {
                // WebAssembly module
                if self.config.wasm_enabled {
                    wasm::load(&content)?;
                }
            }
            _ => {
                return Err(BrowserError::UnsupportedContentType);
            }
        }
        
        self.current_url = String::from(url);
        Ok(())
    }

    /// Fetch resource from URL
    fn fetch(&self, url: &Url) -> Result<Vec<u8>, BrowserError> {
        match url.scheme.as_str() {
            "http" => self.fetch_http(url, false),
            "https" => self.fetch_http(url, true),
            "file" => self.fetch_file(url),
            _ => Err(BrowserError::UnsupportedProtocol),
        }
    }

    /// Fetch via HTTP/HTTPS
    fn fetch_http(&self, url: &Url, _tls: bool) -> Result<Vec<u8>, BrowserError> {
        // Simple HTTP GET implementation
        // For now, just return a basic HTML page
        Ok(Vec::new()) // Placeholder
    }

    /// Fetch local file
    fn fetch_file(&self, _url: &Url) -> Result<Vec<u8>, BrowserError> {
        // File protocol - read from filesystem
        Ok(Vec::new()) // Placeholder
    }

    /// Apply stylesheets to document
    fn apply_stylesheets(&mut self) -> Result<(), BrowserError> {
        if let Some(ref mut doc) = self.document {
            css::apply_styles(doc)?;
        }
        Ok(())
    }

    /// Execute JavaScript in document
    fn execute_scripts(&mut self) -> Result<(), BrowserError> {
        if let Some(ref doc) = self.document {
            for script in &doc.scripts {
                js::execute(&script.content)?;
            }
        }
        Ok(())
    }

    /// Perform layout
    fn layout(&mut self) -> Result<(), BrowserError> {
        if let Some(ref doc) = self.document {
            let tree = layout::layout(doc, self.config.viewport_width, self.config.viewport_height)?;
            self.render_context.layout_tree = Some(tree);
        }
        Ok(())
    }

    /// Render to framebuffer
    fn render(&mut self) -> Result<(), BrowserError> {
        if let Some(ref tree) = self.render_context.layout_tree {
            render::render(tree, &mut self.render_context.framebuffer)?;
        }
        Ok(())
    }
}

/// URL structure
pub struct Url {
    pub scheme: String,
    pub host: String,
    pub port: u16,
    pub path: String,
    pub query: String,
    pub fragment: String,
}

impl Url {
    /// Parse URL string
    pub fn parse(url: &str) -> Result<Self, BrowserError> {
        // Simple URL parsing
        let parts: Vec<&str> = url.split("://").collect();
        if parts.len() != 2 {
            return Err(BrowserError::InvalidUrl);
        }
        
        let scheme = String::from(parts[0]);
        let rest = parts[1];
        
        // Parse host and path
        let (host, path) = if let Some(pos) = rest.find('/') {
            (String::from(&rest[..pos]), String::from(&rest[pos..]))
        } else {
            (String::from(rest), String::from("/"))
        };
        
        // Determine default port
        let port = match scheme.as_str() {
            "http" => 80,
            "https" => 443,
            "ftp" => 21,
            _ => 0,
        };
        
        Ok(Self {
            scheme,
            host,
            port,
            path,
            query: String::new(),
            fragment: String::new(),
        })
    }

    /// Get content type based on extension
    pub fn content_type(&self) -> ContentType {
        if self.path.ends_with(".html") || self.path.ends_with(".htm") {
            ContentType::Html
        } else if self.path.ends_with(".css") {
            ContentType::Css
        } else if self.path.ends_with(".js") {
            ContentType::JavaScript
        } else if self.path.ends_with(".wasm") {
            ContentType::Wasm
        } else {
            ContentType::Html // Default
        }
    }
}

/// Content types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentType {
    Html,
    Css,
    JavaScript,
    Wasm,
    Json,
    Text,
    Image,
    Unknown,
}

/// Browser error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserError {
    Success = 0,
    InvalidUrl = 1,
    NetworkError = 2,
    ParseError = 3,
    UnsupportedProtocol = 4,
    UnsupportedContentType = 5,
    NotFound = 6,
    JsError = 7,
    WasmError = 8,
    Unknown = 255,
}

/// Global browser instance
lazy_static! {
    static ref BROWSER: Mutex<Option<Browser>> = Mutex::new(None);
}

/// Initialize browser engine
pub fn init() {
    println!("[browser] Initializing browser engine...");

    let browser = Browser::new();
    *BROWSER.lock() = Some(browser);

    // Initialize subsystems
    html::init();
    css::init();
    js::init();
    wasm::init();
    layout::init();
    render::init();

    println!("[browser] Browser engine initialized");
}

/// Navigate to URL
pub fn navigate(url: &str) -> Result<(), BrowserError> {
    if let Some(ref mut browser) = *BROWSER.lock() {
        browser.navigate(url)
    } else {
        Err(BrowserError::Unknown)
    }
}

/// Get current page title
pub fn get_title() -> String {
    if let Some(ref browser) = *BROWSER.lock() {
        browser.title.clone()
    } else {
        String::new()
    }
}

/// Print browser statistics
pub fn print_stats() {
    println!("Browser Engine:");
    
    if let Some(ref browser) = *BROWSER.lock() {
        println!("  Current URL: {}", browser.current_url);
        println!("  Title: {}", browser.title);
        println!("  Viewport: {}x{}", browser.config.viewport_width, browser.config.viewport_height);
        println!("  JavaScript: {}", if browser.config.js_enabled { "enabled" } else { "disabled" });
        println!("  WebAssembly: {}", if browser.config.wasm_enabled { "enabled" } else { "disabled" });
        
        if let Some(ref doc) = browser.document {
            println!("  Document elements: {}", doc.element_count());
        }
    } else {
        println!("  Browser not initialized");
    }
}
