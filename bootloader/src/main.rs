#![no_std]
#![no_main]
#![feature(allocator_api)]
#![feature(maybe_uninit_slice)]

//! WebbOS UEFI Bootloader
//!
//! This bootloader initializes the system from UEFI firmware,
//! loads the kernel, sets up page tables, and transitions to long mode.

extern crate alloc;

use alloc::vec::Vec;
use uefi::boot::{allocate_pages, AllocateType, MemoryType};
use uefi::mem::memory_map::{MemoryMap, MemoryMapOwned};
use uefi::proto::media::file::{File, FileAttribute, FileMode};
use uefi::{boot, println, Status};
use uefi::CString16;
use webbos_shared::bootinfo::{BootInfo, FramebufferInfo, PixelFormat, BOOTINFO_MAGIC, BOOTINFO_VERSION};
use webbos_shared::types::{MemoryRegion, MemoryRegionType, PhysAddr, VirtAddr, ByteSize};

mod memory;
mod paging;

/// Simple allocator for UEFI
#[global_allocator]
static ALLOCATOR: uefi::allocator::Allocator = uefi::allocator::Allocator;

/// Kernel load address (physical)
const KERNEL_LOAD_ADDR: PhysAddr = PhysAddr::new(0x100000); // 1MB mark

/// Stack size for kernel
const KERNEL_STACK_SIZE: u64 = 128 * 1024; // 128KB

/// Bootloader entry point
#[entry]
fn main() -> Status {
    // Initialize UEFI services
    uefi::helpers::init().unwrap();

    println!("╔═══════════════════════════════════════╗");
    println!("║      WebbOS UEFI Bootloader           ║");
    println!("║      Version 0.1.0                    ║");
    println!("╚═══════════════════════════════════════╝");
    println!();

    // Load kernel from disk
    let kernel_size = match load_kernel() {
        Ok(size) => size,
        Err(e) => {
            println!("ERROR: Failed to load kernel: {:?}", e);
            return Status::LOAD_ERROR;
        }
    };
    println!("Kernel loaded: {} bytes", kernel_size);

    // Get memory map
    let memory_map = match get_memory_map() {
        Ok(map) => map,
        Err(e) => {
            println!("ERROR: Failed to get memory map: {:?}", e);
            return Status::LOAD_ERROR;
        }
    };
    println!("Memory map obtained: {} entries", memory_map.entries().count());

    // Allocate and initialize boot info
    let boot_info = match allocate_boot_info(&memory_map) {
        Ok(info) => info,
        Err(e) => {
            println!("ERROR: Failed to allocate boot info: {:?}", e);
            return Status::OUT_OF_RESOURCES;
        }
    };

    // Get framebuffer info
    let framebuffer_info = get_framebuffer_info();
    if framebuffer_info.is_valid() {
        println!("Framebuffer: {}x{} @ {:?}", 
            framebuffer_info.width, 
            framebuffer_info.height,
            framebuffer_info.addr
        );
    }

    // Allocate kernel stack
    let stack_top = match allocate_stack() {
        Ok(addr) => addr,
        Err(e) => {
            println!("ERROR: Failed to allocate stack: {:?}", e);
            return Status::OUT_OF_RESOURCES;
        }
    };
    println!("Kernel stack: top={:?}", stack_top);

    // Setup page tables for kernel
    let _page_tables = match paging::setup_kernel_paging(kernel_size) {
        Ok(pt) => pt,
        Err(e) => {
            println!("ERROR: Failed to setup paging: {:?}", e);
            return Status::LOAD_ERROR;
        }
    };
    println!("Page tables initialized");

    // Populate boot info
    unsafe {
        let boot_info_ptr = boot_info.as_mut_ptr::<BootInfo>();
        (*boot_info_ptr).magic = BOOTINFO_MAGIC;
        (*boot_info_ptr).version = BOOTINFO_VERSION;
        (*boot_info_ptr)._reserved = 0;
        (*boot_info_ptr).kernel_addr = KERNEL_LOAD_ADDR;
        (*boot_info_ptr).kernel_size = kernel_size as u64;
        (*boot_info_ptr).kernel_virt_addr = VirtAddr::new(0xFFFF_8000_0010_0000);
        (*boot_info_ptr).framebuffer = framebuffer_info;
        (*boot_info_ptr).rsdp_addr = get_rsdp_addr();
        (*boot_info_ptr).cmdline = None;
        (*boot_info_ptr).bootloader_name = PhysAddr::new(b"WebbOS Bootloader\0".as_ptr() as u64);
        (*boot_info_ptr).stack_top = stack_top;
        (*boot_info_ptr).stack_size = KERNEL_STACK_SIZE;
    }

    // Convert memory map to kernel format
    let kernel_memory_map = convert_memory_map(&memory_map);
    let memory_map_addr = unsafe { 
        copy_memory_map(&kernel_memory_map, boot_info.as_ptr::<BootInfo>().add(1) as *mut MemoryRegion)
    };
    
    unsafe {
        let boot_info_ptr = boot_info.as_mut_ptr::<BootInfo>();
        (*boot_info_ptr).memory_map_addr = memory_map_addr;
        (*boot_info_ptr).memory_map_count = kernel_memory_map.len();
    }

    println!("Boot info prepared");
    println!("Exiting boot services and jumping to kernel...");

    // Exit boot services
    unsafe {
        // We need to get the memory map again after exiting boot services
        let _ = boot::exit_boot_services(MemoryType::LOADER_DATA);
    }

    // Jump to kernel
    // The kernel entry point is at virtual address 0xFFFF_8000_0012_14f0
    // (determined from the ELF entry point offset)
    const KERNEL_ENTRY_VIRT: u64 = 0xFFFF_8000_0012_14f0;
    
    unsafe {
        // Switch to the new page tables
        core::arch::asm!(
            "mov cr3, {0}",
            in(reg) _page_tables.as_u64(),
        );
        
        // Jump to kernel at virtual address
        let kernel_entry: extern "sysv64" fn(*const BootInfo) = 
            core::mem::transmute(KERNEL_ENTRY_VIRT as *const u8);
        kernel_entry(boot_info.as_ptr::<BootInfo>());
    }

    // Should never reach here
    Status::LOAD_ERROR
}

