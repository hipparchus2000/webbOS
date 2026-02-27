#![no_std]
#![no_main]
#![feature(allocator_api)]
#![feature(naked_functions)]
#![feature(fn_align)]
#![feature(core_intrinsics)]
#![allow(internal_features)]

//! WebbOS Raspberry Pi Bootloader (No Serial Version)
//!
//! Minimal bootloader that sets up MMU and jumps directly to kernel.
//! No serial output - relies on framebuffer for display.

extern crate alloc;

mod pi_start;
mod mmu;
mod dtb;
mod memory;
mod uart;      // UART for serial debug output
mod fb_debug;  // Framebuffer for on-screen debug output

use webbos_shared::bootinfo::{BootInfo, FramebufferInfo, PixelFormat, BOOTINFO_MAGIC, BOOTINFO_VERSION};
use core::arch::asm;
use webbos_shared::types::{MemoryRegion, MemoryRegionType, PhysAddr, VirtAddr, ByteSize};

/// Kernel load address (physical) - after the bootloader
const KERNEL_LOAD_ADDR: PhysAddr = PhysAddr::new(0x100000); // 1MB mark

/// Stack size for kernel
const KERNEL_STACK_SIZE: u64 = 128 * 1024; // 128KB

/// Kernel virtual base address (higher half)
const KERNEL_VIRT_BASE: u64 = 0xFFFF_0000_0000_0000;

use core::alloc::{GlobalAlloc, Layout};

/// Bootloader global allocator - simple bump allocator
#[global_allocator]
static BUMP_ALLOCATOR: BumpAllocator = BumpAllocator::new();

/// Simple bump allocator for bootloader use
/// Uses interior mutability via UnsafeCell for thread-safe static allocation
use core::cell::UnsafeCell;

struct BumpAllocator {
    current: UnsafeCell<usize>,
    end: usize,
}

impl BumpAllocator {
    const fn new() -> Self {
        Self {
            current: UnsafeCell::new(0x400000), // Start at 4MB
            end: 0x1000000,    // End at 16MB
        }
    }
}

// SAFETY: Bootloader is single-threaded
unsafe impl Sync for BumpAllocator {}

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let current = *self.current.get();
        let aligned = (current + layout.align() - 1) & !(layout.align() - 1);
        if aligned + layout.size() > self.end {
            return core::ptr::null_mut();
        }
        *self.current.get() = aligned + layout.size();
        aligned as *mut u8
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Bump allocator doesn't support deallocation
    }
}

/// Helper function to allocate pages
pub unsafe fn alloc_pages(count: usize) -> Option<PhysAddr> {
    let size = count * 4096;
    let layout = Layout::from_size_align(size, 4096).ok()?;
    let ptr = unsafe { BUMP_ALLOCATOR.alloc(layout) };
    if ptr.is_null() {
        None
    } else {
        Some(PhysAddr::new(ptr as u64))
    }
}

