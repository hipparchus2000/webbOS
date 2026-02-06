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

/// Maximum event queue size (increased from 256 to handle burst events)
const MAX_EVENTS: usize = 512;

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
        println!("[input] Initializing keyboard...");

        unsafe {
            let ctrl = inb(0x61);
            outb(0x61, ctrl | 0x80);
            outb(0x61, ctrl & 0x7F);

            while inb(0x64) & 0x01 != 0 {
                inb(0x60);
            }

            // Unmask IRQ1 (keyboard interrupt)
            crate::arch::interrupts::unmask_irq(1);
        }

        println!("[input] Keyboard initialized and IRQ1 unmasked");
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
    error_count: u32,
    last_update: u64,
    consecutive_errors: u8,  // Track consecutive errors for recovery
    resync_attempts: u8,     // Track resync attempts
    diagnostic_mode: bool,   // Enable diagnostic logging
    diagnostic_packets_remaining: u8,  // Count of diagnostic packets remaining
}

impl MouseDriver {
    pub const fn new() -> Self {
        Self { 
            x: 400, y: 300, 
            buttons: 0, 
            cycle: 0, 
            packet: [0; 4],
            error_count: 0,
            last_update: 0,
            consecutive_errors: 0,
            resync_attempts: 0,
            diagnostic_mode: false,
            diagnostic_packets_remaining: 0,
        }
    }
    
    /// Flush the PS/2 data buffer to recover from desync
    fn flush_buffer(&self) {
        unsafe {
            // Read all pending data from port 0x60
            // Limit to 16 reads to prevent infinite loop if hardware is stuck
            for _ in 0..16 {
                // Check if data is available (status bit 0)
                if inb(0x64) & 0x01 != 0 {
                    // Read and discard the data byte
                    let _ = inb(0x60);
                } else {
                    // No more data
                    break;
                }
            }
        }
    }
    
    /// Perform full PS/2 mouse reset and re-initialization
    /// NOTE: This may be called from interrupt context - NO PRINTLN!
    fn reset_and_resync(&mut self) {
        // Limit reset attempts to prevent infinite loops
        if self.resync_attempts >= 3 {
            // Too many resets - just flush and hope for the best
            self.flush_buffer();
            self.cycle = 0;
            self.consecutive_errors = 0;
            return;
        }
        
        self.resync_attempts += 1;
        
        // Flush any pending data
        self.flush_buffer();
        
        // Reset packet state
        self.cycle = 0;
        self.packet = [0; 4];
        self.consecutive_errors = 0;
        
        // Re-initialize the mouse (simplified re-init)
        unsafe {
            // Disable mouse
            self.wait_write();
            outb(0x64, 0xA7);
            
            // Small delay
            for _ in 0..10000 {
                core::arch::asm!("nop", options(nomem, nostack));
            }
            
            // Re-enable mouse
            self.wait_write();
            outb(0x64, 0xA8);
            
            // Send enable streaming command
            self.write(0xF4);
            let _ = self.read(); // Read ACK
        }
    }
    
    /// PS/2 ACK response byte
    const PS2_ACK: u8 = 0xFA;
    const PS2_RESEND: u8 = 0xFE;
    const PS2_ERROR: u8 = 0xFC;
    
    pub fn init(&mut self) {
        println!("[input] Initializing mouse...");

        unsafe {
            // Enable mouse port
            self.wait_write();
            outb(0x64, 0xA8);

            // Read command byte
            self.wait_write();
            outb(0x64, 0x20);
            self.wait_read();
            let status = (inb(0x60) | 2) & 0xDF;

            // Write command byte (enable mouse interrupt)
            self.wait_write();
            outb(0x64, 0x60);
            self.wait_write();
            outb(0x60, status);

            // Set defaults (0xF6)
            let defaults_ok = self.write_with_ack(0xF6);
            if !defaults_ok {
                println!("[mouse] Warning: Set defaults command failed");
            }

            // Enable streaming (0xF4) - must succeed for mouse to work
            let streaming_ok = self.write_with_ack(0xF4);
            if !streaming_ok {
                println!("[mouse] Warning: Enable streaming command failed! Mouse may not work.");
                // Don't try to reset here - it can cause more problems
                // Just continue and hope the mouse works anyway
            }

            // Unmask IRQ2 (cascade from slave PIC) and IRQ12 (mouse)
            crate::arch::interrupts::unmask_irq(2);
            crate::arch::interrupts::unmask_irq(12);
        }

        println!("[input] Mouse initialized and IRQ12 unmasked");
        
        // Enable diagnostic mode for first 10 packets to help debug
        self.diagnostic_mode = true;
        self.diagnostic_packets_remaining = 10;
    }
    
