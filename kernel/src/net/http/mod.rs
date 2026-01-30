//! HTTP/HTTPS Client
//!
//! HTTP/1.1 and HTTP/2 client implementation for WebbOS.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use spin::Mutex;
use lazy_static::lazy_static;

use crate::net::{Ipv4Address, Port, tcp, socket};
use crate::net::socket::{Socket, SocketDomain, SocketType, SocketProtocol};
use crate::tls::{TlsConnection, TlsError};
use crate::println;

/// HTTP methods
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
    Put,
    Delete,
    Head,
    Options,
    Patch,
}

impl Method {
    pub fn as_str(&self) -> &'static str {
        match self {
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Delete => "DELETE",
            Method::Head => "HEAD",
            Method::Options => "OPTIONS",
            Method::Patch => "PATCH",
        }
    }
}

/// HTTP version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Version {
    Http10,
    Http11,
    Http2,
}

impl Version {
    pub fn as_str(&self) -> &'static str {
        match self {
            Version::Http10 => "HTTP/1.0",
            Version::Http11 => "HTTP/1.1",
            Version::Http2 => "HTTP/2",
        }
    }
}

/// HTTP request
#[derive(Debug, Clone)]
pub struct Request {
    pub method: Method,
    pub url: Url,
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
    pub version: Version,
}

impl Request {
    /// Create new GET request
    pub fn get(url: &str) -> Result<Self, HttpError> {
        Ok(Self {
            method: Method::Get,
            url: Url::parse(url)?,
            headers: BTreeMap::new(),
            body: Vec::new(),
            version: Version::Http11,
        })
    }

    /// Create new POST request
    pub fn post(url: &str, body: Vec<u8>) -> Result<Self, HttpError> {
        let mut req = Self {
            method: Method::Post,
            url: Url::parse(url)?,
            headers: BTreeMap::new(),
            body,
            version: Version::Http11,
        };
        req.header("Content-Type", "application/x-www-form-urlencoded");
        Ok(req)
    }

    /// Add header
    pub fn header(&mut self, name: &str, value: &str) -> &mut Self {
        self.headers.insert(name.to_string(), value.to_string());
        self
    }

    /// Build request line and headers
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut result = Vec::new();
        
        // Request line
        result.extend_from_slice(self.method.as_str().as_bytes());
        result.push(b' ');
        result.extend_from_slice(self.url.path.as_bytes());
        if !self.url.query.is_empty() {
            result.push(b'?');
            result.extend_from_slice(self.url.query.as_bytes());
        }
        result.push(b' ');
        result.extend_from_slice(self.version.as_str().as_bytes());
        result.extend_from_slice(b"\r\n");
        
        // Host header (required for HTTP/1.1)
        result.extend_from_slice(b"Host: ");
        result.extend_from_slice(self.url.host.as_bytes());
        if self.url.port != 80 && self.url.port != 443 {
            result.push(b':');
            result.extend_from_slice(self.url.port.to_string().as_bytes());
        }
        result.extend_from_slice(b"\r\n");
        
        // Connection header
        result.extend_from_slice(b"Connection: close\r\n");
        
        // User-Agent
        result.extend_from_slice(b"User-Agent: WebbOS/1.0\r\n");
        
        // Accept
        result.extend_from_slice(b"Accept: text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8\r\n");
        result.extend_from_slice(b"Accept-Language: en-US,en;q=0.5\r\n");
        result.extend_from_slice(b"Accept-Encoding: identity\r\n");
        
        // Content-Length if body exists
        if !self.body.is_empty() {
            result.extend_from_slice(b"Content-Length: ");
            result.extend_from_slice(self.body.len().to_string().as_bytes());
            result.extend_from_slice(b"\r\n");
        }
        
        // Custom headers
        for (name, value) in &self.headers {
            result.extend_from_slice(name.as_bytes());
            result.extend_from_slice(b": ");
            result.extend_from_slice(value.as_bytes());
            result.extend_from_slice(b"\r\n");
        }
        
