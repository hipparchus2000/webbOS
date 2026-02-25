//! JavaScript DOM Bindings
//!
//! Exposes DOM API and browser functionality to JavaScript.

use alloc::string::String;
use alloc::format;
use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use spin::Mutex;
use lazy_static::lazy_static;
use core::sync::atomic::{AtomicUsize, AtomicU32, Ordering};

use crate::browser::dom_api::{ElementId, api as dom_api};
use crate::browser::event::{Event, EventType, self};
use crate::browser::js::{Value, Object, Environment};
use crate::println;

/// Current document pointer for DOM operations (atomic for thread safety)
static CURRENT_DOCUMENT_PTR: AtomicUsize = AtomicUsize::new(0);

/// Set document pointer
pub fn set_document_ptr(ptr: usize) {
    CURRENT_DOCUMENT_PTR.store(ptr, Ordering::Relaxed);
}

/// Get document pointer
fn get_document_ptr() -> usize {
    CURRENT_DOCUMENT_PTR.load(Ordering::Relaxed)
}

/// Current element being operated on (for callbacks)
static CURRENT_ELEMENT_ID: AtomicU32 = AtomicU32::new(0);

/// Set current element ID
fn set_current_element(id: ElementId) {
    CURRENT_ELEMENT_ID.store(id, Ordering::Relaxed);
}

/// Get current element ID
fn get_current_element() -> ElementId {
    CURRENT_ELEMENT_ID.load(Ordering::Relaxed)
}

/// Initialize JS environment with DOM bindings
pub fn init_js_environment(document_ptr: usize) {
    set_document_ptr(document_ptr);
    println!("[js_bindings] JS environment initialized with document at {:p}", document_ptr as *const ());
}

/// Register all DOM bindings in environment
pub fn register_dom_bindings(env: &mut Environment) {
    // Create document object
    let mut document = Object::new();
    
    // document.getElementById
    document.set("getElementById", Value::Function(create_native_fn(
        "getElementById", 
        1, 
        js_get_element_by_id as crate::browser::js::NativeFn
    )));
    
    // document.querySelector
    document.set("querySelector", Value::Function(create_native_fn(
        "querySelector",
        1,
        js_query_selector as crate::browser::js::NativeFn
    )));
    
    // document.createElement
    document.set("createElement", Value::Function(create_native_fn(
        "createElement",
        1,
        js_create_element as crate::browser::js::NativeFn
    )));
    
    // document.body (as a getter function for now)
    document.set("getBody", Value::Function(create_native_fn(
        "getBody",
        0,
        js_get_body as crate::browser::js::NativeFn
    )));
    
    env.define("document", Value::Object(document));
    
    // Create window object
    let mut window = Object::new();
    
    // window.alert
    window.set("alert", Value::Function(create_native_fn(
        "alert",
        1,
        js_alert as crate::browser::js::NativeFn
    )));
    
    // window.setTimeout
    window.set("setTimeout", Value::Function(create_native_fn(
        "setTimeout",
        2,
        js_set_timeout as crate::browser::js::NativeFn
    )));
    
    // window.innerWidth / innerHeight
    window.set("innerWidth", Value::Number(1024.0));
    window.set("innerHeight", Value::Number(768.0));
    
    env.define("window", Value::Object(window));
    
    // Create console object
    let mut console = Object::new();
    console.set("log", Value::Function(create_native_fn(
        "console.log",
        1,
        js_console_log as crate::browser::js::NativeFn
    )));
    env.define("console", Value::Object(console));
}

/// Native function implementations

fn js_get_element_by_id(_env: &mut Environment, args: Vec<Value>) -> Value {
    if let Some(Value::String(id)) = args.get(0) {
        if let Some(elem_id) = dom_api::get_element_by_id(get_document_ptr() as *mut _, id) {
            set_current_element(elem_id);
            return create_element_proxy(elem_id);
        }
    }
    Value::Null
}

fn js_query_selector(_env: &mut Environment, args: Vec<Value>) -> Value {
    if let Some(Value::String(selector)) = args.get(0) {
        if let Some(elem_id) = dom_api::query_selector(get_document_ptr() as *mut _, selector) {
            set_current_element(elem_id);
            return create_element_proxy(elem_id);
        }
    }
    Value::Null
}

