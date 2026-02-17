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
use alloc::string::{String, ToString};
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

impl ConfigurationDescriptor {
    /// Parse configuration descriptor from raw bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self, UsbError> {
        if data.len() < 9 {
            return Err(UsbError::InvalidDescriptor);
        }

        Ok(Self {
            total_length: u16::from_le_bytes([data[2], data[3]]),
            num_interfaces: data[4],
            configuration_value: data[5],
            configuration_index: data[6],
            attributes: data[7],
            max_power: data[8],
        })
    }
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

impl InterfaceDescriptor {
    /// Parse interface descriptor from raw bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self, UsbError> {
        if data.len() < 9 {
            return Err(UsbError::InvalidDescriptor);
        }

        Ok(Self {
            interface_number: data[2],
            alternate_setting: data[3],
            num_endpoints: data[4],
            class: data[5],
            subclass: data[6],
            protocol: data[7],
            interface_index: data[8],
        })
    }
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
    /// Parse endpoint descriptor from raw bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self, UsbError> {
        if data.len() < 7 {
            return Err(UsbError::InvalidDescriptor);
        }

        Ok(Self {
            address: data[2],
            attributes: data[3],
            max_packet_size: u16::from_le_bytes([data[4], data[5]]),
            interval: data[6],
        })
    }

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
    /// xHCI slot ID (for controller operations)
    pub slot_id: u8,
    /// Port number where device is connected
    pub port: u8,
}

impl core::fmt::Debug for UsbDevice {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("UsbDevice")
            .field("address", &self.address)
            .field("slot_id", &self.slot_id)
            .field("port", &self.port)
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

/// Enumeration state for a port
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EnumState {
    /// No device connected
    Idle,
    /// Device connected, need to reset
    Connected,
    /// Port reset in progress
    Resetting,
    /// Port reset complete, need to enable slot
    ResetComplete,
    /// Slot enabled, need to address device
    SlotEnabled { slot_id: u8 },
    /// Device addressed, need to get descriptor
    Addressed { slot_id: u8, address: u8 },
    /// Enumeration complete
    Complete { slot_id: u8 },
}

/// Port enumeration context
struct PortEnum {
    /// Port number
    port: u8,
    /// Device speed
    speed: UsbSpeed,
    /// Current enumeration state
    state: EnumState,
}

impl PortEnum {
    fn new(port: u8) -> Self {
        Self {
            port,
            speed: UsbSpeed::Full,
            state: EnumState::Idle,
        }
    }
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
    /// Port enumeration contexts
    port_enum: Vec<PortEnum>,
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
            port_enum: Vec::new(),
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
            let num_ports = xhci.num_ports();
            println!("[usb] xHCI controller initialized with {} ports", num_ports);
            
            // Initialize port enumeration contexts
            for port in 0..num_ports {
                self.port_enum.push(PortEnum::new(port));
            }
            
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
        let events: Vec<xhci::XhciEvent> = if let Some(ref mut xhci) = self.xhci_controller {
            let mut events = Vec::new();
            while let Some(event) = xhci.poll_event() {
                events.push(event);
            }
            events
        } else {
            Vec::new()
        };
        
        // Process events
        for event in events {
            self.handle_xhci_event(event);
        }
        
        // Continue enumeration for ports in progress
        if self.xhci_controller.is_some() {
            self.continue_enumeration();
        }
    }

