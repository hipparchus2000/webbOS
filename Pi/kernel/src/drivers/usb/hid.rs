//! USB HID (Human Interface Device) Driver
//!
//! Handles USB keyboards and mice using the boot protocol.
//! Supports standard USB HID boot keyboards (8-byte reports)
#![allow(dead_code)]

//! and boot mice (3-byte reports).

use core::sync::atomic::{AtomicBool, Ordering};
use spin::Mutex;
use lazy_static::lazy_static;

use crate::println;
use super::dwc_otg::*;
use super::UsbError;
use crate::drivers::input::{EventType, InputEvent, MOD_SHIFT, MOD_CTRL, MOD_ALT};

// =============================================================================
// HID Constants
// =============================================================================

/// HID Class Request Codes
const HID_REQ_GET_REPORT: u8   = 0x01;
const HID_REQ_GET_IDLE: u8     = 0x02;
const HID_REQ_GET_PROTOCOL: u8 = 0x03;
const HID_REQ_SET_REPORT: u8   = 0x09;
const HID_REQ_SET_IDLE: u8     = 0x0A;
const HID_REQ_SET_PROTOCOL: u8 = 0x0B;

/// HID Protocols
const PROTOCOL_BOOT: u8    = 0;
const PROTOCOL_REPORT: u8  = 1;

/// HID Report Types
const REPORT_TYPE_INPUT: u8   = 1;
const REPORT_TYPE_OUTPUT: u8  = 2;
const REPORT_TYPE_FEATURE: u8 = 3;

/// HID Subclass
const HID_SUBCLASS_BOOT: u8 = 1;

/// HID Protocol
const HID_PROTOCOL_NONE: u8     = 0;
const HID_PROTOCOL_KEYBOARD: u8 = 1;
const HID_PROTOCOL_MOUSE: u8    = 2;

/// Standard USB Keycodes to ASCII conversion table
/// Maps USB HID keycodes (0x00-0x57) to ASCII characters
const KEYCODE_TO_ASCII: [u8; 128] = [
    // 0x00 - 0x07
    0, 0, 0, 0, 'a' as u8, 'b' as u8, 'c' as u8, 'd' as u8,
    // 0x08 - 0x0F
    'e' as u8, 'f' as u8, 'g' as u8, 'h' as u8, 'i' as u8, 'j' as u8, 'k' as u8, 'l' as u8,
    // 0x10 - 0x17
    'm' as u8, 'n' as u8, 'o' as u8, 'p' as u8, 'q' as u8, 'r' as u8, 's' as u8, 't' as u8,
    // 0x18 - 0x1F
    'u' as u8, 'v' as u8, 'w' as u8, 'x' as u8, 'y' as u8, 'z' as u8, '1' as u8, '2' as u8,
    // 0x20 - 0x27
    '3' as u8, '4' as u8, '5' as u8, '6' as u8, '7' as u8, '8' as u8, '9' as u8, '0' as u8,
    // 0x28 - 0x2F
    10, 27, 8, 9, ' ' as u8, '-' as u8, '=' as u8, '[' as u8,
    // 0x30 - 0x37
    ']' as u8, '\\' as u8, 0, ';' as u8, '\'' as u8, '`' as u8, ',' as u8, '.' as u8,
    // 0x38 - 0x3F
    '/' as u8, 0, 0, 0, 0, 0, 0, 0,
    // 0x40 - 0x47
    0, 0, 0, 0, 0, 0, 0, 0,
    // 0x48 - 0x4F
    0, 0, 0, 0, 0, 0, 0, 0,
    // 0x50 - 0x57 (arrows, etc)
    0, 0, 0, 0, 0, 0, 0, 0,
    // 0x58 - 0x5F
    0, 0, 0, 0, 0, 0, 0, 0,
    // 0x60 - 0x67
    0, 0, 0, 0, 0, 0, 0, 0,
    // 0x68 - 0x6F
    0, 0, 0, 0, 0, 0, 0, 0,
    // 0x70 - 0x77 (F1-F8)
    0, 0, 0, 0, 0, 0, 0, 0,
    // 0x78 - 0x7F (F9-F12, etc)
    0, 0, 0, 0, 0, 0, 0, 0,
];

