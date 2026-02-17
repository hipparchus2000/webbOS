//! Input Subsystem
//!
//! Handles keyboard and mouse input for WebbOS.
//! 
//! ARCHITECTURE:
//! - Mouse IRQ: Minimal - just updates atomic X/Y position
//! - Timer (20Hz): Polls mouse position, generates events, handles printing
//! This avoids mutex contention and deadlock in IRQ handlers.

#![cfg_attr(target_arch = "aarch64", allow(dead_code))]

use spin::Mutex;
use lazy_static::lazy_static;
use alloc::collections::VecDeque;
use core::sync::atomic::{AtomicU64, AtomicI32, Ordering};

use crate::println;

// Port I/O functions (x86_64 only)
#[cfg(target_arch = "x86_64")]
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

#[cfg(target_arch = "x86_64")]
#[inline]
pub unsafe fn outb(port: u16, value: u8) {
    core::arch::asm!(
        "out dx, al",
        in("dx") port,
        in("al") value,
        options(nomem, nostack)
    );
}

#[cfg(target_arch = "x86_64")]
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

#[cfg(target_arch = "x86_64")]
#[inline]
pub unsafe fn outw(port: u16, value: u16) {
    core::arch::asm!(
        "out dx, ax",
        in("dx") port,
        in("ax") value,
        options(nomem, nostack)
    );
}

// Stub implementations for aarch64
#[cfg(target_arch = "aarch64")]
#[inline]
pub unsafe fn inb(_port: u16) -> u8 { 0 }

#[cfg(target_arch = "aarch64")]
#[inline]
pub unsafe fn outb(_port: u16, _value: u8) {}

#[cfg(target_arch = "aarch64")]
#[inline]
pub unsafe fn inw(_port: u16) -> u16 { 0 }

#[cfg(target_arch = "aarch64")]
#[inline]
pub unsafe fn outw(_port: u16, _value: u16) {}

/// Maximum event queue size
const MAX_EVENTS: usize = 64;

// =============================================================================
// ATOMIC MOUSE STATE - Updated by IRQ handler, read by timer
// =============================================================================

/// Atomic mouse X position (updated by IRQ, read by timer)
static MOUSE_X: AtomicI32 = AtomicI32::new(400);
/// Atomic mouse Y position (updated by IRQ, read by timer)
static MOUSE_Y: AtomicI32 = AtomicI32::new(300);
/// Atomic mouse buttons state
static MOUSE_BTNS: AtomicU64 = AtomicU64::new(0);
/// Mouse IRQ counter (for diagnostics)
static MOUSE_IRQ_COUNT: AtomicU64 = AtomicU64::new(0);
/// Keyboard IRQ counter (for diagnostics)
static KEYBOARD_IRQ_COUNT: AtomicU64 = AtomicU64::new(0);
/// USB Keyboard event counter (for diagnostics)
static USB_KEYBOARD_EVENT_COUNT: AtomicU64 = AtomicU64::new(0);
/// Last reported X (for delta calculation)
static LAST_MOUSE_X: AtomicI32 = AtomicI32::new(400);
/// Last reported Y (for delta calculation)
static LAST_MOUSE_Y: AtomicI32 = AtomicI32::new(300);
/// Screen dimensions for clamping
static SCREEN_WIDTH: AtomicI32 = AtomicI32::new(1280);
static SCREEN_HEIGHT: AtomicI32 = AtomicI32::new(800);

// =============================================================================
// MOUSE SETTINGS AND CONFIGURATION
// =============================================================================

/// Mouse cursor size in pixels (for edge clamping)
pub const CURSOR_WIDTH: i32 = 16;
pub const CURSOR_HEIGHT: i32 = 16;

/// Edge behavior mode for mouse
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeBehavior {
    /// Hard clamp to screen edges (default)
    Clamp,
    /// Wrap around to opposite edge (for multi-monitor)
    Wrap,
    /// Resistance - harder to push past edge
    Resistance,
}

/// Mouse settings configuration
/// Note: This is a public API struct - fields may be used by external code
#[allow(dead_code)]
pub struct MouseSettings {
    /// Mouse speed multiplier (1.0 = normal, 2.0 = double speed)
    pub speed: f32,
    /// Mouse acceleration exponent (1.0 = linear, 2.0 = accelerated)
    pub acceleration: f32,
    /// Edge behavior mode
    pub edge_behavior: EdgeBehavior,
    /// Edge resistance factor (0.0-1.0, only used with Resistance mode)
    /// 1.0 = full resistance (can't push past), 0.5 = half resistance
    pub edge_resistance: f32,
    /// Margin from screen edge to keep cursor fully visible
    pub edge_margin: i32,
}

