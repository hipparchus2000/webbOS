//! USB Subsystem for WebbOS
//!
//! Provides USB host controller support including:
//! - xHCI (USB 3.0/3.1) controller driver
//! - USB hub support
//! - HID devices (keyboard, mouse)
//! - Mass storage devices
//!
//! # Architecture
//!
//! ```
//! ┌─────────────────────────────────────┐
//! │  USB Manager                        │
//! │  - Device enumeration               │
//! │  - Driver matching                    │
//! ├─────────────────────────────────────┤
//! │  USB Device Drivers                 │
//! │  ├── HID (keyboard/mouse)          │
//! │  ├── Mass Storage                   │
//! │  └── Hub                            │
//! ├─────────────────────────────────────┤
//! │  Host Controller Drivers            │
//! │  └── xHCI (USB 3.0)                 │
//! └─────────────────────────────────────┘
//! ```

use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::format;
use spin::Mutex;
use lazy_static::lazy_static;

use crate::println;
use crate::error::UsbError;

// Submodules
pub mod xhci;
pub mod hub;
pub mod hid;
pub mod mass_storage;

/// USB version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbVersion {
    /// USB 1.0/1.1 (Low/Full Speed)
    Usb1_1,
    /// USB 2.0 (High Speed)
    Usb2_0,
    /// USB 3.0 (SuperSpeed)
    Usb3_0,
    /// USB 3.1 (SuperSpeed+)
    Usb3_1,
    /// USB 3.2
    Usb3_2,
}

/// USB device speed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbSpeed {
    /// Low speed (1.5 Mbps)
    Low = 0,
    /// Full speed (12 Mbps)
    Full = 1,
    /// High speed (480 Mbps)
    High = 2,
    /// SuperSpeed (5 Gbps)
    Super = 3,
    /// SuperSpeed+ (10 Gbps)
    SuperPlus = 4,
}

impl UsbSpeed {
    /// Get speed in Mbps
    pub fn mbps(&self) -> u32 {
        match self {
            UsbSpeed::Low => 2,
            UsbSpeed::Full => 12,
            UsbSpeed::High => 480,
            UsbSpeed::Super => 5000,
            UsbSpeed::SuperPlus => 10000,
        }
    }

    /// Get speed from descriptor value
    pub fn from_descriptor(value: u8) -> Self {
        match value {
            1 => UsbSpeed::Full,
            2 => UsbSpeed::High,
            3 => UsbSpeed::Super,
            4 => UsbSpeed::SuperPlus,
            _ => UsbSpeed::Low,
        }
    }
}

/// USB device class codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum UsbClass {
    /// Interface defined at class level
    Interface = 0x00,
    /// Audio device
    Audio = 0x01,
    /// Communications device
    Communications = 0x02,
    /// Human Interface Device
    Hid = 0x03,
    /// Physical device
    Physical = 0x05,
    /// Still imaging device
    StillImaging = 0x06,
    /// Printer device
    Printer = 0x07,
    /// Mass storage device
    MassStorage = 0x08,
    /// Hub device
    Hub = 0x09,
    /// Communications data
    CdcData = 0x0A,
    /// Smart card
    SmartCard = 0x0B,
    /// Content security
    ContentSecurity = 0x0D,
    /// Video device
    Video = 0x0E,
    /// Personal healthcare
    PersonalHealthcare = 0x0F,
    /// Audio/Video device
    AudioVideo = 0x10,
    /// Billboard device
    Billboard = 0x11,
    /// USB Type-C bridge
    TypeCBridge = 0x12,
    /// Diagnostic device
    Diagnostic = 0xDC,
    /// Wireless controller
    Wireless = 0xE0,
    /// Miscellaneous
    Miscellaneous = 0xEF,
    /// Vendor specific
    VendorSpecific = 0xFF,
}

/// USB device descriptor
#[derive(Debug, Clone)]
pub struct DeviceDescriptor {
    /// USB specification version (BCD)
    pub usb_version: u16,
    /// Device class
    pub class: u8,
    /// Device subclass
    pub subclass: u8,
    /// Device protocol
    pub protocol: u8,
    /// Endpoint 0 max packet size
    pub max_packet_size: u8,
    /// Vendor ID
    pub vendor_id: u16,
    /// Product ID
    pub product_id: u16,
    /// Device version (BCD)
    pub device_version: u16,
    /// Manufacturer string index
    pub manufacturer_index: u8,
    /// Product string index
    pub product_index: u8,
    /// Serial number string index
    pub serial_index: u8,
    /// Number of configurations
    pub num_configurations: u8,
}

