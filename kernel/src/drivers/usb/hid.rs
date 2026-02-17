//! USB HID (Human Interface Device) Driver
//!
//! Supports keyboards, mice, and other HID devices.

use alloc::vec::Vec;
use alloc::string::String;
use alloc::format;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::println;
use crate::error::UsbError;
use super::{UsbDriver, UsbDevice, UsbClass, EndpointDescriptor, TransferType};

/// USB Key Event - sent from HID driver to input subsystem
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UsbKeyEvent {
    pub keycode: u8,
    pub pressed: bool,
    pub modifiers: u8,
}

/// Callback type for USB keyboard events
pub type UsbKeyboardCallback = fn(UsbKeyEvent);

/// Global callback for USB keyboard events (set by input subsystem)
static mut USB_KEYBOARD_CALLBACK: Option<UsbKeyboardCallback> = None;

/// Flag to indicate if USB keyboard callback is registered
static USB_KEYBOARD_REGISTERED: AtomicBool = AtomicBool::new(false);

/// Register a callback for USB keyboard events
/// This is called by the input subsystem to receive USB keyboard events
pub fn register_usb_keyboard_callback(callback: UsbKeyboardCallback) {
    unsafe {
        USB_KEYBOARD_CALLBACK = Some(callback);
    }
    USB_KEYBOARD_REGISTERED.store(true, Ordering::SeqCst);
}

/// Check if a USB keyboard callback is registered
pub fn is_usb_keyboard_registered() -> bool {
    USB_KEYBOARD_REGISTERED.load(Ordering::Relaxed)
}

/// Send a USB key event to the input subsystem
fn send_usb_key_event(event: UsbKeyEvent) {
    if USB_KEYBOARD_REGISTERED.load(Ordering::Relaxed) {
        unsafe {
            if let Some(callback) = USB_KEYBOARD_CALLBACK {
                callback(event);
            }
        }
    }
}

/// USB Mouse Event - passed to input subsystem
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UsbMouseEvent {
    pub x_delta: i8,
    pub y_delta: i8,
    pub buttons: u8, // bit 0=left, 1=right, 2=middle
    pub wheel: i8,
}

impl UsbMouseEvent {
    /// Create a new USB mouse event
    pub const fn new(x_delta: i8, y_delta: i8, buttons: u8, wheel: i8) -> Self {
        Self {
            x_delta,
            y_delta,
            buttons,
            wheel,
        }
    }

    /// Check if left button is pressed
    pub fn left_button(&self) -> bool {
        self.buttons & 0x01 != 0
    }

    /// Check if right button is pressed
    pub fn right_button(&self) -> bool {
        self.buttons & 0x02 != 0
    }

    /// Check if middle button is pressed
    pub fn middle_button(&self) -> bool {
        self.buttons & 0x04 != 0
    }
}

/// Callback type for USB mouse events
pub type UsbMouseCallback = fn(UsbMouseEvent);

/// Global callback for USB mouse events (set by input subsystem)
static mut USB_MOUSE_CALLBACK: Option<UsbMouseCallback> = None;

/// Flag to indicate if USB mouse callback is registered
static USB_MOUSE_REGISTERED: AtomicBool = AtomicBool::new(false);

/// Register a callback for USB mouse events
/// This is called by the input subsystem to receive USB mouse events
pub fn register_usb_mouse_callback(callback: UsbMouseCallback) {
    unsafe {
        USB_MOUSE_CALLBACK = Some(callback);
    }
    USB_MOUSE_REGISTERED.store(true, Ordering::SeqCst);
}

/// Check if a USB mouse callback is registered
pub fn is_usb_mouse_registered() -> bool {
    USB_MOUSE_REGISTERED.load(Ordering::Relaxed)
}

/// Send a USB mouse event to the input subsystem
fn send_usb_mouse_event(event: UsbMouseEvent) {
    if USB_MOUSE_REGISTERED.load(Ordering::Relaxed) {
        unsafe {
            if let Some(callback) = USB_MOUSE_CALLBACK {
                callback(event);
            }
        }
    }
}

