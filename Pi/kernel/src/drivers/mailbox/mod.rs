//! VideoCore Mailbox Driver for Raspberry Pi
//!
//! The mailbox is a communication mechanism between the ARM CPU and the
//! VideoCore GPU. It uses a set of registers for sending/receiving messages.
//!
//! Mailbox base addresses:
//! - Pi 3 (BCM2837): 0x3F00B880
//! - Pi 4 (BCM2711): 0xFE00B880
//!
//! Channels:
//! - 0: Power management
//! - 1: Framebuffer (legacy, not used)
//! - 2: Virtual UART
//! - 3: VCHIQ
//! - 4: LEDs
//! - 5: Buttons
//! - 6: Touch screen
//! - 7: Property interface (ARM -> VC)
//! - 8: Property interface (VC -> ARM)
//! - 9: Property interface (ARM -> VC, no response)
//!
//! We use channel 8 (Property interface) to communicate with the GPU
//! for framebuffer allocation and other configuration.

use core::ptr::{read_volatile, write_volatile};
use core::sync::atomic::{fence, Ordering};
use spin::Mutex;
use lazy_static::lazy_static;

use crate::println;

// Mailbox base addresses
/// Mailbox base for Raspberry Pi 3 (BCM2837)
pub const MAILBOX_BASE_PI3: usize = 0x3F00B880;
/// Mailbox base for Raspberry Pi 4 (BCM2711)
pub const MAILBOX_BASE_PI4: usize = 0xFE00B880;

// Mailbox register offsets (from base)
const REG_READ: usize = 0x00;      // Read from mailbox
const REG_STATUS: usize = 0x18;    // Status register
const REG_WRITE: usize = 0x20;     // Write to mailbox

// Status register flags
const STATUS_FULL: u32 = 0x80000000;  // Mailbox full (can't write)
const STATUS_EMPTY: u32 = 0x40000000; // Mailbox empty (can't read)

// Channel numbers
const CHANNEL_POWER: u8 = 0;
const CHANNEL_FB: u8 = 1;
const CHANNEL_VUART: u8 = 2;
const CHANNEL_VCHIQ: u8 = 3;
const CHANNEL_LEDS: u8 = 4;
const CHANNEL_BUTTONS: u8 = 5;
const CHANNEL_TOUCH: u8 = 6;
const CHANNEL_PROPERTY_ARM_VC: u8 = 8;    // ARM -> VC property interface
const CHANNEL_PROPERTY_VC_ARM: u8 = 9;    // VC -> ARM property interface
const CHANNEL_PROPERTY_ARM_VC_NO_RESP: u8 = 10; // ARM -> VC, no response

/// Property tags (mailbox messages)
pub mod tags {
    // VideoCore info
    pub const GET_FIRMWARE_REVISION: u32 = 0x00000001;
    
    // Hardware info
    pub const GET_BOARD_MODEL: u32 = 0x00010001;
    pub const GET_BOARD_REVISION: u32 = 0x00010002;
    pub const GET_MAC_ADDRESS: u32 = 0x00010003;
    pub const GET_BOARD_SERIAL: u32 = 0x00010004;
    pub const GET_ARM_MEMORY: u32 = 0x00010005;
    pub const GET_VC_MEMORY: u32 = 0x00010006;
    pub const GET_CLOCKS: u32 = 0x00010007;
    
    // Power management
    pub const GET_POWER_STATE: u32 = 0x00020001;
    pub const GET_TIMING: u32 = 0x00020002;
    pub const SET_POWER_STATE: u32 = 0x00028001;
    
    // Clock management
    pub const GET_CLOCK_STATE: u32 = 0x00030001;
    pub const SET_CLOCK_STATE: u32 = 0x00038001;
    pub const GET_CLOCK_RATE: u32 = 0x00030002;
    pub const SET_CLOCK_RATE: u32 = 0x00038002;
    pub const GET_MAX_CLOCK_RATE: u32 = 0x00030004;
    pub const GET_MIN_CLOCK_RATE: u32 = 0x00030007;
    pub const GET_TURBO: u32 = 0x00030009;
    pub const SET_TURBO: u32 = 0x00038009;
    