/// Load kernel from disk
fn load_kernel() -> uefi::Result<usize> {
    let fs = boot::get_image_file_system(boot::image_handle())?;
    let mut fs = fs;
    
    // Open root directory
    let mut root = fs.open_volume()?;
    
    // Open kernel file - use literal16! macro
    let file = root.open(
        uefi::cstr16!("kernel.elf"),
        FileMode::Read,
        FileAttribute::empty(),
    )?;
    
    let mut file = file.into_regular_file().ok_or_else(|| uefi::Error::new(Status::NOT_FOUND, ()))?;
    
    // Get file size
    let file_info = file.get_boxed_info::<uefi::proto::media::file::FileInfo>()?;
    let file_size = file_info.file_size() as usize;
    
    println!("Kernel file size: {} bytes", file_size);
    
    // Allocate pages for kernel
    let pages = (file_size + 0xFFF) / 0x1000;
    let kernel_pages = allocate_pages(
        AllocateType::AnyPages,
        MemoryType::LOADER_DATA,
        pages,
    )?;
    
    // Read kernel into memory
    let kernel_buffer = unsafe {
        core::slice::from_raw_parts_mut(kernel_pages.as_ptr(), file_size)
    };
    let bytes_read = file.read(kernel_buffer)?;
    
    if bytes_read != file_size {
        println!("WARNING: Read {} bytes, expected {}", bytes_read, file_size);
    }
    
    // Parse ELF header and load segments (simplified - assumes flat binary for now)
    // TODO: Implement proper ELF loading
    
    Ok(bytes_read)
}

/// Get memory map from UEFI
fn get_memory_map() -> uefi::Result<MemoryMapOwned, ()> {
    let memory_map = uefi::boot::memory_map(MemoryType::LOADER_DATA)?;
    Ok(memory_map)
}

/// Allocate boot info structure
fn allocate_boot_info(_memory_map: &MemoryMapOwned) -> uefi::Result<PhysAddr, ()> {
    // Allocate 2 pages for boot info (should be plenty)
    let pages = allocate_pages(
        AllocateType::AnyPages,
        MemoryType::LOADER_DATA,
        2,
    )?;
    
    // Zero the memory
    unsafe {
        core::ptr::write_bytes(pages.as_ptr(), 0, 0x2000);
    }
    
    Ok(PhysAddr::new(pages.as_ptr() as u64))
}