/// HID modifier key bits
pub const HID_MOD_LEFT_CTRL: u8 = 0x01;
pub const HID_MOD_LEFT_SHIFT: u8 = 0x02;
pub const HID_MOD_LEFT_ALT: u8 = 0x04;
pub const HID_MOD_LEFT_GUI: u8 = 0x08;
pub const HID_MOD_RIGHT_CTRL: u8 = 0x10;
pub const HID_MOD_RIGHT_SHIFT: u8 = 0x20;
pub const HID_MOD_RIGHT_ALT: u8 = 0x40;
pub const HID_MOD_RIGHT_GUI: u8 = 0x80;

/// HID driver
pub struct HidDriver {
    name: &'static str,
    keyboards: Vec<HidKeyboard>,
    mice: Vec<HidMouse>,
}

/// HID keyboard device
#[derive(Debug)]
pub struct HidKeyboard {
    /// USB address
    pub address: u8,
    /// Interface number
    pub interface: u8,
    /// Endpoint for input reports
    pub endpoint: u8,
    /// Max packet size for endpoint
    pub max_packet_size: u16,
    /// Polling interval in ms
    pub interval: u8,
    /// Current key states (previous report for comparison)
    pub prev_keys: [u8; 6],
    /// Current modifier state
    pub prev_modifiers: u8,
    /// Track which keys are currently pressed
    pub key_states: [bool; 256],
}

/// HID mouse device
#[derive(Debug)]
pub struct HidMouse {
    /// USB address
    pub address: u8,
    /// Interface number
    pub interface: u8,
    /// Endpoint for input reports
    pub endpoint: u8,
    /// Max packet size for endpoint
    pub max_packet_size: u16,
    /// Polling interval in ms
    pub interval: u8,
    /// X position
    pub x: i32,
    /// Y position
    pub y: i32,
    /// Button states
    pub buttons: u8,
}

/// Standard HID keyboard report (boot protocol)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct KeyboardReport {
    /// Modifier keys (Ctrl, Shift, Alt, GUI)
    pub modifiers: u8,
    /// Reserved
    pub reserved: u8,
    /// Key codes (up to 6 simultaneous keys)
    pub keys: [u8; 6],
}

/// Standard HID mouse report (boot protocol)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct MouseReport {
    /// Button states
    pub buttons: u8,
    /// X movement
    pub x: i8,
    /// Y movement
    pub y: i8,
    /// Wheel movement
    pub wheel: i8,
}

impl HidDriver {
    /// Create new HID driver
    pub const fn new() -> Self {
        Self {
            name: "USB HID",
            keyboards: Vec::new(),
            mice: Vec::new(),
        }
    }

    /// Get connected keyboards
    pub fn keyboards(&self) -> &[HidKeyboard] {
        &self.keyboards
    }

    /// Get connected mice
    pub fn mice(&self) -> &[HidMouse] {
        &self.mice
    }

    /// Get mutable reference to keyboards (for polling)
    pub fn keyboards_mut(&mut self) -> &mut Vec<HidKeyboard> {
        &mut self.keyboards
    }

    /// Get mutable reference to mice (for polling)
    pub fn mice_mut(&mut self) -> &mut Vec<HidMouse> {
        &mut self.mice
    }

    /// Poll keyboard interrupt endpoint and process reports
    /// This should be called periodically (e.g., from timer interrupt)
    pub fn poll_keyboards(&mut self) {
        for i in 0..self.keyboards.len() {
            // Get keyboard data for processing
            let keyboard = &self.keyboards[i];
            let addr = keyboard.address;
            let ep = keyboard.endpoint;
            let _max_size = keyboard.max_packet_size;
            
            // TODO: Actually read from interrupt endpoint via xHCI
            // For now, this is a placeholder for the polling mechanism
            // In a real implementation, this would:
            // 1. Queue an interrupt transfer on the endpoint
            // 2. Check if data is available
            // 3. Parse the report and call process_keyboard_report
            
            // The actual USB transfer would look like:
            // let report = xhci.read_interrupt_endpoint(addr, ep, max_size);
            // self.process_keyboard_report(addr, &report);
            
            let _ = (addr, ep); // Silence unused warnings for now
        }
    }