    // Voltage management
    pub const GET_VOLTAGE: u32 = 0x00030003;
    pub const SET_VOLTAGE: u32 = 0x00038003;
    pub const GET_MAX_VOLTAGE: u32 = 0x00030005;
    pub const GET_MIN_VOLTAGE: u32 = 0x00030008;
    pub const GET_TEMPERATURE: u32 = 0x00030006;
    pub const GET_MAX_TEMPERATURE: u32 = 0x0003000A;
    
    // Framebuffer (deprecated, use Allocate Buffer)
    pub const ALLOCATE_FRAMEBUFFER: u32 = 0x00040001;
    pub const RELEASE_FRAMEBUFFER: u32 = 0x00048001;
    pub const BLANK_SCREEN: u32 = 0x00040002;
    pub const GET_PHYSICAL_SIZE: u32 = 0x00040003;
    pub const TEST_PHYSICAL_SIZE: u32 = 0x00044003;
    pub const SET_PHYSICAL_SIZE: u32 = 0x00048003;
    pub const GET_VIRTUAL_SIZE: u32 = 0x00040004;
    pub const TEST_VIRTUAL_SIZE: u32 = 0x00044004;
    pub const SET_VIRTUAL_SIZE: u32 = 0x00048004;
    pub const GET_DEPTH: u32 = 0x00040005;
    pub const TEST_DEPTH: u32 = 0x00044005;
    pub const SET_DEPTH: u32 = 0x00048005;
    pub const GET_PIXEL_ORDER: u32 = 0x00040006;
    pub const TEST_PIXEL_ORDER: u32 = 0x00044006;
    pub const SET_PIXEL_ORDER: u32 = 0x00048006;
    pub const GET_ALPHA_MODE: u32 = 0x00040007;
    pub const TEST_ALPHA_MODE: u32 = 0x00044007;
    pub const SET_ALPHA_MODE: u32 = 0x00048007;
    pub const GET_PITCH: u32 = 0x00040008;
    pub const GET_VIRTUAL_OFFSET: u32 = 0x00040009;
    pub const TEST_VIRTUAL_OFFSET: u32 = 0x00044009;
    pub const SET_VIRTUAL_OFFSET: u32 = 0x00048009;
    pub const GET_OVERSCAN: u32 = 0x0004000A;
    pub const TEST_OVERSCAN: u32 = 0x0004400A;
    pub const SET_OVERSCAN: u32 = 0x0004800A;
    pub const GET_PALETTE: u32 = 0x0004000B;
    pub const TEST_PALETTE: u32 = 0x0004400B;
    pub const SET_PALETTE: u32 = 0x0004800B;
    pub const SET_CURSOR_INFO: u32 = 0x00008010;
    pub const SET_CURSOR_STATE: u32 = 0x00008011;
}

/// Response codes
pub const CODE_REQUEST: u32 = 0x00000000;
pub const CODE_RESPONSE_SUCCESS: u32 = 0x80000000;
pub const CODE_RESPONSE_FAILURE: u32 = 0x80000001;

/// Mailbox driver state
pub struct Mailbox {
    base_addr: usize,
    initialized: bool,
}

impl Mailbox {
    /// Create a new mailbox instance with the given base address
    const fn new(base_addr: usize) -> Self {
        Self {
            base_addr,
            initialized: false,
        }
    }
    
    /// Check if mailbox is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
    
    /// Initialize the mailbox driver
    pub fn init(&mut self) {
        if self.initialized {
            return;
        }
        
        println!("[mailbox] Initializing VideoCore mailbox...");
        println!("[mailbox] Base address: 0x{:08X}", self.base_addr);
        
        self.initialized = true;
        println!("[mailbox] Mailbox driver initialized");
    }
    
    /// Check if mailbox is full (can't write)
    fn is_full(&self) -> bool {
        unsafe {
            let status = read_volatile((self.base_addr + REG_STATUS) as *const u32);
            (status & STATUS_FULL) != 0
        }
    }
    
    /// Check if mailbox is empty (can't read)
    fn is_empty(&self) -> bool {
        unsafe {
            let status = read_volatile((self.base_addr + REG_STATUS) as *const u32);
            (status & STATUS_EMPTY) != 0
        }
    }
    
