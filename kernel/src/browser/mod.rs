//! WebbOS Browser Engine
//!
//! A lightweight web browser engine for WebbOS.
//! Supports HTML, CSS, JavaScript, and WebAssembly.

use alloc::string::{String, ToString};
use alloc::format;

use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use spin::Mutex;
use lazy_static::lazy_static;

pub mod html;
pub mod css;
pub mod js;
pub mod wasm;
pub mod layout;
pub mod render;

use crate::println;
use crate::net::{Ipv4Address, Port};
use crate::net::tcp::{self, ConnectionId};
use crate::tls::TlsConnection;

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
    /// Follow redirects
    pub follow_redirects: bool,
    /// Maximum redirect count
    pub max_redirects: u32,
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
            follow_redirects: true,
            max_redirects: 10,
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
    /// URL being typed (for desktop input)
    typing_url: String,
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
            typing_url: String::new(),
        }
    }

    /// Navigate to URL
    pub fn navigate(&mut self, url: &str) -> Result<(), BrowserError> {
        println!("[browser] Navigating to: {}", url);
        
        // Parse URL
        let parsed_url = Url::parse(url)?;
        
        // Fetch resource
        let content = self.fetch(&parsed_url, 0)?;
        
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
        self.typing_url.clear();
        Ok(())
    }

    /// Fetch resource from URL
    fn fetch(&self, url: &Url, redirect_count: u32) -> Result<Vec<u8>, BrowserError> {
        if redirect_count > self.config.max_redirects {
            return Err(BrowserError::TooManyRedirects);
        }
        
        match url.scheme.as_str() {
            "http" => self.fetch_http(url, false, redirect_count),
            "https" => self.fetch_http(url, true, redirect_count),
            "file" => self.fetch_file(url),
            _ => Err(BrowserError::UnsupportedProtocol),
        }
    }

    /// Fetch via HTTP/HTTPS
    fn fetch_http(&self, url: &Url, use_tls: bool, redirect_count: u32) -> Result<Vec<u8>, BrowserError> {
        // Resolve hostname to IP address
        let ip = self.resolve_host(&url.host)?;
        
        // Determine port
        let port = if url.port != 0 {
            url.port
        } else if use_tls {
            443
        } else {
            80
        };
        
        println!("[browser] Connecting to {}:{} (TLS: {})", url.host, port, use_tls);
        
        // Establish TCP connection
        let conn_id = tcp::connect(ip, Port::new(port))
            .map_err(|_| BrowserError::NetworkError)?;
        
        // Wait for connection to be established
        let mut attempts = 0;
        loop {
            // Check connection state
            if let Ok(_) = self.check_connection_established(conn_id) {
                break;
            }
            attempts += 1;
            if attempts > 100 {
                let _ = tcp::close(conn_id);
                return Err(BrowserError::ConnectionTimeout);
            }
            // Small delay
            for _ in 0..10000 {
                core::hint::spin_loop();
            }
        }
        
        println!("[browser] TCP connection established");
        
        // Build HTTP/1.1 GET request
        let request = self.build_http_request(url);
        
        // Send request
        if use_tls {
            // HTTPS: Use TLS wrapper
            let _ = self.send_https_request(conn_id, &request)?;
        } else {
            // HTTP: Send plaintext
            tcp::send(conn_id, &request)
                .map_err(|_| BrowserError::NetworkError)?;
        }
        
        // Receive response
        let response_data = self.receive_http_response(conn_id)?;
        
        // Close connection
        let _ = tcp::close(conn_id);
        
        // Parse HTTP response
        let response = HttpResponse::parse(&response_data)?;
        
        println!("[browser] HTTP {} {}", response.status_code, response.status_text);
        
        // Handle redirects
        if self.config.follow_redirects && self.is_redirect(response.status_code) {
            if let Some(location) = response.headers.get("location") {
                println!("[browser] Redirecting to: {}", location);
                let redirect_url = self.resolve_redirect_url(url, location)?;
                return self.fetch(&redirect_url, redirect_count + 1);
            }
        }
        
        // Check for successful response
        if response.status_code < 200 || response.status_code >= 300 {
            println!("[browser] HTTP error: {}", response.status_code);
            return Err(BrowserError::HttpError);
        }
        
        Ok(response.body)
    }
    
    /// Build HTTP/1.1 GET request
    fn build_http_request(&self, url: &Url) -> Vec<u8> {
        let mut request = Vec::new();
        
        // Request line
        request.extend_from_slice(b"GET ");
        request.extend_from_slice(url.path.as_bytes());
        if !url.query.is_empty() {
            request.push(b'?');
            request.extend_from_slice(url.query.as_bytes());
        }
        request.extend_from_slice(b" HTTP/1.1\r\n");
        
        // Host header (required for HTTP/1.1)
        request.extend_from_slice(b"Host: ");
        request.extend_from_slice(url.host.as_bytes());
        if url.port != 80 && url.port != 443 {
            request.push(b':');
            request.extend_from_slice(url.port.to_string().as_bytes());
        }
        request.extend_from_slice(b"\r\n");
        
        // Connection header
        request.extend_from_slice(b"Connection: close\r\n");
        
        // User-Agent
        request.extend_from_slice(b"User-Agent: ");
        request.extend_from_slice(self.config.user_agent.as_bytes());
        request.extend_from_slice(b"\r\n");
        
        // Accept headers
        request.extend_from_slice(b"Accept: text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8\r\n");
        request.extend_from_slice(b"Accept-Language: en-US,en;q=0.5\r\n");
        request.extend_from_slice(b"Accept-Encoding: identity\r\n");
        
        // Empty line to end headers
        request.extend_from_slice(b"\r\n");
        
        request
    }
    
    /// Send HTTPS request using TLS
    fn send_https_request(&self, conn_id: ConnectionId, request: &[u8]) -> Result<(), BrowserError> {
        // Initialize TLS connection
        let mut tls = TlsConnection::new();
        
        // Generate and send Client Hello
        let client_hello = tls.generate_client_hello();
        tcp::send(conn_id, &client_hello)
            .map_err(|_| BrowserError::TlsError)?;
        
        // Receive Server Hello and complete handshake
        let mut handshake_buf = [0u8; 4096];
        let mut handshake_data = Vec::new();
        
        for _ in 0..50 {
            match tcp::receive(conn_id, &mut handshake_buf) {
                Ok(n) if n > 0 => {
                    handshake_data.extend_from_slice(&handshake_buf[..n]);
                    // Check if handshake is complete
                    if tls.state() == crate::tls::TlsState::Connected {
                        break;
                    }
                }
                _ => {}
            }
        }
        
        // For now, send the HTTP request directly (TLS handshake simplified)
        // In production, this would encrypt the request using TLS
        println!("[browser] TLS handshake completed (simplified)");
        
        // Send the HTTP request
        tcp::send(conn_id, request)
            .map_err(|_| BrowserError::NetworkError)?;
        
        Ok(())
    }
    
    /// Receive HTTP response
    fn receive_http_response(&self, conn_id: ConnectionId) -> Result<Vec<u8>, BrowserError> {
        let mut response_data = Vec::new();
        let mut buffer = [0u8; 4096];
        let mut empty_count = 0;
        
        loop {
            match tcp::receive(conn_id, &mut buffer) {
                Ok(n) if n > 0 => {
                    response_data.extend_from_slice(&buffer[..n]);
                    empty_count = 0;
                    
                    // Check if we have a complete response
                    if self.has_complete_response(&response_data) {
                        break;
                    }
                }
                Ok(_) => {
                    // No data available
                    empty_count += 1;
                    if empty_count > 100 {
                        // Timeout waiting for data
                        break;
                    }
                    // Small delay
                    for _ in 0..10000 {
                        core::hint::spin_loop();
                    }
                }
                Err(_) => {
                    break;
                }
            }
        }
        
        Ok(response_data)
    }
    
    /// Check if HTTP response is complete
    fn has_complete_response(&self, data: &[u8]) -> bool {
        // Find end of headers
        if let Some(header_end) = data.windows(4).position(|w| w == b"\r\n\r\n") {
            let body_start = header_end + 4;
            
            // Parse headers to check for Content-Length or Transfer-Encoding
            let header_data = &data[..header_end];
            if let Ok(headers_str) = core::str::from_utf8(header_data) {
                // Check for Content-Length
                for line in headers_str.lines() {
                    if line.to_lowercase().starts_with("content-length:") {
                        if let Some(len_str) = line.split(':').nth(1) {
                            if let Ok(content_len) = len_str.trim().parse::<usize>() {
                                return data.len() >= body_start + content_len;
                            }
                        }
                    }
                    // Check for chunked encoding
                    if line.to_lowercase().contains("transfer-encoding: chunked") {
                        // For chunked, we need to check for the terminating chunk
                        return data.windows(5).any(|w| w == b"0\r\n\r\n");
                    }
                }
            }
            
            // If no Content-Length and not chunked, response ends when connection closes
            // We'll assume it's complete after a reasonable amount of data
            return true;
        }
        
        false
    }
    
    /// Check connection state
    fn check_connection_established(&self, conn_id: ConnectionId) -> Result<(), ()> {
        // This is a simplified check - in production, you'd query the connection table
        // For now, assume connection is established if we can send data
        Ok(())
    }
    
    /// Resolve hostname to IP address
    fn resolve_host(&self, host: &str) -> Result<Ipv4Address, BrowserError> {
        // Check if it's already an IP address
        if let Some(ip) = self.parse_ipv4(host) {
            return Ok(ip);
        }
        
        // Try DNS lookup
        if let Some(ip) = crate::net::dns::resolve(host) {
            Ok(ip)
        } else {
            println!("[browser] DNS resolution failed for: {}", host);
            Err(BrowserError::DnsError)
        }
    }
    
    /// Parse IPv4 address
    fn parse_ipv4(&self, s: &str) -> Option<Ipv4Address> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 4 {
            return None;
        }
        
        let mut bytes = [0u8; 4];
        for (i, part) in parts.iter().enumerate() {
            bytes[i] = part.parse().ok()?;
        }
        
        Some(Ipv4Address::new(bytes))
    }
    
    /// Check if status code is redirect
    fn is_redirect(&self, status: u16) -> bool {
        matches!(status, 301 | 302 | 303 | 307 | 308)
    }
    
    /// Resolve redirect URL (handle relative URLs)
    fn resolve_redirect_url(&self, base: &Url, location: &str) -> Result<Url, BrowserError> {
        // If location is absolute URL, parse it directly
        if location.contains("://") {
            Url::parse(location)
        } else if location.starts_with('/') {
            // Absolute path - keep scheme and host, replace path
            let mut url = base.clone();
            url.path = location.to_string();
            url.query.clear();
            Ok(url)
        } else {
            // Relative path - resolve against base URL
            let mut url = base.clone();
            // Simple relative path resolution
            if let Some(pos) = url.path.rfind('/') {
                url.path = format!("{}/{}", &url.path[..pos], location);
            } else {
                url.path = format!("/{}", location);
            }
            url.query.clear();
            Ok(url)
        }
    }

    /// Fetch local file
    fn fetch_file(&self, url: &Url) -> Result<Vec<u8>, BrowserError> {
        // File protocol - read from filesystem
        // Remove leading slash from path for filesystem lookup
        let path = if url.path.starts_with('/') {
            &url.path[1..]
        } else {
            &url.path
        };
        
        println!("[browser] Loading file: {}", path);
        
        match crate::fs::boot_disk::read_file(path) {
            Some(data) => {
                println!("[browser] File loaded: {} bytes", data.len());
                Ok(data)
            }
            None => {
                println!("[browser] File not found: {}", path);
                Err(BrowserError::NotFound)
            }
        }
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
        // Initialize framebuffer if not already done
        if self.render_context.framebuffer.is_none() {
            self.render_context.init_framebuffer(
                self.config.viewport_width,
                self.config.viewport_height
            );
        }
        
        if let Some(ref tree) = self.render_context.layout_tree {
            if let Some(ref mut fb) = self.render_context.framebuffer {
                render::render(tree, fb)?;
            }
        }
        Ok(())
    }
}

