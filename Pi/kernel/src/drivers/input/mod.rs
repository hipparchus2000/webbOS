//! Input Subsystem
//!
//! Handles keyboard and mouse input for WebbOS on Raspberry Pi.
//! Uses USB HID via the DWC OTG controller instead of PS/2.
//!
//! ARCHITECTURE:
//! - USB polling at 1000Hz from timer
//! - Events queued and processed by desktop/input handlers
//! - Atomic mouse position for thread safety

use spin::Mutex;
use lazy_static::lazy_static;
use alloc::collections::VecDeque;
use core::sync::atomic::{AtomicU64, AtomicI32, Ordering};

use crate::println;
use crate::drivers::usb::hid;

/// Maximum event queue size
const MAX_EVENTS: usize = 64;

// =============================================================================
// ATOMIC MOUSE STATE - Updated by USB poll, read by timer
// =============================================================================

/// Atomic mouse X position (updated by USB poll, read by timer)
static MOUSE_X: AtomicI32 = AtomicI32::new(400);
/// Atomic mouse Y position (updated by USB poll, read by timer)
static MOUSE_Y: AtomicI32 = AtomicI32::new(300);
/// Atomic mouse buttons state
static MOUSE_BTNS: AtomicU64 = AtomicU64::new(0);
/// Mouse poll counter (for diagnostics)
static MOUSE_POLL_COUNT: AtomicU64 = AtomicU64::new(0);
/// Keyboard poll counter (for diagnostics)
static KEYBOARD_POLL_COUNT: AtomicU64 = AtomicU64::new(0);
/// Last reported X (for delta calculation)
static LAST_MOUSE_X: AtomicI32 = AtomicI32::new(400);
/// Last reported Y (for delta calculation)
static LAST_MOUSE_Y: AtomicI32 = AtomicI32::new(300);
/// Screen dimensions for clamping
static SCREEN_WIDTH: AtomicI32 = AtomicI32::new(1280);
static SCREEN_HEIGHT: AtomicI32 = AtomicI32::new(800);

/// Input event types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    KeyPress,
    KeyRelease,
    MouseMove,
    MouseButtonPress,
    MouseButtonRelease,
    MouseScroll,
}

/// Mouse buttons
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left = 0,
    Right = 1,
    Middle = 2,
}

/// Input event
#[derive(Debug, Clone, Copy)]
pub struct InputEvent {
    pub event_type: EventType,
    pub keycode: u16,
    pub ascii: u8,
    pub x: i32,
    pub y: i32,
    pub button: u8,
    pub scroll: i8,
    pub modifiers: u8,
}

/// Key modifiers
pub const MOD_SHIFT: u8 = 0x01;
pub const MOD_CTRL: u8 = 0x02;
pub const MOD_ALT: u8 = 0x04;
pub const MOD_CAPS: u8 = 0x08;
pub const MOD_NUM: u8 = 0x10;

/// USB Keyboard driver wrapper
pub struct KeyboardDriver {
    shift_pressed: bool,
    ctrl_pressed: bool,
    alt_pressed: bool,
    caps_lock: bool,
    num_lock: bool,
}

impl KeyboardDriver {
    pub const fn new() -> Self {
        Self {
            shift_pressed: false,
            ctrl_pressed: false,
            alt_pressed: false,
            caps_lock: false,
            num_lock: true,
        }
    }
    
    pub fn init(&mut self) {
        println!("[input] Initializing USB keyboard...");
        
        if crate::drivers::usb::is_keyboard_connected() {
            println!("[input] USB keyboard detected");
        } else {
            println!("[input] No USB keyboard connected (will use if connected later)");
        }
    }
    
    /// Process an input event from USB HID
    pub fn process_event(&mut self, event: &InputEvent) -> Option<InputEvent> {
        // Update modifier state
        self.shift_pressed = (event.modifiers & MOD_SHIFT) != 0;
        self.ctrl_pressed = (event.modifiers & MOD_CTRL) != 0;
        self.alt_pressed = (event.modifiers & MOD_ALT) != 0;
        self.caps_lock = (event.modifiers & MOD_CAPS) != 0;
        self.num_lock = (event.modifiers & MOD_NUM) != 0;
        
        Some(*event)
    }
}