    /// Write a message to a mailbox channel
    /// 
    /// # Safety
    /// The buffer must be 16-byte aligned and properly formatted as a mailbox message.
    fn write(&self, channel: u8, data: u32) {
        // Wait until mailbox is not full
        while self.is_full() {
            core::hint::spin_loop();
        }
        
        // Memory fence to ensure proper ordering
        fence(Ordering::SeqCst);
        
        // Write data with channel in lower 4 bits
        let value = (data & !0xF) | (channel as u32 & 0xF);
        unsafe {
            write_volatile((self.base_addr + REG_WRITE) as *mut u32, value);
        }
        
        fence(Ordering::SeqCst);
    }
    
    /// Read a message from a mailbox channel
    /// 
    /// Returns the data word (channel in lower 4 bits)
    fn read(&self, channel: u8) -> u32 {
        loop {
            // Wait until mailbox is not empty
            while self.is_empty() {
                core::hint::spin_loop();
            }
            
            fence(Ordering::SeqCst);
            
            unsafe {
                let data = read_volatile((self.base_addr + REG_READ) as *const u32);
                
                // Check if this is from the channel we expect
                if (data & 0xF) == channel as u32 {
                    return data & !0xF;
                }
            }
            
            // Not our channel, keep waiting
        }
    }
    
    /// Send a property message and wait for response
    /// 
    /// # Safety
    /// The buffer must be 16-byte aligned and properly formatted as a property message.
    /// The buffer will be modified in-place by the GPU.
    pub unsafe fn send_property_message(&self, buffer: *mut u32) -> bool {
        if !self.initialized {
            return false;
        }
        
        // Get the bus address (convert ARM physical address to bus address)
        // On Pi, bus address = physical address | 0xC0000000
        let bus_addr = ((buffer as usize) | 0xC0000000) as u32;
        
        // Write to property channel
        self.write(CHANNEL_PROPERTY_ARM_VC, bus_addr);
        
        // Read response
        let response = self.read(CHANNEL_PROPERTY_ARM_VC);
        
        // Verify we got the right buffer back
        if response != bus_addr {
            println!("[mailbox] Error: Unexpected response address 0x{:08X}", response);
            return false;
        }
        
        // Check response code
        let response_code = read_volatile(buffer.add(1));
        if response_code != CODE_RESPONSE_SUCCESS {
            println!("[mailbox] Error: Response code 0x{:08X}", response_code);
            return false;
        }
        
        true
    }
    
    /// Send a property message using a reference to a message buffer
    /// 
    /// This is a safe wrapper around send_property_message
    pub fn call<T: MailboxMessage>(&self, message: &mut T) -> bool {
        unsafe {
            self.send_property_message(message.as_mut_ptr())
        }
    }
    
    /// Get the board model
    pub fn get_board_model(&self) -> Option<u32> {
        let mut msg = PropertyMessage::new();
        msg.add_tag(tags::GET_BOARD_MODEL, &[], 1);
        
        if self.call(&mut msg) {
            Some(msg.get_response_u32(0))
        } else {
            None
        }
    }
    
    /// Get the board revision
    pub fn get_board_revision(&self) -> Option<u32> {
        let mut msg = PropertyMessage::new();
        msg.add_tag(tags::GET_BOARD_REVISION, &[], 1);
        
        if self.call(&mut msg) {
            Some(msg.get_response_u32(0))
        } else {
            None
        }
    }
    
    /// Get ARM memory (base and size)
    pub fn get_arm_memory(&self) -> Option<(u32, u32)> {
        let mut msg = PropertyMessage::new();
        msg.add_tag(tags::GET_ARM_MEMORY, &[], 2);
        
        if self.call(&mut msg) {
            Some((msg.get_response_u32(0), msg.get_response_u32(1)))
        } else {
            None
        }
    }
    
    /// Get the current temperature in millidegrees Celsius
    pub fn get_temperature(&self) -> Option<u32> {
        let mut msg = PropertyMessage::new();
        // Temperature ID 0 is the SoC temperature
        msg.add_tag(tags::GET_TEMPERATURE, &[0], 2);
        
        if self.call(&mut msg) {
            Some(msg.get_response_u32(1))
        } else {
            None
        }
    }
}

/// Trait for mailbox message types
trait MailboxMessage {
    fn as_mut_ptr(&mut self) -> *mut u32;
}

/// Property message buffer for mailbox communication
/// 
/// This struct represents a mailbox property message buffer.
/// The const parameter N is the maximum number of u32 values in the response.
/// Maximum size for property message buffer (in u32 words)
const MAX_PROP_BUFFER_SIZE: usize = 128; // 512 bytes max

