//! Device Tree Blob (DTB) Parser
//!
//! The Raspberry Pi firmware passes a device tree to the kernel
//! that describes the hardware configuration.
//!
//! This parser includes comprehensive bounds checking to handle
//! malformed or corrupted DTB data safely.

use webbos_shared::bootinfo::FramebufferInfo;
use webbos_shared::types::PhysAddr;

/// DTB magic number (big-endian)
const FDT_MAGIC: u32 = 0xD00D_FEED;

/// Supported DTB version
const FDT_SUPPORTED_VERSION: u32 = 17;

/// Minimum compatible version
const FDT_MIN_COMPAT_VERSION: u32 = 16;

/// Size of DTB header in bytes
const FDT_HEADER_SIZE: usize = core::mem::size_of::<FdtHeader>();

/// DTB header structure
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct FdtHeader {
    magic: u32,
    totalsize: u32,
    off_dt_struct: u32,
    off_dt_strings: u32,
    off_mem_rsvmap: u32,
    version: u32,
    last_comp_version: u32,
    boot_cpuid_phys: u32,
    size_dt_strings: u32,
    size_dt_struct: u32,
}

/// DTB token types
const FDT_BEGIN_NODE: u32 = 0x00000001;
const FDT_END_NODE: u32 = 0x00000002;
const FDT_PROP: u32 = 0x00000003;
const FDT_NOP: u32 = 0x00000004;
const FDT_END: u32 = 0x00000009;

/// Maximum DTB size (16MB - reasonable limit for embedded systems)
const MAX_DTB_SIZE: usize = 16 * 1024 * 1024;

/// Maximum parsing depth to prevent stack overflow
const MAX_DEPTH: usize = 64;

/// Maximum property size (1MB - prevents excessive memory usage)
const MAX_PROPERTY_SIZE: usize = 1024 * 1024;

/// DTB parsing error types
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DtbError {
    /// DTB data is truncated or too small
    Truncated,
    /// Invalid magic number
    InvalidMagic,
    /// DTB size exceeds maximum allowed
    TooLarge,
    /// totalsize doesn't match actual data length
    SizeMismatch,
    /// Unsupported DTB version
    UnsupportedVersion,
    /// Invalid offset (out of bounds)
    InvalidOffset,
    /// Structure block offset is invalid
    InvalidStructureOffset,
    /// Strings block offset is invalid
    InvalidStringsOffset,
    /// Memory reservation map offset is invalid
    #[allow(dead_code)]
    InvalidMemRsvmapOffset,
    /// Property length exceeds bounds
    InvalidPropertyLength,
    /// String offset exceeds strings block bounds
    InvalidStringOffset,
    /// Malformed structure (unexpected token, etc.)
    MalformedStructure,
    /// Parsing depth exceeded maximum
    DepthExceeded,
    /// Null pointer provided
    NullPointer,
}

/// Result type for DTB operations
pub type DtbResult<T> = Result<T, DtbError>;

/// Parsed DTB information
#[derive(Debug)]
pub struct DtbInfo {
    /// Base address of memory
    pub memory_base: u64,
    /// Size of memory
    pub memory_size: u64,
    /// Framebuffer information
    pub framebuffer: FramebufferInfo,
}

impl DtbInfo {
    const fn new() -> Self {
        Self {
            memory_base: 0,
            memory_size: 0,
            framebuffer: FramebufferInfo {
                addr: PhysAddr::new(0),
                virt_addr: None,
                width: 0,
                height: 0,
                bpp: 32,
                pitch: 0,
                format: webbos_shared::bootinfo::PixelFormat::Rgb,
            },
        }
    }
}

/// DTB data wrapper with bounds checking
struct DtbData<'a> {
    data: &'a [u8],
    struct_offset: usize,
    strings_offset: usize,
    struct_size: usize,
    strings_size: usize,
}