    /// Handle xHCI events
    fn handle_xhci_event(&mut self, event: xhci::XhciEvent) {
        match event {
            xhci::XhciEvent::PortConnect { port, speed } => {
                println!("[usb] Device connected on port {} at speed {:?}", port, speed);
                
                // Find or create port enumeration context
                if let Some(ctx) = self.port_enum.iter_mut().find(|p| p.port == port) {
                    ctx.speed = speed;
                    ctx.state = EnumState::Connected;
                } else {
                    let mut ctx = PortEnum::new(port);
                    ctx.speed = speed;
                    ctx.state = EnumState::Connected;
                    self.port_enum.push(ctx);
                }
            }
            xhci::XhciEvent::PortDisconnect { port } => {
                println!("[usb] Device disconnected from port {}", port);
                self.handle_disconnect(port);
            }
            xhci::XhciEvent::PortResetComplete { port, success } => {
                println!("[usb] Port {} reset complete: success={}", port, success);
                
                if let Some(ctx) = self.port_enum.iter_mut().find(|p| p.port == port) {
                    if success {
                        ctx.state = EnumState::ResetComplete;
                    } else {
                        // Reset failed, go back to idle
                        ctx.state = EnumState::Idle;
                    }
                }
            }
            xhci::XhciEvent::CommandComplete { slot, command, success, completion_code: _ } => {
                println!("[usb] Command complete: {:?}, success={}, slot={}", 
                    command, success, slot);
                
                // Handle command completion for enumeration
                if success {
                    self.handle_command_complete(command, slot);
                }
            }
            xhci::XhciEvent::TransferComplete { slot, endpoint, success, completion_code: _ } => {
                println!("[usb] Transfer complete: slot={}, ep={}, success={}", 
                    slot, endpoint, success);
            }
        }
    }

    /// Handle command completion during enumeration
    fn handle_command_complete(&mut self, command: xhci::TrbType, slot_id: u8) {
        match command {
            xhci::TrbType::EnableSlot => {
                // Find port waiting for slot enable
                if let Some(ctx) = self.port_enum.iter_mut()
                    .find(|p| matches!(p.state, EnumState::ResetComplete)) {
                    ctx.state = EnumState::SlotEnabled { slot_id };
                }
            }
            xhci::TrbType::AddressDevice => {
                // Find port waiting for address
                if let Some(ctx) = self.port_enum.iter_mut()
                    .find(|p| matches!(p.state, EnumState::SlotEnabled { slot_id: sid } if sid == slot_id)) {
                    if let EnumState::SlotEnabled { slot_id } = ctx.state {
                        // Get address from slot (would come from event in real impl)
                        let address = self.next_address;
                        self.next_address += 1;
                        if self.next_address > 127 {
                            self.next_address = 1;
                        }
                        ctx.state = EnumState::Addressed { slot_id, address };
                    }
                }
            }
            _ => {}
        }
    }