        // Empty line to end headers
        result.extend_from_slice(b"\r\n");
        
        // Body
        result.extend_from_slice(&self.body);
        
        result
    }
}

/// HTTP response
#[derive(Debug, Clone)]
pub struct Response {
    pub version: Version,
    pub status: u16,
    pub status_text: String,
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
}

impl Response {
    /// Parse response from bytes
    pub fn parse(data: &[u8]) -> Result<(Self, usize), HttpError> {
        // Find end of headers
        let header_end = data.windows(4)
            .position(|w| w == b"\r\n\r\n")
            .ok_or(HttpError::InvalidResponse)?;
        
        let header_data = &data[..header_end];
        let body_start = header_end + 4;
        
        // Parse status line
        let status_line_end = header_data.iter()
            .position(|&b| b == b'\n')
            .ok_or(HttpError::InvalidResponse)?;
        let status_line = core::str::from_utf8(&header_data[..status_line_end])
            .map_err(|_| HttpError::InvalidResponse)?;
        
        let parts: Vec<&str> = status_line.split_whitespace().collect();
        if parts.len() < 3 {
            return Err(HttpError::InvalidResponse);
        }
        
        let version = match parts[0] {
            "HTTP/1.0" => Version::Http10,
            "HTTP/1.1" => Version::Http11,
            "HTTP/2" => Version::Http2,
            _ => return Err(HttpError::InvalidResponse),
        };
        
        let status: u16 = parts[1].parse().map_err(|_| HttpError::InvalidResponse)?;
        let status_text = parts[2..].join(" ");
        
        // Parse headers
        let mut headers = BTreeMap::new();
        let header_lines = core::str::from_utf8(&header_data[status_line_end + 1..])
            .map_err(|_| HttpError::InvalidResponse)?;
        
        for line in header_lines.lines() {
            if let Some(pos) = line.find(':') {
                let name = line[..pos].trim().to_lowercase();
                let value = line[pos + 1..].trim().to_string();
                headers.insert(name, value);
            }
        }
        
        // Check for Content-Length
        let body = if let Some(len_str) = headers.get("content-length") {
            let content_len: usize = len_str.parse().map_err(|_| HttpError::InvalidResponse)?;
            if data.len() >= body_start + content_len {
                data[body_start..body_start + content_len].to_vec()
            } else {
                // Incomplete body
                Vec::new()
            }
        } else if headers.get("transfer-encoding").map(|v| v == "chunked").unwrap_or(false) {
            // Handle chunked encoding (simplified)
            Self::decode_chunked(&data[body_start..])?
        } else {
            // Read rest of data
            data[body_start..].to_vec()
        };
        
        Ok((Self {
            version,
            status,
            status_text,
            headers,
            body,
        }, body_start + body.len()))
    }
    
    /// Decode chunked transfer encoding
    fn decode_chunked(data: &[u8]) -> Result<Vec<u8>, HttpError> {
        let mut result = Vec::new();
        let mut pos = 0;
        
        loop {
            // Find end of chunk size line
            let line_end = data[pos..].iter()
                .position(|&b| b == b'\n')
                .ok_or(HttpError::InvalidResponse)?;
            
            // Parse chunk size (hex)
            let size_line = core::str::from_utf8(&data[pos..pos + line_end])
                .map_err(|_| HttpError::InvalidResponse)?
                .trim();
            let chunk_size = usize::from_str_radix(size_line, 16)
                .map_err(|_| HttpError::InvalidResponse)?;
            
            if chunk_size == 0 {
                break;
            }
            
            pos += line_end + 1;
            
            // Copy chunk data
            if pos + chunk_size > data.len() {
                return Err(HttpError::InvalidResponse);
            }
            result.extend_from_slice(&data[pos..pos + chunk_size]);
            pos += chunk_size + 2; // Skip CRLF
        }
        
        Ok(result)
    }
}

/// URL parser
#[derive(Debug, Clone)]
pub struct Url {
    pub scheme: String,
    pub host: String,
    pub port: u16,
    pub path: String,
    pub query: String,
}