    /// Poll mouse interrupt endpoint and process reports
    pub fn poll_mice(&mut self) {
        for i in 0..self.mice.len() {
            let mouse = &self.mice[i];
            let addr = mouse.address;
            let ep = mouse.endpoint;
            let _max_size = mouse.max_packet_size;
            
            // TODO: Actually read from interrupt endpoint via xHCI
            // Similar to poll_keyboards()
            
            let _ = (addr, ep); // Silence unused warnings for now
        }
    }

    /// Process keyboard report from interrupt endpoint
    /// This is called when a keyboard report is received
    pub fn process_keyboard_report(&mut self, address: u8, report: &KeyboardReport) {
        let modifiers = report.modifiers;
        let keys = report.keys;
        
        // Collect modifier change info to process after keyboard borrow ends
        let mut modifier_change: Option<(u8, u8)> = None;
        
        // Find the keyboard in our list
        if let Some(keyboard) = self.keyboards.iter_mut().find(|k| k.address == address) {
            let prev_modifiers = keyboard.prev_modifiers;

            // Check for modifier changes
            if modifiers != prev_modifiers {
                // Update modifiers first
                keyboard.prev_modifiers = modifiers;
                // Queue modifier change for processing after borrow ends
                modifier_change = Some((prev_modifiers, modifiers));
            }

            // Check for key changes by comparing with previous report
            // Keys that were in prev_keys but not in keys are released
            // Keys that are in keys but not in prev_keys are pressed

            // First, check for released keys
            for &prev_key in keyboard.prev_keys.iter() {
                if prev_key != 0 && !keys.contains(&prev_key) {
                    // Key was released
                    keyboard.key_states[prev_key as usize] = false;
                    let event = UsbKeyEvent {
                        keycode: prev_key,
                        pressed: false,
                        modifiers,
                    };
                    send_usb_key_event(event);
                }
            }

            // Then, check for newly pressed keys
            for &key in keys.iter() {
                if key != 0 && !keyboard.prev_keys.contains(&key) {
                    // Key was pressed
                    keyboard.key_states[key as usize] = true;
                    let event = UsbKeyEvent {
                        keycode: key,
                        pressed: true,
                        modifiers,
                    };
                    send_usb_key_event(event);
                }
            }

            // Update previous keys state
            keyboard.prev_keys = keys;
        }
        
        // Process modifier change after keyboard borrow ends
        if let Some((prev_mods, new_mods)) = modifier_change {
            self.handle_modifier_changes(address, prev_mods, new_mods);
        }
    }

    /// Handle modifier key changes
    fn handle_modifier_changes(&mut self, _address: u8, prev_mods: u8, new_mods: u8) {
        // Check each modifier bit
        let changed = prev_mods ^ new_mods;
        
        if changed & HID_MOD_LEFT_CTRL != 0 {
            let event = UsbKeyEvent {
                keycode: 0xE0, // Left Ctrl (internal code)
                pressed: new_mods & HID_MOD_LEFT_CTRL != 0,
                modifiers: new_mods,
            };
            send_usb_key_event(event);
        }
        if changed & HID_MOD_LEFT_SHIFT != 0 {
            let event = UsbKeyEvent {
                keycode: 0xE1, // Left Shift (internal code)
                pressed: new_mods & HID_MOD_LEFT_SHIFT != 0,
                modifiers: new_mods,
            };
            send_usb_key_event(event);
        }
        if changed & HID_MOD_LEFT_ALT != 0 {
            let event = UsbKeyEvent {
                keycode: 0xE2, // Left Alt (internal code)
                pressed: new_mods & HID_MOD_LEFT_ALT != 0,
                modifiers: new_mods,
            };
            send_usb_key_event(event);
        }
        if changed & HID_MOD_LEFT_GUI != 0 {
            let event = UsbKeyEvent {
                keycode: 0xE3, // Left GUI (internal code)
                pressed: new_mods & HID_MOD_LEFT_GUI != 0,
                modifiers: new_mods,
            };
            send_usb_key_event(event);
        }
        if changed & HID_MOD_RIGHT_CTRL != 0 {
            let event = UsbKeyEvent {
                keycode: 0xE4, // Right Ctrl (internal code)
                pressed: new_mods & HID_MOD_RIGHT_CTRL != 0,
                modifiers: new_mods,
            };
            send_usb_key_event(event);
        }
        if changed & HID_MOD_RIGHT_SHIFT != 0 {
            let event = UsbKeyEvent {
                keycode: 0xE5, // Right Shift (internal code)
                pressed: new_mods & HID_MOD_RIGHT_SHIFT != 0,
                modifiers: new_mods,
            };
            send_usb_key_event(event);
        }
        if changed & HID_MOD_RIGHT_ALT != 0 {
            let event = UsbKeyEvent {
                keycode: 0xE6, // Right Alt (internal code)
                pressed: new_mods & HID_MOD_RIGHT_ALT != 0,
                modifiers: new_mods,
            };
            send_usb_key_event(event);
        }
        if changed & HID_MOD_RIGHT_GUI != 0 {
            let event = UsbKeyEvent {
                keycode: 0xE7, // Right GUI (internal code)
                pressed: new_mods & HID_MOD_RIGHT_GUI != 0,
                modifiers: new_mods,
            };
            send_usb_key_event(event);
        }
    }

