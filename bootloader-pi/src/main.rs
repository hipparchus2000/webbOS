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
    // Initialize UART for early output
    uart_init();
    
    uart_puts("\n╔═══════════════════════════════════════╗\n");
    uart_puts("║      WebbOS Pi Bootloader             ║\n");
    uart_puts("║      Version 0.1.0                    ║\n");
    uart_puts("╚═══════════════════════════════════════╝\n\n");
    
    uart_puts("DTB address: ");
    uart_hex(dtb_addr as u64);
    uart_puts("\n");
    
    // Parse device tree
    let dt = DeviceTree::new(dtb_addr);
    
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

// Device Tree support
struct DeviceTree {
    base: usize,
}

impl DeviceTree {
    fn new(base: usize) -> Self {
        Self { base }
    }
    
    fn get_memory_info(&self) -> (u64, u64) {
        // Parse DTB for memory info
        // For now, return Pi 4 defaults
        (0x0, 1024 * 1024 * 1024) // 1GB
    }
    
    fn get_framebuffer_info(&self) -> FramebufferInfo {
        FramebufferInfo {
            addr: PhysAddr::new(0x3E000000), // Pi 4 default
            virt_addr: None,
            width: 1024,
            height: 768,
            pitch: 1024 * 4,
            bpp: 32,
            format: PixelFormat::Bgr,
        }
    }
}

// UART driver (PL011 on Pi 4)
const UART0_BASE: usize = 0xFE201000;
const UART0_DR: usize = UART0_BASE;
const UART0_FR: usize = UART0_BASE + 0x18;

fn uart_init() {
    // UART initialized by GPU firmware
}

fn uart_putc(c: u8) {
    unsafe {
        while (core::ptr::read_volatile(UART0_FR as *const u32) & (1 << 5)) != 0 {}
        core::ptr::write_volatile(UART0_DR as *mut u32, c as u32);
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