/// Shifted keycodes to ASCII
const KEYCODE_TO_ASCII_SHIFT: [u8; 128] = [
    // 0x00 - 0x07
    0, 0, 0, 0, 'A' as u8, 'B' as u8, 'C' as u8, 'D' as u8,
    // 0x08 - 0x0F
    'E' as u8, 'F' as u8, 'G' as u8, 'H' as u8, 'I' as u8, 'J' as u8, 'K' as u8, 'L' as u8,
    // 0x10 - 0x17
    'M' as u8, 'N' as u8, 'O' as u8, 'P' as u8, 'Q' as u8, 'R' as u8, 'S' as u8, 'T' as u8,
    // 0x18 - 0x1F
    'U' as u8, 'V' as u8, 'W' as u8, 'X' as u8, 'Y' as u8, 'Z' as u8, '!' as u8, '@' as u8,
    // 0x20 - 0x27
    '#' as u8, '$' as u8, '%' as u8, '^' as u8, '&' as u8, '*' as u8, '(' as u8, ')' as u8,
    // 0x28 - 0x2F
    10, 27, 8, 9, ' ' as u8, '_' as u8, '+' as u8, '{' as u8,
    // 0x30 - 0x37
    '}' as u8, '|' as u8, 0, ':' as u8, '"' as u8, '~' as u8, '<' as u8, '>' as u8,
    // 0x38 - 0x3F
    '?' as u8, 0, 0, 0, 0, 0, 0, 0,
    // 0x40 - 0x47
    0, 0, 0, 0, 0, 0, 0, 0,
    // 0x48 - 0x4F
    0, 0, 0, 0, 0, 0, 0, 0,
    // 0x50 - 0x57
    0, 0, 0, 0, 0, 0, 0, 0,
    // 0x58 - 0x5F
    0, 0, 0, 0, 0, 0, 0, 0,
    // 0x60 - 0x67
    0, 0, 0, 0, 0, 0, 0, 0,
    // 0x68 - 0x6F
    0, 0, 0, 0, 0, 0, 0, 0,
    // 0x70 - 0x77
    0, 0, 0, 0, 0, 0, 0, 0,
    // 0x78 - 0x7F
    0, 0, 0, 0, 0, 0, 0, 0,
];

/// Special keycodes
const KEYCODE_LEFT_CTRL: u8   = 0xE0;
const KEYCODE_LEFT_SHIFT: u8  = 0xE1;
const KEYCODE_LEFT_ALT: u8    = 0xE2;
const KEYCODE_LEFT_GUI: u8    = 0xE3;
const KEYCODE_RIGHT_CTRL: u8  = 0xE4;
const KEYCODE_RIGHT_SHIFT: u8 = 0xE5;
const KEYCODE_RIGHT_ALT: u8   = 0xE6;
const KEYCODE_RIGHT_GUI: u8   = 0xE7;

// =============================================================================
// HID Device State
// =============================================================================

/// Type of HID device
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HidDeviceType {
    Keyboard,
    Mouse,
    Unknown,
}

/// HID Device Information
#[derive(Debug, Clone, Copy)]
pub struct HidDevice {
    pub device_type: HidDeviceType,
    pub device_addr: u8,
    pub interface_num: u8,
    pub endpoint_in: u8,
    pub endpoint_out: Option<u8>,
    pub max_packet_size: u16,
    pub interval: u8,
    pub protocol: u8,
}

impl HidDevice {
    const fn new() -> Self {
        Self {
            device_type: HidDeviceType::Unknown,
            device_addr: 0,
            interface_num: 0,
            endpoint_in: 0,
            endpoint_out: None,
            max_packet_size: 8,
            interval: 10,
            protocol: HID_PROTOCOL_NONE,
        }
    }
}

/// Keyboard state
struct KeyboardState {
    modifier_keys: u8,
    pressed_keys: [u8; 6],
    last_pressed_keys: [u8; 6],
    shift_pressed: bool,
    ctrl_pressed: bool,
    alt_pressed: bool,
    caps_lock: bool,
    num_lock: bool,
}

impl KeyboardState {
    const fn new() -> Self {
        Self {
            modifier_keys: 0,
            pressed_keys: [0; 6],
            last_pressed_keys: [0; 6],
            shift_pressed: false,
            ctrl_pressed: false,
            alt_pressed: false,
            caps_lock: false,
            num_lock: true,
        }
    }
}