fn js_create_element(_env: &mut Environment, args: Vec<Value>) -> Value {
    if let Some(Value::String(tag)) = args.get(0) {
        let elem_id = dom_api::create_element(get_document_ptr() as *mut _, tag);
        set_current_element(elem_id);
        return create_element_proxy(elem_id);
    }
    Value::Null
}

fn js_get_body(_env: &mut Environment, _args: Vec<Value>) -> Value {
    if let Some(body_id) = dom_api::query_selector(get_document_ptr() as *mut _, "body") {
        set_current_element(body_id);
        return create_element_proxy(body_id);
    }
    Value::Null
}

fn js_alert(_env: &mut Environment, args: Vec<Value>) -> Value {
    let msg = args.get(0).map(|v| v.to_string()).unwrap_or_default();
    println!("[JS ALERT] {}", msg);
    Value::Undefined
}

fn js_set_timeout(_env: &mut Environment, args: Vec<Value>) -> Value {
    if let Some(Value::Number(delay)) = args.get(1) {
        let delay_ms = *delay as u64;
        println!("[JS setTimeout] scheduled for {}ms", delay_ms);
        // TODO: Actually schedule the callback
        return Value::Number(1.0);
    }
    Value::Number(0.0)
}

fn js_console_log(_env: &mut Environment, args: Vec<Value>) -> Value {
    let msg = args.get(0).map(|v| v.to_string()).unwrap_or_default();
    println!("[JS console.log] {}", msg);
    Value::Undefined
}

/// Create an element proxy object
fn create_element_proxy(element_id: ElementId) -> Value {
    let mut obj = Object::new();
    
    // Store element ID
    obj.set("_elementId", Value::Number(element_id as f64));
    
    // Tag name
    obj.set("tagName", Value::String(dom_api::get_tag_name(element_id)));
    
    // ID property
    obj.set("id", Value::String(dom_api::get_id(element_id)));
    
    // className property
    obj.set("className", Value::String(dom_api::get_class_name(element_id)));
    
    // innerHTML (as property for now)
    obj.set("innerHTML", Value::String(dom_api::get_inner_html(element_id)));
    
    // textContent
    obj.set("textContent", Value::String(dom_api::get_text_content(element_id)));
    
    // Methods
    obj.set("getAttribute", Value::Function(create_native_fn(
        "getAttribute",
        1,
        js_element_get_attribute as crate::browser::js::NativeFn
    )));
    
    obj.set("setAttribute", Value::Function(create_native_fn(
        "setAttribute",
        2,
        js_element_set_attribute as crate::browser::js::NativeFn
    )));
    
    obj.set("addEventListener", Value::Function(create_native_fn(
        "addEventListener",
        2,
        js_element_add_event_listener as crate::browser::js::NativeFn
    )));
    
    Value::Object(obj)
}

fn js_element_get_attribute(_env: &mut Environment, args: Vec<Value>) -> Value {
    if let Some(Value::String(name)) = args.get(0) {
        if let Some(value) = dom_api::get_attribute(get_current_element(), name) {
            return Value::String(value);
        }
    }
    Value::Null
}

fn js_element_set_attribute(_env: &mut Environment, args: Vec<Value>) -> Value {
    if let (Some(Value::String(name)), Some(Value::String(value))) = (args.get(0), args.get(1)) {
        dom_api::set_attribute(get_current_element(), name, value);
    }
    Value::Undefined
}

fn js_element_add_event_listener(_env: &mut Environment, args: Vec<Value>) -> Value {
    if let Some(Value::String(event_type_str)) = args.get(0) {
        if let Some(_event_type) = EventType::from_str(event_type_str) {
            let element_id = get_current_element();
            println!("[JS] Registered {} listener on element {}", event_type_str, element_id);
            
            // Store the callback reference
            // TODO: Actually store and invoke the callback
        }
    }
    Value::Undefined
}

/// Helper to create native function
fn create_native_fn(name: &'static str, arity: usize, func: fn(&mut Environment, Vec<Value>) -> Value) -> crate::browser::js::Function {
    crate::browser::js::Function {
        name: String::from(name),
        params: (0..arity).map(|i| format!("arg{}", i)).collect(),
        body: Vec::new(),
        native: Some(func),
    }
}

/// Initialize bindings module
pub fn init() {
    println!("[js_bindings] DOM bindings initialized");
}