    /// Write command to mouse and wait for ACK, with retry on RESEND
    fn write_with_ack(&self, cmd: u8) -> bool {
        const MAX_RETRIES: u8 = 3;
        
        for _ in 0..MAX_RETRIES {
            self.write(cmd);
            let response = self.read();
            
            match response {
                Self::PS2_ACK => return true,
                Self::PS2_RESEND => {
                    // Wait a bit before retry
                    unsafe {
                        for _ in 0..1000 {
                            core::arch::asm!("nop", options(nomem, nostack));
                        }
                    }
                    continue;
                }
                Self::PS2_ERROR => {
                    println!("[mouse] PS/2 error response received");
                    return false;
                }
                _ => {
                    // Unexpected response, might be old data
                    println!("[mouse] Unexpected PS/2 response: 0x{:02X}", response);
                    // Flush and retry
                    self.flush_buffer();
                }
            }
        }
        
        false
    }
    
    pub fn handle_interrupt(&mut self) -> Option<InputEvent> {
        // Check status register to verify this is mouse data
        // Bit 5 of status register (0x64) indicates if data is from mouse (1) or keyboard (0)
        let status = unsafe { inb(0x64) };
        let _is_mouse_data = status & 0x20 != 0;
        
        let data = unsafe { inb(0x60) };
        
        // NOTE: Don't use println! in interrupt handlers - it can deadlock!
        // Diagnostic logging removed to prevent lock contention
        
        // Check for timeout - reset cycle if it's been too long
        let current_time = crate::arch::interrupts::get_timer_ticks();
        if self.cycle != 0 && current_time > self.last_update + 100 {
            // Timeout - flush buffer and reset to sync state
            self.flush_buffer();
            self.cycle = 0;
            self.error_count += 1;
            self.consecutive_errors += 1;
            
            // Only do full reset after many consecutive timeouts (reduced aggressiveness)
            if self.consecutive_errors >= 50 {
                // Can't print here - would deadlock. Set flag for main loop to report.
                self.reset_and_resync();
            }
        }
        self.last_update = current_time;
        
        match self.cycle {
            0 => {
                // Looking for sync byte (bit 3 must be set, overflow bits clear)
                if data & 0x08 != 0 && data & 0xC0 == 0 {
                    // Valid first byte: sync bit set, overflow bits clear
                    self.packet[0] = data;
                    self.cycle = 1;
                    // Reset consecutive errors on valid sync
                    self.consecutive_errors = 0;
                    
                    // Reset resync attempts after sustained good operation
                    if self.resync_attempts > 0 && self.error_count < 10 {
                        self.resync_attempts = 0;
                    }
                } else {
                    // Invalid sync byte - just log it, don't be too aggressive
                    self.error_count += 1;
                    self.consecutive_errors += 1;
                    
                    // Only flush after many consecutive errors (reduced aggressiveness)
                    if self.consecutive_errors >= 20 {
                        self.flush_buffer();
                        self.consecutive_errors = 0;
                    }
                    
                    // Only reset after very many errors (reduced aggressiveness)
                    if self.error_count > 500 {
                        // Can't print here - would deadlock
                        self.error_count = 0;
                        self.reset_and_resync();
                    }
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
                // Clear consecutive errors and reduce error count on successful packet
                self.consecutive_errors = 0;
                self.error_count = self.error_count.saturating_sub(1);
                self.process_packet()
            }
            _ => {
                // Invalid state - reset
                self.cycle = 0;
                self.error_count += 1;
                self.consecutive_errors += 1;
                None
            }
        }
    }
    
    fn process_packet(&mut self) -> Option<InputEvent> {
        let flags = self.packet[0];
        
        // Check for overflow conditions
        if flags & 0xC0 != 0 {
            // Overflow bit set, ignore this packet
            return None;
        }
        
        // PS/2 mouse protocol: sign bits are in the flags byte
        // bit 4 = X sign (1 = negative), bit 5 = Y sign (1 = negative)
        // We need to properly sign-extend the 9-bit movement values
        let x_sign_extend = if flags & 0x10 != 0 { 0xFFFFFF00u32 } else { 0 };
        let y_sign_extend = if flags & 0x20 != 0 { 0xFFFFFF00u32 } else { 0 };
        
        let x_movement = (self.packet[1] as u32 | x_sign_extend) as i32 as i16;
        let y_movement = (self.packet[2] as u32 | y_sign_extend) as i32 as i16;

        // Sanity check on movement values (shouldn't move more than 100 pixels in one packet)
        if x_movement.abs() > 100 || y_movement.abs() > 100 {
            return None;
        }

        let x_delta = x_movement as i32;
        let y_delta = y_movement as i32;

        self.x += x_delta;
        self.y -= y_delta;

        // Use hardcoded screen dimensions (1280x800) to avoid locking in interrupt handler
        // IMPORTANT: Do NOT lock mutexes in interrupt handlers - causes deadlock!
        self.x = self.x.max(0).min(1279);
        self.y = self.y.max(0).min(799);
        
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
    pub fn error_count(&self) -> u32 { self.error_count }
    
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
            // Note: Don't print in interrupt handler - can cause deadlock
        }
    }
    