/// Mouse state
struct MouseState {
    buttons: u8,
    x: i32,
    y: i32,
    wheel: i8,
}

impl MouseState {
    const fn new() -> Self {
        Self {
            buttons: 0,
            x: 400,
            y: 300,
            wheel: 0,
        }
    }
}

/// HID Driver State
struct HidDriverState {
    initialized: bool,
    devices: [Option<HidDevice>; 4],
    keyboard: KeyboardState,
    mouse: MouseState,
    event_queue: [Option<InputEvent>; 32],
    queue_head: usize,
    queue_tail: usize,
}

impl HidDriverState {
    const fn new() -> Self {
        Self {
            initialized: false,
            devices: [None; 4],
            keyboard: KeyboardState::new(),
            mouse: MouseState::new(),
            event_queue: [None; 32],
            queue_head: 0,
            queue_tail: 0,
        }
    }
}

lazy_static! {
    static ref HID_STATE: Mutex<HidDriverState> = Mutex::new(HidDriverState::new());
}

static KEYBOARD_CONNECTED: AtomicBool = AtomicBool::new(false);
static MOUSE_CONNECTED: AtomicBool = AtomicBool::new(false);

// =============================================================================
// Initialization
// =============================================================================

/// Initialize the HID driver
pub fn init() {
    println!("[usb/hid] Initializing HID driver...");
    
    let mut state = HID_STATE.lock();
    state.initialized = true;
    
    println!("[usb/hid] HID driver initialized");
}

/// Check if keyboard is connected
pub fn is_keyboard_connected() -> bool {
    KEYBOARD_CONNECTED.load(Ordering::Relaxed)
}

/// Check if mouse is connected
pub fn is_mouse_connected() -> bool {
    MOUSE_CONNECTED.load(Ordering::Relaxed)
}

// =============================================================================
// Device Enumeration
// =============================================================================

/// Parse configuration descriptor to find HID interfaces
fn parse_hid_interfaces(device_addr: u8, config_desc: &[u8]) -> Option<HidDevice> {
    let mut offset = 0;
    let total_len = config_desc.len();
    
    let mut current_interface: Option<u8> = None;
    let mut interface_class: u8 = 0;
    let mut interface_subclass: u8 = 0;
    let mut interface_protocol: u8 = 0;
    
    while offset + 2 <= total_len {
        let desc_len = config_desc[offset] as usize;
        let desc_type = config_desc[offset + 1];
        
        if desc_len == 0 || offset + desc_len > total_len {
            break;
        }
        
        match desc_type {
            DESC_INTERFACE => {
                if desc_len >= 9 {
                    interface_class = config_desc[offset + 5];
                    interface_subclass = config_desc[offset + 6];
                    interface_protocol = config_desc[offset + 7];
                    current_interface = Some(config_desc[offset + 2]);
                    
                    println!("[usb/hid] Found interface {}: class={}, subclass={}, protocol={}",
                        current_interface.unwrap(), interface_class, interface_subclass, interface_protocol);
                }
            }
            DESC_HID => {
                println!("[usb/hid] Found HID descriptor for interface {:?}", current_interface);
            }
            DESC_ENDPOINT => {
                if desc_len >= 7 && interface_class == CLASS_HID {
                    let ep_addr = config_desc[offset + 2];
                    let ep_attrs = config_desc[offset + 3];
                    let max_packet = (config_desc[offset + 4] as u16) | ((config_desc[offset + 5] as u16) << 8);
                    let interval = config_desc[offset + 6];
                    
                    // Check if this is an interrupt IN endpoint
                    let ep_type = (ep_attrs & 0x03) as u8;
                    let is_in = (ep_addr & 0x80) != 0;
                    
                    if ep_type == EP_TYPE_INTERRUPT && is_in {
                        // Determine device type from protocol
                        let device_type = if interface_subclass == HID_SUBCLASS_BOOT {
                            match interface_protocol {
                                HID_PROTOCOL_KEYBOARD => HidDeviceType::Keyboard,
                                HID_PROTOCOL_MOUSE => HidDeviceType::Mouse,
                                _ => HidDeviceType::Unknown,
                            }
                        } else {
                            HidDeviceType::Unknown
                        };
                        
                        if device_type != HidDeviceType::Unknown {
                            println!("[usb/hid] Found {:?} endpoint: {}, max_packet={}", 
                                device_type, ep_addr, max_packet);
                            
                            return Some(HidDevice {
                                device_type,
                                device_addr,
                                interface_num: current_interface.unwrap_or(0),
                                endpoint_in: ep_addr & 0x0F,
                                endpoint_out: None,
                                max_packet_size: max_packet,
                                interval,
                                protocol: interface_protocol,
                            });
                        }
                    }
                }
            }
            _ => {}
        }
        
        offset += desc_len;
    }
    
    None
}