/// Main entry point from assembly
/// 
/// x0 = Device tree blob physical address
#[no_mangle]
pub extern "C" fn rust_main(dtb_addr: u64) -> ! {
    // Initialize UART for serial debug output (works even without framebuffer)
    uart::init();
    uart::puts("\n\n[bootloader] Starting WebbOS...\n");
    
    // Parse device tree to get memory info and framebuffer
    uart::puts("[bootloader] Parsing DTB...\n");
    let dtb_info = match unsafe { dtb::parse_dtb(dtb_addr) } {
        Some(info) => {
            // Initialize framebuffer debug output
            fb_debug::init(&info.framebuffer);
            if fb_debug::is_initialized() {
                fb_debug::puts("WebbOS Bootloader\n");
                fb_debug::puts("=================\n\n");
                fb_debug::puts("Framebuffer initialized!\n");
                fb_debug::puts("Screen: ");
                fb_debug::putdec(info.framebuffer.width as u64);
                fb_debug::puts("x");
                fb_debug::putdec(info.framebuffer.height as u64);
                fb_debug::puts("\n\n");
            }
            info
        },
        None => {
            // Default values for Raspberry Pi 3 with 1GB RAM
            uart::puts("[bootloader] Warning: DTB parsing failed, using defaults\n");
            dtb::DtbInfo {
                memory_base: 0,
                memory_size: 0x40000000, // 1GB
                framebuffer: FramebufferInfo {
                    addr: PhysAddr::new(0),
                    virt_addr: None,
                    width: 1024,
                    height: 768,
                    pitch: 4096,
                    bpp: 32,
                    format: PixelFormat::Rgb,
                },
            }
        }
    };

    // Load kernel from memory
    uart::puts("[bootloader] Loading kernel...\n");
    fb_debug::puts("Loading kernel... ");
    let kernel_size = match load_kernel() {
        Ok(size) => {
            uart::puts("[bootloader] Kernel loaded, size: ");
            uart::putdec(size as u64);
            uart::puts(" bytes\n");
            fb_debug::putdec(size as u64);
            fb_debug::puts(" bytes OK\n");
            size
        },
        Err(_) => {
            uart::puts("[bootloader] ERROR: Failed to load kernel\n");
            fb_debug::puts("FAILED\n");
            boot_fail()
        },
    };

    // Set up initial page tables
    uart::puts("[bootloader] Setting up page tables...\n");
    fb_debug::puts("Setting up MMU... ");
    let page_tables = match mmu::setup_page_tables(kernel_size) {
        Ok(pt) => {
            uart::puts("[bootloader] Page tables ready\n");
            fb_debug::puts("OK\n");
            pt
        },
        Err(_) => {
            uart::puts("[bootloader] ERROR: Failed to setup page tables\n");
            fb_debug::puts("FAILED\n");
            boot_fail()
        },
    };

    // Allocate and set up boot info
    let boot_info_addr = match allocate_boot_info(&dtb_info) {
        Some(addr) => addr,
        None => boot_fail(),
    };

    // Populate boot info structure
    unsafe {
        let boot_info = &mut *(boot_info_addr.as_mut_ptr::<BootInfo>());
        boot_info.magic = BOOTINFO_MAGIC;
        boot_info.version = BOOTINFO_VERSION;
        boot_info._reserved = 0;
        boot_info.kernel_addr = KERNEL_LOAD_ADDR;
        boot_info.kernel_size = kernel_size as u64;
        boot_info.kernel_virt_addr = VirtAddr::new(KERNEL_VIRT_BASE + 0x100000);
        boot_info.framebuffer = dtb_info.framebuffer;
        boot_info.rsdp_addr = None;
        boot_info.cmdline = None;
        boot_info.bootloader_name = PhysAddr::new(b"WebbOS Pi Bootloader\0".as_ptr() as u64);
        boot_info.stack_top = VirtAddr::new(KERNEL_VIRT_BASE + 0x500000 + KERNEL_STACK_SIZE);
        boot_info.stack_size = KERNEL_STACK_SIZE;
        boot_info.memory_map_addr = PhysAddr::new((boot_info_addr.as_u64() + core::mem::size_of::<BootInfo>() as u64) as u64);
        boot_info.memory_map_count = create_memory_map(&dtb_info, boot_info.memory_map_addr.as_mut_ptr::<MemoryRegion>());
    }

    // Enable MMU and jump to kernel
    uart::puts("[bootloader] Enabling MMU and jumping to kernel at 0x");
    uart::puthex(KERNEL_VIRT_BASE + 0x100000);
    uart::puts("\n[bootloader] Goodbye!\n\n");
    
    fb_debug::puts("\nBooting kernel...\n");
    
    unsafe {
        mmu::enable_mmu_and_jump(page_tables, boot_info_addr, KERNEL_VIRT_BASE + 0x100000);
    }
}

/// Load kernel from memory - kernel is already at 0x100000
fn load_kernel() -> Result<usize, ()> {
    // Kernel binary is embedded at 0x100000 (1MB mark)
    // Just return approximate size
    Ok(0x50000) // 320KB
}

/// Allocate boot info structure
fn allocate_boot_info(_dtb_info: &dtb::DtbInfo) -> Option<PhysAddr> {
    unsafe { alloc_pages(2) }
}

/// Create memory map from DTB info
fn create_memory_map(dtb_info: &dtb::DtbInfo, dest: *mut MemoryRegion) -> usize {
    let regions = [
        // Main RAM region
        MemoryRegion::new(
            PhysAddr::new(dtb_info.memory_base),
            ByteSize::new(dtb_info.memory_size),
            MemoryRegionType::Available,
        ),
    ];

    unsafe {
        core::ptr::copy_nonoverlapping(regions.as_ptr(), dest, regions.len());
    }

    regions.len()
}

/// Boot failure - print error and halt
fn boot_fail() -> ! {
    uart::puts("\n[bootloader] FATAL: Boot failed, halting\n");
    fb_debug::puts("\n*** BOOT FAILED ***\n");
    fb_debug::puts("System halted\n");
    loop {
        unsafe { asm!("wfe") };
    }
}

/// Panic handler
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        unsafe { asm!("wfe") };
    }
}
