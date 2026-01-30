//! Input Subsystem
//!
//! Handles keyboard and mouse input for WebbOS.

use spin::Mutex;
use lazy_static::lazy_static;
use alloc::collections::VecDeque;

use crate::println;
// Port I/O functions
#[inline]
pub unsafe fn inb(port: u16) -> u8 {
    let result: u8;
    core::arch::asm!(
        "in al, dx",
        in("dx") port,
        out("al") result,
        options(nomem, nostack)
    );
    result
}

#[inline]
pub unsafe fn outb(port: u16, value: u8) {
    core::arch::asm!(
        "out dx, al",
        in("dx") port,
        in("al") value,
        options(nomem, nostack)
    );
}

#[inline]
pub unsafe fn inw(port: u16) -> u16 {
    let result: u16;
    core::arch::asm!(
        "in ax, dx",
        in("dx") port,
        out("ax") result,
        options(nomem, nostack)
    );
    result
}

#[inline]
pub unsafe fn outw(port: u16, value: u16) {
    core::arch::asm!(
        "out dx, ax",
        in("dx") port,
        in("ax") value,
        options(nomem, nostack)
    );
}

/// Maximum event queue size
const MAX_EVENTS: usize = 256;

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

/// Keyboard driver
pub struct KeyboardDriver {
    shift_pressed: bool,
    ctrl_pressed: bool,
    alt_pressed: bool,
    caps_lock: bool,
    num_lock: bool,
}

impl KeyboardDriver {
    const fn new() -> Self {
        Self {
            shift_pressed: false,
            ctrl_pressed: false,
            alt_pressed: false,
            caps_lock: false,
            num_lock: true,
        }
    }
    
    pub fn init(&mut self) {
        println!("[input] Initializing keyboard...");
        
        unsafe {
            let ctrl = inb(0x61);
            outb(0x61, ctrl | 0x80);
            outb(0x61, ctrl & 0x7F);
            
            while inb(0x64) & 0x01 != 0 {
                inb(0x60);
            }
        }
        
        println!("[input] Keyboard initialized");
    }
    
    pub fn handle_interrupt(&mut self) -> Option<InputEvent> {
        let scancode = unsafe { inb(0x60) };
        
        if scancode == 0xE0 {
            return None;
        }
        
        let is_release = scancode & 0x80 != 0;
        let keycode = scancode & 0x7F;
        
        match keycode {
            0x2A | 0x36 => self.shift_pressed = !is_release,
            0x1D => self.ctrl_pressed = !is_release,
            0x38 => self.alt_pressed = !is_release,
            0x3A => if is_release { self.caps_lock = !self.caps_lock; }
            0x45 => if is_release { self.num_lock = !self.num_lock; }
            _ => {}
        }
        
        let mut modifiers = 0u8;
        if self.shift_pressed { modifiers |= MOD_SHIFT; }
        if self.ctrl_pressed { modifiers |= MOD_CTRL; }
        if self.alt_pressed { modifiers |= MOD_ALT; }
        if self.caps_lock { modifiers |= MOD_CAPS; }
        if self.num_lock { modifiers |= MOD_NUM; }
        
        let ascii = if is_release {
            0
        } else {
            scancode_to_ascii(keycode, self.shift_pressed, self.caps_lock)
        };
        
        Some(InputEvent {
            event_type: if is_release { EventType::KeyRelease } else { EventType::KeyPress },
            keycode: keycode as u16,
            ascii,
            x: 0, y: 0, button: 0, scroll: 0, modifiers,
        })
    }
}

fn scancode_to_ascii(scancode: u8, shift: bool, caps: bool) -> u8 {
    let base_table: [u8; 128] = [
        0, 27, 49, 50, 51, 52, 53, 54,
        55, 56, 57, 48, 45, 61, 8, 9,
        113, 119, 101, 114, 116, 121, 117, 105,
        111, 112, 91, 93, 10, 0, 97, 115,
        100, 102, 103, 104, 106, 107, 108, 59,
        39, 96, 0, 92, 122, 120, 99, 118,
        98, 110, 109, 44, 46, 47, 0, 42,
        0, 32, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
    ];
    
    let shift_table: [u8; 128] = [
        0, 27, 33, 64, 35, 36, 37, 94,
        38, 42, 40, 41, 95, 43, 8, 9,
        81, 87, 69, 82, 84, 89, 85, 73,
        79, 80, 123, 125, 10, 0, 65, 83,
        68, 70, 71, 72, 74, 75, 76, 58,
        34, 126, 0, 124, 90, 88, 67, 86,
        66, 78, 77, 60, 62, 63, 0, 42,
        0, 32, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
    ];
    
    if scancode >= 128 {
        return 0;
    }
    
    let use_shift = shift ^ caps;
    if use_shift {
        shift_table[scancode as usize]
    } else {
        base_table[scancode as usize]
    }
}

/// Mouse driver
pub struct MouseDriver {
    x: i32, y: i32,
    buttons: u8,
    cycle: u8,
    packet: [u8; 4],
}

impl MouseDriver {
    const fn new() -> Self {
        Self { x: 400, y: 300, buttons: 0, cycle: 0, packet: [0; 4] }
    }
    
    pub fn init(&mut self) {
        println!("[input] Initializing mouse...");
        
        unsafe {
            self.wait_write();
            outb(0x64, 0xA8);
            
            self.wait_write();
            outb(0x64, 0x20);
            self.wait_read();
            let status = (inb(0x60) | 2) & 0xDF;
            
            self.wait_write();
            outb(0x64, 0x60);
            self.wait_write();
            outb(0x60, status);
            
            self.write(0xF6);
            self.read();
            
            self.write(0xF4);
            self.read();
        }
        
        println!("[input] Mouse initialized");
    }
    