/// Property message buffer for mailbox communication
/// 
/// This struct represents a mailbox property message buffer.
/// Uses a fixed-size buffer large enough for most property messages.
#[repr(C, align(16))]
pub struct PropertyMessage {
    /// Buffer size in bytes
    size: u32,
    /// Request/response code
    code: u32,
    /// Tag buffer (fixed size)
    tags: [u32; MAX_PROP_BUFFER_SIZE],
    /// Current write position in tags
    tag_pos: usize,
    /// Number of response values expected
    response_count: usize,
}

impl PropertyMessage {
    /// Create a new property message
    pub fn new() -> Self {
        Self {
            size: 0, // Will be calculated when finalized
            code: CODE_REQUEST,
            tags: [0; MAX_PROP_BUFFER_SIZE],
            tag_pos: 0,
            response_count: 0,
        }
    }
    
    /// Add a tag to the message
    /// 
    /// # Arguments
    /// * `tag_id` - The tag identifier
    /// * `request_values` - Values to send in the request
    /// * `response_buffer_size` - Number of u32 values expected in response
    pub fn add_tag(&mut self, tag_id: u32, request_values: &[u32], response_buffer_size: usize) {
        let idx = self.tag_pos;
        
        // Tag identifier
        self.tags[idx] = tag_id;
        // Buffer size (in bytes)
        self.tags[idx + 1] = (request_values.len().max(response_buffer_size) * 4) as u32;
        // Request/response code (0 = request)
        self.tags[idx + 2] = 0;
        
        // Copy request values
        for (i, &val) in request_values.iter().enumerate() {
            self.tags[idx + 3 + i] = val;
        }
        
        self.tag_pos += 3 + request_values.len().max(response_buffer_size);
        self.response_count += response_buffer_size;
    }
    
    /// Finalize the message and update size
    fn finalize(&mut self) {
        // Add end tag (0)
        self.tags[self.tag_pos] = 0;
        self.tag_pos += 1;
        
        // Calculate total size
        self.size = (12 + self.tag_pos * 4) as u32; // Header (8 bytes) + tags + end tag
    }
    
    /// Get a response value at the given index
    pub fn get_response_u32(&self, index: usize) -> u32 {
        // Skip header tags to find response values
        // Each tag has: id (4), size (4), code (4), then data
        let mut pos = 0;
        let mut current_idx = 0;
        
        while self.tags[pos] != 0 && current_idx <= index {
            let tag_size = self.tags[pos + 1] as usize / 4;
            let data_start = pos + 3;
            
            if current_idx + tag_size > index {
                return self.tags[data_start + (index - current_idx)];
            }
            
            current_idx += tag_size;
            pos += 3 + tag_size;
        }
        
        0
    }
}

impl MailboxMessage for PropertyMessage {
    fn as_mut_ptr(&mut self) -> *mut u32 {
        self.finalize();
        self as *mut _ as *mut u32
    }
}

/// Default mailbox instance (Pi 3 base address by default)
lazy_static! {
    static ref MAILBOX: Mutex<Mailbox> = Mutex::new(Mailbox::new(MAILBOX_BASE_PI3));
}

/// Initialize the mailbox driver
pub fn init() {
    MAILBOX.lock().init();
}

/// Get the global mailbox instance
pub fn mailbox() -> &'static Mutex<Mailbox> {
    &MAILBOX
}

/// Set the mailbox base address (call before init for Pi 4)
pub fn set_base_address(base: usize) {
    let mut mb = MAILBOX.lock();
    if !mb.initialized {
        mb.base_addr = base;
    }
}

/// Print mailbox info
pub fn print_info() {
    let mb = MAILBOX.lock();
    if mb.initialized {
        println!("Mailbox Information:");
        println!("  Base address: 0x{:08X}", mb.base_addr);
        
        // Try to get board info
        if let Some(model) = mb.get_board_model() {
            println!("  Board model: 0x{:08X}", model);
        }
        if let Some(revision) = mb.get_board_revision() {
            println!("  Board revision: 0x{:08X}", revision);
        }
        if let Some((base, size)) = mb.get_arm_memory() {
            println!("  ARM memory: {} MB at 0x{:08X}", size / (1024 * 1024), base);
        }
    } else {
        println!("Mailbox driver not initialized");
    }
}