impl DeviceDescriptor {
    /// Parse device descriptor from raw bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self, UsbError> {
        if data.len() < 18 {
            return Err(UsbError::InvalidDescriptor);
        }

        Ok(Self {
            usb_version: u16::from_le_bytes([data[2], data[3]]),
            class: data[4],
            subclass: data[5],
            protocol: data[6],
            max_packet_size: data[7],
            vendor_id: u16::from_le_bytes([data[8], data[9]]),
            product_id: u16::from_le_bytes([data[10], data[11]]),
            device_version: u16::from_le_bytes([data[12], data[13]]),
            manufacturer_index: data[14],
            product_index: data[15],
            serial_index: data[16],
            num_configurations: data[17],
        })
    }
}

/// USB configuration descriptor
#[derive(Debug, Clone)]
pub struct ConfigurationDescriptor {
    /// Total length of this configuration
    pub total_length: u16,
    /// Number of interfaces
    pub num_interfaces: u8,
    /// Configuration value
    pub configuration_value: u8,
    /// Configuration string index
    pub configuration_index: u8,
    /// Attributes
    pub attributes: u8,
    /// Maximum power (in 2mA units)
    pub max_power: u8,
}

/// USB interface descriptor
#[derive(Debug, Clone)]
pub struct InterfaceDescriptor {
    /// Interface number
    pub interface_number: u8,
    /// Alternate setting
    pub alternate_setting: u8,
    /// Number of endpoints
    pub num_endpoints: u8,
    /// Interface class
    pub class: u8,
    /// Interface subclass
    pub subclass: u8,
    /// Interface protocol
    pub protocol: u8,
    /// Interface string index
    pub interface_index: u8,
}

/// USB endpoint descriptor
#[derive(Debug, Clone)]
pub struct EndpointDescriptor {
    /// Endpoint address (direction + number)
    pub address: u8,
    /// Attributes (transfer type, etc.)
    pub attributes: u8,
    /// Max packet size
    pub max_packet_size: u16,
    /// Polling interval (for interrupt endpoints)
    pub interval: u8,
}

impl EndpointDescriptor {
    /// Get endpoint number (0-15)
    pub fn number(&self) -> u8 {
        self.address & 0x0F
    }

    /// Check if endpoint is IN (device to host)
    pub fn is_in(&self) -> bool {
        (self.address & 0x80) != 0
    }

    /// Check if endpoint is OUT (host to device)
    pub fn is_out(&self) -> bool {
        !self.is_in()
    }

    /// Get transfer type
    pub fn transfer_type(&self) -> TransferType {
        match self.attributes & 0x03 {
            0 => TransferType::Control,
            1 => TransferType::Isochronous,
            2 => TransferType::Bulk,
            3 => TransferType::Interrupt,
            _ => unreachable!(),
        }
    }
}

/// USB transfer types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferType {
    /// Control transfer
    Control,
    /// Isochronous transfer
    Isochronous,
    /// Bulk transfer
    Bulk,
    /// Interrupt transfer
    Interrupt,
}

/// USB device representation
pub struct UsbDevice {
    /// Device address (1-127)
    pub address: u8,
    /// Device speed
    pub speed: UsbSpeed,
    /// Device descriptor
    pub device_descriptor: DeviceDescriptor,
    /// Current configuration
    pub configuration: u8,
    /// Driver assigned to this device
    pub driver: Option<Box<dyn UsbDriver>>,
    /// Raw configuration descriptors
    pub configurations: Vec<u8>,
}

impl core::fmt::Debug for UsbDevice {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("UsbDevice")
            .field("address", &self.address)
            .field("speed", &self.speed)
            .field("device_descriptor", &self.device_descriptor)
            .field("configuration", &self.configuration)
            .field("has_driver", &self.driver.is_some())
            .field("configurations_len", &self.configurations.len())
            .finish()
    }
}

/// USB driver trait
pub trait UsbDriver: Send + Sync {
    /// Get the name of this driver
    fn name(&self) -> &str;
    
    /// Check if this driver supports the given device
    fn supports(&self, device: &UsbDevice) -> bool;
    
    /// Initialize the driver for a device
    fn init(&mut self, device: &mut UsbDevice) -> Result<(), UsbError>;
    
    /// Handle device disconnect
    fn disconnect(&mut self, device: &UsbDevice);
}

/// USB manager - manages all USB devices and controllers
pub struct UsbManager {
    /// List of active USB devices
    devices: Vec<UsbDevice>,
    /// Registered drivers (excluded from Debug)
    drivers: Vec<Box<dyn UsbDriver>>,
    /// Next device address to assign
    next_address: u8,
    /// xHCI controller if present
    xhci_controller: Option<xhci::XhciController>,
}