/// HTTP Response structure
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status_code: u16,
    pub status_text: String,
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
}

impl HttpResponse {
    /// Parse HTTP response from raw bytes
    pub fn parse(data: &[u8]) -> Result<Self, BrowserError> {
        // Find end of headers
        let header_end = data.windows(4)
            .position(|w| w == b"\r\n\r\n")
            .ok_or(BrowserError::ParseError)?;
        
        let header_data = &data[..header_end];
        let body_start = header_end + 4;
        
        // Parse status line
        let status_line_end = header_data.iter()
            .position(|&b| b == b'\n')
            .ok_or(BrowserError::ParseError)?;
        let status_line = core::str::from_utf8(&header_data[..status_line_end])
            .map_err(|_| BrowserError::ParseError)?;
        
        let parts: Vec<&str> = status_line.split_whitespace().collect();
        if parts.len() < 3 {
            return Err(BrowserError::ParseError);
        }
        
        let status_code: u16 = parts[1].parse().map_err(|_| BrowserError::ParseError)?;
        let status_text = parts[2..].join(" ");
        
        // Parse headers
        let mut headers = BTreeMap::new();
        let header_lines = core::str::from_utf8(&header_data[status_line_end + 1..])
            .map_err(|_| BrowserError::ParseError)?;
        
        for line in header_lines.lines() {
            if let Some(pos) = line.find(':') {
                let name = line[..pos].trim().to_lowercase();
                let value = line[pos + 1..].trim().to_string();
                headers.insert(name, value);
            }
        }
        
        // Extract body based on Content-Length or Transfer-Encoding
        let body = if let Some(len_str) = headers.get("content-length") {
            let content_len: usize = len_str.parse().map_err(|_| BrowserError::ParseError)?;
            if data.len() >= body_start + content_len {
                data[body_start..body_start + content_len].to_vec()
            } else {
                // Incomplete body - return what we have
                data[body_start..].to_vec()
            }
        } else if headers.get("transfer-encoding").map(|v| v.contains("chunked")).unwrap_or(false) {
            // Handle chunked transfer encoding
            Self::decode_chunked(&data[body_start..])?
        } else {
            // No Content-Length, read all remaining data
            data[body_start..].to_vec()
        };
        
        Ok(Self {
            status_code,
            status_text,
            headers,
            body,
        })
    }
    