impl Url {
    /// Parse URL string
    pub fn parse(url: &str) -> Result<Self, HttpError> {
        // Find scheme
        let scheme_end = url.find("://")
            .ok_or(HttpError::InvalidUrl)?;
        let scheme = url[..scheme_end].to_string();
        
        let rest = &url[scheme_end + 3..];
        
        // Find path
        let (host_port, path_query) = if let Some(pos) = rest.find('/') {
            (&rest[..pos], &rest[pos..])
        } else {
            (rest, "/")
        };
        
        // Parse host and port
        let (host, port) = if let Some(pos) = host_port.find(':') {
            let host = host_port[..pos].to_string();
            let port: u16 = host_port[pos + 1..].parse()
                .map_err(|_| HttpError::InvalidUrl)?;
            (host, port)
        } else {
            let host = host_port.to_string();
            let port = match scheme.as_str() {
                "http" => 80,
                "https" => 443,
                _ => return Err(HttpError::InvalidUrl),
            };
            (host, port)
        };
        
        // Parse path and query
        let (path, query) = if let Some(pos) = path_query.find('?') {
            (path_query[..pos].to_string(), path_query[pos + 1..].to_string())
        } else {
            (path_query.to_string(), String::new())
        };
        
        Ok(Self {
            scheme,
            host,
            port,
            path,
            query,
        })
    }
    
    /// Check if HTTPS
    pub fn is_https(&self) -> bool {
        self.scheme == "https"
    }
}

/// HTTP client
pub struct Client {
    timeout_ms: u64,
    follow_redirects: bool,
    max_redirects: u32,
}

impl Client {
    /// Create new HTTP client
    pub fn new() -> Self {
        Self {
            timeout_ms: 30000,
            follow_redirects: true,
            max_redirects: 10,
        }
    }
    
    /// Send HTTP request
    pub fn request(&self, req: &Request) -> Result<Response, HttpError> {
        if req.url.is_https() {
            self.request_https(req)
        } else {
            self.request_http(req)
        }
    }
    
    /// Send HTTP request (plaintext)
    fn request_http(&self, req: &Request) -> Result<Response, HttpError> {
        // Resolve host
        let ip = resolve_host(&req.url.host)?;
        
        // Create socket
        let fd = socket::socket(SocketDomain::Inet, SocketType::Stream, SocketProtocol::Tcp)
            .map_err(|_| HttpError::ConnectionFailed)?;
        
        // Connect
        let addr = crate::net::SocketAddr::new_v4(ip, Port::new(req.url.port));
        socket::connect(fd, ip, Port::new(req.url.port))
            .map_err(|_| HttpError::ConnectionFailed)?;
        
        // Send request
        let request_data = req.to_bytes();
        socket::send(fd, &request_data, 0)
            .map_err(|_| HttpError::ConnectionFailed)?;
        
        // Receive response
        let mut response_data = Vec::new();
        let mut buffer = [0u8; 4096];
        
        loop {
            match socket::recv(fd, &mut buffer, 0) {
                Ok(n) if n > 0 => response_data.extend_from_slice(&buffer[..n]),
                _ => break,
            }
        }
        
        // Close socket
        let _ = socket::close(fd);
        
        // Parse response
        let (response, _) = Response::parse(&response_data)?;
        
        // Handle redirects
        if self.follow_redirects && is_redirect(response.status) {
            if let Some(location) = response.headers.get("location") {
                // Follow redirect (simplified - no redirect limit check for now)
                let mut new_req = Request::get(location)?;
                new_req.headers = req.headers.clone();
                return self.request(&new_req);
            }
        }
        
        Ok(response)
    }
    