impl MouseSettings {
    /// Default mouse settings
    pub const fn default() -> Self {
        Self {
            speed: 1.0,
            acceleration: 1.0,
            edge_behavior: EdgeBehavior::Clamp,
            edge_resistance: 0.5,
            edge_margin: 0, // No margin by default, cursor can touch edge
        }
    }
    
    /// Settings with cursor margin to keep it fully visible
    pub const fn with_margin(margin: i32) -> Self {
        Self {
            speed: 1.0,
            acceleration: 1.0,
            edge_behavior: EdgeBehavior::Clamp,
            edge_resistance: 0.5,
            edge_margin: margin,
        }
    }
}

/// Global mouse settings (atomic for thread-safe access)
/// Note: Using integer atomics since core::sync::atomic doesn't have AtomicF32

/// Mouse speed (stored as fraction to avoid floating point in atomics)
static MOUSE_SPEED_NUM: AtomicI32 = AtomicI32::new(10); // 10 = 1.0
static MOUSE_SPEED_DENOM: AtomicI32 = AtomicI32::new(10);

/// Edge behavior stored as u8 (0=Clamp, 1=Wrap, 2=Resistance)
static MOUSE_EDGE_BEHAVIOR: AtomicU64 = AtomicU64::new(0);

/// Edge resistance factor (0-100, where 100 = 1.0)
static MOUSE_EDGE_RESISTANCE: AtomicU64 = AtomicU64::new(50);