/// USB Mouse driver wrapper
pub struct MouseDriver {
    x: i32,
    y: i32,
    buttons: u8,
    screen_width: i32,
    screen_height: i32,
}

impl MouseDriver {
    pub const fn new() -> Self {
        Self { 
            x: 400, 
            y: 300, 
            buttons: 0,
            screen_width: 1280,
            screen_height: 800,
        }
    }
    
    /// Set screen dimensions for mouse clamping
    pub fn set_screen_dimensions(&mut self, width: i32, height: i32) {
        self.screen_width = width;
        self.screen_height = height;
        
        // Update atomic screen dimensions
        SCREEN_WIDTH.store(width, Ordering::Relaxed);
        SCREEN_HEIGHT.store(height, Ordering::Relaxed);
        
        // Clamp current position to new bounds
        let current_x = MOUSE_X.load(Ordering::Relaxed);
        let current_y = MOUSE_Y.load(Ordering::Relaxed);
        MOUSE_X.store(current_x.max(0).min(width - 1), Ordering::Relaxed);
        MOUSE_Y.store(current_y.max(0).min(height - 1), Ordering::Relaxed);
    }
    
    pub fn init(&mut self) {
        println!("[input] Initializing USB mouse...");
        
        if crate::drivers::usb::is_mouse_connected() {
            println!("[input] USB mouse detected");
        } else {
            println!("[input] No USB mouse connected (will use if connected later)");
        }
    }
    
    /// Process an input event from USB HID
    pub fn process_event(&mut self, event: &InputEvent) -> Option<InputEvent> {
        match event.event_type {
            EventType::MouseMove => {
                // Update atomic position
                MOUSE_X.store(event.x, Ordering::Relaxed);
                MOUSE_Y.store(event.y, Ordering::Relaxed);
                MOUSE_BTNS.store(event.button as u64, Ordering::Relaxed);
                self.x = event.x;
                self.y = event.y;
                MOUSE_POLL_COUNT.fetch_add(1, Ordering::Relaxed);
            }
            EventType::MouseButtonPress | EventType::MouseButtonRelease => {
                MOUSE_BTNS.store(event.button as u64, Ordering::Relaxed);
                self.buttons = event.button;
            }
            _ => {}
        }
        
        Some(*event)
    }
    
    pub fn position(&self) -> (i32, i32) { 
        (MOUSE_X.load(Ordering::Relaxed), MOUSE_Y.load(Ordering::Relaxed))
    }
    
    pub fn set_position(&mut self, x: i32, y: i32) { 
        self.x = x; 
        self.y = y;
        MOUSE_X.store(x, Ordering::Relaxed);
        MOUSE_Y.store(y, Ordering::Relaxed);
    }
    
    pub fn buttons(&self) -> u8 { 
        MOUSE_BTNS.load(Ordering::Relaxed) as u8
    }
}

/// Input manager
pub struct InputManager {
    keyboard: KeyboardDriver,
    mouse: MouseDriver,
    events: VecDeque<InputEvent>,
}

impl InputManager {
    const fn new() -> Self {
        Self { 
            keyboard: KeyboardDriver::new(), 
            mouse: MouseDriver::new(), 
            events: VecDeque::new() 
        }
    }
    
    pub fn init(&mut self) {
        self.keyboard.init();
        self.mouse.init();
    }
    
    pub fn poll_event(&mut self) -> Option<InputEvent> { 
        self.events.pop_front() 
    }
    
    pub fn has_events(&self) -> bool { 
        !self.events.is_empty() 
    }
    
    pub fn event_queue_len(&self) -> usize { 
        self.events.len() 
    }
    
    pub fn mouse_position(&self) -> (i32, i32) { 
        self.mouse.position() 
    }
    