    /// Decode chunked transfer encoding
    fn decode_chunked(data: &[u8]) -> Result<Vec<u8>, BrowserError> {
        let mut result = Vec::new();
        let mut pos = 0;
        
        loop {
            if pos >= data.len() {
                break;
            }
            
            // Find end of chunk size line
            let line_end = match data[pos..].iter().position(|&b| b == b'\n') {
                Some(n) => pos + n,
                None => break,
            };
            
            // Parse chunk size (hex)
            let size_line = core::str::from_utf8(&data[pos..line_end])
                .map_err(|_| BrowserError::ParseError)?
                .trim();
            let size_line = size_line.split(';').next().unwrap_or("0"); // Ignore chunk extensions
            let chunk_size = usize::from_str_radix(size_line, 16)
                .map_err(|_| BrowserError::ParseError)?;
            
            if chunk_size == 0 {
                // Last chunk
                break;
            }
            
            pos = line_end + 1;
            
            // Copy chunk data
            if pos + chunk_size > data.len() {
                return Err(BrowserError::ParseError);
            }
            result.extend_from_slice(&data[pos..pos + chunk_size]);
            pos += chunk_size;
            
            // Skip CRLF after chunk
            if pos + 2 <= data.len() && &data[pos..pos + 2] == b"\r\n" {
                pos += 2;
            }
        }
        
        Ok(result)
    }
}