/// Edge margin in pixels
static MOUSE_EDGE_MARGIN: AtomicI32 = AtomicI32::new(0);

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
pub const MOD_GUI: u8 = 0x40; // Windows/Super key

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
            #[cfg(target_arch = "x86_64")]
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
    screen_width: i32,       // Screen width for clamping
    screen_height: i32,      // Screen height for clamping
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
            screen_width: 1280,   // Default, can be updated
            screen_height: 800,   // Default, can be updated
        }
    }
    
    /// Set screen dimensions for mouse clamping
    pub fn set_screen_dimensions(&mut self, width: i32, height: i32) {
        self.screen_width = width;
        self.screen_height = height;
        // Clamp current position to new bounds considering cursor size
        let max_x = (self.screen_width - CURSOR_WIDTH).max(0);
        let max_y = (self.screen_height - CURSOR_HEIGHT).max(0);
        self.x = self.x.max(0).min(max_x);
        self.y = self.y.max(0).min(max_y);
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
            #[cfg(target_arch = "x86_64")]
            {
                crate::arch::interrupts::unmask_irq(2);
                crate::arch::interrupts::unmask_irq(12);
            }
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
    
    /// IRQ handler - minimal work, just update atomic position
    /// NO allocations, NO printing, NO complex logic!
    pub fn handle_interrupt(&mut self) {
        // Check status register
        let status = unsafe { inb(0x64) };
        
        // Check if data is available (bit 0)
        if status & 0x01 == 0 {
            return; // No data - spurious interrupt
        }
        
        let data = unsafe { inb(0x60) };
        
        // Check for timeout
        let current_time = crate::arch::interrupts::get_timer_ticks();
        if self.cycle != 0 && current_time > self.last_update + 100 {
            self.flush_buffer();
            self.cycle = 0;
            self.consecutive_errors += 1;
            if self.consecutive_errors >= 10 {
                self.reset_and_resync();
            }
        }
        self.last_update = current_time;
        
        match self.cycle {
            0 => {
                // Looking for sync byte
                if data & 0x08 != 0 && data & 0xC0 == 0 {
                    self.packet[0] = data;
                    self.cycle = 1;
                    self.consecutive_errors = 0;
                } else {
                    self.consecutive_errors += 1;
                    if self.consecutive_errors >= 5 {
                        self.flush_buffer();
                        self.cycle = 0;
                        self.consecutive_errors = 0;
                    }
                }
            }
            1 => {
                self.packet[1] = data;
                self.cycle = 2;
            }
            2 => {
                self.packet[2] = data;
                self.cycle = 0;
                self.consecutive_errors = 0;
                // Process packet - updates atomic position
                self.process_packet();
            }
            _ => {
                self.cycle = 0;
            }
        }
    }
    
    /// Process packet and update atomic mouse state
    /// This is called from IRQ handler - must be fast and use atomics only!
    fn process_packet(&mut self) {
        let flags = self.packet[0];
        
        // Check for overflow conditions
        if flags & 0xC0 != 0 {
            return; // Overflow - ignore packet
        }
        
        // PS/2 mouse protocol: sign bits are in the flags byte
        let x_sign_extend = if flags & 0x10 != 0 { 0xFFFFFF00u32 } else { 0 };
        let y_sign_extend = if flags & 0x20 != 0 { 0xFFFFFF00u32 } else { 0 };
        
        let x_movement = (self.packet[1] as u32 | x_sign_extend) as i32 as i16;
        let y_movement = (self.packet[2] as u32 | y_sign_extend) as i32 as i16;

        // Sanity check on movement values
        if x_movement.abs() > 100 || y_movement.abs() > 100 {
            return;
        }

        // Get current position from atomics
        let mut x = MOUSE_X.load(Ordering::Relaxed);
        let mut y = MOUSE_Y.load(Ordering::Relaxed);
        let screen_w = SCREEN_WIDTH.load(Ordering::Relaxed);
        let screen_h = SCREEN_HEIGHT.load(Ordering::Relaxed);
        
        // Apply mouse speed/acceleration settings
        let speed_num = MOUSE_SPEED_NUM.load(Ordering::Relaxed) as f32;
        let speed_denom = MOUSE_SPEED_DENOM.load(Ordering::Relaxed) as f32;
        let speed = if speed_denom > 0.0 { speed_num / speed_denom } else { 1.0 };
        
        let x_delta = (x_movement as f32 * speed) as i32;
        let y_delta = (y_movement as f32 * speed) as i32;
        
        // Update position
        x += x_delta;
        y -= y_delta;
        
        // Get edge behavior and clamp considering cursor size
        let edge_behavior = MOUSE_EDGE_BEHAVIOR.load(Ordering::Relaxed);
        let margin = MOUSE_EDGE_MARGIN.load(Ordering::Relaxed);
        
        // Calculate bounds considering cursor size and margin
        let min_x = margin;
        let min_y = margin;
        let max_x = (screen_w - CURSOR_WIDTH - margin).max(min_x);
        let max_y = (screen_h - CURSOR_HEIGHT - margin).max(min_y);
        
        match edge_behavior {
            1 => {
                // Wrap mode - for multi-monitor support
                if x < min_x { x = max_x; }
                else if x > max_x { x = min_x; }
                if y < min_y { y = max_y; }
                else if y > max_y { y = min_y; }
            }
            2 => {
                // Resistance mode - apply resistance when approaching edges
                let resistance = MOUSE_EDGE_RESISTANCE.load(Ordering::Relaxed) as f32 / 100.0;
                let edge_threshold = 50; // Pixels from edge where resistance starts
                
                // Check if we're at edges and apply resistance
                if x < min_x + edge_threshold {
                    let dist = (x - min_x) as f32;
                    if dist < 0.0 {
                        // Trying to push past edge - apply resistance
                        x = min_x + (dist * (1.0 - resistance)) as i32;
                    }
                }
                if x > max_x - edge_threshold {
                    let dist = (x - max_x) as f32;
                    if dist > 0.0 {
                        x = max_x + (dist * (1.0 - resistance)) as i32;
                    }
                }
                if y < min_y + edge_threshold {
                    let dist = (y - min_y) as f32;
                    if dist < 0.0 {
                        y = min_y + (dist * (1.0 - resistance)) as i32;
                    }
                }
                if y > max_y - edge_threshold {
                    let dist = (y - max_y) as f32;
                    if dist > 0.0 {
                        y = max_y + (dist * (1.0 - resistance)) as i32;
                    }
                }
                
                // Final clamp to ensure we don't go too far
                x = x.max(min_x - edge_threshold).min(max_x + edge_threshold);
                y = y.max(min_y - edge_threshold).min(max_y + edge_threshold);
            }
            _ => {
                // Default clamp mode
                x = x.max(min_x).min(max_x);
                y = y.max(min_y).min(max_y);
            }
        }
        
        // Write back to atomics
        MOUSE_X.store(x, Ordering::Relaxed);
        MOUSE_Y.store(y, Ordering::Relaxed);
        MOUSE_BTNS.store((flags & 0x07) as u64, Ordering::Relaxed);
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

// =============================================================================
// USB KEYBOARD SUPPORT
// =============================================================================

/// USB Keyboard driver - receives events from USB HID subsystem
pub struct UsbKeyboardDriver {
    shift_pressed: bool,
    ctrl_pressed: bool,
    alt_pressed: bool,
    gui_pressed: bool,
    caps_lock: bool,
    num_lock: bool,
    key_states: [bool; 256],
}

impl UsbKeyboardDriver {
    pub const fn new() -> Self {
        Self {
            shift_pressed: false,
            ctrl_pressed: false,
            alt_pressed: false,
            gui_pressed: false,
            caps_lock: false,
            num_lock: true,
            key_states: [false; 256],
        }
    }

    /// Handle USB key event from HID driver
    fn handle_usb_key_event(&mut self, event: crate::drivers::usb::hid::UsbKeyEvent) {
        // Update modifier state
        let mods = event.modifiers;
        self.ctrl_pressed = (mods & 0x11) != 0;   // Left or Right Ctrl
        self.shift_pressed = (mods & 0x22) != 0;  // Left or Right Shift
        self.alt_pressed = (mods & 0x44) != 0;    // Left or Right Alt
        self.gui_pressed = (mods & 0x88) != 0;    // Left or Right GUI

        // Track key state (keycode is u8, so always < 256)
        self.key_states[event.keycode as usize] = event.pressed;

        // Handle caps lock toggle on key release
        if !event.pressed && event.keycode == 0x39 { // Caps Lock
            self.caps_lock = !self.caps_lock;
        }

        // Handle num lock toggle on key release
        if !event.pressed && event.keycode == 0x53 { // Num Lock
            self.num_lock = !self.num_lock;
        }
    }

    /// Convert USB HID keycode to internal InputEvent
    fn to_input_event(&self, event: crate::drivers::usb::hid::UsbKeyEvent) -> Option<InputEvent> {
        use crate::drivers::usb::hid;

        // Build modifiers byte
        let mut modifiers = 0u8;
        if self.shift_pressed { modifiers |= MOD_SHIFT; }
        if self.ctrl_pressed { modifiers |= MOD_CTRL; }
        if self.alt_pressed { modifiers |= MOD_ALT; }
        if self.gui_pressed { modifiers |= MOD_GUI; }
        if self.caps_lock { modifiers |= MOD_CAPS; }
        if self.num_lock { modifiers |= MOD_NUM; }

        // Convert USB keycode to scancode
        let scancode = hid::keycode_to_scancode(event.keycode);
        let keycode = scancode.unwrap_or(event.keycode as u16);

        // Calculate ASCII if this is a key press
        let ascii = if event.pressed {
            hid::keycode_to_ascii(event.keycode, self.shift_pressed)
                .map(|c| c as u8)
                .unwrap_or(0)
        } else {
            0
        };

        Some(InputEvent {
            event_type: if event.pressed { EventType::KeyPress } else { EventType::KeyRelease },
            keycode,
            ascii,
            x: 0,
            y: 0,
            button: 0,
            scroll: 0,
            modifiers,
        })
    }
}

/// Global USB keyboard driver instance (for IRQ handler)
static mut USB_KEYBOARD_DRIVER: UsbKeyboardDriver = UsbKeyboardDriver::new();

/// Callback function registered with USB HID subsystem
/// This is called from USB context when a keyboard event occurs
fn usb_keyboard_event_callback(event: crate::drivers::usb::hid::UsbKeyEvent) {
    USB_KEYBOARD_EVENT_COUNT.fetch_add(1, Ordering::Relaxed);

    unsafe {
        // Update driver state
        USB_KEYBOARD_DRIVER.handle_usb_key_event(event);

        // Convert to input event and add to queue
        if let Some(input_event) = USB_KEYBOARD_DRIVER.to_input_event(event) {
            // Try to add to the main event queue
            // Use try_lock to avoid deadlock in interrupt context
            if let Some(mut manager) = INPUT_MANAGER.try_lock() {
                if manager.events.len() < MAX_EVENTS {
                    manager.events.push_back(input_event);
                }
            }
        }
    }
}

/// Initialize USB keyboard support
/// This registers our callback with the USB HID subsystem
pub fn init_usb_keyboard() {
    println!("[input] Initializing USB keyboard support...");

    // Register our callback with the USB HID subsystem
    crate::drivers::usb::hid::register_usb_keyboard_callback(usb_keyboard_event_callback);

    println!("[input] USB keyboard support initialized");
}

/// Check if USB keyboard is registered
pub fn is_usb_keyboard_active() -> bool {
    crate::drivers::usb::hid::is_usb_keyboard_registered()
}

/// Get USB keyboard event count (for diagnostics)
pub fn get_usb_keyboard_event_count() -> u64 {
    USB_KEYBOARD_EVENT_COUNT.load(Ordering::Relaxed)
}

// =============================================================================
// INPUT MANAGER
// =============================================================================

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
        // Mouse interrupt now only updates atomic position
        // No event generated - timer polling handles that
        self.mouse.handle_interrupt();
    }
    
    pub fn poll_event(&mut self) -> Option<InputEvent> { self.events.pop_front() }
    pub fn has_events(&self) -> bool { !self.events.is_empty() }
    pub fn event_queue_len(&self) -> usize { self.events.len() }
    pub fn mouse_position(&self) -> (i32, i32) { self.mouse.position() }
    pub fn set_mouse_position(&mut self, x: i32, y: i32) { self.mouse.set_position(x, y); }
    pub fn mouse_buttons(&self) -> u8 { self.mouse.buttons() }
    pub fn set_mouse_dimensions(&mut self, width: i32, height: i32) { 
        self.mouse.set_screen_dimensions(width, height); 
    }
}