    /// Process mouse report from interrupt endpoint
    /// Updates mouse state and sends events to input subsystem
    pub fn process_mouse_report(&mut self, address: u8, report: &MouseReport) {
        if let Some(mouse) = self.mice.iter_mut().find(|m| m.address == address) {
            // Update internal mouse tracking state
            mouse.x += report.x as i32;
            mouse.y += report.y as i32;
            
            // Check for changes - only send event if something changed
            let buttons_changed = mouse.buttons != report.buttons;
            let position_changed = report.x != 0 || report.y != 0 || report.wheel != 0;
            
            if buttons_changed {
                mouse.buttons = report.buttons;
            }
            
            // Send event to input subsystem if there are changes
            if position_changed || buttons_changed {
                let event = UsbMouseEvent::new(
                    report.x,
                    report.y,
                    report.buttons,
                    report.wheel,
                );
                
                // Send to input subsystem via callback
                send_usb_mouse_event(event);
            }
        }
    }

    /// Parse configuration descriptors to find HID interfaces
    fn parse_hid_interfaces(&self, device: &UsbDevice) -> Vec<HidInterfaceInfo> {
        let mut interfaces = Vec::new();
        
        if device.configurations.is_empty() {
            return interfaces;
        }

        let config = &device.configurations;
        let mut offset = 0;

        while offset + 2 <= config.len() {
            let desc_len = config[offset];
            let desc_type = config[offset + 1];

            if desc_len == 0 || offset + desc_len as usize > config.len() {
                break;
            }

            // Interface descriptor type is 0x04
            if desc_type == 0x04 && offset + 9 <= config.len() {
                let iface_class = config[offset + 5];
                let iface_subclass = config[offset + 6];
                let iface_protocol = config[offset + 7];
                let iface_num = config[offset + 2];

                // HID class is 0x03
                if iface_class == UsbClass::Hid as u8 {
                    // Look for interrupt IN endpoint
                    let mut endpoint = 0u8;
                    let mut max_packet = 8u16;
                    let mut interval = 10u8;

                    // Scan ahead for endpoint descriptors
                    let mut ep_offset = offset + desc_len as usize;
                    while ep_offset + 2 <= config.len() {
                        let ep_desc_len = config[ep_offset];
                        let ep_desc_type = config[ep_offset + 1];

                        if ep_desc_len == 0 || ep_offset + ep_desc_len as usize > config.len() {
                            break;
                        }

                        if ep_desc_type == 0x05 && ep_offset + 7 <= config.len() {
                            // Endpoint descriptor
                            let ep_addr = config[ep_offset + 2];
                            let ep_attrs = config[ep_offset + 3];
                            let ep_max_packet = u16::from_le_bytes([
                                config[ep_offset + 4],
                                config[ep_offset + 5]
                            ]);
                            let ep_interval = config[ep_offset + 6];

                            // Check if interrupt IN endpoint
                            if (ep_addr & 0x80) != 0 && (ep_attrs & 0x03) == 0x03 {
                                endpoint = ep_addr;
                                max_packet = ep_max_packet;
                                interval = ep_interval;
                                break;
                            }
                        }

                        ep_offset += ep_desc_len as usize;
                    }

                    interfaces.push(HidInterfaceInfo {
                        interface_number: iface_num,
                        interface_class: iface_class,
                        interface_subclass: iface_subclass,
                        interface_protocol: iface_protocol,
                        endpoint,
                        max_packet_size: max_packet,
                        interval,
                    });
                }
            }

            offset += desc_len as usize;
        }

        interfaces
    }
}