impl core::fmt::Debug for UsbManager {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("UsbManager")
            .field("devices", &self.devices)
            .field("num_drivers", &self.drivers.len())
            .field("next_address", &self.next_address)
            .field("xhci_controller", &self.xhci_controller.is_some())
            .finish()
    }
}

impl UsbManager {
    /// Create new USB manager
    pub const fn new() -> Self {
        Self {
            devices: Vec::new(),
            drivers: Vec::new(),
            next_address: 1,
            xhci_controller: None,
        }
    }

    /// Register a USB driver
    pub fn register_driver(&mut self, driver: Box<dyn UsbDriver>) {
        println!("[usb] Registering driver: {}", driver.name());
        self.drivers.push(driver);
    }

    /// Initialize USB subsystem
    pub fn init(&mut self) -> Result<(), UsbError> {
        println!("[usb] Initializing USB subsystem...");

        // Try to initialize xHCI controller
        if let Ok(xhci) = xhci::XhciController::init() {
            println!("[usb] xHCI controller initialized");
            self.xhci_controller = Some(xhci);
        } else {
            println!("[usb] No xHCI controller found");
        }

        // Register built-in drivers
        self.register_driver(Box::new(hid::HidDriver::new()));
        self.register_driver(Box::new(mass_storage::MassStorageDriver::new()));
        self.register_driver(Box::new(hub::HubDriver::new()));

        println!("[usb] USB subsystem initialized with {} drivers", self.drivers.len());
        Ok(())
    }

    /// Poll for USB events
    pub fn poll(&mut self) {
        // Poll xHCI controller
        if let Some(ref mut xhci) = self.xhci_controller {
            if let Some(event) = xhci.poll_event() {
                self.handle_xhci_event(event);
            }
        }
    }

    /// Handle xHCI events
    fn handle_xhci_event(&mut self, event: xhci::XhciEvent) {
        match event {
            xhci::XhciEvent::PortConnect { port, speed } => {
                println!("[usb] Device connected on port {} at speed {:?}", port, speed);
                // TODO: Enumerate device
            }
            xhci::XhciEvent::PortDisconnect { port } => {
                println!("[usb] Device disconnected from port {}", port);
                // TODO: Remove device
            }
            _ => {}
        }
    }

    /// Get list of connected devices
    pub fn devices(&self) -> &[UsbDevice] {
        &self.devices
    }

    /// Print USB subsystem status
    pub fn print_status(&self) {
        println!("USB Subsystem:");
        
        if self.xhci_controller.is_some() {
            println!("  xHCI Controller: Present");
        } else {
            println!("  xHCI Controller: Not found");
        }
        
        println!("  Registered drivers: {}", self.drivers.len());
        for driver in &self.drivers {
            println!("    - {}", driver.name());
        }
        
        println!("  Connected devices: {}", self.devices.len());
        for device in &self.devices {
            println!("    - Address {}: VID={:04X} PID={:04X} {:?}",
                device.address,
                device.device_descriptor.vendor_id,
                device.device_descriptor.product_id,
                device.speed
            );
        }
    }
}

// SAFETY: UsbManager is thread-safe due to Mutex usage
unsafe impl Send for UsbManager {}
unsafe impl Sync for UsbManager {}

/// Global USB manager instance
lazy_static! {
    static ref USB_MANAGER: Mutex<UsbManager> = Mutex::new(UsbManager::new());
}

/// Initialize USB subsystem
pub fn init() -> Result<(), UsbError> {
    USB_MANAGER.lock().init()
}

/// Poll for USB events
pub fn poll() {
    USB_MANAGER.lock().poll();
}

/// Print USB status
pub fn print_status() {
    USB_MANAGER.lock().print_status();
}

/// Get list of USB devices
pub fn devices() -> Vec<UsbDeviceInfo> {
    let manager = USB_MANAGER.lock();
    manager.devices.iter().map(|d| UsbDeviceInfo {
        address: d.address,
        vendor_id: d.device_descriptor.vendor_id,
        product_id: d.device_descriptor.product_id,
        class: d.device_descriptor.class,
        speed: d.speed,
    }).collect()
}

/// USB device information (simplified for external use)
#[derive(Debug, Clone)]
pub struct UsbDeviceInfo {
    /// Device address
    pub address: u8,
    /// Vendor ID
    pub vendor_id: u16,
    /// Product ID
    pub product_id: u16,
    /// Device class
    pub class: u8,
    /// Device speed
    pub speed: UsbSpeed,
}