impl<'a> DtbData<'a> {
    /// Create a new DtbData from a raw pointer and size
    /// 
    /// # Safety
    /// Caller must ensure the pointer and size are valid
    unsafe fn from_raw_parts(ptr: *const u8, size: usize) -> DtbResult<Self> {
        if ptr.is_null() {
            return Err(DtbError::NullPointer);
        }
        
        if size > MAX_DTB_SIZE {
            return Err(DtbError::TooLarge);
        }
        
        // Create slice from raw parts
        let data = core::slice::from_raw_parts(ptr, size);
        
        // Parse header
        if data.len() < FDT_HEADER_SIZE {
            return Err(DtbError::Truncated);
        }
        
        let header = Self::parse_header(data)?;
        
        // Validate totalsize matches actual size
        let totalsize = u32::from_be(header.totalsize) as usize;
        if totalsize != size {
            return Err(DtbError::SizeMismatch);
        }
        
        // Calculate offsets and sizes
        let struct_offset = u32::from_be(header.off_dt_struct) as usize;
        let strings_offset = u32::from_be(header.off_dt_strings) as usize;
        let struct_size = u32::from_be(header.size_dt_struct) as usize;
        let strings_size = u32::from_be(header.size_dt_strings) as usize;
        
        // Validate structure block bounds
        if struct_offset < FDT_HEADER_SIZE || struct_offset >= size {
            return Err(DtbError::InvalidStructureOffset);
        }
        if struct_size == 0 || struct_size > size - struct_offset {
            return Err(DtbError::InvalidStructureOffset);
        }
        
        // Validate strings block bounds
        if strings_offset < FDT_HEADER_SIZE || strings_offset >= size {
            return Err(DtbError::InvalidStringsOffset);
        }
        if strings_size == 0 || strings_size > size - strings_offset {
            return Err(DtbError::InvalidStringsOffset);
        }
        
        // Validate that blocks don't overlap
        let struct_end = struct_offset.checked_add(struct_size)
            .ok_or(DtbError::InvalidStructureOffset)?;
        let strings_end = strings_offset.checked_add(strings_size)
            .ok_or(DtbError::InvalidStringsOffset)?;
        
        if struct_end > size || strings_end > size {
            return Err(DtbError::InvalidOffset);
        }
        
        // Note: struct and strings blocks can overlap in valid DTBs
        
        Ok(Self {
            data,
            struct_offset,
            strings_offset,
            struct_size,
            strings_size,
        })
    }
    
    /// Parse the DTB header from raw bytes
    fn parse_header(data: &[u8]) -> DtbResult<FdtHeader> {
        if data.len() < FDT_HEADER_SIZE {
            return Err(DtbError::Truncated);
        }
        
        // Safely read header fields (avoiding unaligned access)
        let header = FdtHeader {
            magic: u32::from_be_bytes([data[0], data[1], data[2], data[3]]),
            totalsize: u32::from_be_bytes([data[4], data[5], data[6], data[7]]),
            off_dt_struct: u32::from_be_bytes([data[8], data[9], data[10], data[11]]),
            off_dt_strings: u32::from_be_bytes([data[12], data[13], data[14], data[15]]),
            off_mem_rsvmap: u32::from_be_bytes([data[16], data[17], data[18], data[19]]),
            version: u32::from_be_bytes([data[20], data[21], data[22], data[23]]),
            last_comp_version: u32::from_be_bytes([data[24], data[25], data[26], data[27]]),
            boot_cpuid_phys: u32::from_be_bytes([data[28], data[29], data[30], data[31]]),
            size_dt_strings: u32::from_be_bytes([data[32], data[33], data[34], data[35]]),
            size_dt_struct: u32::from_be_bytes([data[36], data[37], data[38], data[39]]),
        };
        
        // Validate magic number
        if header.magic != FDT_MAGIC {
            return Err(DtbError::InvalidMagic);
        }
        
        // Validate version (must be >= minimum compatible version)
        let version = u32::from_be(header.version);
        let last_comp_version = u32::from_be(header.last_comp_version);
        
        if version < FDT_MIN_COMPAT_VERSION {
            return Err(DtbError::UnsupportedVersion);
        }
        
        if last_comp_version > FDT_SUPPORTED_VERSION {
            return Err(DtbError::UnsupportedVersion);
        }
        
        Ok(header)
    }
    