lazy_static! {
    static ref INPUT_MANAGER: Mutex<InputManager> = Mutex::new(InputManager::new());
}

pub fn init() {
    println!("[input] Initializing input subsystem...");
    INPUT_MANAGER.lock().init();
    
    // Also initialize the IRQ-specific drivers
    unsafe {
        IRQ_KEYBOARD_DRIVER.init();
        IRQ_MOUSE_DRIVER.init();
    }
    
    // Initialize last position from current position
    LAST_MOUSE_X.store(MOUSE_X.load(Ordering::Relaxed), Ordering::Relaxed);
    LAST_MOUSE_Y.store(MOUSE_Y.load(Ordering::Relaxed), Ordering::Relaxed);
    
    println!("[input] Input subsystem ready (timer-based mouse polling)");
    
    // Initialize USB keyboard support
    init_usb_keyboard();
}

// Separate static drivers for interrupt handlers
// These ONLY update atomic state - NO events, NO queues!
static mut IRQ_KEYBOARD_DRIVER: KeyboardDriver = KeyboardDriver::new();
static mut IRQ_MOUSE_DRIVER: MouseDriver = MouseDriver::new();

/// Keyboard IRQ handler - just accumulates events in queue
pub fn handle_keyboard_interrupt() { 
    KEYBOARD_IRQ_COUNT.fetch_add(1, Ordering::Relaxed);
    unsafe {
        if let Some(event) = IRQ_KEYBOARD_DRIVER.handle_interrupt() {
            // Push to main queue - this is safe because we use Mutex
            INPUT_MANAGER.lock().events.push_back(event);
        }
    }
}