/// HID interface information parsed from descriptors
#[derive(Debug, Clone)]
struct HidInterfaceInfo {
    interface_number: u8,
    interface_class: u8,
    interface_subclass: u8,
    interface_protocol: u8,
    endpoint: u8,
    max_packet_size: u16,
    interval: u8,
}

impl UsbDriver for HidDriver {
    fn name(&self) -> &str {
        self.name
    }

    fn supports(&self, device: &UsbDevice) -> bool {
        // HID class is 0x03
        device.device_descriptor.class == UsbClass::Hid as u8 ||
        device.device_descriptor.class == 0x00 // Class defined at interface level
    }

    fn init(&mut self, device: &mut UsbDevice) -> Result<(), UsbError> {
        println!("[usb-hid] Initializing HID device at address {}", device.address);

        // Parse configuration descriptors to find HID interfaces
        let interfaces = self.parse_hid_interfaces(device);

        if interfaces.is_empty() {
            println!("[usb-hid] No HID interfaces found");
            return Err(UsbError::InvalidDescriptor);
        }

        for iface in &interfaces {
            println!("[usb-hid] Found HID interface {}: subclass=0x{:02X}, protocol=0x{:02X}",
                iface.interface_number,
                iface.interface_subclass,
                iface.interface_protocol
            );

            // Determine device type from protocol
            // 0 = None, 1 = Keyboard, 2 = Mouse
            match iface.interface_protocol {
                1 => {
                    // Keyboard
                    println!("[usb-hid] Detected keyboard on interface {}", iface.interface_number);
                    
                    let keyboard = HidKeyboard {
                        address: device.address,
                        interface: iface.interface_number,
                        endpoint: iface.endpoint,
                        max_packet_size: iface.max_packet_size,
                        interval: iface.interval,
                        prev_keys: [0; 6],
                        prev_modifiers: 0,
                        key_states: [false; 256],
                    };
                    
                    self.keyboards.push(keyboard);
                    
                    println!("[usb-hid] Keyboard registered: addr={}, ep={}",
                        device.address, iface.endpoint);
                }
                2 => {
                    // Mouse
                    println!("[usb-hid] Detected mouse on interface {}", iface.interface_number);
                    
                    let mouse = HidMouse {
                        address: device.address,
                        interface: iface.interface_number,
                        endpoint: iface.endpoint,
                        max_packet_size: iface.max_packet_size,
                        interval: iface.interval,
                        x: 0,
                        y: 0,
                        buttons: 0,
                    };
                    
                    self.mice.push(mouse);
                    
                    println!("[usb-hid] Mouse registered: addr={}, ep={}",
                        device.address, iface.endpoint);
                }
                _ => {
                    println!("[usb-hid] Unknown HID protocol: {}", iface.interface_protocol);
                }
            }
        }

        println!("[usb-hid] HID device initialized: {} keyboards, {} mice",
            self.keyboards.len(), self.mice.len());
        
        // Start polling will be done by the main USB polling loop
        // which calls poll_keyboards() and poll_mice() periodically
        
        Ok(())
    }