/// Set HID protocol (boot or report)
fn set_protocol(device: &HidDevice, protocol: u8) -> Result<(), UsbError> {
    let setup = SetupPacket::new(
        0x21, // Class request, interface
        HID_REQ_SET_PROTOCOL,
        protocol as u16,
        device.interface_num as u16,
        0,
    );
    
    control_transfer(device.device_addr, 0, &setup, None, 0)?;
    println!("[usb/hid] Set protocol to {}", if protocol == PROTOCOL_BOOT { "boot" } else { "report" });
    Ok(())
}

/// Set idle rate
fn set_idle(device: &HidDevice, duration: u8, report_id: u8) -> Result<(), UsbError> {
    let setup = SetupPacket::new(
        0x21, // Class request, interface
        HID_REQ_SET_IDLE,
        ((duration as u16) << 8) | (report_id as u16),
        device.interface_num as u16,
        0,
    );
    
    control_transfer(device.device_addr, 0, &setup, None, 0)?;
    Ok(())
}

/// Enumerate and configure a HID device
pub fn enumerate_device(device_addr: u8) -> Result<(), UsbError> {
    println!("[usb/hid] Enumerating HID device at address {}", device_addr);
    
    // Get device descriptor
    let dev_desc = get_device_descriptor(device_addr)?;
    println!("[usb/hid] Device: class={}, subclass={}, protocol={}",
        dev_desc.b_device_class, dev_desc.b_device_subclass, dev_desc.b_device_protocol);
    
    // Get configuration descriptor
    let mut config_buffer = [0u8; 256];
    let config_len = get_config_descriptor(device_addr, &mut config_buffer)?;
    
    if config_len < 9 {
        return Err(UsbError::InvalidDescriptor);
    }
    
    // Parse and find HID interfaces
    let hid_device = parse_hid_interfaces(device_addr, &config_buffer[..config_len]);
    
    if let Some(device) = hid_device {
        // Set configuration
        set_configuration(device_addr, 1)?;
        
        // Set boot protocol
        set_protocol(&device, PROTOCOL_BOOT)?;
        
        // Set idle rate (no report until data changes)
        set_idle(&device, 0, 0)?;
        
        // Add to device list
        let mut state = HID_STATE.lock();
        for i in 0..state.devices.len() {
            if state.devices[i].is_none() {
                // Update connection status
                match device.device_type {
                    HidDeviceType::Keyboard => {
                        KEYBOARD_CONNECTED.store(true, Ordering::Relaxed);
                        println!("[usb/hid] Keyboard connected at address {}", device_addr);
                    }
                    HidDeviceType::Mouse => {
                        MOUSE_CONNECTED.store(true, Ordering::Relaxed);
                        println!("[usb/hid] Mouse connected at address {}", device_addr);
                    }
                    _ => {}
                }
                
                state.devices[i] = Some(device);
                break;
            }
        }
    } else {
        println!("[usb/hid] No HID interface found");
    }
    
    Ok(())
}

// =============================================================================
// Report Processing
// =============================================================================