    pub fn set_mouse_position(&mut self, x: i32, y: i32) { 
        self.mouse.set_position(x, y); 
    }
    
    pub fn mouse_buttons(&self) -> u8 { 
        self.mouse.buttons() 
    }
    
    pub fn set_mouse_dimensions(&mut self, width: i32, height: i32) { 
        self.mouse.set_screen_dimensions(width, height); 
    }
    
    /// Add an event to the queue
    pub fn queue_event(&mut self, event: InputEvent) {
        if self.events.len() < MAX_EVENTS {
            self.events.push_back(event);
        }
    }
}

lazy_static! {
    static ref INPUT_MANAGER: Mutex<InputManager> = Mutex::new(InputManager::new());
}

/// Initialize the input subsystem
pub fn init() {
    println!("[input] Initializing input subsystem (USB HID)...");
    
    INPUT_MANAGER.lock().init();
    
    // Initialize last position from current position
    LAST_MOUSE_X.store(MOUSE_X.load(Ordering::Relaxed), Ordering::Relaxed);
    LAST_MOUSE_Y.store(MOUSE_Y.load(Ordering::Relaxed), Ordering::Relaxed);
    
    println!("[input] Input subsystem ready (USB HID)");
}

/// Poll USB HID for new input events
/// 
/// This should be called regularly (e.g., from timer interrupt)
pub fn poll() {
    // Poll the USB subsystem
    crate::drivers::usb::poll();
    
    // Process any events from USB HID
    let mut manager = INPUT_MANAGER.lock();
    
    while let Some(event) = crate::drivers::usb::get_event() {
        match event.event_type {
            EventType::KeyPress | EventType::KeyRelease => {
                KEYBOARD_POLL_COUNT.fetch_add(1, Ordering::Relaxed);
                if let Some(processed) = manager.keyboard.process_event(&event) {
                    manager.queue_event(processed);
                }
            }
            EventType::MouseMove | EventType::MouseButtonPress | EventType::MouseButtonRelease => {
                if let Some(processed) = manager.mouse.process_event(&event) {
                    manager.queue_event(processed);
                }
            }
            _ => {}
        }
    }
}

/// Poll mouse from timer (20Hz) - generates events, does printing
/// This is where all the heavy lifting happens!
pub fn poll_mouse_from_timer() -> Option<InputEvent> {
    // Read current atomic position
    let current_x = MOUSE_X.load(Ordering::Relaxed);
    let current_y = MOUSE_Y.load(Ordering::Relaxed);
    let buttons = MOUSE_BTNS.load(Ordering::Relaxed) as u8;
    
    // Read last reported position
    let last_x = LAST_MOUSE_X.load(Ordering::Relaxed);
    let last_y = LAST_MOUSE_Y.load(Ordering::Relaxed);
    
    // Check if position changed
    if current_x != last_x || current_y != last_y {
        // Update last reported position
        LAST_MOUSE_X.store(current_x, Ordering::Relaxed);
        LAST_MOUSE_Y.store(current_y, Ordering::Relaxed);
        
        // Return movement event
        return Some(InputEvent {
            event_type: EventType::MouseMove,
            keycode: 0,
            ascii: 0,
            x: current_x,
            y: current_y,
            button: buttons,
            scroll: 0,
            modifiers: 0,
        });
    }
    
    None
}

/// Get current mouse position (from atomics)
pub fn mouse_position() -> (i32, i32) { 
    (MOUSE_X.load(Ordering::Relaxed), MOUSE_Y.load(Ordering::Relaxed))
}

/// Get mouse buttons (from atomics)
pub fn mouse_buttons() -> u8 {
    MOUSE_BTNS.load(Ordering::Relaxed) as u8
}

