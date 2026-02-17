//! USB HID (Human Interface Device) Driver
//!
//! Supports keyboards, mice, and other HID devices.

use alloc::vec::Vec;
use alloc::string::String;
use alloc::format;

use crate::println;
use crate::error::UsbError;
use super::{UsbDriver, UsbDevice, UsbClass};

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
    /// Current key states
    pub keys: [bool; 256],
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

    /// Process keyboard report
    fn process_keyboard_report(&mut self, address: u8, report: &KeyboardReport) {
        // Convert modifiers and keys to events
        // This would integrate with the input subsystem
        
        let modifiers = report.modifiers;
        let keys = report.keys;
        
        // Check for changes
        if modifiers != 0 || keys.iter().any(|&k| k != 0) {
            // TODO: Send to input subsystem
        }
    }

    /// Process mouse report
    fn process_mouse_report(&mut self, address: u8, report: &MouseReport) {
        // Update mouse position
        if let Some(mouse) = self.mice.iter_mut().find(|m| m.address == address) {
            mouse.x += report.x as i32;
            mouse.y += report.y as i32;
            mouse.buttons = report.buttons;
            
            // TODO: Send to input subsystem
        }
    }
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
        
        // TODO:
        // 1. Parse configuration descriptors to find HID interfaces
        // 2. Determine if keyboard or mouse (or other)
        // 3. Set boot protocol if supported
        // 4. Configure endpoints
        // 5. Start polling for reports
        
        println!("[usb-hid] HID device initialized");
        Ok(())
    }

    fn disconnect(&mut self, device: &UsbDevice) {
        println!("[usb-hid] HID device disconnected from address {}", device.address);
        
        // Remove from our lists
        self.keyboards.retain(|k| k.address != device.address);
        self.mice.retain(|m| m.address != device.address);
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

/// Convert USB keycode to ASCII (simplified)
pub fn keycode_to_ascii(keycode: u8, shift: bool) -> Option<char> {
    // Standard USB HID keycodes to ASCII mapping
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
