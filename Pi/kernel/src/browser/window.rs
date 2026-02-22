//! Window and Document Objects
//!
//! Implements the window and document global objects for JavaScript.

use alloc::string::String;
use alloc::vec::Vec;
use spin::Mutex;
use lazy_static::lazy_static;

use crate::browser::js::{Value, Object};
use crate::println;

/// Window object - global scope
pub struct Window {
    pub inner_width: u32,
    pub inner_height: u32,
    pub screen_width: u32,
    pub screen_height: u32,
    pub location: Location,
}

impl Window {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            inner_width: width,
            inner_height: height,
            screen_width: width,
            screen_height: height,
            location: Location::new(),
        }
    }
    
    pub fn alert(&self, message: &str) {
        println!("[window.alert] {}", message);
    }
    
    pub fn confirm(&self, message: &str) -> bool {
        println!("[window.confirm] {} (returning true)", message);
        true
    }
    
    pub fn prompt(&self, message: &str, default: &str) -> String {
        println!("[window.prompt] {} (returning default: {})", message, default);
        String::from(default)
    }
    
    pub fn to_js_value(&self) -> Value {
        let mut obj = Object::new();
        obj.set("innerWidth", Value::Number(self.inner_width as f64));
        obj.set("innerHeight", Value::Number(self.inner_height as f64));
        obj.set("location", self.location.to_js_value());
        Value::Object(obj)
    }
}

/// Location object
pub struct Location {
    pub href: String,
    pub protocol: String,
    pub host: String,
    pub pathname: String,
}

impl Location {
    pub fn new() -> Self {
        Self {
            href: String::from("http://localhost/"),
            protocol: String::from("http:"),
            host: String::from("localhost"),
            pathname: String::from("/"),
        }
    }
    
    pub fn to_js_value(&self) -> Value {
        let mut obj = Object::new();
        obj.set("href", Value::String(self.href.clone()));
        obj.set("protocol", Value::String(self.protocol.clone()));
        obj.set("host", Value::String(self.host.clone()));
        obj.set("pathname", Value::String(self.pathname.clone()));
        Value::Object(obj)
    }
}

/// Document object (simplified)
pub struct JsDocument {
    pub title: String,
    pub url: String,
}

impl JsDocument {
    pub fn new() -> Self {
        Self {
            title: String::from("Untitled"),
            url: String::from("http://localhost/"),
        }
    }
    
    pub fn to_js_value(&self) -> Value {
        let mut obj = Object::new();
        obj.set("title", Value::String(self.title.clone()));
        obj.set("URL", Value::String(self.url.clone()));
        Value::Object(obj)
    }
}

/// Global instances
lazy_static! {
    static ref GLOBAL_WINDOW: Mutex<Option<Window>> = Mutex::new(None);
    static ref GLOBAL_DOCUMENT: Mutex<Option<JsDocument>> = Mutex::new(None);
}

/// Initialize window and document
pub fn init(window_width: u32, window_height: u32) {
    *GLOBAL_WINDOW.lock() = Some(Window::new(window_width, window_height));
    *GLOBAL_DOCUMENT.lock() = Some(JsDocument::new());
    println!("[window] Window and Document initialized ({}x{})", window_width, window_height);
}

/// Get global window
pub fn window() -> Option<spin::MutexGuard<'static, Option<Window>>> {
    Some(GLOBAL_WINDOW.lock())
}

/// Get global document
pub fn document() -> Option<spin::MutexGuard<'static, Option<JsDocument>>> {
    Some(GLOBAL_DOCUMENT.lock())
}