/// Set screen dimensions for mouse clamping
pub fn set_mouse_screen_dimensions(width: i32, height: i32) {
    SCREEN_WIDTH.store(width, Ordering::Relaxed);
    SCREEN_HEIGHT.store(height, Ordering::Relaxed);
    
    // Re-clamp current mouse position to new bounds
    let current_x = MOUSE_X.load(Ordering::Relaxed);
    let current_y = MOUSE_Y.load(Ordering::Relaxed);
    let clamped_x = current_x.max(0).min(width - 1);
    let clamped_y = current_y.max(0).min(height - 1);
    MOUSE_X.store(clamped_x, Ordering::Relaxed);
    MOUSE_Y.store(clamped_y, Ordering::Relaxed);
    LAST_MOUSE_X.store(clamped_x, Ordering::Relaxed);
    LAST_MOUSE_Y.store(clamped_y, Ordering::Relaxed);
    
    // Also update the mouse driver
    INPUT_MANAGER.lock().set_mouse_dimensions(width, height);
}

/// Get interrupt counters for diagnostics
/// 
/// Note: On USB, these are poll counts, not IRQ counts
pub fn get_irq_counts() -> (u64, u64) {
    (
        KEYBOARD_POLL_COUNT.load(Ordering::Relaxed), 
        MOUSE_POLL_COUNT.load(Ordering::Relaxed)
    )
}

/// Poll keyboard from timer (20Hz) - generates events
pub fn poll_keyboard_from_timer() -> Option<InputEvent> {
    INPUT_MANAGER.lock().poll_event()
}

/// Legacy poll_event - combines keyboard queue and mouse timer
pub fn poll_event() -> Option<InputEvent> {
    // First check keyboard queue
    if let Some(event) = INPUT_MANAGER.lock().poll_event() {
        return Some(event);
    }
    // Then check mouse timer
    poll_mouse_from_timer()
}

/// Poll keyboard for input (non-interrupt mode) - legacy
pub fn poll_keyboard() -> Option<InputEvent> {
    INPUT_MANAGER.lock().poll_event()
}

/// Wait for a key press
pub fn wait_key() -> InputEvent {
    loop {
        // Poll USB for new events
        poll();
        
        if let Some(event) = poll_event() {
            if event.event_type == EventType::KeyPress {
                return event;
            }
        }
        
        core::hint::spin_loop();
    }
}

/// Get a key press if available
pub fn get_key() -> Option<InputEvent> {
    // Poll USB for new events
    poll();
    
    if let Some(event) = poll_event() {
        if event.event_type == EventType::KeyPress {
            return Some(event);
        }
    }
    None
}

/// Print input status information
pub fn print_info() {
    let manager = INPUT_MANAGER.lock();
    let (x, y) = manager.mouse_position();
    let (kb_polls, mouse_polls) = get_irq_counts();
    
    println!("Input Status:");
    println!("  Mouse position: ({}, {})", x, y);
    println!("  Mouse buttons: {:03b}", manager.mouse_buttons());
    println!("  Events in queue: {}", manager.events.len());
    println!("  Keyboard polls: {}", kb_polls);
    println!("  Mouse polls: {}", mouse_polls);
    println!("  USB Keyboard: {}", 
        if crate::drivers::usb::is_keyboard_connected() { "connected" } else { "not connected" });
    println!("  USB Mouse: {}", 
        if crate::drivers::usb::is_mouse_connected() { "connected" } else { "not connected" });
}

// =============================================================================
// Legacy IRQ handler stubs (for compatibility with existing code)
// =============================================================================

/// Keyboard interrupt handler - legacy stub
/// 
/// On USB, there's no keyboard IRQ. Events are polled.
pub fn handle_keyboard_interrupt() { 
    // USB HID uses polling, not interrupts
    KEYBOARD_POLL_COUNT.fetch_add(1, Ordering::Relaxed);
}

/// Mouse interrupt handler - legacy stub
/// 
/// On USB, there's no mouse IRQ. Events are polled.
pub fn handle_mouse_interrupt() { 
    // USB HID uses polling, not interrupts
    MOUSE_POLL_COUNT.fetch_add(1, Ordering::Relaxed);
}