/// Mouse IRQ handler - MINIMAL, just updates atomic position
pub fn handle_mouse_interrupt() { 
    MOUSE_IRQ_COUNT.fetch_add(1, Ordering::Relaxed);
    unsafe {
        IRQ_MOUSE_DRIVER.handle_interrupt();
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
    
    // Re-clamp current mouse position to new bounds considering cursor size
    let margin = MOUSE_EDGE_MARGIN.load(Ordering::Relaxed);
    let current_x = MOUSE_X.load(Ordering::Relaxed);
    let current_y = MOUSE_Y.load(Ordering::Relaxed);
    
    let min_x = margin;
    let min_y = margin;
    let max_x = (width - CURSOR_WIDTH - margin).max(min_x);
    let max_y = (height - CURSOR_HEIGHT - margin).max(min_y);
    
    let clamped_x = current_x.max(min_x).min(max_x);
    let clamped_y = current_y.max(min_y).min(max_y);
    
    MOUSE_X.store(clamped_x, Ordering::Relaxed);
    MOUSE_Y.store(clamped_y, Ordering::Relaxed);
    LAST_MOUSE_X.store(clamped_x, Ordering::Relaxed);
    LAST_MOUSE_Y.store(clamped_y, Ordering::Relaxed);
    
    unsafe { 
        IRQ_MOUSE_DRIVER.set_screen_dimensions(width, height); 
    }
}

/// Clamp mouse position to screen bounds considering cursor size
/// Returns clamped (x, y) coordinates
pub fn clamp_mouse_position(x: i32, y: i32) -> (i32, i32) {
    let screen_w = SCREEN_WIDTH.load(Ordering::Relaxed);
    let screen_h = SCREEN_HEIGHT.load(Ordering::Relaxed);
    let margin = MOUSE_EDGE_MARGIN.load(Ordering::Relaxed);
    
    let min_x = margin;
    let min_y = margin;
    let max_x = (screen_w - CURSOR_WIDTH - margin).max(min_x);
    let max_y = (screen_h - CURSOR_HEIGHT - margin).max(min_y);
    
    (x.max(min_x).min(max_x), y.max(min_y).min(max_y))
}

/// Clamp mouse position to screen bounds (raw, no cursor size consideration)
/// Use this for hit-testing where the cursor tip position matters
pub fn clamp_mouse_position_raw(x: i32, y: i32) -> (i32, i32) {
    let screen_w = SCREEN_WIDTH.load(Ordering::Relaxed);
    let screen_h = SCREEN_HEIGHT.load(Ordering::Relaxed);
    
    (x.max(0).min(screen_w - 1), y.max(0).min(screen_h - 1))
}

/// Get current mouse settings
pub fn get_mouse_settings() -> MouseSettings {
    let speed_num = MOUSE_SPEED_NUM.load(Ordering::Relaxed);
    let speed_denom = MOUSE_SPEED_DENOM.load(Ordering::Relaxed);
    let speed = if speed_denom != 0 { 
        speed_num as f32 / speed_denom as f32 
    } else { 
        1.0 
    };
    
    let edge_behavior = match MOUSE_EDGE_BEHAVIOR.load(Ordering::Relaxed) {
        1 => EdgeBehavior::Wrap,
        2 => EdgeBehavior::Resistance,
        _ => EdgeBehavior::Clamp,
    };
    
    let edge_resistance = MOUSE_EDGE_RESISTANCE.load(Ordering::Relaxed) as f32 / 100.0;
    let edge_margin = MOUSE_EDGE_MARGIN.load(Ordering::Relaxed);
    
    MouseSettings {
        speed,
        acceleration: 1.0, // TODO: Implement acceleration
        edge_behavior,
        edge_resistance,
        edge_margin,
    }
}

/// Set mouse speed (1.0 = normal)
pub fn set_mouse_speed(speed: f32) {
    // Store as fraction to avoid floating point in atomics
    let scaled = (speed * 10.0) as i32;
    MOUSE_SPEED_NUM.store(scaled.max(1), Ordering::Relaxed);
    MOUSE_SPEED_DENOM.store(10, Ordering::Relaxed);
}

/// Set edge behavior mode
pub fn set_mouse_edge_behavior(behavior: EdgeBehavior) {
    let value = match behavior {
        EdgeBehavior::Clamp => 0,
        EdgeBehavior::Wrap => 1,
        EdgeBehavior::Resistance => 2,
    };
    MOUSE_EDGE_BEHAVIOR.store(value, Ordering::Relaxed);
}

/// Set edge resistance factor (0.0 - 1.0)
pub fn set_mouse_edge_resistance(resistance: f32) {
    let clamped = resistance.max(0.0).min(1.0);
    MOUSE_EDGE_RESISTANCE.store((clamped * 100.0) as u64, Ordering::Relaxed);
}

/// Set edge margin in pixels (keeps cursor fully visible)
pub fn set_mouse_edge_margin(margin: i32) {
    MOUSE_EDGE_MARGIN.store(margin.max(0), Ordering::Relaxed);
    // Re-clamp current position with new margin
    let (x, y) = mouse_position();
    let (clamped_x, clamped_y) = clamp_mouse_position(x, y);
    MOUSE_X.store(clamped_x, Ordering::Relaxed);
    MOUSE_Y.store(clamped_y, Ordering::Relaxed);
}

/// Get interrupt counters for diagnostics
pub fn get_irq_counts() -> (u64, u64, u64) {
    (
        KEYBOARD_IRQ_COUNT.load(Ordering::Relaxed),
        MOUSE_IRQ_COUNT.load(Ordering::Relaxed),
        USB_KEYBOARD_EVENT_COUNT.load(Ordering::Relaxed)
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
    let (kb_irq, mouse_irq, usb_events) = get_irq_counts();
    
    println!("Input Status:");
    println!("  Mouse position: ({}, {})", x, y);
    println!("  Mouse buttons: {:03b}", manager.mouse_buttons());
    println!("  Events in queue: {}", manager.events.len());
    println!("  PS/2 IRQs - Keyboard: {}, Mouse: {}", kb_irq, mouse_irq);
    println!("  USB keyboard events: {}", usb_events);
    println!("  USB keyboard active: {}", is_usb_keyboard_active());
}