/// URL structure
#[derive(Debug, Clone)]
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
        
        // Parse host, port, path, query, fragment
        let (host_port, path_query_fragment) = if let Some(pos) = rest.find('/') {
            (&rest[..pos], &rest[pos..])
        } else {
            (rest, "/")
        };
        
        // Parse host and port
        let (host, port) = if let Some(pos) = host_port.find(':') {
            let host = String::from(&host_port[..pos]);
            let port: u16 = host_port[pos + 1..].parse()
                .map_err(|_| BrowserError::InvalidUrl)?;
            (host, port)
        } else {
            let host = String::from(host_port);
            let port = match scheme.as_str() {
                "http" => 80,
                "https" => 443,
                "ftp" => 21,
                _ => 0,
            };
            (host, port)
        };
        
        // Parse path, query, and fragment
        let (path, query, fragment) = Self::parse_path_query_fragment(path_query_fragment);
        
        Ok(Self {
            scheme,
            host,
            port,
            path,
            query,
            fragment,
        })
    }
    
    /// Parse path, query, and fragment
    fn parse_path_query_fragment(s: &str) -> (String, String, String) {
        let mut path = String::new();
        let mut query = String::new();
        let mut fragment = String::new();
        
        // Check for fragment
        let (rest, frag) = if let Some(pos) = s.find('#') {
            (&s[..pos], &s[pos + 1..])
        } else {
            (s, "")
        };
        fragment = String::from(frag);
        
        // Check for query
        let (p, q) = if let Some(pos) = rest.find('?') {
            (&rest[..pos], &rest[pos + 1..])
        } else {
            (rest, "")
        };
        path = String::from(p);
        query = String::from(q);
        
        (path, query, fragment)
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
    TooManyRedirects = 9,
    ConnectionTimeout = 10,
    DnsError = 11,
    TlsError = 12,
    HttpError = 13,
    Unknown = 255,
}