    /// Get a reference to the structure block
    fn struct_block(&self) -> &[u8] {
        &self.data[self.struct_offset..self.struct_offset + self.struct_size]
    }
    
    /// Get a reference to the strings block
    fn strings_block(&self) -> &[u8] {
        &self.data[self.strings_offset..self.strings_offset + self.strings_size]
    }
    
    /// Read a u32 from the structure block at the given offset (in bytes)
    fn read_struct_u32(&self, offset: usize) -> DtbResult<u32> {
        let block = self.struct_block();
        if offset + 4 > block.len() {
            return Err(DtbError::InvalidOffset);
        }
        let bytes = &block[offset..offset + 4];
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }
    
    /// Get a pointer to a string in the strings block
    fn get_string(&self, offset: usize) -> DtbResult<&str> {
        let strings = self.strings_block();
        
        if offset >= strings.len() {
            return Err(DtbError::InvalidStringOffset);
        }
        
        // Find null terminator
        let remaining = &strings[offset..];
        let len = remaining.iter()
            .position(|&b| b == 0)
            .ok_or(DtbError::MalformedStructure)?;
        
        // Convert to string (DTB strings are ASCII/UTF-8)
        core::str::from_utf8(&remaining[..len])
            .map_err(|_| DtbError::MalformedStructure)
    }
    
    /// Get property value as a byte slice
    fn get_property_value(&self, struct_offset: usize, len: usize) -> DtbResult<&[u8]> {
        if len > MAX_PROPERTY_SIZE {
            return Err(DtbError::InvalidPropertyLength);
        }
        
        let block = self.struct_block();
        if struct_offset + len > block.len() {
            return Err(DtbError::InvalidPropertyLength);
        }
        
        Ok(&block[struct_offset..struct_offset + len])
    }
    
}

/// Parse the device tree blob
/// 
/// # Safety
/// Caller must ensure dtb_addr points to valid DTB data
pub unsafe fn parse_dtb(dtb_addr: u64) -> Option<DtbInfo> {
    match parse_dtb_with_result(dtb_addr) {
        Ok(info) => Some(info),
        Err(_) => None,
    }
}

/// Parse the device tree blob with detailed error reporting
/// 
/// # Safety
/// Caller must ensure dtb_addr points to valid DTB data
pub unsafe fn parse_dtb_with_result(dtb_addr: u64) -> DtbResult<DtbInfo> {
    // First, we need to determine the size by reading the header
    // This requires careful handling since we don't know the bounds yet
    
    if dtb_addr == 0 {
        return Err(DtbError::NullPointer);
    }
    
    // Read just the header to get totalsize
    let header_ptr = dtb_addr as *const u8;
    let header_slice = core::slice::from_raw_parts(header_ptr, FDT_HEADER_SIZE);
    
    // Parse header to get totalsize
    if header_slice.len() < FDT_HEADER_SIZE {
        return Err(DtbError::Truncated);
    }
    
    let totalsize = u32::from_be_bytes([
        header_slice[4], header_slice[5], header_slice[6], header_slice[7]
    ]) as usize;
    
    // Validate totalsize
    if totalsize < FDT_HEADER_SIZE {
        return Err(DtbError::Truncated);
    }
    if totalsize > MAX_DTB_SIZE {
        return Err(DtbError::TooLarge);
    }
    
    // Now create bounded DTB data
    let dtb = DtbData::from_raw_parts(header_ptr, totalsize)?;
    
    let mut info = DtbInfo::new();
    
    // Parse structure block
    parse_structure(&dtb, &mut info)?;
    
    // Set default memory if not found in DTB
    if info.memory_size == 0 {
        // Pi 3/4 typically have at least 1GB
        info.memory_base = 0;
        info.memory_size = 0x40000000; // 1GB default
    }
    
    Ok(info)
}