    fn disconnect(&mut self, device: &UsbDevice) {
        println!("[usb-hid] HID device disconnected from address {}", device.address);

        // Remove keyboards from this device
        let prev_kb_count = self.keyboards.len();
        self.keyboards.retain(|k| k.address != device.address);
        let removed_kb = prev_kb_count - self.keyboards.len();
        
        // Remove mice from this device
        let prev_mouse_count = self.mice.len();
        self.mice.retain(|m| m.address != device.address);
        let removed_mice = prev_mouse_count - self.mice.len();

        if removed_kb > 0 || removed_mice > 0 {
            println!("[usb-hid] Removed {} keyboard(s) and {} mouse(es)",
                removed_kb, removed_mice);
        }
    }
}

/// HID descriptor
#[derive(Debug, Clone)]
pub struct HidDescriptor {
    /// HID version (BCD)
    pub hid_version: u16,
    /// Country code
    pub country_code: u8,
    /// Number of descriptors
    pub num_descriptors: u8,
    /// Report descriptor type
    pub report_type: u8,
    /// Report descriptor length
    pub report_length: u16,
}

impl HidDescriptor {
    /// Parse HID descriptor from raw bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self, UsbError> {
        if data.len() < 9 {
            return Err(UsbError::InvalidDescriptor);
        }

        Ok(Self {
            hid_version: u16::from_le_bytes([data[2], data[3]]),
            country_code: data[4],
            num_descriptors: data[5],
            report_type: data[6],
            report_length: u16::from_le_bytes([data[7], data[8]]),
        })
    }
}

/// HID usage pages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum HidUsagePage {
    GenericDesktop = 0x01,
    Simulation = 0x02,
    VR = 0x03,
    Sport = 0x04,
    Game = 0x05,
    GenericDeviceControls = 0x06,
    Keyboard = 0x07,
    LEDs = 0x08,
    Button = 0x09,
    Ordinal = 0x0A,
    Telephony = 0x0B,
    Consumer = 0x0C,
    Digitizer = 0x0D,
    Haptics = 0x0E,
    PhysicalInputDevice = 0x0F,
    Unicode = 0x10,
    SoC = 0x11,
    EyeAndHeadTracker = 0x12,
    AuxiliaryDisplay = 0x14,
    Sensors = 0x20,
    MedicalInstrument = 0x40,
    BridalPage = 0x41,
    LightingAndIllumination = 0x59,
    USBMonitor = 0x80,
    PowerDevice = 0x84,
    BatterySystem = 0x85,
    BarCodeScanner = 0x8C,
    MagneticStripeReader = 0x8E,
    CameraControl = 0x90,
    Arcade = 0x91,
    VendorDefined = 0xFF00,
}

/// HID usage IDs for Generic Desktop page
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum HidGenericDesktopUsage {
    Pointer = 0x01,
    Mouse = 0x02,
    Joystick = 0x04,
    GamePad = 0x05,
    Keyboard = 0x06,
    Keypad = 0x07,
    MultiAxisController = 0x08,
    X = 0x30,
    Y = 0x31,
    Z = 0x32,
    Rx = 0x33,
    Ry = 0x34,
    Rz = 0x35,
    Slider = 0x36,
    Dial = 0x37,
    Wheel = 0x38,
    HatSwitch = 0x39,
}

/// Convert USB HID keycode to internal modifier bits (for input subsystem)
pub fn hid_modifiers_to_internal(hid_mods: u8) -> u8 {
    let mut internal = 0u8;
    
    // Map HID modifiers to internal modifier format
    // Note: Internal format uses different bit positions
    if hid_mods & (HID_MOD_LEFT_SHIFT | HID_MOD_RIGHT_SHIFT) != 0 {
        internal |= 0x01; // MOD_SHIFT
    }
    if hid_mods & (HID_MOD_LEFT_CTRL | HID_MOD_RIGHT_CTRL) != 0 {
        internal |= 0x02; // MOD_CTRL
    }
    if hid_mods & (HID_MOD_LEFT_ALT | HID_MOD_RIGHT_ALT) != 0 {
        internal |= 0x04; // MOD_ALT
    }
    if hid_mods & (HID_MOD_LEFT_GUI | HID_MOD_RIGHT_GUI) != 0 {
        internal |= 0x40; // MOD_GUI (optional)
    }
    
    internal
}

