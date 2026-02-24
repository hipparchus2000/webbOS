//! USB Subsystem
//!
//! USB Host Controller and HID drivers for Raspberry Pi.
//! Uses the DWC OTG controller found in BCM2835/2836/2837/2711.

pub mod dwc_otg;
pub mod hid;

use crate::println;

/// USB Error types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UsbError {
    /// Success
    Success,
    /// Device not found
    NotFound,
    /// Initialization failed
    InitFailed,
    /// I/O error
    IoError,
    /// Timeout
    Timeout,
    /// Invalid descriptor
    InvalidDescriptor,
    /// Device not responding
    DeviceNotResponding,
    /// No channels available
    NoChannels,
    /// Invalid channel
    InvalidChannel,
    /// Transfer error
    TransferError,
    /// STALL received
    Stall,
    /// NAK received
    Nak,
    /// Device disconnected
    Disconnected,
}

/// USB Device State
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeviceState {
    Disconnected,
    Connected,
    Resetting,
    Addressed,
    Configured,
}

/// USB Controller State
struct UsbState {
    initialized: bool,
    device_state: DeviceState,
    last_poll_tick: u64,
    enumeration_pending: bool,
}

impl UsbState {
    const fn new() -> Self {
        Self {
            initialized: false,
            device_state: DeviceState::Disconnected,
            last_poll_tick: 0,
            enumeration_pending: false,
        }
    }
}

use spin::Mutex;
use lazy_static::lazy_static;

lazy_static! {
    static ref USB_STATE: Mutex<UsbState> = Mutex::new(UsbState::new());
}

/// Initialize the USB subsystem
pub fn init() {
    println!("[usb] Initializing USB subsystem...");
    
    // Initialize DWC OTG controller
    if let Err(e) = dwc_otg::init() {
        println!("[usb] Failed to initialize DWC OTG: {:?}", e);
        return;
    }
    
    // Initialize HID driver
    hid::init();
    
    let mut state = USB_STATE.lock();
    state.initialized = true;
    
    println!("[usb] USB subsystem initialized");
    println!("[usb] Waiting for device connection...");
}

/// Poll the USB subsystem for events and input
/// 
/// This should be called regularly from a timer interrupt or main loop.
/// On Pi, this runs at approximately 1000Hz polling rate.
pub fn poll() {
    let mut state = USB_STATE.lock();
    
    if !state.initialized {
        return;
    }
    
    // Poll the DWC OTG controller
    dwc_otg::poll();
    
    // Check for device connection
    match state.device_state {
        DeviceState::Disconnected => {
            if dwc_otg::is_device_connected() {
                println!("[usb] Device connected!");
                state.device_state = DeviceState::Connected;
                state.enumeration_pending = true;
            }
        }
        DeviceState::Connected => {
            // Start enumeration if pending
            if state.enumeration_pending {
                drop(state);
                
                // Reset the port
                if let Err(e) = dwc_otg::reset_port() {
                    println!("[usb] Port reset failed: {:?}", e);
                    state = USB_STATE.lock();
                    state.device_state = DeviceState::Disconnected;
                    return;
                }
                
                // Assign address 1
                let new_addr = dwc_otg::alloc_device_address();
                if let Err(e) = dwc_otg::set_address(0, new_addr) {
                    println!("[usb] Set address failed: {:?}", e);
                    state = USB_STATE.lock();
                    state.device_state = DeviceState::Disconnected;
                    return;
                }
                
                // Enumerate as HID device
                if let Err(e) = hid::enumerate_device(new_addr) {
                    println!("[usb] HID enumeration failed: {:?}", e);
                    // Device might not be HID, that's okay
                }
                
                state = USB_STATE.lock();
                state.device_state = DeviceState::Configured;
                state.enumeration_pending = false;
            }
        }
        DeviceState::Configured => {
            // Check if device is still connected
            if !dwc_otg::is_device_connected() {
                println!("[usb] Device disconnected!");
                state.device_state = DeviceState::Disconnected;
            }
        }
        _ => {}
    }
    
    drop(state);
    
    // Poll HID devices for input
    hid::poll();
}

/// Get an input event from the USB HID subsystem
/// 
/// Returns `Some(InputEvent)` if an event is available, `None` otherwise.
/// This is the main interface for the input subsystem.
pub fn get_event() -> Option<crate::drivers::input::InputEvent> {
    hid::get_event()
}

/// Check if a USB keyboard is connected
pub fn is_keyboard_connected() -> bool {
    hid::is_keyboard_connected()
}

/// Check if a USB mouse is connected
pub fn is_mouse_connected() -> bool {
    hid::is_mouse_connected()
}

/// Get current mouse position from USB mouse
pub fn get_mouse_position() -> (i32, i32) {
    hid::get_mouse_position()
}

/// Set mouse position
pub fn set_mouse_position(x: i32, y: i32) {
    hid::set_mouse_position(x, y)
}

/// Get current mouse button state
pub fn get_mouse_buttons() -> u8 {
    hid::get_mouse_buttons()
}

/// Print USB subsystem status
pub fn print_info() {
    let state = USB_STATE.lock();
    
    println!("USB Subsystem Status:");
    println!("  Initialized: {}", state.initialized);
    println!("  Device state: {:?}", state.device_state);
    
    hid::print_info();
}
