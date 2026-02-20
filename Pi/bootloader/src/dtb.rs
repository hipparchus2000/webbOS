//! Device Tree Blob (DTB) Parser
//!
//! The Raspberry Pi firmware passes a device tree to the kernel
//! that describes the hardware configuration.

use webbos_shared::bootinfo::FramebufferInfo;
use webbos_shared::types::PhysAddr;

/// DTB magic number
const FDT_MAGIC: u32 = 0xD00D_FEED;

/// DTB header structure
#[repr(C)]
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

/// Parse the device tree blob
pub fn parse_dtb(dtb_addr: u64) -> Option<DtbInfo> {
    let header = unsafe { &*(dtb_addr as *const FdtHeader) };

    // Check magic number
    if u32::from_be(header.magic) != FDT_MAGIC {
        return None;
    }

    let mut info = DtbInfo::new();

    // Parse memory reservation map (not used but good to skip)
    let mem_rsvmap_off = u32::from_be(header.off_mem_rsvmap) as usize;
    
    // Parse structure block
    let struct_off = u32::from_be(header.off_dt_struct) as usize;
    let strings_off = u32::from_be(header.off_dt_strings) as usize;

    // Simple parser - just extract memory info
    // In a full implementation, we'd traverse the tree
    unsafe {
        let base = dtb_addr as usize;
        let struct_ptr = (base + struct_off) as *const u32;
        let strings_base = base + strings_off;

        parse_structure(struct_ptr, strings_base, &mut info);
    }

    // Set default memory if not found in DTB
    if info.memory_size == 0 {
        // Pi 3/4 typically have at least 1GB
        info.memory_base = 0;
        info.memory_size = 0x40000000; // 1GB default
    }

    Some(info)
}

/// Parse the structure block
unsafe fn parse_structure(mut ptr: *const u32, strings_base: usize, info: &mut DtbInfo) {
    let mut depth = 0;
    let mut current_node = "";

    loop {
        let token = u32::from_be(*ptr);
        ptr = ptr.add(1);

        match token {
            FDT_BEGIN_NODE => {
                depth += 1;
                // Read node name (null-terminated string)
                let name_ptr = ptr as *const u8;
                let name_len = strlen(name_ptr);
                current_node = core::str::from_utf8_unchecked(
                    core::slice::from_raw_parts(name_ptr, name_len)
                );
                
                // Check for memory node
                if current_node.starts_with("memory@") || current_node == "memory" {
                    parse_memory_node(ptr, info);
                }
                
                // Skip to after null terminator (aligned to 4 bytes)
                ptr = ((name_ptr as usize + name_len + 4) & !3) as *const u32;
            }
            FDT_END_NODE => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            FDT_PROP => {
                let len = u32::from_be(*ptr) as usize;
                let nameoff = u32::from_be(*ptr.add(1)) as usize;
                ptr = ptr.add(2);

                // Get property name from strings block
                let name_ptr = (strings_base + nameoff) as *const u8;
                let name_len = strlen(name_ptr);
                let name = core::str::from_utf8_unchecked(
                    core::slice::from_raw_parts(name_ptr, name_len)
                );

                // Parse property value
                if name == "reg" && len == 16 {
                    // Memory region: base (8 bytes) + size (8 bytes)
                    let base = u64::from_be(*(ptr as *const u64));
                    let size = u64::from_be(*((ptr as *const u64).add(1)));
                    if info.memory_size == 0 || base == 0 {
                        info.memory_base = base;
                        info.memory_size = size;
                    }
                }

                // Skip property value (aligned to 4 bytes)
                ptr = ((ptr as usize + len + 3) & !3) as *const u32;
            }
            FDT_NOP => {
                // Do nothing
            }
            FDT_END => {
                break;
            }
            _ => {
                // Unknown token, skip
            }
        }
    }
}

/// Parse memory node specifically
unsafe fn parse_memory_node(_ptr: *const u32, _info: &mut DtbInfo) {
    // Memory properties are handled in the main parser
}

/// Calculate length of null-terminated string
unsafe fn strlen(ptr: *const u8) -> usize {
    let mut len = 0;
    while *ptr.add(len) != 0 {
        len += 1;
    }
    len
}