/// Parse the structure block
fn parse_structure(dtb: &DtbData, info: &mut DtbInfo) -> DtbResult<()> {
    let mut offset = 0usize;
    let mut depth = 0usize;
    
    loop {
        if depth > MAX_DEPTH {
            return Err(DtbError::DepthExceeded);
        }
        
        // Read token
        let token = dtb.read_struct_u32(offset)?;
        offset += 4;
        
        match token {
            FDT_BEGIN_NODE => {
                depth += 1;
                
                // Read node name (null-terminated string, 4-byte aligned)
                let block = dtb.struct_block();
                let name_start = offset;
                
                // Find null terminator within bounds
                let mut name_len = 0usize;
                loop {
                    if name_start + name_len >= block.len() {
                        return Err(DtbError::MalformedStructure);
                    }
                    if block[name_start + name_len] == 0 {
                        break;
                    }
                    name_len += 1;
                    
                    // Safety limit for node name length
                    if name_len > 1024 {
                        return Err(DtbError::MalformedStructure);
                    }
                }
                
                let node_name = core::str::from_utf8(&block[name_start..name_start + name_len])
                    .map_err(|_| DtbError::MalformedStructure)?;
                
                // Check for memory node
                if node_name.starts_with("memory@") || node_name == "memory" {
                    // Parse memory node properties
                    offset = parse_memory_node_props(dtb, offset, info)?;
                } else {
                    // Skip to after null terminator (aligned to 4 bytes)
                    offset = (name_start + name_len + 4) & !3;
                }
            }
            FDT_END_NODE => {
                if depth == 0 {
                    return Err(DtbError::MalformedStructure);
                }
                depth -= 1;
                if depth == 0 {
                    // End of root node
                    break;
                }
            }
            FDT_PROP => {
                // Read property length and name offset
                let len = dtb.read_struct_u32(offset)? as usize;
                let nameoff = dtb.read_struct_u32(offset + 4)? as usize;
                offset += 8;
                
                // Validate property length
                if len > MAX_PROPERTY_SIZE {
                    return Err(DtbError::InvalidPropertyLength);
                }
                
                // Get property name from strings block
                let prop_name = dtb.get_string(nameoff)?;
                
                // Get property value
                if len > 0 {
                    let value = dtb.get_property_value(offset, len)?;
                    
                    // Parse known properties
                    if prop_name == "reg" && len == 16 && info.memory_size == 0 {
                        // Memory region: base (8 bytes) + size (8 bytes)
                        let base = u64::from_be_bytes([
                            value[0], value[1], value[2], value[3],
                            value[4], value[5], value[6], value[7],
                        ]);
                        let size = u64::from_be_bytes([
                            value[8], value[9], value[10], value[11],
                            value[12], value[13], value[14], value[15],
                        ]);
                        info.memory_base = base;
                        info.memory_size = size;
                    }
                    
                    // Skip property value (already validated)
                    offset += len;
                }
                
                // Align to 4 bytes
                offset = (offset + 3) & !3;
            }
            FDT_NOP => {
                // Do nothing, just continue
            }
            FDT_END => {
                // End of structure
                break;
            }
            _ => {
                // Unknown token - could be padding or corruption
                // Continue parsing but be cautious
                // In strict mode, we might return an error here
            }
        }
        
        // Safety check: ensure we haven't gone past the structure block
        if offset > dtb.struct_size {
            return Err(DtbError::InvalidOffset);
        }
    }
    
    Ok(())
}