    pub fn handle_interrupt(&mut self) -> Option<InputEvent> {
        let data = unsafe { inb(0x60) };
        
        match self.cycle {
            0 => {
                if data & 0x08 != 0 {
                    self.packet[0] = data;
                    self.cycle = 1;
                }
                None
            }
            1 => {
                self.packet[1] = data;
                self.cycle = 2;
                None
            }
            2 => {
                self.packet[2] = data;
                self.cycle = 0;
                self.process_packet()
            }
            _ => {
                self.cycle = 0;
                None
            }
        }
    }
    
    fn process_packet(&mut self) -> Option<InputEvent> {
        let flags = self.packet[0];
        let x_movement = self.packet[1] as i8 as i16;
        let y_movement = self.packet[2] as i8 as i16;
        
        let x_delta = x_movement as i32;
        let y_delta = y_movement as i32;
        
        self.x += x_delta;
        self.y -= y_delta;
        
        self.x = self.x.max(0).min(1023);
        self.y = self.y.max(0).min(767);
        
        let new_buttons = flags & 0x07;
        let button_change = self.buttons ^ new_buttons;
        self.buttons = new_buttons;
        
        if x_delta != 0 || y_delta != 0 {
            Some(InputEvent {
                event_type: EventType::MouseMove,
                keycode: 0, ascii: 0, x: self.x, y: self.y,
                button: new_buttons, scroll: 0, modifiers: 0,
            })
        } else if button_change != 0 {
            let button = button_change.trailing_zeros() as u8;
            let pressed = new_buttons & button_change != 0;
            
            Some(InputEvent {
                event_type: if pressed { EventType::MouseButtonPress } else { EventType::MouseButtonRelease },
                keycode: 0, ascii: 0, x: self.x, y: self.y,
                button, scroll: 0, modifiers: 0,
            })
        } else {
            None
        }
    }
    
    pub fn position(&self) -> (i32, i32) { (self.x, self.y) }
    pub fn set_position(&mut self, x: i32, y: i32) { self.x = x; self.y = y; }
    pub fn buttons(&self) -> u8 { self.buttons }
    
    fn wait_write(&self) { unsafe { while inb(0x64) & 0x02 != 0 {} } }
    fn wait_read(&self) { unsafe { while inb(0x64) & 0x01 == 0 {} } }
    
    fn write(&self, data: u8) {
        unsafe {
            self.wait_write();
            outb(0x64, 0xD4);
            self.wait_write();
            outb(0x60, data);
        }
    }
    
    fn read(&self) -> u8 {
        unsafe {
            self.wait_read();
            inb(0x60)
        }
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
        Self { keyboard: KeyboardDriver::new(), mouse: MouseDriver::new(), events: VecDeque::new() }
    }
    
    pub fn init(&mut self) {
        self.keyboard.init();
        self.mouse.init();
    }
    
    pub fn handle_keyboard(&mut self) {
        if let Some(event) = self.keyboard.handle_interrupt() {
            if self.events.len() < MAX_EVENTS {
                self.events.push_back(event);
            }
        }
    }
    
    pub fn handle_mouse(&mut self) {
        if let Some(event) = self.mouse.handle_interrupt() {
            if self.events.len() < MAX_EVENTS {
                self.events.push_back(event);
            }
        }
    }
    
    pub fn poll_event(&mut self) -> Option<InputEvent> { self.events.pop_front() }
    pub fn has_events(&self) -> bool { !self.events.is_empty() }
    pub fn mouse_position(&self) -> (i32, i32) { self.mouse.position() }
    pub fn set_mouse_position(&mut self, x: i32, y: i32) { self.mouse.set_position(x, y); }
    pub fn mouse_buttons(&self) -> u8 { self.mouse.buttons() }
}

lazy_static! {
    static ref INPUT_MANAGER: Mutex<InputManager> = Mutex::new(InputManager::new());
}

pub fn init() {
    println!("[input] Initializing input subsystem...");
    INPUT_MANAGER.lock().init();
    println!("[input] Input subsystem ready");
}

pub fn handle_keyboard_interrupt() { INPUT_MANAGER.lock().handle_keyboard(); }
pub fn handle_mouse_interrupt() { INPUT_MANAGER.lock().handle_mouse(); }
pub fn poll_event() -> Option<InputEvent> { INPUT_MANAGER.lock().poll_event() }
pub fn has_events() -> bool { INPUT_MANAGER.lock().has_events() }
pub fn mouse_position() -> (i32, i32) { INPUT_MANAGER.lock().mouse_position() }

pub fn wait_key() -> InputEvent {
    loop {
        if let Some(event) = poll_event() {
            if event.event_type == EventType::KeyPress {
                return event;
            }
        }
        core::hint::spin_loop();
    }
}

pub fn get_key() -> Option<InputEvent> {
    if let Some(event) = poll_event() {
        if event.event_type == EventType::KeyPress {
            return Some(event);
        }
    }
    None
}

pub fn print_info() {
    let manager = INPUT_MANAGER.lock();
    let (x, y) = manager.mouse_position();
    println!("Input Status:");
    println!("  Mouse position: ({}, {})", x, y);
    println!("  Mouse buttons: {:03b}", manager.mouse_buttons());
    println!("  Events in queue: {}", manager.events.len());
}
