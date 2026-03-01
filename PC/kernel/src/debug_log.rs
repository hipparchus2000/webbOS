//! Debug logging system for kernel boot
//!
//! Stores debug messages in a circular buffer that can be
//! retrieved by the bootloader if the kernel crashes.

use alloc::vec::Vec;
use core::sync::atomic::{AtomicUsize, Ordering};
use spin::Mutex;

/// Maximum number of debug messages
const MAX_MESSAGES: usize = 64;

/// Maximum length of each message
const MSG_LEN: usize = 128;

/// Debug message buffer - stored in BSS, preserved on panic
static DEBUG_BUFFER: Mutex<DebugBuffer> = Mutex::new(DebugBuffer::new());

/// Message counter for ordering
static MESSAGE_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Debug buffer structure
pub struct DebugBuffer {
    messages: [[u8; MSG_LEN]; MAX_MESSAGES],
    indices: [usize; MAX_MESSAGES], // Message order index
    count: usize,
    next_index: usize,
}

impl DebugBuffer {
    const fn new() -> Self {
        Self {
            messages: [[0u8; MSG_LEN]; MAX_MESSAGES],
            indices: [0; MAX_MESSAGES],
            count: 0,
            next_index: 0,
        }
    }

    /// Add a message to the buffer
    fn add_message(&mut self, msg: &str) {
        let idx = self.next_index % MAX_MESSAGES;
        
        // Copy message bytes
        let bytes = msg.as_bytes();
        let len = bytes.len().min(MSG_LEN - 1);
        self.messages[idx][..len].copy_from_slice(&bytes[..len]);
        self.messages[idx][len] = 0; // Null terminate
        
        // Store order index
        self.indices[idx] = self.next_index;
        
        self.next_index += 1;
        if self.count < MAX_MESSAGES {
            self.count += 1;
        }
    }

    /// Get all messages in order
    pub fn get_messages(&self) -> Vec<&str> {
        // Create ordered list of indices
        let mut ordered: [(usize, usize); MAX_MESSAGES] = [(0, 0); MAX_MESSAGES];
        for i in 0..self.count.min(MAX_MESSAGES) {
            ordered[i] = (self.indices[i], i);
        }
        
        // Sort by index
        ordered.sort_by_key(|&(idx, _)| idx);
        
        // Return messages in order
        ordered.iter()
            .take(self.count)
            .filter_map(|&(_, buf_idx)| {
                let msg = &self.messages[buf_idx];
                // Find null terminator
                let len = msg.iter().position(|&b| b == 0).unwrap_or(MSG_LEN);
                core::str::from_utf8(&msg[..len]).ok()
            })
            .collect()
    }
}

/// Log a debug message
pub fn log(msg: &str) {
    let mut buffer = DEBUG_BUFFER.lock();
    buffer.add_message(msg);
    
    // Also increment global counter
    MESSAGE_COUNT.fetch_add(1, Ordering::Relaxed);
}

/// Log a formatted debug message (simple version without alloc)
/// For formatted messages, format first then call log()
#[macro_export]
macro_rules! debug_log {
    ($msg:expr) => {
        $crate::debug_log::log($msg)
    };
}

/// Get the debug buffer for returning to bootloader
pub fn get_buffer() -> &'static Mutex<DebugBuffer> {
    &DEBUG_BUFFER
}

/// Get message count
pub fn message_count() -> usize {
    MESSAGE_COUNT.load(Ordering::Relaxed)
}

/// Clear the debug buffer
pub fn clear() {
    let mut buffer = DEBUG_BUFFER.lock();
    buffer.count = 0;
    buffer.next_index = 0;
    MESSAGE_COUNT.store(0, Ordering::Relaxed);
}