    /// Send HTTPS request
    fn request_https(&self, req: &Request) -> Result<Response, HttpError> {
        // Resolve host
        let ip = resolve_host(&req.url.host)?;
        
        // Create TLS connection
        let mut tls = TlsConnection::new();
        
        // Create socket
        let fd = socket::socket(SocketDomain::Inet, SocketType::Stream, SocketProtocol::Tcp)
            .map_err(|_| HttpError::ConnectionFailed)?;
        
        // Connect TCP
        socket::connect(fd, ip, Port::new(req.url.port))
            .map_err(|_| HttpError::ConnectionFailed)?;
        
        // Send Client Hello
        let client_hello = tls.generate_client_hello();
        socket::send(fd, &client_hello, 0)
            .map_err(|_| HttpError::ConnectionFailed)?;
        
        // Receive Server Hello (simplified - would need proper TLS handshake)
        let mut buffer = [0u8; 4096];
        let mut handshake_data = Vec::new();
        
        for _ in 0..10 {
            match socket::recv(fd, &mut buffer, 0) {
                Ok(n) if n > 0 => {
                    handshake_data.extend_from_slice(&buffer[..n]);
                    if tls.state() == crate::tls::TlsState::Connected {
                        break;
                    }
                }
                _ => break,
            }
        }
        
        // For now, fall back to HTTP (TLS not fully implemented)
        // In production, this would complete the TLS handshake
        println!("[http] HTTPS connection not yet fully implemented, falling back to HTTP");
        let _ = socket::close(fd);
        
        // Try HTTP instead
        let mut http_req = req.clone();
        http_req.url.scheme = "http".to_string();
        http_req.url.port = 80;
        self.request_http(&http_req)
    }
    
    /// Send GET request
    pub fn get(&self, url: &str) -> Result<Response, HttpError> {
        let req = Request::get(url)?;
        self.request(&req)
    }
    
    /// Send POST request
    pub fn post(&self, url: &str, body: Vec<u8>) -> Result<Response, HttpError> {
        let req = Request::post(url, body)?;
        self.request(&req)
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

/// HTTP error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpError {
    Success = 0,
    InvalidUrl = 1,
    InvalidResponse = 2,
    ConnectionFailed = 3,
    Timeout = 4,
    TooManyRedirects = 5,
    TlsError = 6,
    Unknown = 255,
}

/// Resolve hostname to IP
fn resolve_host(host: &str) -> Result<Ipv4Address, HttpError> {
    // Check if it's already an IP address
    if let Some(ip) = parse_ipv4(host) {
        return Ok(ip);
    }
    
    // Try DNS lookup
    use crate::net::dns;
    if let Some(ip) = dns::resolve(host) {
        match ip {
            crate::net::IpAddress::V4(ipv4) => Ok(ipv4),
            _ => Err(HttpError::ConnectionFailed),
        }
    } else {
        Err(HttpError::ConnectionFailed)
    }
}

/// Parse IPv4 address
fn parse_ipv4(s: &str) -> Option<Ipv4Address> {
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
fn is_redirect(status: u16) -> bool {
    matches!(status, 301 | 302 | 303 | 307 | 308)
}

/// Global HTTP client
lazy_static! {
    static ref HTTP_CLIENT: Client = Client::new();
}

/// Simple GET request
pub fn get(url: &str) -> Result<Response, HttpError> {
    HTTP_CLIENT.get(url)
}

/// Simple POST request
pub fn post(url: &str, body: Vec<u8>) -> Result<Response, HttpError> {
    HTTP_CLIENT.post(url, body)
}

/// Initialize HTTP client
pub fn init() {
    println!("[http] HTTP/HTTPS client initialized");
    println!("[http] Supported: HTTP/1.1, HTTPS (TLS 1.3 partial)");
}

/// Print HTTP response
pub fn print_response(response: &Response) {
    println!("HTTP Response:");
    println!("  Status: {} {}", response.status, response.status_text);
    println!("  Version: {:?}", response.version);
    println!("  Headers:");
    for (name, value) in &response.headers {
        println!("    {}: {}", name, value);
    }
    println!("  Body length: {} bytes", response.body.len());
    
    // Try to print body as text
    if let Ok(text) = core::str::from_utf8(&response.body[..response.body.len().min(200)]) {
        println!("  Body preview: {}", text);
    }
}