/// Process a keyboard boot report (8 bytes)
fn process_keyboard_report(report: &[u8]) {
    if report.len() < 8 {
        return;
    }
    
    let mut state = HID_STATE.lock();
    
    // Update modifier state
    let modifiers = report[0];
    state.keyboard.modifier_keys = modifiers;
    state.keyboard.ctrl_pressed = (modifiers & 0x11) != 0; // Left or right ctrl
    state.keyboard.shift_pressed = (modifiers & 0x22) != 0; // Left or right shift
    state.keyboard.alt_pressed = (modifiers & 0x44) != 0; // Left or right alt
    
    // Save last pressed keys
    state.keyboard.last_pressed_keys = state.keyboard.pressed_keys;
    
    // Get current pressed keys (bytes 2-7)
    let mut new_keys = [0u8; 6];
    for i in 0..6 {
        new_keys[i] = report[2 + i];
    }
    state.keyboard.pressed_keys = new_keys;
    
    // Find released keys (in last but not in current)
    for i in 0..6 {
        let key = state.keyboard.last_pressed_keys[i];
        if key != 0 && !new_keys.contains(&key) {
            // Key released
            let ascii = keycode_to_ascii(key, state.keyboard.shift_pressed, state.keyboard.caps_lock);
            
            let event = InputEvent {
                event_type: EventType::KeyRelease,
                keycode: key as u16,
                ascii,
                x: 0, y: 0, button: 0, scroll: 0,
                modifiers: build_modifiers(&state.keyboard),
            };
            
            queue_event(&mut state, event);
        }
    }
    
    // Find newly pressed keys
    for i in 0..6 {
        let key = new_keys[i];
        if key != 0 && !state.keyboard.last_pressed_keys.contains(&key) {
            // Key pressed
            let ascii = keycode_to_ascii(key, state.keyboard.shift_pressed, state.keyboard.caps_lock);
            
            // Handle special keys
            match key {
                0x39 => { // Caps Lock
                    state.keyboard.caps_lock = !state.keyboard.caps_lock;
                }
                0x53 => { // Num Lock
                    state.keyboard.num_lock = !state.keyboard.num_lock;
                }
                _ => {}
            }
            
            let event = InputEvent {
                event_type: EventType::KeyPress,
                keycode: key as u16,
                ascii,
                x: 0, y: 0, button: 0, scroll: 0,
                modifiers: build_modifiers(&state.keyboard),
            };
            
            queue_event(&mut state, event);
        }
    }
}

/// Process a mouse boot report (3 or 4 bytes)
fn process_mouse_report(report: &[u8]) {
    if report.len() < 3 {
        return;
    }
    
    let mut state = HID_STATE.lock();
    
    let buttons = report[0];
    let x_movement = report[1] as i8 as i32;
    let y_movement = report[2] as i8 as i32;
    let _wheel = if report.len() >= 4 { report[3] as i8 } else { 0 };
    
    // Update position
    state.mouse.x += x_movement;
    state.mouse.y += y_movement;
    state.mouse.buttons = buttons;
    
    // Clamp to screen (use default if not set)
    state.mouse.x = state.mouse.x.max(0).min(1279);
    state.mouse.y = state.mouse.y.max(0).min(799);
    
    // Generate movement event if position changed
    if x_movement != 0 || y_movement != 0 {
        let event = InputEvent {
            event_type: EventType::MouseMove,
            keycode: 0,
            ascii: 0,
            x: state.mouse.x,
            y: state.mouse.y,
            button: buttons,
            scroll: 0,
            modifiers: 0,
        };
        
        queue_event(&mut state, event);
    }
    
    // Check for button changes
    let last_buttons = state.mouse.buttons;
    let changed_buttons = last_buttons ^ buttons;
    
    for i in 0..3 {
        let mask = 1 << i;
        if (changed_buttons & mask) != 0 {
            let event = InputEvent {
                event_type: if (buttons & mask) != 0 {
                    EventType::MouseButtonPress
                } else {
                    EventType::MouseButtonRelease
                },
                keycode: 0,
                ascii: 0,
                x: state.mouse.x,
                y: state.mouse.y,
                button: i as u8,
                scroll: 0,
                modifiers: 0,
            };
            
            queue_event(&mut state, event);
        }
    }
    
    // Update stored buttons
    state.mouse.buttons = buttons;
}

/// Convert USB keycode to ASCII
fn keycode_to_ascii(keycode: u8, shift: bool, caps: bool) -> u8 {
    if keycode >= 0x80 {
        return 0;
    }
    
    // Handle letters with caps lock
    let is_letter = (0x04..=0x1D).contains(&keycode);
    let use_shift = if is_letter { shift ^ caps } else { shift };
    
    if use_shift {
        KEYCODE_TO_ASCII_SHIFT[keycode as usize]
    } else {
        KEYCODE_TO_ASCII[keycode as usize]
    }
}

