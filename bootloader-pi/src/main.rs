#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(fn_align)]

//! WebbOS Raspberry Pi Bootloader
//!
//! This bootloader initializes the Raspberry Pi and loads the kernel.
//! Loaded by GPU firmware at 0x80000.

use core::arch::{asm, global_asm};
use webbos_shared::bootinfo::{BootInfo, FramebufferInfo, PixelFormat, BOOTINFO_MAGIC, BOOTINFO_VERSION};
use webbos_shared::types::{MemoryRegion, MemoryRegionType, PhysAddr, VirtAddr, ByteSize};

// Include assembly stub
global_asm!(include_str!("boot.S"));

// Constants
const KERNEL_LOAD_ADDR: usize = 0x100000; // 1MB mark
static BOOTLOADER_NAME: &[u8] = b"WebbOS-Pi-Bootloader\0";

// Memory map storage (at a fixed location after the bootloader)
const MEM_MAP_ADDR: usize = 0x90000;
const MAX_MEMORY_REGIONS: usize = 32;

/// Bootloader main function - called from assembly
/// x0 = Device Tree Blob (DTB) physical address
#[no_mangle]
pub extern "C" fn bootloader_main(dtb_addr: usize) -> ! {
    // Initialize UART for early output (uses DTB to detect correct address)
    uart_init(dtb_addr);
    
    uart_puts("\n╔═══════════════════════════════════════╗\n");
    uart_puts("║      WebbOS Pi Bootloader             ║\n");
    uart_puts("║      Version 0.1.0                    ║\n");
    uart_puts("╚═══════════════════════════════════════╝\n\n");
    
    uart_puts("DTB address: ");
    uart_hex(dtb_addr as u64);
    uart_puts("\n");
    
    // Parse device tree
    let dt = DeviceTree::new(dtb_addr);
    
    // Verify DTB magic
    if !dt.verify_magic() {
        uart_puts("ERROR: Invalid DTB magic!\n");
        // Fall back to hardcoded values
    }
    
    // Get memory info
    let (mem_base, mem_size) = dt.get_memory_info();
    uart_puts("Memory: base=");
    uart_hex(mem_base);
    uart_puts(" size=");
    uart_hex(mem_size);
    uart_puts("\n");
    
    // Get framebuffer info
    let fb_info = dt.get_framebuffer_info();
    uart_puts("Framebuffer: ");
    uart_dec(fb_info.width as u64);
    uart_puts("x");
    uart_dec(fb_info.height as u64);
    uart_puts(" @ ");
    uart_hex(fb_info.addr.as_u64());
    uart_puts("\n");
    
    // Get UART info
    if let Some(uart_base) = dt.get_uart_base() {
        uart_puts("UART base: ");
        uart_hex(uart_base);
        uart_puts("\n");
    }
    
    // Load kernel
    uart_puts("Loading kernel...\n");
    let kernel_entry = load_kernel();
    uart_puts("Kernel entry: ");
    uart_hex(kernel_entry as u64);
    uart_puts("\n");
    
    // Set up memory map
    let mem_map = unsafe { 
        core::slice::from_raw_parts_mut(MEM_MAP_ADDR as *mut MemoryRegion, MAX_MEMORY_REGIONS)
    };
    
    mem_map[0] = MemoryRegion {
        base: PhysAddr::new(mem_base),
        size: ByteSize::new(mem_size),
        region_type: MemoryRegionType::Available,
    };
    mem_map[1] = MemoryRegion {
        base: PhysAddr::new(0),
        size: ByteSize::new(0x100000), // First 1MB reserved
        region_type: MemoryRegionType::Reserved,
    };
    
    // Prepare BootInfo (static to ensure it lives long enough)
    static mut BOOT_INFO: BootInfo = BootInfo {
        magic: 0,
        version: 0,
        _reserved: 0,
        memory_map_addr: PhysAddr::new(0),
        memory_map_count: 0,
        kernel_addr: PhysAddr::new(0),
        kernel_size: 0,
        kernel_virt_addr: VirtAddr::new(0),
        framebuffer: FramebufferInfo {
            addr: PhysAddr::new(0),
            virt_addr: None,
            width: 0,
            height: 0,
            bpp: 0,
            pitch: 0,
            format: PixelFormat::Rgb,
        },
        rsdp_addr: None,
        cmdline: None,
        bootloader_name: PhysAddr::new(0),
        stack_top: VirtAddr::new(0),
        stack_size: 0,
    };
    
    let boot_info = unsafe {
        BOOT_INFO.magic = BOOTINFO_MAGIC;
        BOOT_INFO.version = BOOTINFO_VERSION;
        BOOT_INFO.memory_map_addr = PhysAddr::new(MEM_MAP_ADDR as u64);
        BOOT_INFO.memory_map_count = 2;
        BOOT_INFO.kernel_addr = PhysAddr::new(KERNEL_LOAD_ADDR as u64);
        BOOT_INFO.kernel_size = get_kernel_size() as u64;
        BOOT_INFO.kernel_virt_addr = VirtAddr::new(KERNEL_LOAD_ADDR as u64);
        BOOT_INFO.framebuffer = fb_info;
        BOOT_INFO.stack_top = VirtAddr::new(0xFFFF_8000_0000_0000u64 + 0x500000);
        BOOT_INFO.stack_size = 128 * 1024;
        BOOT_INFO.bootloader_name = PhysAddr::new(BOOTLOADER_NAME.as_ptr() as u64);
        &BOOT_INFO
    };
    
    uart_puts("Boot info prepared\n");
    uart_puts("Jumping to kernel...\n\n");
    
    // Jump to kernel
    unsafe {
        let kernel_fn: extern "C" fn(&'static BootInfo) -> ! = 
            core::mem::transmute::<usize, _>(kernel_entry);
        kernel_fn(boot_info);
    }
    
    // Should not reach here
    loop {
        unsafe { asm!("wfe", options(nomem, nostack)); }
    }
}

/// Get kernel size
fn get_kernel_size() -> usize {
    // Placeholder - in real implementation, this would be determined by build
    0xF00000 - KERNEL_LOAD_ADDR // Up to 15MB
}

/// Load kernel from its load address
fn load_kernel() -> usize {
    // Check for ELF magic
    let elf_magic = unsafe { 
        core::slice::from_raw_parts(KERNEL_LOAD_ADDR as *const u8, 4) 
    };
    
    if elf_magic == [0x7F, b'E', b'L', b'F'] {
        uart_puts("ELF kernel detected\n");
        unsafe { load_elf_kernel(KERNEL_LOAD_ADDR) }
    } else {
        uart_puts("Raw binary kernel\n");
        KERNEL_LOAD_ADDR
    }
}

/// Load ELF kernel
unsafe fn load_elf_kernel(addr: usize) -> usize {
    let elf = &*(addr as *const Elf64Hdr);
    
    uart_puts("ELF entry point: ");
    uart_hex(elf.e_entry);
    uart_puts("\n");
    
    let phdrs = core::slice::from_raw_parts(
        (addr + elf.e_phoff as usize) as *const Elf64Phdr,
        elf.e_phnum as usize,
    );
    
    for phdr in phdrs.iter().filter(|p| p.p_type == PT_LOAD && p.p_filesz > 0) {
        uart_puts("Loading segment: dest=");
        uart_hex(phdr.p_paddr);
        uart_puts(" size=");
        uart_dec(phdr.p_filesz);
        uart_puts("\n");
        
        let src = (addr + phdr.p_offset as usize) as *const u8;
        let dst = phdr.p_paddr as *mut u8;
        core::ptr::copy_nonoverlapping(src, dst, phdr.p_filesz as usize);
        
        // Zero BSS
        if phdr.p_memsz > phdr.p_filesz {
            let bss_start = (phdr.p_paddr + phdr.p_filesz) as *mut u8;
            let bss_size = (phdr.p_memsz - phdr.p_filesz) as usize;
            core::ptr::write_bytes(bss_start, 0, bss_size);
        }
    }
    
    elf.e_entry as usize
}

// ELF structures
#[repr(C)]
struct Elf64Hdr {
    e_ident: [u8; 16],
    e_type: u16,
    e_machine: u16,
    e_version: u32,
    e_entry: u64,
    e_phoff: u64,
    e_shoff: u64,
    e_flags: u32,
    e_ehsize: u16,
    e_phentsize: u16,
    e_phnum: u16,
    e_shentsize: u16,
    e_shnum: u16,
    e_shstrndx: u16,
}

#[repr(C)]
struct Elf64Phdr {
    p_type: u32,
    p_flags: u32,
    p_offset: u64,
    p_vaddr: u64,
    p_paddr: u64,
    p_filesz: u64,
    p_memsz: u64,
    p_align: u64,
}

const PT_LOAD: u32 = 1;

// =============================================================================
// Device Tree Blob (DTB) Parser
// =============================================================================

/// DTB magic number
const DTB_MAGIC: u32 = 0xd00dfeed;

/// DTB token types
const FDT_BEGIN_NODE: u32 = 0x1;
const FDT_END_NODE: u32 = 0x2;
const FDT_PROP: u32 = 0x3;
const FDT_NOP: u32 = 0x4;
const FDT_END: u32 = 0x9;

/// DTB Header structure (40 bytes)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct DtbHeader {
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

/// Device Tree parser
struct DeviceTree {
    base: usize,
    header: DtbHeader,
    struct_block: usize,
    strings_block: usize,
}

impl DeviceTree {
    /// Create a new DeviceTree parser from the DTB base address
    fn new(base: usize) -> Self {
        // Read header from memory
        let header = unsafe { *(base as *const DtbHeader) };
        
        // Convert offsets to addresses (handle endianness)
        let struct_block = base + u32::from_be(header.off_dt_struct) as usize;
        let strings_block = base + u32::from_be(header.off_dt_strings) as usize;
        
        Self {
            base,
            header,
            struct_block,
            strings_block,
        }
    }
    
    /// Verify the DTB magic number
    fn verify_magic(&self) -> bool {
        u32::from_be(self.header.magic) == DTB_MAGIC
    }
    
    /// Get memory information from the /memory node
    fn get_memory_info(&self) -> (u64, u64) {
        // Default values for Pi 4 (1GB)
        let mut base = 0u64;
        let mut size = 1024 * 1024 * 1024u64;
        
        // Try to find /memory node
        if let Some((node_base, node_size)) = self.find_memory_node() {
            base = node_base;
            size = node_size;
        }
        
        (base, size)
    }
    
    /// Find the /memory node and extract reg property
    fn find_memory_node(&self) -> Option<(u64, u64)> {
        let mut walker = DtbWalker::new(self.struct_block, self.strings_block);
        
        // Walk through the device tree
        while let Some(token) = walker.next_token() {
            match token {
                DtbToken::BeginNode(name) => {
                    if name == "memory" || name.starts_with("memory@") {
                        // Found memory node, look for reg property
                        return self.parse_memory_reg(&mut walker);
                    }
                }
                DtbToken::End => break,
                _ => {}
            }
        }
        
        None
    }
    
    /// Parse the reg property from a memory node
    fn parse_memory_reg(&self, walker: &mut DtbWalker) -> Option<(u64, u64)> {
        while let Some(token) = walker.next_token() {
            match token {
                DtbToken::Property(name, value) => {
                    if name == "reg" && value.len() >= 16 {
                        // reg is typically two 64-bit values: base and size
                        // Format depends on #address-cells and #size-cells (usually 2 each)
                        let base = self.read_u64_be(value, 0);
                        let size = self.read_u64_be(value, 8);
                        return Some((base, size));
                    }
                }
                DtbToken::BeginNode(_) => {
                    // Skip nested nodes
                    walker.skip_node();
                }
                DtbToken::EndNode => break,
                DtbToken::End => break,
                _ => {}
            }
        }
        
        None
    }
    
    /// Get framebuffer information
    fn get_framebuffer_info(&self) -> FramebufferInfo {
        // Default values
        let mut info = FramebufferInfo {
            addr: PhysAddr::new(0x3E000000),
            virt_addr: None,
            width: 1024,
            height: 768,
            pitch: 1024 * 4,
            bpp: 32,
            format: PixelFormat::Bgr,
        };
        
        // Try to find framebuffer info in /chosen or /soc/fb
        if let Some(fb) = self.find_framebuffer_node() {
            info = fb;
        }
        
        info
    }
    
    /// Find framebuffer information in device tree
    fn find_framebuffer_node(&self) -> Option<FramebufferInfo> {
        // Look in /chosen node for framebuffer info
        let mut walker = DtbWalker::new(self.struct_block, self.strings_block);
        
        while let Some(token) = walker.next_token() {
            match token {
                DtbToken::BeginNode(name) => {
                    if name == "chosen" {
                        return self.parse_chosen_framebuffer(&mut walker);
                    }
                }
                DtbToken::End => break,
                _ => {}
            }
        }
        
        None
    }
    
    /// Parse framebuffer info from /chosen node
    fn parse_chosen_framebuffer(&self, walker: &mut DtbWalker) -> Option<FramebufferInfo> {
        let mut width = 1024u32;
        let mut height = 768u32;
        let mut addr = 0u64;
        let bpp = 32u32;
        
        while let Some(token) = walker.next_token() {
            match token {
                DtbToken::Property(name, value) => {
                    match name {
                        "bootargs" => {
                            // Parse bootargs for video mode
                            if let Some(args) = core::str::from_utf8(value).ok() {
                                // Look for video= or similar settings
                                if let Some((w, h)) = self.parse_video_mode(args) {
                                    width = w;
                                    height = h;
                                }
                            }
                        }
                        "linux,initrd-start" => {
                            if value.len() >= 4 {
                                addr = self.read_u32_be(value, 0) as u64;
                            }
                        }
                        _ => {}
                    }
                }
                DtbToken::BeginNode(name) => {
                    // Check for simple-framebuffer subnode
                    if name.starts_with("framebuffer") {
                        return self.parse_simple_framebuffer(walker);
                    }
                    walker.skip_node();
                }
                DtbToken::EndNode => break,
                DtbToken::End => break,
                _ => {}
            }
        }
        
        if addr != 0 {
            Some(FramebufferInfo {
                addr: PhysAddr::new(addr),
                virt_addr: None,
                width,
                height,
                pitch: width * 4,
                bpp,
                format: PixelFormat::Bgr,
            })
        } else {
            None
        }
    }
    
    /// Parse simple-framebuffer subnode
    fn parse_simple_framebuffer(&self, walker: &mut DtbWalker) -> Option<FramebufferInfo> {
        let mut width = 1024u32;
        let mut height = 768u32;
        let mut stride = 4096u32;
        let mut format = PixelFormat::Bgr;
        
        while let Some(token) = walker.next_token() {
            match token {
                DtbToken::Property(name, value) => {
                    match name {
                        "width" => width = self.read_u32_be(value, 0),
                        "height" => height = self.read_u32_be(value, 0),
                        "stride" => stride = self.read_u32_be(value, 0),
                        "format" => {
                            if let Some(fmt) = core::str::from_utf8(value).ok() {
                                if fmt.starts_with("r5g6b5") {
                                    format = PixelFormat::Rgb;
                                } else if fmt.starts_with("a8r8g8b8") || fmt.starts_with("x8r8g8b8") {
                                    format = PixelFormat::Bgr;
                                }
                            }
                        }
                        _ => {}
                    }
                }
                DtbToken::BeginNode(_) => {
                    walker.skip_node();
                }
                DtbToken::EndNode => break,
                DtbToken::End => break,
                _ => {}
            }
        }
        
        Some(FramebufferInfo {
            addr: PhysAddr::new(0), // Will be filled later
            virt_addr: None,
            width,
            height,
            pitch: stride,
            bpp: if stride == width * 2 { 16 } else { 32 },
            format,
        })
    }
    
    /// Parse video mode from bootargs string (e.g., "video=1920x1080")
    fn parse_video_mode(&self, args: &str) -> Option<(u32, u32)> {
        // Look for video= parameter
        if let Some(pos) = args.find("video=") {
            let start = pos + 6;
            let rest = &args[start..];
            
            // Find resolution pattern like 1920x1080
            let end = rest.find(' ').unwrap_or(rest.len());
            let video_spec = &rest[..end];
            
            if let Some(x_pos) = video_spec.find('x') {
                if let Ok(w) = video_spec[..x_pos].parse::<u32>() {
                    // Parse height (may have other params after)
                    let h_str = &video_spec[x_pos + 1..];
                    let h_end = h_str.find(|c: char| !c.is_ascii_digit()).unwrap_or(h_str.len());
                    if let Ok(h) = h_str[..h_end].parse::<u32>() {
                        return Some((w, h));
                    }
                }
            }
        }
        
        None
    }
    
    /// Get UART base address
    fn get_uart_base(&self) -> Option<u64> {
        // Look for serial device in /soc
        let mut walker = DtbWalker::new(self.struct_block, self.strings_block);
        
        while let Some(token) = walker.next_token() {
            match token {
                DtbToken::BeginNode(name) => {
                    if name == "soc" {
                        return self.find_uart_in_soc(&mut walker);
                    }
                }
                DtbToken::End => break,
                _ => {}
            }
        }
        
        None
    }
    
    /// Find UART in /soc node
    fn find_uart_in_soc(&self, walker: &mut DtbWalker) -> Option<u64> {
        while let Some(token) = walker.next_token() {
            match token {
                DtbToken::BeginNode(name) => {
                    // Look for serial@ or uart@ nodes
                    if name.starts_with("serial@") || name.starts_with("uart@") {
                        // Check if this is the PL011 UART
                        if let Some(reg) = self.parse_uart_reg(walker) {
                            return Some(reg);
                        }
                    } else if name != "" {
                        // Skip other nodes
                        walker.skip_node();
                    }
                }
                DtbToken::EndNode => break,
                DtbToken::End => break,
                _ => {}
            }
        }
        
        None
    }
    
    /// Parse UART reg property
    fn parse_uart_reg(&self, walker: &mut DtbWalker) -> Option<u64> {
        let mut compatible_pl011 = false;
        let mut reg: Option<u64> = None;
        
        while let Some(token) = walker.next_token() {
            match token {
                DtbToken::Property(name, value) => {
                    match name {
                        "compatible" => {
                            // Check if compatible with arm,pl011
                            if let Some(compat) = core::str::from_utf8(value).ok() {
                                if compat.contains("pl011") || compat.contains("arm,pl011") {
                                    compatible_pl011 = true;
                                }
                            }
                        }
                        "reg" => {
                            if value.len() >= 8 {
                                // For #address-cells=2, #size-cells=2
                                if value.len() >= 16 {
                                    reg = Some(self.read_u64_be(value, 0));
                                } else {
                                    reg = Some(self.read_u32_be(value, 0) as u64);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                DtbToken::BeginNode(_) => {
                    walker.skip_node();
                }
                DtbToken::EndNode => break,
                DtbToken::End => break,
                _ => {}
            }
        }
        
        if compatible_pl011 || reg.is_some() {
            reg
        } else {
            None
        }
    }
    
    /// Read big-endian u32 from byte slice
    fn read_u32_be(&self, data: &[u8], offset: usize) -> u32 {
        if offset + 4 > data.len() {
            return 0;
        }
        u32::from_be_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ])
    }
    
    /// Read big-endian u64 from byte slice
    fn read_u64_be(&self, data: &[u8], offset: usize) -> u64 {
        if offset + 8 > data.len() {
            return 0;
        }
        let hi = self.read_u32_be(data, offset) as u64;
        let lo = self.read_u32_be(data, offset + 4) as u64;
        (hi << 32) | lo
    }
}

/// DTB token types during parsing
#[derive(Debug)]
enum DtbToken<'a> {
    BeginNode(&'a str),
    EndNode,
    Property(&'a str, &'a [u8]),
    Nop,
    End,
}

/// DTB structure block walker
struct DtbWalker {
    ptr: usize,
    strings_block: usize,
}

impl DtbWalker {
    /// Create a new walker starting at the structure block
    fn new(struct_block: usize, strings_block: usize) -> Self {
        Self {
            ptr: struct_block,
            strings_block,
        }
    }
    
    /// Get the next token from the structure block
    fn next_token<'a>(&mut self) -> Option<DtbToken<'a>> {
        loop {
            let token = unsafe { core::ptr::read_volatile(self.ptr as *const u32) };
            let token_be = u32::from_be(token);
            
            match token_be {
                FDT_BEGIN_NODE => {
                    self.ptr += 4;
                    // Read null-terminated node name
                    let name = self.read_string();
                    // Align to 4-byte boundary
                    self.align_ptr();
                    return Some(DtbToken::BeginNode(name));
                }
                FDT_END_NODE => {
                    self.ptr += 4;
                    return Some(DtbToken::EndNode);
                }
                FDT_PROP => {
                    self.ptr += 4;
                    // Read property header
                    let len = u32::from_be(unsafe { 
                        core::ptr::read_volatile(self.ptr as *const u32) 
                    }) as usize;
                    let nameoff = u32::from_be(unsafe { 
                        core::ptr::read_volatile((self.ptr + 4) as *const u32) 
                    }) as usize;
                    self.ptr += 8;
                    
                    // Get property name from strings block
                    let name = self.get_string(nameoff);
                    
                    // Get property value
                    let value = unsafe { 
                        core::slice::from_raw_parts(self.ptr as *const u8, len) 
                    };
                    self.ptr += len;
                    // Align to 4-byte boundary
                    self.align_ptr();
                    
                    return Some(DtbToken::Property(name, value));
                }
                FDT_NOP => {
                    self.ptr += 4;
                    // Continue to next token
                }
                FDT_END => {
                    self.ptr += 4;
                    return Some(DtbToken::End);
                }
                _ => {
                    // Unknown token, might be corrupted
                    return None;
                }
            }
        }
    }
    
    /// Skip the current node (after BeginNode has been consumed)
    fn skip_node(&mut self) {
        let mut depth = 1;
        
        while depth > 0 {
            if let Some(token) = self.next_token() {
                match token {
                    DtbToken::BeginNode(_) => depth += 1,
                    DtbToken::EndNode => depth -= 1,
                    _ => {}
                }
            } else {
                break;
            }
        }
    }
    
    /// Read a null-terminated string from current pointer
    fn read_string<'a>(&self) -> &'a str {
        let start = self.ptr;
        let mut len = 0;
        
        // Find null terminator
        unsafe {
            while core::ptr::read_volatile((start + len) as *const u8) != 0 {
                len += 1;
            }
        }
        
        // Convert to str
        let bytes = unsafe { core::slice::from_raw_parts(start as *const u8, len) };
        core::str::from_utf8(bytes).unwrap_or("")
    }
    
    /// Get string from strings block at given offset
    fn get_string<'a>(&self, offset: usize) -> &'a str {
        let addr = self.strings_block + offset;
        let mut len = 0;
        
        // Find null terminator
        unsafe {
            while core::ptr::read_volatile((addr + len) as *const u8) != 0 {
                len += 1;
            }
        }
        
        let bytes = unsafe { core::slice::from_raw_parts(addr as *const u8, len) };
        core::str::from_utf8(bytes).unwrap_or("")
    }
    
    /// Align pointer to 4-byte boundary
    fn align_ptr(&mut self) {
        self.ptr = (self.ptr + 3) & !3;
    }
}

// =============================================================================
// UART Driver (PL011)
// =============================================================================

/// UART base address - defaults to Pi 3 for QEMU raspi3b testing
/// Will be updated from DTB during initialization if available
/// - Pi 3: 0x3F201000
/// - Pi 4: 0xFE201000  
/// - Pi 5: 0x107D001000
static mut UART_BASE: usize = 0x3F201000; // Pi 3 default for QEMU

const UART_DR: usize = 0x00;
const UART_FR: usize = 0x18;

/// Initialize UART - updates base address from DTB if available
/// Falls back to Pi 3 default (0x3F201000) for QEMU raspi3b
fn uart_init(dtb_addr: usize) {
    // Try to get UART address from DTB
    let dt = DeviceTree::new(dtb_addr);
    
    if dt.verify_magic() {
        if let Some(uart_base) = dt.get_uart_base() {
            unsafe { UART_BASE = uart_base as usize; }
        }
    }
    // If DTB parsing fails, keep the default Pi 3 address
}

fn uart_putc(c: u8) {
    let base = unsafe { UART_BASE };
    unsafe {
        while (core::ptr::read_volatile((base + UART_FR) as *const u32) & (1 << 5)) != 0 {}
        core::ptr::write_volatile((base + UART_DR) as *mut u32, c as u32);
    }
}

fn uart_puts(s: &str) {
    for c in s.bytes() {
        if c == b'\n' {
            uart_putc(b'\r');
        }
        uart_putc(c);
    }
}

fn uart_hex(n: u64) {
    const HEX: &[u8] = b"0123456789ABCDEF";
    uart_puts("0x");
    for i in (0..16).rev() {
        uart_putc(HEX[((n >> (i * 4)) & 0xF) as usize]);
    }
}

fn uart_dec(n: u64) {
    if n == 0 {
        uart_putc(b'0');
        return;
    }
    
    let mut buf = [0u8; 20];
    let mut i = 0;
    let mut n = n;
    
    while n > 0 {
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
        i += 1;
    }
    
    for j in (0..i).rev() {
        uart_putc(buf[j]);
    }
}

// Panic handler
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    uart_puts("\n*** BOOTLOADER PANIC ***\n");
    if let Some(loc) = info.location() {
        uart_puts("Location: ");
        uart_puts(loc.file());
        uart_puts(":");
        uart_dec(loc.line() as u64);
        uart_puts("\n");
    }
    loop {
        unsafe { asm!("wfe", options(nomem, nostack)); }
    }
}
