//! USB Subsystem (PC/x86_64)
//!
//! USB Host Controller and HID drivers for PC.
//! Currently uses PS/2 for input (see drivers::input).
//! Future: xHCI USB controller support.
//!
//! NOTE: USB architecture differs between PC (x86_64) and Pi (ARM):
//! - PC uses xHCI/UHCI/OHCI controllers accessed via PCI
//! - Pi uses DWC OTG controller (ARM-specific)

#![allow(dead_code)]

use crate::println;

/// USB Error types (matching Pi API for consistency)
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
    /// Not implemented
    NotImplemented,
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

/// USB Controller State (stub for future xHCI)
struct UsbState {
    initialized: bool,
    device_state: DeviceState,
    xhci_available: bool,
}

impl UsbState {
    const fn new() -> Self {
        Self {
            initialized: false,
            device_state: DeviceState::Disconnected,
            xhci_available: false,
        }
    }
}

use spin::Mutex;
use lazy_static::lazy_static;

lazy_static! {
    static ref USB_STATE: Mutex<UsbState> = Mutex::new(UsbState::new());
}

/// Initialize the USB subsystem
/// 
/// Currently a stub - input is handled via PS/2 on PC.
/// Future: Detect and initialize xHCI controller via PCI.
pub fn init() {
    println!("[usb] USB subsystem initialization (PC/x86_64)...");
    
    // Check if xHCI controller is available via PCI
    if check_xhci_available() {
        println!("[usb] xHCI controller detected - USB 3.0 support available");
        println!("[usb] NOTE: xHCI driver not yet implemented, using PS/2 input");
        
        let mut state = USB_STATE.lock();
        state.xhci_available = true;
        state.initialized = false; // Not fully initialized yet
    } else {
        println!("[usb] No xHCI controller detected");
        println!("[usb] Input will use PS/2 (keyboard/mouse)");
    }
    
    let mut state = USB_STATE.lock();
    state.initialized = true;
    
    println!("[usb] USB subsystem ready");
}

/// Check if xHCI USB controller is available via PCI
fn check_xhci_available() -> bool {
    // Look for USB controller class (0x0C), subclass 0x03 (USB), xHCI PI (0x30)
    use crate::drivers::pci;
    
    // Check for xHCI controller (USB 3.0)
    for bus in 0..=255u8 {
        for device in 0..32u8 {
            let class = pci::read_config8(bus, device, 0, 0x0B);
            let subclass = pci::read_config8(bus, device, 0, 0x0A);
            let prog_if = pci::read_config8(bus, device, 0, 0x09);
            
            if class == 0x0C && subclass == 0x03 {
                // Found USB controller
                match prog_if {
                    0x30 => {
                        let vendor = pci::read_config16(bus, device, 0, 0x00);
                        let dev_id = pci::read_config16(bus, device, 0, 0x02);
                        println!("[usb] Found xHCI controller: {:04X}:{:04X} at {:02X}:{:02X}.0",
                            vendor, dev_id, bus, device);
                        return true;
                    }
                    0x20 => {
                        println!("[usb] Found EHCI (USB 2.0) controller");
                    }
                    0x10 => {
                        println!("[usb] Found OHCI controller");
                    }
                    0x00 => {
                        println!("[usb] Found UHCI controller");
                    }
                    _ => {}
                }
            }
        }
    }
    
    false
}

/// Poll the USB subsystem (stub for Pi compatibility)
/// 
/// On PC, this is a no-op since input is handled via PS/2 IRQs.
/// Future: Poll xHCI for USB HID events.
pub fn poll() {
    let state = USB_STATE.lock();
    
    if !state.initialized {
        return;
    }
    
    // Future: Poll xHCI controller for USB events
    // For now, input is handled by PS/2 driver in drivers::input
}

/// Get an input event from USB HID (stub for Pi compatibility)
/// 
/// On PC, use drivers::input::poll_event() instead.
/// Future: Return USB HID events from xHCI.
pub fn get_event() -> Option<crate::drivers::input::InputEvent> {
    // Future: Return events from USB HID
    // For now, PC uses PS/2 via drivers::input
    None
}

/// Check if a USB keyboard is connected (stub for Pi compatibility)
pub fn is_keyboard_connected() -> bool {
    // Future: Check if USB keyboard is connected via xHCI
    // For now, PC uses PS/2 keyboard
    false
}

/// Check if a USB mouse is connected (stub for Pi compatibility)
pub fn is_mouse_connected() -> bool {
    // Future: Check if USB mouse is connected via xHCI
    // For now, PC uses PS/2 mouse
    false
}

/// Get current mouse position from USB mouse (stub for Pi compatibility)
pub fn get_mouse_position() -> (i32, i32) {
    // Future: Get position from USB mouse
    // For now, delegate to PS/2 driver
    crate::drivers::input::mouse_position()
}

/// Set mouse position (stub for Pi compatibility)
pub fn set_mouse_position(x: i32, y: i32) {
    crate::drivers::input::set_mouse_screen_dimensions(
        crate::drivers::input::mouse_position().0 + x,
        crate::drivers::input::mouse_position().1 + y
    );
}

/// Get current mouse button state (stub for Pi compatibility)
pub fn get_mouse_buttons() -> u8 {
    // Future: Get buttons from USB mouse
    // For now, delegate to PS/2 driver
    crate::drivers::input::mouse_buttons()
}

/// Print USB subsystem status
pub fn print_info() {
    let state = USB_STATE.lock();
    
    println!("USB Subsystem Status:");
    println!("  Initialized: {}", state.initialized);
    println!("  Device state: {:?}", state.device_state);
    println!("  xHCI available: {}", state.xhci_available);
    println!("  Input method: PS/2 (via drivers::input)");
    println!("  USB HID: Not implemented (future xHCI driver)");
}

// =============================================================================
// Future xHCI support placeholders
// =============================================================================

/// xHCI controller base address (will be read from PCI BAR)
static XHCI_BASE: core::sync::atomic::AtomicU64 = 
    core::sync::atomic::AtomicU64::new(0);

/// Initialize xHCI controller (future implementation)
#[allow(unused)]
fn init_xhci() -> Result<(), UsbError> {
    // Future: Full xHCI initialization
    // 1. Read BAR from PCI
    // 2. Map MMIO region
    // 3. Reset controller
    // 4. Set up event rings
    // 5. Configure ports
    // 6. Enable interrupts
    Err(UsbError::NotImplemented)
}
