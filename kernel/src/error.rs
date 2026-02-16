//! Error types for the kernel

use alloc::string::{String, ToString};

/// VFAT filesystem error type
#[derive(Debug, Clone, PartialEq)]
pub enum VFatError {
    /// Invalid parameter passed to function
    InvalidParameter(String),
    /// I/O error
    Io(IoError),
    /// Filesystem corruption detected
    Corruption(String),
    /// Not found error
    NotFound(String),
    /// Out of memory
    OutOfMemory,
    /// Unsupported feature
    Unsupported(String),
}

/// I/O error type for no-std environment
#[derive(Debug, Clone, PartialEq)]
pub enum IoError {
    /// Device error
    DeviceError(String),
    /// Timeout
    Timeout,
    /// Invalid data
    InvalidData(String),
    /// Would block (non-blocking operation)
    WouldBlock,
    /// Other I/O error
    Other(String),
}

impl VFatError {
    /// Create a new InvalidParameter error
    pub fn invalid_param(msg: &str) -> Self {
        VFatError::InvalidParameter(msg.to_string())
    }
    
    /// Create a new Corruption error
    pub fn corruption(msg: &str) -> Self {
        VFatError::Corruption(msg.to_string())
    }
    
    /// Create a new NotFound error
    pub fn not_found(msg: &str) -> Self {
        VFatError::NotFound(msg.to_string())
    }
    
    /// Create a new Io error
    pub fn io(kind: IoError) -> Self {
        VFatError::Io(kind)
    }
    
    /// Create a new Unsupported error
    pub fn unsupported(msg: &str) -> Self {
        VFatError::Unsupported(msg.to_string())
    }
}

impl IoError {
    /// Create a new DeviceError
    pub fn device_error(msg: &str) -> Self {
        IoError::DeviceError(msg.to_string())
    }
    
    /// Create a new InvalidData error
    pub fn invalid_data(msg: &str) -> Self {
        IoError::InvalidData(msg.to_string())
    }
    
    /// Create a new Other error
    pub fn other(msg: &str) -> Self {
        IoError::Other(msg.to_string())
    }
    
    /// Create a new Timeout error
    pub fn timeout() -> Self {
        IoError::Timeout
    }
    
    /// Create a new NotFound error (maps to DeviceError for now)
    pub fn not_found(msg: &str) -> Self {
        IoError::DeviceError(msg.to_string())
    }
    
    /// Create a new InvalidInput error
    pub fn invalid_input(msg: &str) -> Self {
        IoError::InvalidData(msg.to_string())
    }
    
    /// Create a new AlreadyExists error
    pub fn already_exists(msg: &str) -> Self {
        IoError::DeviceError(msg.to_string())
    }
    
    /// Create a new PermissionDenied error
    pub fn permission_denied(msg: &str) -> Self {
        IoError::DeviceError(msg.to_string())
    }
}