/// Convert USB keycode to ASCII (simplified)
pub fn keycode_to_ascii(keycode: u8, shift: bool) -> Option<char> {
    // Standard USB HID keycodes to ASCII mapping
    // USB HID keycodes start at 0x04 for 'a', 0x1E for '1', etc.
    let ascii = match keycode {
        0x04 => if shift { 'A' } else { 'a' },
        0x05 => if shift { 'B' } else { 'b' },
        0x06 => if shift { 'C' } else { 'c' },
        0x07 => if shift { 'D' } else { 'd' },
        0x08 => if shift { 'E' } else { 'e' },
        0x09 => if shift { 'F' } else { 'f' },
        0x0A => if shift { 'G' } else { 'g' },
        0x0B => if shift { 'H' } else { 'h' },
        0x0C => if shift { 'I' } else { 'i' },
        0x0D => if shift { 'J' } else { 'j' },
        0x0E => if shift { 'K' } else { 'k' },
        0x0F => if shift { 'L' } else { 'l' },
        0x10 => if shift { 'M' } else { 'm' },
        0x11 => if shift { 'N' } else { 'n' },
        0x12 => if shift { 'O' } else { 'o' },
        0x13 => if shift { 'P' } else { 'p' },
        0x14 => if shift { 'Q' } else { 'q' },
        0x15 => if shift { 'R' } else { 'r' },
        0x16 => if shift { 'S' } else { 's' },
        0x17 => if shift { 'T' } else { 't' },
        0x18 => if shift { 'U' } else { 'u' },
        0x19 => if shift { 'V' } else { 'v' },
        0x1A => if shift { 'W' } else { 'w' },
        0x1B => if shift { 'X' } else { 'x' },
        0x1C => if shift { 'Y' } else { 'y' },
        0x1D => if shift { 'Z' } else { 'z' },
        0x1E => if shift { '!' } else { '1' },
        0x1F => if shift { '@' } else { '2' },
        0x20 => if shift { '#' } else { '3' },
        0x21 => if shift { '$' } else { '4' },
        0x22 => if shift { '%' } else { '5' },
        0x23 => if shift { '^' } else { '6' },
        0x24 => if shift { '&' } else { '7' },
        0x25 => if shift { '*' } else { '8' },
        0x26 => if shift { '(' } else { '9' },
        0x27 => if shift { ')' } else { '0' },
        0x28 => '\n',  // Enter
        0x29 => 0x1B as char,  // Escape
        0x2A => 0x08 as char,  // Backspace
        0x2B => '\t',  // Tab
        0x2C => ' ',   // Space
        0x2D => if shift { '_' } else { '-' },
        0x2E => if shift { '+' } else { '=' },
        0x2F => if shift { '{' } else { '[' },
        0x30 => if shift { '}' } else { ']' },
        0x31 => if shift { '|' } else { '\\' },
        0x33 => if shift { ':' } else { ';' },
        0x34 => if shift { '"' } else { '\'' },
        0x35 => if shift { '~' } else { '`' },
        0x36 => if shift { '<' } else { ',' },
        0x37 => if shift { '>' } else { '.' },
        0x38 => if shift { '?' } else { '/' },
        _ => return None,
    };
    
    Some(ascii)
}