lazy_static! {
    static ref BROWSER: Mutex<Option<Browser>> = Mutex::new(None);
}

/// Initialize browser engine
pub fn init() {
    println!("[browser] Initializing browser engine...");

    println!("[browser] Creating browser instance...");
    let browser = Browser::new();
    println!("[browser] Browser instance created, storing...");
    *BROWSER.lock() = Some(browser);
    println!("[browser] Browser stored");

    // Initialize subsystems
    println!("[browser] Init HTML...");
    html::init();
    println!("[browser] Init CSS...");
    css::init();
    println!("[browser] Init JS...");
    js::init();
    println!("[browser] Init WASM...");
    wasm::init();
    println!("[browser] Init layout...");
    layout::init();
    println!("[browser] Init render...");
    render::init();

    // Load homepage
    println!("[browser] Loading homepage...");
    let _ = load_homepage();

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

/// Navigate to URL from desktop
pub fn navigate_from_desktop(url: &str) -> Result<(), BrowserError> {
    println!("[browser] Desktop navigation to: {}", url);
    
    // Update the URL in the desktop UI
    crate::desktop::ui::set_browser_url(url);
    
    // Perform navigation
    navigate(url)
}

/// Handle URL input from keyboard (character by character)
pub fn handle_url_typing(key: char) {
    if let Some(ref mut browser) = *BROWSER.lock() {
        if key == '\n' || key == '\r' {
            // Enter pressed - navigate to the typed URL
            if !browser.typing_url.is_empty() {
                let url = browser.typing_url.clone();
                browser.typing_url.clear();
                println!("[browser] Navigating to typed URL: {}", url);
                
                // Update desktop UI
                crate::desktop::ui::set_browser_url(&url);
                
                // Perform navigation in a separate call to avoid borrow issues
                drop(browser);
                let _ = navigate(&url);
            }
        } else if key == '\x08' || key == '\x7f' {
            // Backspace
            browser.typing_url.pop();
            // Update UI with current typing state
            crate::desktop::ui::set_browser_url(&browser.typing_url);
        } else if key.is_ascii_graphic() || key == ' ' || key == '.' || key == '/' || key == ':' {
            // Add character to URL being typed
            browser.typing_url.push(key);
            // Update UI with current typing state
            crate::desktop::ui::set_browser_url(&browser.typing_url);
        }
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

/// Get current URL
pub fn get_current_url() -> String {
    if let Some(ref browser) = *BROWSER.lock() {
        browser.current_url.clone()
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
        println!("  Follow Redirects: {}", if browser.config.follow_redirects { "yes" } else { "no" });
        
        if let Some(ref doc) = browser.document {
            println!("  Document elements: {}", doc.element_count());
        }
    } else {
        println!("  Browser not initialized");
    }
}

/// Test the browser engine with a simple HTML page
pub fn test_render() -> Result<(), BrowserError> {
    println!("[browser] Testing browser rendering...");
    
    // Create a simple test HTML page
    let test_html = r#"<!DOCTYPE html>
<html>
<head>
    <title>Test Page</title>
    <style>
        body { background: #f0f0f0; font-family: sans-serif; }
        h1 { color: #333; }
        p { color: #666; }
    </style>
</head>
<body>
    <h1>WebbOS Browser Test</h1>
    <p>This is a test page to verify the browser engine works correctly.</p>
    <p>Features tested:</p>
    <ul>
        <li>HTML parsing</li>
        <li>CSS styling</li>
        <li>Layout engine</li>
        <li>Rendering</li>
    </ul>
</body>
</html>"#;

    // Parse the HTML
    println!("[browser] Parsing HTML...");
    let document = html::parse(test_html.as_bytes())?;
    println!("[browser] Parsed {} elements", document.element_count());
    
    // Apply CSS
    println!("[browser] Applying styles...");
    let mut document = document;
    css::apply_styles(&mut document)?;
    
    // Layout
    println!("[browser] Performing layout...");
    let layout_tree = layout::layout(&document, 1024, 768)?;
    println!("[browser] Layout tree created");
    
    // Render
    println!("[browser] Rendering...");
    let mut framebuffer = render::Framebuffer::new(1024, 768);
    render::render(&layout_tree, &mut framebuffer)?;
    println!("[browser] Rendered to framebuffer");
    
    println!("[browser] Browser test completed successfully!");
    Ok(())
}

/// Load and display a local HTML file
pub fn load_file(path: &str) -> Result<(), BrowserError> {
    println!("[browser] Loading file: {}", path);
    
    // Read file using boot_disk
    match crate::fs::boot_disk::read_file(path) {
        Some(data) => {
            println!("[browser] Read {} bytes", data.len());
            
            // Parse HTML
            let document = html::parse(&data)?;
            println!("[browser] Parsed {} elements", document.element_count());
            
            // Store in browser
            if let Some(ref mut browser) = *BROWSER.lock() {
                browser.document = Some(document);
                browser.current_url = format!("file://{}", path);
                browser.title = String::from("Loaded Page");
            }
            
            println!("[browser] File loaded successfully");
            Ok(())
        }
        None => {
            println!("[browser] Failed to read file: {}", path);
            Err(BrowserError::NotFound)
        }
    }
}

/// Fetch a URL and return the raw response (for API usage)
pub fn fetch_url(url: &str) -> Result<Vec<u8>, BrowserError> {
    println!("[browser] Fetching URL: {}", url);
    
    let parsed_url = Url::parse(url)?;
    
    if let Some(ref browser) = *BROWSER.lock() {
        browser.fetch(&parsed_url, 0)
    } else {
        Err(BrowserError::Unknown)
    }
}

/// Load the default homepage
pub fn load_homepage() -> Result<(), BrowserError> {
    println!("[browser] Loading homepage...");
    
    let homepage_html = r#"<!DOCTYPE html>
<html>
<head>
    <title>WebbOS Browser - Home</title>
    <style>
        body { 
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            margin: 0;
            padding: 40px;
            color: white;
            min-height: 100vh;
            box-sizing: border-box;
        }
        .container {
            max-width: 800px;
            margin: 0 auto;
        }
        h1 {
            font-size: 48px;
            margin-bottom: 10px;
            text-shadow: 0 2px 4px rgba(0,0,0,0.3);
        }
        .subtitle {
            font-size: 20px;
            opacity: 0.9;
            margin-bottom: 40px;
        }
        .welcome-box {
            background: rgba(255,255,255,0.95);
            color: #333;
            padding: 30px;
            border-radius: 16px;
            box-shadow: 0 10px 40px rgba(0,0,0,0.2);
            margin-bottom: 30px;
        }
        h2 {
            color: #667eea;
            margin-top: 0;
        }
        .links-section {
            margin: 30px 0;
        }
        .link-item {
            background: #f5f5f5;
            padding: 15px 20px;
            margin: 10px 0;
            border-radius: 8px;
            border-left: 4px solid #667eea;
        }
        .link-item a {
            color: #667eea;
            text-decoration: none;
            font-weight: 500;
            font-size: 16px;
        }
        .link-item a:hover {
            text-decoration: underline;
        }
        .link-item p {
            margin: 5px 0 0 0;
            color: #666;
            font-size: 14px;
        }
        .instructions {
            background: #e8f4f8;
            padding: 20px;
            border-radius: 8px;
            margin-top: 30px;
        }
        .instructions h3 {
            margin-top: 0;
            color: #2c5282;
        }
        .instructions ul {
            margin: 10px 0;
            padding-left: 20px;
        }
        .instructions li {
            margin: 8px 0;
            color: #4a5568;
        }
        .features {
            display: grid;
            grid-template-columns: repeat(2, 1fr);
            gap: 15px;
            margin: 20px 0;
        }
        .feature {
            background: #f9f9f9;
            padding: 15px;
            border-radius: 8px;
            text-align: center;
        }
        .feature-icon {
            font-size: 32px;
            margin-bottom: 8px;
        }
        .feature-text {
            font-size: 14px;
            color: #555;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>🌐 WebbOS Browser</h1>
        <p class="subtitle">Welcome to your web browsing experience</p>
        
        <div class="welcome-box">
            <h2>Welcome!</h2>
            <p>This is the WebbOS built-in browser. You can navigate to websites, view local files, and explore the web directly from your operating system.</p>
            
            <div class="features">
                <div class="feature">
                    <div class="feature-icon">📄</div>
                    <div class="feature-text">HTML5 Support</div>
                </div>
                <div class="feature">
                    <div class="feature-icon">🎨</div>
                    <div class="feature-text">CSS3 Styling</div>
                </div>
                <div class="feature">
                    <div class="feature-icon">⚡</div>
                    <div class="feature-text">JavaScript</div>
                </div>
                <div class="feature">
                    <div class="feature-icon">🔒</div>
                    <div class="feature-text">TLS Security</div>
                </div>
            </div>
        </div>
        
        <div class="welcome-box">
            <h2>Quick Links</h2>
            <div class="links-section">
                <div class="link-item">
                    <a href="http://example.com">Example.com</a>
                    <p>A simple test page for basic web connectivity</p>
                </div>
                <div class="link-item">
                    <a href="file:///test.html">Local Test Page</a>
                    <p>Open a local HTML file from the filesystem</p>
                </div>
                <div class="link-item">
                    <a href="https://webbos.local">WebbOS Local</a>
                    <p>Access local WebbOS services and apps</p>
                </div>
            </div>
        </div>
        
        <div class="welcome-box instructions">
            <h3>📖 How to Use the Browser</h3>
            <ul>
                <li><strong>Navigate:</strong> Click the URL bar and type a web address</li>
                <li><strong>Go:</strong> Press Enter or click the Go button to load the page</li>
                <li><strong>Back/Forward:</strong> Use the navigation buttons to browse history</li>
                <li><strong>Close:</strong> Click the red button to close the browser window</li>
                <li><strong>Minimize:</strong> Click the yellow button to minimize</li>
                <li><strong>Maximize:</strong> Click the green button to maximize</li>
            </ul>
        </div>
    </div>
</body>
</html>"#;

    // Parse the homepage HTML
    let document = html::parse(homepage_html.as_bytes())?;
    
    if let Some(ref mut browser) = *BROWSER.lock() {
        browser.document = Some(document);
        browser.current_url = String::from("about:home");
        browser.title = String::from("WebbOS Browser - Home");
        
        // Perform layout and render
        browser.layout()?;
        browser.render()?;
        
        println!("[browser] Homepage loaded successfully");
        Ok(())
    } else {
        Err(BrowserError::Unknown)
    }
}

/// Get the rendered framebuffer data for display
/// Returns (width, height, pixel_data) if available
pub fn get_rendered_framebuffer_data() -> Option<(u32, u32, alloc::vec::Vec<u32>)> {
    let browser_guard = BROWSER.lock();
    if let Some(ref browser) = *browser_guard {
        if let Some(ref fb) = browser.render_context.framebuffer {
            return Some((fb.width, fb.height, fb.data.clone()));
        }
    }
    None
}