/// Get framebuffer information from GOP
fn get_framebuffer_info() -> FramebufferInfo {
    use uefi::proto::console::gop::{GraphicsOutput, PixelFormat as GopPixelFormat};
    
    let handle = match boot::get_handle_for_protocol::<GraphicsOutput>() {
        Ok(h) => h,
        Err(_) => return FramebufferInfo::default(),
    };
    
    let mut gop = match boot::open_protocol_exclusive::<GraphicsOutput>(handle) {
        Ok(g) => g,
        Err(_) => return FramebufferInfo::default(),
    };
    
    let mode = gop.current_mode_info();
    let stride = mode.stride();
    let (width, height) = mode.resolution();
    
    let (format, bpp) = match mode.pixel_format() {
        GopPixelFormat::Rgb => (PixelFormat::Rgb, 32),
        GopPixelFormat::Bgr => (PixelFormat::Bgr, 32),
        GopPixelFormat::Bitmask { .. } => (PixelFormat::Rgb, 32),
        GopPixelFormat::BltOnly => return FramebufferInfo::default(),
    };
    
    FramebufferInfo {
        addr: PhysAddr::new(gop.frame_buffer().as_mut_ptr() as u64),
        virt_addr: None,
        width: width as u32,
        height: height as u32,
        bpp,
        pitch: (stride * 4) as u32,
        format,
    }
}

/// Get RSDP address for ACPI
fn get_rsdp_addr() -> Option<PhysAddr> {
    // Try to get RSDP from system configuration table
    // This is a simplified version - full implementation would search config tables
    None
}

/// Allocate kernel stack
fn allocate_stack() -> uefi::Result<VirtAddr, ()> {
    let pages = ((KERNEL_STACK_SIZE as usize) + 0xFFF) / 0x1000;
    let stack_pages = allocate_pages(
        AllocateType::AnyPages,
        MemoryType::LOADER_DATA,
        pages,
    )?;
    
    // Stack grows down, so return top of allocated region
    Ok(VirtAddr::new(
        (stack_pages.as_ptr() as u64) + (pages as u64 * 0x1000)
    ))
}

/// Convert UEFI memory map to kernel format
fn convert_memory_map(uefi_map: &MemoryMapOwned) -> Vec<MemoryRegion> {
    let mut regions = Vec::new();
    
    for desc in uefi_map.entries() {
        let region_type = match desc.ty {
            MemoryType::CONVENTIONAL => MemoryRegionType::Available,
            MemoryType::LOADER_CODE | MemoryType::LOADER_DATA => MemoryRegionType::Bootloader,
            MemoryType::BOOT_SERVICES_CODE | MemoryType::BOOT_SERVICES_DATA => MemoryRegionType::Available,
            MemoryType::RUNTIME_SERVICES_CODE | MemoryType::RUNTIME_SERVICES_DATA => MemoryRegionType::Reserved,
            MemoryType::ACPI_RECLAIM => MemoryRegionType::AcpiReclaimable,
            MemoryType::ACPI_NON_VOLATILE => MemoryRegionType::AcpiNvs,
            _ => MemoryRegionType::Reserved,
        };
        
        regions.push(MemoryRegion::new(
            PhysAddr::new(desc.phys_start),
            ByteSize::new(desc.page_count * 0x1000),
            region_type,
        ));
    }
    
    regions
}

/// Copy memory map to boot info location
unsafe fn copy_memory_map(map: &[MemoryRegion], dest: *mut MemoryRegion) -> PhysAddr {
    let _size = map.len() * core::mem::size_of::<MemoryRegion>();
    core::ptr::copy_nonoverlapping(map.as_ptr(), dest, map.len());
    PhysAddr::new(dest as u64)
}

/// Panic handler
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("BOOTLOADER PANIC: {}", info);
    loop {
        unsafe { core::arch::asm!("hlt") };
    }
}

/// Entry point macro
mod entry {
    pub use uefi::entry;
}
use entry::entry;