/// Convert USB HID keycode to scancode (for compatibility with PS/2 scancodes)
/// This maps USB HID keycodes to a simplified scancode set
pub fn keycode_to_scancode(keycode: u8) -> Option<u16> {
    // USB HID keycode to simplified scancode mapping
    // This provides compatibility with existing PS/2 scancode handling
    let scancode = match keycode {
        // Letters
        0x04 => 0x1E, // A
        0x05 => 0x30, // B
        0x06 => 0x2E, // C
        0x07 => 0x20, // D
        0x08 => 0x12, // E
        0x09 => 0x21, // F
        0x0A => 0x22, // G
        0x0B => 0x23, // H
        0x0C => 0x17, // I
        0x0D => 0x24, // J
        0x0E => 0x25, // K
        0x0F => 0x26, // L
        0x10 => 0x32, // M
        0x11 => 0x31, // N
        0x12 => 0x18, // O
        0x13 => 0x19, // P
        0x14 => 0x10, // Q
        0x15 => 0x13, // R
        0x16 => 0x1F, // S
        0x17 => 0x14, // T
        0x18 => 0x16, // U
        0x19 => 0x2F, // V
        0x1A => 0x11, // W
        0x1B => 0x2D, // X
        0x1C => 0x15, // Y
        0x1D => 0x2C, // Z
        
        // Numbers
        0x1E => 0x02, // 1
        0x1F => 0x03, // 2
        0x20 => 0x04, // 3
        0x21 => 0x05, // 4
        0x22 => 0x06, // 5
        0x23 => 0x07, // 6
        0x24 => 0x08, // 7
        0x25 => 0x09, // 8
        0x26 => 0x0A, // 9
        0x27 => 0x0B, // 0
        
        // Special keys
        0x28 => 0x1C, // Enter
        0x29 => 0x01, // Escape
        0x2A => 0x0E, // Backspace
        0x2B => 0x0F, // Tab
        0x2C => 0x39, // Space
        0x2D => 0x0C, // -
        0x2E => 0x0D, // =
        0x2F => 0x1A, // [
        0x30 => 0x1B, // ]
        0x31 => 0x2B, // \
        0x33 => 0x27, // ;
        0x34 => 0x28, // '
        0x35 => 0x29, // `
        0x36 => 0x33, // ,
        0x37 => 0x34, // .
        0x38 => 0x35, // /
        
        // Function keys
        0x3A => 0x3B, // F1
        0x3B => 0x3C, // F2
        0x3C => 0x3D, // F3
        0x3D => 0x3E, // F4
        0x3E => 0x3F, // F5
        0x3F => 0x40, // F6
        0x40 => 0x41, // F7
        0x41 => 0x42, // F8
        0x42 => 0x43, // F9
        0x43 => 0x44, // F10
        0x44 => 0x57, // F11
        0x45 => 0x58, // F12
        
        // Navigation
        0x49 => 0xE047, // Insert (extended)
        0x4A => 0xE052, // Home (extended)
        0x4B => 0xE049, // Page Up (extended)
        0x4C => 0xE053, // Delete (extended)
        0x4D => 0xE04F, // End (extended)
        0x4E => 0xE051, // Page Down (extended)
        0x4F => 0xE04D, // Right Arrow (extended)
        0x50 => 0xE04B, // Left Arrow (extended)
        0x51 => 0xE050, // Down Arrow (extended)
        0x52 => 0xE048, // Up Arrow (extended)
        
        // Modifiers (we handle these specially, but include for completeness)
        0xE0 => 0x1D, // Left Ctrl
        0xE1 => 0x2A, // Left Shift
        0xE2 => 0x38, // Left Alt
        0xE3 => 0xE05B, // Left GUI (Windows key)
        0xE4 => 0xE01D, // Right Ctrl
        0xE5 => 0x36, // Right Shift
        0xE6 => 0xE038, // Right Alt
        0xE7 => 0xE05C, // Right GUI
        
        _ => return None,
    };
    
    Some(scancode)
}

/// Check if a USB HID keycode is a modifier key
pub fn is_modifier_key(keycode: u8) -> bool {
    matches!(keycode, 0xE0..=0xE7)
}

/// Get modifier bit for a modifier keycode
pub fn modifier_key_to_bit(keycode: u8) -> Option<u8> {
    match keycode {
        0xE0 => Some(HID_MOD_LEFT_CTRL),
        0xE1 => Some(HID_MOD_LEFT_SHIFT),
        0xE2 => Some(HID_MOD_LEFT_ALT),
        0xE3 => Some(HID_MOD_LEFT_GUI),
        0xE4 => Some(HID_MOD_RIGHT_CTRL),
        0xE5 => Some(HID_MOD_RIGHT_SHIFT),
        0xE6 => Some(HID_MOD_RIGHT_ALT),
        0xE7 => Some(HID_MOD_RIGHT_GUI),
        _ => None,
    }
}