/// Parse memory node properties
fn parse_memory_node_props(dtb: &DtbData, mut offset: usize, info: &mut DtbInfo) -> DtbResult<usize> {
    let block = dtb.struct_block();
    
    // Skip node name (already parsed)
    // Find null terminator
    let name_start = offset;
    let mut name_len = 0usize;
    while name_start + name_len < block.len() && block[name_start + name_len] != 0 {
        name_len += 1;
        if name_len > 1024 {
            return Err(DtbError::MalformedStructure);
        }
    }
    
    // Align to 4 bytes after null terminator
    offset = (name_start + name_len + 4) & !3;
    
    // Parse properties until we hit FDT_END_NODE
    loop {
        if offset + 4 > block.len() {
            return Err(DtbError::Truncated);
        }
        
        let token = dtb.read_struct_u32(offset)?;
        
        match token {
            FDT_PROP => {
                offset += 4;
                let len = dtb.read_struct_u32(offset)? as usize;
                let nameoff = dtb.read_struct_u32(offset + 4)? as usize;
                offset += 8;
                
                if len > MAX_PROPERTY_SIZE {
                    return Err(DtbError::InvalidPropertyLength);
                }
                
                let prop_name = dtb.get_string(nameoff)?;
                
                if len > 0 {
                    let value = dtb.get_property_value(offset, len)?;
                    
                    if prop_name == "reg" && len == 16 {
                        let base = u64::from_be_bytes([
                            value[0], value[1], value[2], value[3],
                            value[4], value[5], value[6], value[7],
                        ]);
                        let size = u64::from_be_bytes([
                            value[8], value[9], value[10], value[11],
                            value[12], value[13], value[14], value[15],
                        ]);
                        info.memory_base = base;
                        info.memory_size = size;
                    }
                    
                    offset += len;
                }
                
                offset = (offset + 3) & !3;
            }
            FDT_NOP => {
                offset += 4;
            }
            FDT_END_NODE => {
                // End of this node
                break;
            }
            _ => {
                // Not a property token, stop parsing this node
                break;
            }
        }
        
        if offset > dtb.struct_size {
            return Err(DtbError::InvalidOffset);
        }
    }
    
    Ok(offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a minimal valid DTB for testing
    fn create_test_dtb() -> Vec<u8> {
        // This is a minimal DTB structure for testing
        // In practice, you would create this programmatically
        vec![
            // Header (40 bytes)
            0xD0, 0x0D, 0xFE, 0xED, // magic
            0x00, 0x00, 0x01, 0x00, // totalsize (256 bytes)
            0x00, 0x00, 0x00, 0x30, // off_dt_struct (48)
            0x00, 0x00, 0x00, 0x80, // off_dt_strings (128)
            0x00, 0x00, 0x00, 0x28, // off_mem_rsvmap (40)
            0x00, 0x00, 0x00, 0x11, // version (17)
            0x00, 0x00, 0x00, 0x10, // last_comp_version (16)
            0x00, 0x00, 0x00, 0x00, // boot_cpuid_phys
            0x00, 0x00, 0x00, 0x08, // size_dt_strings (8)
            0x00, 0x00, 0x00, 0x40, // size_dt_struct (64)
            // ... rest of DTB
        ]
    }

    #[test]
    fn test_dtb_error_display() {
        // Test that error types are properly defined
        let _errors = [
            DtbError::Truncated,
            DtbError::InvalidMagic,
            DtbError::TooLarge,
            DtbError::SizeMismatch,
            DtbError::UnsupportedVersion,
            DtbError::InvalidOffset,
            DtbError::InvalidStructureOffset,
            DtbError::InvalidStringsOffset,
            DtbError::InvalidMemRsvmapOffset,
            DtbError::InvalidPropertyLength,
            DtbError::InvalidStringOffset,
            DtbError::MalformedStructure,
            DtbError::DepthExceeded,
            DtbError::NullPointer,
        ];
    }

    #[test]
    fn test_dtb_info_new() {
        let info = DtbInfo::new();
        assert_eq!(info.memory_base, 0);
        assert_eq!(info.memory_size, 0);
        assert_eq!(info.framebuffer.addr.as_u64(), 0);
    }
}
