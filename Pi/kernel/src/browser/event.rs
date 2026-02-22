//! Event System (Simplified)
//!
//! Provides basic DOM event handling for user interactions.

use alloc::string::String;
use alloc::vec::Vec;
use spin::Mutex;
use lazy_static::lazy_static;

use crate::browser::dom_api::ElementId;
use crate::browser::js::Value;
use crate::println;

/// Event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    Click,
    DblClick,
    MouseDown,
    MouseUp,
    MouseMove,
    KeyDown,
    KeyUp,
    Submit,
    Change,
    Input,
    Focus,
    Blur,
    Load,
}

impl EventType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "click" => Some(Self::Click),
            "dblclick" => Some(Self::DblClick),
            "mousedown" => Some(Self::MouseDown),
            "mouseup" => Some(Self::MouseUp),
            "mousemove" => Some(Self::MouseMove),
            "keydown" => Some(Self::KeyDown),
            "keyup" => Some(Self::KeyUp),
            "submit" => Some(Self::Submit),
            "change" => Some(Self::Change),
            "input" => Some(Self::Input),
            "focus" => Some(Self::Focus),
            "blur" => Some(Self::Blur),
            "load" => Some(Self::Load),
            _ => None,
        }
    }
}

/// Event object
#[derive(Debug, Clone)]
pub struct Event {
    pub event_type: EventType,
    pub target: ElementId,
    pub client_x: i32,
    pub client_y: i32,
    pub key_code: u32,
    pub key: String,
}

impl Event {
    pub fn new(event_type: EventType, target: ElementId) -> Self {
        Self {
            event_type,
            target,
            client_x: 0,
            client_y: 0,
            key_code: 0,
            key: String::new(),
        }
    }
    
    pub fn mouse(event_type: EventType, target: ElementId, x: i32, y: i32) -> Self {
        let mut e = Self::new(event_type, target);
        e.client_x = x;
        e.client_y = y;
        e
    }
    
    pub fn keyboard(event_type: EventType, target: ElementId, key: &str) -> Self {
        let mut e = Self::new(event_type, target);
        e.key = String::from(key);
        e
    }
    
    pub fn to_js_value(&self) -> Value {
        let mut obj = crate::browser::js::Object::new();
        obj.set("target", Value::Number(self.target as f64));
        obj.set("clientX", Value::Number(self.client_x as f64));
        obj.set("clientY", Value::Number(self.client_y as f64));
        obj.set("key", Value::String(self.key.clone()));
        Value::Object(obj)
    }
}

/// Initialize event system
pub fn init() {
    println!("[event] Event system initialized");
}