    pub fn handle_mouse(&mut self) {
        if let Some(event) = self.mouse.handle_interrupt() {
            if self.events.len() < MAX_EVENTS {
                self.events.push_back(event);
            }
            // Note: Don't print in interrupt handler - can cause deadlock
        }
    }
    
    pub fn poll_event(&mut self) -> Option<InputEvent> { self.events.pop_front() }
    pub fn has_events(&self) -> bool { !self.events.is_empty() }
    pub fn event_queue_len(&self) -> usize { self.events.len() }
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

use core::sync::atomic::{AtomicU64, Ordering};

// Counters for diagnostics (visible from main thread)
static MOUSE_IRQ_COUNT: AtomicU64 = AtomicU64::new(0);
static KEYBOARD_IRQ_COUNT: AtomicU64 = AtomicU64::new(0);

// Separate static drivers for interrupt handlers
// These are ONLY accessed by interrupt handlers, preventing deadlock with main thread
static mut IRQ_KEYBOARD_DRIVER: KeyboardDriver = KeyboardDriver::new();
static mut IRQ_MOUSE_DRIVER: MouseDriver = MouseDriver::new();

/// Lock-free SPSC queue for IRQ events
/// Producer: interrupt handler | Consumer: main thread
struct LockFreeQueue {
    buffer: [InputEvent; MAX_EVENTS],
    head: AtomicU64, // Consumer (main thread) reads from here
    tail: AtomicU64, // Producer (IRQ) writes here
}

impl LockFreeQueue {
    const fn new() -> Self {
        Self {
            buffer: [InputEvent { 
                event_type: EventType::KeyPress, 
                keycode: 0, ascii: 0, x: 0, y: 0, button: 0, scroll: 0, modifiers: 0 
            }; MAX_EVENTS],
            head: AtomicU64::new(0),
            tail: AtomicU64::new(0),
        }
    }
    
    /// Push from interrupt handler (producer)
    fn push(&self, event: InputEvent) -> bool {
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Acquire);
        