/// Build modifier byte from keyboard state
fn build_modifiers(kbd: &KeyboardState) -> u8 {
    let mut mods = 0u8;
    if kbd.shift_pressed { mods |= MOD_SHIFT; }
    if kbd.ctrl_pressed { mods |= MOD_CTRL; }
    if kbd.alt_pressed { mods |= MOD_ALT; }
    if kbd.caps_lock { mods |= 0x08; }
    if kbd.num_lock { mods |= 0x10; }
    mods
}

/// Queue an input event
fn queue_event(state: &mut HidDriverState, event: InputEvent) {
    let next_tail = (state.queue_tail + 1) % state.event_queue.len();
    
    if next_tail != state.queue_head {
        state.event_queue[state.queue_tail] = Some(event);
        state.queue_tail = next_tail;
    }
}

/// Poll for HID input from all connected devices
pub fn poll() {
    let mut state = HID_STATE.lock();
    
    for i in 0..state.devices.len() {
        if let Some(device) = state.devices[i] {
            match device.device_type {
                HidDeviceType::Keyboard => {
                    let mut report = [0u8; 8];
                    match interrupt_in_transfer(
                        device.device_addr,
                        device.endpoint_in,
                        device.max_packet_size,
                        &mut report
                    ) {
                        Ok(_) => {
                            drop(state);
                            process_keyboard_report(&report);
                            state = HID_STATE.lock();
                        }
                        Err(UsbError::Timeout) => {
                            // No data available, this is normal
                        }
                        Err(_e) => {
                            // Other error, might be device disconnected
                        }
                    }
                }
                HidDeviceType::Mouse => {
                    let mut report = [0u8; 4];
                    match interrupt_in_transfer(
                        device.device_addr,
                        device.endpoint_in,
                        device.max_packet_size,
                        &mut report
                    ) {
                        Ok(_) => {
                            drop(state);
                            process_mouse_report(&report);
                            state = HID_STATE.lock();
                        }
                        Err(UsbError::Timeout) => {
                            // No data available
                        }
                        Err(_e) => {
                            // Other error
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

/// Get the next input event from the queue
pub fn get_event() -> Option<InputEvent> {
    let mut state = HID_STATE.lock();
    
    if state.queue_head == state.queue_tail {
        return None;
    }
    
    let event = state.event_queue[state.queue_head];
    state.queue_head = (state.queue_head + 1) % state.event_queue.len();
    
    event
}

/// Get current mouse position
pub fn get_mouse_position() -> (i32, i32) {
    let state = HID_STATE.lock();
    (state.mouse.x, state.mouse.y)
}

/// Set mouse position
pub fn set_mouse_position(x: i32, y: i32) {
    let mut state = HID_STATE.lock();
    state.mouse.x = x.max(0).min(1279);
    state.mouse.y = y.max(0).min(799);
}

/// Get current mouse buttons
pub fn get_mouse_buttons() -> u8 {
    let state = HID_STATE.lock();
    state.mouse.buttons
}

/// Get keyboard state
pub fn get_keyboard_state() -> (bool, bool, bool, bool, bool) {
    let state = HID_STATE.lock();
    (
        state.keyboard.shift_pressed,
        state.keyboard.ctrl_pressed,
        state.keyboard.alt_pressed,
        state.keyboard.caps_lock,
        state.keyboard.num_lock,
    )
}

/// Print HID status information
pub fn print_info() {
    let state = HID_STATE.lock();
    
    println!("HID Status:");
    println!("  Initialized: {}", state.initialized);
    println!("  Keyboard connected: {}", is_keyboard_connected());
    println!("  Mouse connected: {}", is_mouse_connected());
    println!("  Mouse position: ({}, {})", state.mouse.x, state.mouse.y);
    println!("  Mouse buttons: {:03b}", state.mouse.buttons);
    println!("  Event queue: head={}, tail={}", state.queue_head, state.queue_tail);
    
    for (i, device) in state.devices.iter().enumerate() {
        if let Some(dev) = device {
            println!("  Device {}: {:?} at addr {}, ep {}",
                i, dev.device_type, dev.device_addr, dev.endpoint_in);
        }
    }
}
