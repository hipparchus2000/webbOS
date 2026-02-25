//! JavaScript DOM Bindings
//!
//! Exposes DOM API and browser functionality to JavaScript.

#![allow(dead_code)]

use alloc::string::String;
use alloc::format;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicUsize, AtomicU32, Ordering};

use spin::Mutex;

use crate::browser::dom_api::{ElementId, api as dom_api};
use crate::browser::event::{EventType};
use crate::browser::js::{Value, Object, Environment};
use crate::println;

/// In-memory localStorage (persists during the session)
static LOCAL_STORAGE: Mutex<BTreeMap<String, String>> = Mutex::new(BTreeMap::new());

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
    
    // Create localStorage object
    let mut local_storage = Object::new();
    local_storage.set("setItem", Value::Function(create_native_fn(
        "setItem",
        2,
        js_local_storage_set_item as crate::browser::js::NativeFn
    )));
    local_storage.set("getItem", Value::Function(create_native_fn(
        "getItem",
        1,
        js_local_storage_get_item as crate::browser::js::NativeFn
    )));
    local_storage.set("removeItem", Value::Function(create_native_fn(
        "removeItem",
        1,
        js_local_storage_remove_item as crate::browser::js::NativeFn
    )));
    local_storage.set("clear", Value::Function(create_native_fn(
        "clear",
        0,
        js_local_storage_clear as crate::browser::js::NativeFn
    )));
    local_storage.set("key", Value::Function(create_native_fn(
        "key",
        1,
        js_local_storage_key as crate::browser::js::NativeFn
    )));
    // length property - return as a getter function since we don't have property getters
    local_storage.set("length", Value::Function(create_native_fn(
        "length",
        0,
        js_local_storage_length as crate::browser::js::NativeFn
    )));
    env.define("localStorage", Value::Object(local_storage));
    
    // Create AudioContext constructor
    env.define("AudioContext", Value::Function(create_native_fn(
        "AudioContext",
        0,
        js_audio_context_new as crate::browser::js::NativeFn
    )));
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

/// localStorage.setItem(key, value)
fn js_local_storage_set_item(_env: &mut Environment, args: Vec<Value>) -> Value {
    if args.len() >= 2 {
        let key = args[0].to_string();
        let value = args[1].to_string();
        LOCAL_STORAGE.lock().insert(key, value);
    }
    Value::Undefined
}

/// localStorage.getItem(key) -> value or null
fn js_local_storage_get_item(_env: &mut Environment, args: Vec<Value>) -> Value {
    if let Some(key) = args.get(0) {
        let key = key.to_string();
        let storage = LOCAL_STORAGE.lock();
        if let Some(value) = storage.get(&key) {
            return Value::String(value.clone());
        }
    }
    Value::Null
}

/// localStorage.removeItem(key)
fn js_local_storage_remove_item(_env: &mut Environment, args: Vec<Value>) -> Value {
    if let Some(key) = args.get(0) {
        let key = key.to_string();
        LOCAL_STORAGE.lock().remove(&key);
    }
    Value::Undefined
}

/// localStorage.clear()
fn js_local_storage_clear(_env: &mut Environment, _args: Vec<Value>) -> Value {
    LOCAL_STORAGE.lock().clear();
    Value::Undefined
}

/// localStorage.key(index) -> key or null
fn js_local_storage_key(_env: &mut Environment, args: Vec<Value>) -> Value {
    if let Some(Value::Number(index)) = args.get(0) {
        let index = *index as usize;
        let storage = LOCAL_STORAGE.lock();
        if let Some((key, _)) = storage.iter().nth(index) {
            return Value::String(key.clone());
        }
    }
    Value::Null
}

/// localStorage.length() -> number of items
fn js_local_storage_length(_env: &mut Environment, _args: Vec<Value>) -> Value {
    Value::Number(LOCAL_STORAGE.lock().len() as f64)
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
    use crate::browser::js::BindingPattern;
    crate::browser::js::Function {
        name: String::from(name),
        params: (0..arity).map(|i| BindingPattern::Identifier(format!("arg{}", i))).collect(),
        body: Vec::new(),
        native: Some(func),
        is_arrow: false,
        arrow_expr: None,
        this_binding: None,
    }
}

/// Web Audio API Implementation

fn js_audio_context_new(_env: &mut Environment, _args: Vec<Value>) -> Value {
    use crate::browser::js::Object;
    
    let mut audio_ctx = Object::new();
    
    // AudioContext properties
    audio_ctx.set("sampleRate", Value::Number(44100.0));
    audio_ctx.set("state", Value::String(String::from("running")));
    audio_ctx.set("currentTime", Value::Number(0.0));
    
    // Methods
    audio_ctx.set("createOscillator", Value::Function(create_native_fn(
        "createOscillator",
        0,
        js_create_oscillator as crate::browser::js::NativeFn
    )));
    
    audio_ctx.set("createGain", Value::Function(create_native_fn(
        "createGain",
        0,
        js_create_gain as crate::browser::js::NativeFn
    )));
    
    audio_ctx.set("createBiquadFilter", Value::Function(create_native_fn(
        "createBiquadFilter",
        0,
        js_create_biquad_filter as crate::browser::js::NativeFn
    )));
    
    audio_ctx.set("resume", Value::Function(create_native_fn(
        "resume",
        0,
        js_audio_resume as crate::browser::js::NativeFn
    )));
    
    audio_ctx.set("suspend", Value::Function(create_native_fn(
        "suspend",
        0,
        js_audio_suspend as crate::browser::js::NativeFn
    )));
    
    audio_ctx.set("close", Value::Function(create_native_fn(
        "close",
        0,
        js_audio_close as crate::browser::js::NativeFn
    )));
    
    println!("[WebAudio] AudioContext created");
    Value::Object(audio_ctx)
}