        // Check if queue is full
        if tail.wrapping_sub(head) >= MAX_EVENTS as u64 {
            return false;
        }
        
        let idx = (tail % MAX_EVENTS as u64) as usize;
        unsafe {
            let ptr = &self.buffer[idx] as *const InputEvent as *mut InputEvent;
            ptr.write(event);
        }
        
        self.tail.store(tail.wrapping_add(1), Ordering::Release);
        true
    }
    
    /// Pop from main thread (consumer)
    fn pop(&self) -> Option<InputEvent> {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);
        
        if head == tail {
            return None;
        }
        
        let idx = (head % MAX_EVENTS as u64) as usize;
        let event = unsafe { self.buffer[idx].clone() };
        
        self.head.store(head.wrapping_add(1), Ordering::Release);
        Some(event)
    }
    
    fn len(&self) -> usize {
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Relaxed);
        tail.wrapping_sub(head) as usize
    }
}

// Lock-free queues for IRQ events
static KEYBOARD_QUEUE: LockFreeQueue = LockFreeQueue::new();
static MOUSE_QUEUE: LockFreeQueue = LockFreeQueue::new();

pub fn handle_keyboard_interrupt() { 
    KEYBOARD_IRQ_COUNT.fetch_add(1, Ordering::Relaxed);
    // CRITICAL: Use the IRQ-only driver, NOT INPUT_MANAGER (which would deadlock)
    unsafe {
        if let Some(event) = IRQ_KEYBOARD_DRIVER.handle_interrupt() {
            KEYBOARD_QUEUE.push(event);
        }
    }
}

pub fn handle_mouse_interrupt() { 
    MOUSE_IRQ_COUNT.fetch_add(1, Ordering::Relaxed);
    // CRITICAL: Use the IRQ-only driver, NOT INPUT_MANAGER (which would deadlock)
    unsafe {
        if let Some(event) = IRQ_MOUSE_DRIVER.handle_interrupt() {
            MOUSE_QUEUE.push(event);
        }
    }
}

pub fn poll_event() -> Option<InputEvent> { 
    // First check lock-free IRQ queues
    if let Some(event) = KEYBOARD_QUEUE.pop() {
        return Some(event);
    }
    if let Some(event) = MOUSE_QUEUE.pop() {
        return Some(event);
    }
    // Then check main manager queue (for events generated internally)
    INPUT_MANAGER.lock().poll_event() 
}

pub fn has_events() -> bool { 
    KEYBOARD_QUEUE.len() > 0 || MOUSE_QUEUE.len() > 0 || INPUT_MANAGER.lock().has_events()
}

pub fn event_queue_len() -> usize { 
    KEYBOARD_QUEUE.len() + MOUSE_QUEUE.len() + INPUT_MANAGER.lock().event_queue_len() 
}

pub fn mouse_position() -> (i32, i32) { 
    // Use the IRQ driver position since it's most up-to-date
    unsafe { IRQ_MOUSE_DRIVER.position() }
}

pub fn mouse_buttons() -> u8 {
    unsafe { IRQ_MOUSE_DRIVER.buttons() }
}

/// Get interrupt counters for diagnostics
pub fn get_irq_counts() -> (u64, u64) {
    (KEYBOARD_IRQ_COUNT.load(Ordering::Relaxed), MOUSE_IRQ_COUNT.load(Ordering::Relaxed))
}

/// Poll keyboard for input (non-interrupt mode)
pub fn poll_keyboard() -> Option<InputEvent> {
    // First check if there are any pending events
    if let Some(event) = poll_event() {
        return Some(event);
    }
    
    // Poll the keyboard hardware directly
    unsafe {
        // Check if data is available (status port 0x64, bit 0)
        if (inb(0x64) & 0x01) != 0 {
            // Process through keyboard driver's interrupt handler
            // (which just reads and processes the scancode)
            let mut manager = INPUT_MANAGER.lock();
            manager.keyboard.handle_interrupt()
        } else {
            None
        }
    }
}

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