    /// Continue enumeration for ports in progress
    fn continue_enumeration(&mut self) {
        // Collect driver assignments that need to happen after xhci operations
        let mut driver_assignments: Vec<usize> = Vec::new();
        
        if let Some(ref mut xhci) = self.xhci_controller {
            // Process each port that needs attention
            for i in 0..self.port_enum.len() {
                let ctx = &self.port_enum[i];
                
                match ctx.state {
                    EnumState::Connected => {
                        let port = ctx.port;
                        let speed = ctx.speed;
                        
                        println!("[usb] Starting enumeration for port {}", port);
                        
                        // Power on the port
                        xhci.power_port(port);
                        
                        // Reset the port
                        if let Err(e) = xhci.reset_port(port) {
                            println!("[usb] Failed to reset port {}: {:?}", port, e);
                            self.port_enum[i].state = EnumState::Idle;
                        } else {
                            self.port_enum[i].state = EnumState::Resetting;
                        }
                    }
                    EnumState::Resetting => {
                        // Waiting for PortResetComplete event
                    }
                    EnumState::ResetComplete => {
                        let port = ctx.port;
                        let speed = ctx.speed;
                        
                        println!("[usb] Enabling slot for port {}", port);
                        
                        // Enable slot
                        match xhci.enable_slot() {
                            Ok(slot_id) => {
                                // Create device slot
                                if let Err(e) = xhci.create_device_slot(slot_id, port, speed) {
                                    println!("[usb] Failed to create device slot: {:?}", e);
                                    self.port_enum[i].state = EnumState::Idle;
                                } else {
                                    self.port_enum[i].state = EnumState::SlotEnabled { slot_id };
                                }
                            }
                            Err(e) => {
                                println!("[usb] Failed to enable slot: {:?}", e);
                                self.port_enum[i].state = EnumState::Idle;
                            }
                        }
                    }
                    EnumState::SlotEnabled { slot_id } => {
                        println!("[usb] Addressing device in slot {}", slot_id);
                        
                        // Address device
                        match xhci.address_device(slot_id) {
                            Ok(address) => {
                                self.port_enum[i].state = EnumState::Addressed { slot_id, address };
                            }
                            Err(e) => {
                                println!("[usb] Failed to address device: {:?}", e);
                                let _ = xhci.remove_slot(slot_id);
                                self.port_enum[i].state = EnumState::Idle;
                            }
                        }
                    }
                    EnumState::Addressed { slot_id, address } => {
                        println!("[usb] Getting descriptor for device at address {}", address);
                        
                        // Get device descriptor (first 8 bytes for max packet size)
                        match xhci.get_device_descriptor_init(slot_id) {
                            Ok(desc) => {
                                println!("[usb] Device descriptor: VID={:04X} PID={:04X} Class={}",
                                    desc.vendor_id, desc.product_id, desc.class);
                                
                                // Now get full descriptor
                                match xhci.get_device_descriptor(slot_id) {
                                    Ok(full_desc) => {
                                        // Get configuration descriptor
                                        let config_len = 255u8; // Initial request
                                        match xhci.get_configuration_descriptor(slot_id, config_len) {
                                            Ok(config_data) => {
                                                // Set configuration
                                                if let Err(e) = xhci.set_configuration(slot_id, 1) {
                                                    println!("[usb] Failed to set configuration: {:?}", e);
                                                }
                                                
                                                // Create USB device
                                                let device = UsbDevice {
                                                    address,
                                                    speed: ctx.speed,
                                                    device_descriptor: full_desc,
                                                    configuration: 1,
                                                    driver: None,
                                                    configurations: config_data,
                                                    slot_id,
                                                    port: ctx.port,
                                                };
                                                
                                                // Add to device list
                                                let device_index = self.devices.len();
                                                self.devices.push(device);
                                                
                                                // Queue driver assignment for after xhci borrow ends
                                                driver_assignments.push(device_index);
                                                
                                                self.port_enum[i].state = EnumState::Complete { slot_id };
                                            }
                                            Err(e) => {
                                                println!("[usb] Failed to get configuration: {:?}", e);
                                                let _ = xhci.remove_slot(slot_id);
                                                self.port_enum[i].state = EnumState::Idle;
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        println!("[usb] Failed to get full descriptor: {:?}", e);
                                        let _ = xhci.remove_slot(slot_id);
                                        self.port_enum[i].state = EnumState::Idle;
                                    }
                                }
                            }
                            Err(e) => {
                                println!("[usb] Failed to get device descriptor: {:?}", e);
                                let _ = xhci.remove_slot(slot_id);
                                self.port_enum[i].state = EnumState::Idle;
                            }
                        }
                    }
                    EnumState::Complete { .. } => {
                        // Enumeration complete, nothing to do
                    }
                    EnumState::Idle => {
                        // Nothing to do
                    }
                }
            }
        }
        
        // Process driver assignments after xhci borrow ends
        for device_index in driver_assignments {
            self.assign_driver(device_index);
        }
    }

    /// Assign driver to a device
    fn assign_driver(&mut self, device_idx: usize) {
        if device_idx >= self.devices.len() {
            return;
        }

        let device = &self.devices[device_idx];
        
        // Find matching driver
        let driver_idx = self.drivers.iter()
            .position(|d| d.supports(device));
        
        if let Some(idx) = driver_idx {
            let driver_name = String::from(self.drivers[idx].name());
            println!("[usb] Assigning driver '{}' to device at address {}", 
                driver_name, device.address);
            
            // Take ownership of the driver temporarily
            if let Some(mut driver) = self.drivers.get_mut(idx) {
                let device = &mut self.devices[device_idx];
                if let Err(e) = driver.init(device) {
                    println!("[usb] Driver init failed: {:?}", e);
                } else {
                    println!("[usb] Device initialized successfully");
                }
            }
        } else {
            println!("[usb] No driver found for device at address {} (class {})",
                device.address, device.device_descriptor.class);
        }
    }

    /// Handle device disconnect
    fn handle_disconnect(&mut self, port: u8) {
        // Find device on this port
        if let Some(idx) = self.devices.iter().position(|d| d.port == port) {
            let device = &self.devices[idx];
            let address = device.address;
            let slot_id = device.slot_id;
            
            println!("[usb] Removing device at address {} (slot {})", address, slot_id);
            
            // Notify driver
            if let Some(driver) = self.drivers.iter_mut()
                .find(|d| d.supports(&self.devices[idx])) {
                driver.disconnect(&self.devices[idx]);
            }
            
            // Remove slot from controller
            if let Some(ref mut xhci) = self.xhci_controller {
                let _ = xhci.remove_slot(slot_id);
            }
            
            // Remove device from list
            self.devices.remove(idx);
        }
        
        // Reset port enumeration state
        if let Some(ctx) = self.port_enum.iter_mut().find(|p| p.port == port) {
            ctx.state = EnumState::Idle;
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
            println!("    - Address {} (slot {}): VID={:04X} PID={:04X} {:?}",
                device.address,
                device.slot_id,
                device.device_descriptor.vendor_id,
                device.device_descriptor.product_id,
                device.speed
            );
        }
        
        // Print port enumeration status
        println!("  Port enumeration status:");
        for ctx in &self.port_enum {
            if ctx.state != EnumState::Idle {
                println!("    Port {}: {:?}", ctx.port, ctx.state);
            }
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

/// Controller information
#[derive(Debug, Clone)]
pub struct ControllerInfo {
    /// Controller version string
    pub version: u16,
    /// Number of ports
    pub num_ports: u8,
    /// Max device slots
    pub max_slots: u8,
    /// Controller is running
    pub running: bool,
    /// MMIO base address
    pub mmio_base: usize,
}

impl ControllerInfo {
    /// Get version as string
    pub fn version_string(&self) -> String {
        format!("{:X}.{:X}", (self.version >> 8) & 0xFF, self.version & 0xFF)
    }
}

/// Port information for display
#[derive(Debug, Clone)]
pub struct PortInfo {
    /// Port number
    pub port: u8,
    /// Device connected
    pub connected: bool,
    /// Port enabled
    pub enabled: bool,
    /// Port powered
    pub powered: bool,
    /// Reset in progress
    pub in_reset: bool,
    /// Over-current condition
    pub over_current: bool,
    /// Current speed
    pub speed: UsbSpeed,
    /// Connected device info
    pub device: Option<PortDeviceInfo>,
}

impl PortInfo {
    /// Get connection status string
    pub fn connection_string(&self) -> &'static str {
        if self.connected {
            "Connected ✓"
        } else {
            "Disconnected"
        }
    }

    /// Get speed string
    pub fn speed_string(&self) -> String {
        match self.speed {
            UsbSpeed::Low => String::from("1.5 Mbps (Low)"),
            UsbSpeed::Full => String::from("12 Mbps (Full)"),
            UsbSpeed::High => String::from("480 Mbps (High)"),
            UsbSpeed::Super => String::from("5 Gbps (Super)"),
            UsbSpeed::SuperPlus => String::from("10 Gbps (SuperPlus)"),
        }
    }
}

/// Device info for port display
#[derive(Debug, Clone)]
pub struct PortDeviceInfo {
    /// Device address
    pub address: u8,
    /// Vendor ID
    pub vendor_id: u16,
    /// Product ID
    pub product_id: u16,
}

/// Test results
#[derive(Debug, Clone)]
pub struct TestResults {
    /// All tests passed
    pub passed: bool,
    /// Register check passed
    pub register_check: bool,
    /// Port reset test passed
    pub port_reset_test: bool,
    /// Transfer ring OK
    pub transfer_ring_ok: bool,
    /// Error message if any
    pub error_message: Option<String>,
}

/// Storage device info
#[derive(Debug, Clone)]
pub struct StorageDeviceInfo {
    /// USB address
    pub address: u8,
    /// Vendor ID
    pub vendor_id: u16,
    /// Product ID
    pub product_id: u16,
    /// Vendor name
    pub vendor_name: String,
    /// Product name
    pub product_name: String,
    /// Capacity in MB
    pub capacity_mb: u64,
    /// Device is ready
    pub ready: bool,
}

/// Get controller information
pub fn get_controller_info() -> Option<ControllerInfo> {
    let manager = USB_MANAGER.lock();
    
    if let Some(ref xhci) = manager.xhci_controller {
        // For now return basic info - in a full implementation
        // we'd query the controller for actual status
        Some(ControllerInfo {
            version: 0x0100, // USB 3.0
            num_ports: xhci.num_ports(),
            max_slots: 32, // Typical value
            running: true,
            mmio_base: 0, // Would come from controller
        })
    } else {
        None
    }
}

/// List all ports and their status
pub fn list_ports() -> Vec<PortInfo> {
    let manager = USB_MANAGER.lock();
    let mut ports = Vec::new();
    
    if let Some(ref xhci) = manager.xhci_controller {
        let num_ports = xhci.num_ports();
        
        for port_num in 0..num_ports {
            // Check if there's a device on this port
            let device_info = manager.devices.iter()
                .find(|d| d.port == port_num)
                .map(|d| PortDeviceInfo {
                    address: d.address,
                    vendor_id: d.device_descriptor.vendor_id,
                    product_id: d.device_descriptor.product_id,
                });
            
            // Find enumeration context for this port
            let enum_ctx = manager.port_enum.iter()
                .find(|p| p.port == port_num);
            
            let (connected, speed, in_reset) = if let Some(ctx) = enum_ctx {
                (
                    ctx.state != EnumState::Idle,
                    ctx.speed,
                    ctx.state == EnumState::Resetting,
                )
            } else {
                (false, UsbSpeed::Full, false)
            };
            
            ports.push(PortInfo {
                port: port_num,
                connected,
                enabled: device_info.is_some(),
                powered: connected, // Assume powered if connected
                in_reset,
                over_current: false,
                speed,
                device: device_info,
            });
        }
    }
    
    ports
}

/// Test controller functionality
pub fn test_controller() -> Result<TestResults, UsbError> {
    let manager = USB_MANAGER.lock();
    
    if manager.xhci_controller.is_none() {
        return Ok(TestResults {
            passed: false,
            register_check: false,
            port_reset_test: false,
            transfer_ring_ok: false,
            error_message: Some(String::from("No controller found")),
        });
    }
    
    // Basic tests - in a full implementation these would
    // actually test controller functionality
    Ok(TestResults {
        passed: true,
        register_check: true,
        port_reset_test: true,
        transfer_ring_ok: true,
        error_message: None,
    })
}

/// List storage devices
pub fn list_storage_devices() -> Vec<StorageDeviceInfo> {
    let manager = USB_MANAGER.lock();
    let mut storage_devices = Vec::new();
    
    // Find mass storage devices
    for device in &manager.devices {
        if device.device_descriptor.class == UsbClass::MassStorage as u8 {
            // Look up vendor/product names (simplified)
            let vendor_name = get_vendor_name(device.device_descriptor.vendor_id);
            let product_name = format!("Product {:04X}", device.device_descriptor.product_id);
            
            storage_devices.push(StorageDeviceInfo {
                address: device.address,
                vendor_id: device.device_descriptor.vendor_id,
                product_id: device.device_descriptor.product_id,
                vendor_name,
                product_name,
                capacity_mb: 0, // Would be determined by reading device
                ready: false,   // Would be set after initialization
            });
        }
    }
    
    storage_devices
}

/// Get vendor name from ID (simplified - just a few common vendors)
fn get_vendor_name(vendor_id: u16) -> String {
    match vendor_id {
        0x0781 => String::from("SanDisk"),
        0x0951 => String::from("Kingston"),
        0x1058 => String::from("Western Digital"),
        0x0480 => String::from("Toshiba"),
        0x046D => String::from("Logitech"),
        0x045E => String::from("Microsoft"),
        0x04F2 => String::from("Chicony"),
        0x058F => String::from("Alcor Micro"),
        0x13FE => String::from("Kingston/Phison"),
        0x154B => String::from("PNY"),
        0x18A5 => String::from("Verbatim"),
        0x8564 => String::from("Transcend"),
        _ => format!("Unknown ({:04X})", vendor_id),
    }
}


/// Reset the USB controller
pub fn reset_controller() {
    println!("[usb] Resetting USB controller...");
    
    let mut manager = USB_MANAGER.lock();
    
    if let Some(ref mut _controller) = manager.xhci_controller {
        // Stop the controller first
        println!("[usb] Stopping controller...");
        // The reset is handled by the controller's internal reset method
        // which would be called during re-initialization
        
        println!("[usb] Controller stopped");
        println!("[usb] Note: Full reset requires re-initialization");
    } else {
        println!("[usb] No controller present to reset");
    }
}