fn js_create_oscillator(_env: &mut Environment, _args: Vec<Value>) -> Value {
    use crate::browser::js::Object;
    
    let mut osc = Object::new();
    osc.set("type", Value::String(String::from("sine")));
    osc.set("frequency", create_audio_param(440.0));
    osc.set("detune", create_audio_param(0.0));
    
    // Methods
    osc.set("start", Value::Function(create_native_fn(
        "start",
        1,
        js_oscillator_start as crate::browser::js::NativeFn
    )));
    
    osc.set("stop", Value::Function(create_native_fn(
        "stop",
        1,
        js_oscillator_stop as crate::browser::js::NativeFn
    )));
    
    osc.set("connect", Value::Function(create_native_fn(
        "connect",
        1,
        js_node_connect as crate::browser::js::NativeFn
    )));
    
    println!("[WebAudio] OscillatorNode created");
    Value::Object(osc)
}

fn js_create_gain(_env: &mut Environment, _args: Vec<Value>) -> Value {
    use crate::browser::js::Object;
    
    let mut gain = Object::new();
    gain.set("gain", create_audio_param(1.0));
    
    gain.set("connect", Value::Function(create_native_fn(
        "connect",
        1,
        js_node_connect as crate::browser::js::NativeFn
    )));
    
    println!("[WebAudio] GainNode created");
    Value::Object(gain)
}

fn js_create_biquad_filter(_env: &mut Environment, _args: Vec<Value>) -> Value {
    use crate::browser::js::Object;
    
    let mut filter = Object::new();
    filter.set("type", Value::String(String::from("lowpass")));
    filter.set("frequency", create_audio_param(350.0));
    filter.set("Q", create_audio_param(1.0));
    filter.set("gain", create_audio_param(0.0));
    
    filter.set("connect", Value::Function(create_native_fn(
        "connect",
        1,
        js_node_connect as crate::browser::js::NativeFn
    )));
    
    println!("[WebAudio] BiquadFilterNode created");
    Value::Object(filter)
}

fn js_oscillator_start(_env: &mut Environment, args: Vec<Value>) -> Value {
    let _when = match args.get(0) {
        Some(Value::Number(n)) => *n,
        _ => 0.0,
    };
    
    // Try to play a beep sound
    if let Err(e) = crate::drivers::audio::beep() {
        println!("[WebAudio] Could not play beep: {:?}", e);
    }
    
    Value::Undefined
}

fn js_oscillator_stop(_env: &mut Environment, _args: Vec<Value>) -> Value {
    crate::drivers::audio::stop();
    Value::Undefined
}

fn js_node_connect(_env: &mut Environment, _args: Vec<Value>) -> Value {
    // Return the destination for chaining
    _args.get(0).cloned().unwrap_or(Value::Undefined)
}

fn js_audio_resume(_env: &mut Environment, _args: Vec<Value>) -> Value {
    // Create a promise-like object (simplified)
    use crate::browser::js::Object;
    let mut promise = Object::new();
    promise.set("then", Value::Function(create_native_fn(
        "then",
        1,
        js_promise_then as crate::browser::js::NativeFn
    )));
    Value::Object(promise)
}

fn js_audio_suspend(_env: &mut Environment, _args: Vec<Value>) -> Value {
    crate::drivers::audio::stop();
    use crate::browser::js::Object;
    let mut promise = Object::new();
    promise.set("then", Value::Function(create_native_fn(
        "then",
        1,
        js_promise_then as crate::browser::js::NativeFn
    )));
    Value::Object(promise)
}

fn js_audio_close(_env: &mut Environment, _args: Vec<Value>) -> Value {
    crate::drivers::audio::stop();
    use crate::browser::js::Object;
    let mut promise = Object::new();
    promise.set("then", Value::Function(create_native_fn(
        "then",
        1,
        js_promise_then as crate::browser::js::NativeFn
    )));
    Value::Object(promise)
}

fn js_promise_then(_env: &mut Environment, args: Vec<Value>) -> Value {
    // Call the callback immediately
    if let Some(Value::Function(func)) = args.get(0) {
        if let Some(native) = func.native {
            return native(_env, Vec::new());
        }
    }
    Value::Undefined
}

fn js_audio_param_set_value_at_time(_env: &mut Environment, args: Vec<Value>) -> Value {
    // AudioParam automation - simplified
    args.get(0).cloned().unwrap_or(Value::Undefined)
}

fn js_audio_param_exponential_ramp(_env: &mut Environment, args: Vec<Value>) -> Value {
    // AudioParam automation - simplified
    args.get(0).cloned().unwrap_or(Value::Undefined)
}

/// Create an AudioParam object
fn create_audio_param(default_value: f64) -> Value {
    use crate::browser::js::Object;
    let mut param = Object::new();
    param.set("value", Value::Number(default_value));
    param.set("defaultValue", Value::Number(default_value));
    param.set("minValue", Value::Number(-3.4028235e38));
    param.set("maxValue", Value::Number(3.4028235e38));
    
    param.set("setValueAtTime", Value::Function(create_native_fn(
        "setValueAtTime",
        2,
        js_audio_param_set_value_at_time as crate::browser::js::NativeFn
    )));
    
    param.set("exponentialRampToValueAtTime", Value::Function(create_native_fn(
        "exponentialRampToValueAtTime",
        2,
        js_audio_param_exponential_ramp as crate::browser::js::NativeFn
    )));
    
    Value::Object(param)
}

pub fn init() {
    println!("[js_bindings] DOM bindings initialized");
}
