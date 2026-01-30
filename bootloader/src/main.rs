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
    // This corresponds to physical address 0x1214f0 in the ELF
    const KERNEL_ENTRY_VIRT: u64 = 0xFFFF_8000_0012_14f0;
    
    println!("Jumping to kernel at {:#x}...", KERNEL_ENTRY_VIRT);
    
    unsafe {
        // Disable interrupts during page table switch
        core::arch::asm!("cli");
        
        // Switch to the new page tables
        core::arch::asm!(
            "mov cr3, {0}",
            in(reg) _page_tables.as_u64(),
        );
        
        // Jump to kernel at virtual address
        // The kernel's _start function expects:
        // - RDI = pointer to BootInfo
        // - Stack at 0xFFFF_8000_0050_0000 (set up by kernel's _start)
        let kernel_entry: extern "sysv64" fn(*const BootInfo) = 
            core::mem::transmute(KERNEL_ENTRY_VIRT as *const u8);
        kernel_entry(boot_info.as_ptr::<BootInfo>());
    }

    // Should never reach here
    Status::LOAD_ERROR
}

/// ELF64 header
#[repr(C)]
struct Elf64Header {
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

/// ELF64 program header
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

const ELFMAG: [u8; 4] = [0x7f, b'E', b'L', b'F'];
const PT_LOAD: u32 = 1;

/// Load kernel from disk and parse ELF
fn load_kernel() -> uefi::Result<usize> {
    let fs = boot::get_image_file_system(boot::image_handle())?;
    let mut fs = fs;
    
    // Open root directory
    let mut root = fs.open_volume()?;
    
    // Open kernel file
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
    
    // Read entire file into temporary buffer
    let temp_pages = allocate_pages(
        AllocateType::AnyPages,
        MemoryType::LOADER_DATA,
        (file_size + 0xFFF) / 0x1000,
    )?;
    
    let file_buffer = unsafe {
        core::slice::from_raw_parts_mut(temp_pages.as_ptr(), file_size)
    };
    let bytes_read = file.read(file_buffer)?;
    
    if bytes_read != file_size {
        println!("WARNING: Read {} bytes, expected {}", bytes_read, file_size);
    }
    
    // Parse ELF header
    let elf_header = unsafe { &*(file_buffer.as_ptr() as *const Elf64Header) };
    
    // Verify ELF magic
    if elf_header.e_ident[0..4] != ELFMAG {
        println!("ERROR: Invalid ELF magic");
        return Err(uefi::Error::new(Status::LOAD_ERROR, ()));
    }
    
    println!("ELF entry point: {:#x}", elf_header.e_entry);
    println!("Program headers: {} at offset {:#x}", elf_header.e_phnum, elf_header.e_phoff);
    
    // Load each program segment at the correct physical address
    let phdr_table = unsafe {
        core::slice::from_raw_parts(
            file_buffer.as_ptr().add(elf_header.e_phoff as usize) as *const Elf64Phdr,
            elf_header.e_phnum as usize,
        )
    };
    
    let mut max_addr = 0usize;
    
    for phdr in phdr_table {
        if phdr.p_type == PT_LOAD {
            // The ELF file has virtual addresses in p_paddr for some segments
            // We need to convert to physical addresses
            // Kernel virtual base is 0xFFFF_8000_0000_0000
            const KERNEL_VIRT_BASE: u64 = 0xFFFF_8000_0000_0000;
            
            let mut dest_addr = phdr.p_paddr as usize;
            // If the address is in the higher half, convert to physical
            if dest_addr as u64 >= KERNEL_VIRT_BASE {
                dest_addr = (dest_addr as u64 - KERNEL_VIRT_BASE) as usize;
            }
            
            let src_offset = phdr.p_offset as usize;
            let filesz = phdr.p_filesz as usize;
            let memsz = phdr.p_memsz as usize;
            
            println!("Loading segment: src={:#x} -> dest={:#x} (phys), size={:#x}/{:#x}",
                src_offset, dest_addr, filesz, memsz);
            
            // Copy data from file to destination
            unsafe {
                let src = file_buffer.as_ptr().add(src_offset);
                let dst = dest_addr as *mut u8;
                core::ptr::copy_nonoverlapping(src, dst, filesz);
                
                // Zero the rest if mem_size > file_size
                if memsz > filesz {
                    core::ptr::write_bytes(dst.add(filesz), 0, memsz - filesz);
                }
            }
            
            // Track highest physical address
            if dest_addr + memsz > max_addr {
                max_addr = dest_addr + memsz;
            }
        }
    }
    
    Ok(max_addr)
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

/// Allocate kernel stack at fixed physical address 0x500000
/// 
/// The kernel expects the stack at virtual address 0xFFFF_8000_0050_0000,
/// so we allocate it at physical address 0x500000 to match.
fn allocate_stack() -> uefi::Result<VirtAddr, ()> {
    let pages = ((KERNEL_STACK_SIZE as usize) + 0xFFF) / 0x1000;
    
    // Allocate at fixed address 0x500000 (5MB)
    // This matches the virtual address 0xFFFF_8000_0050_0000 in higher half
    let stack_pages = allocate_pages(
        AllocateType::Address(0x500000),
        MemoryType::LOADER_DATA,
        pages,
    )?;
    
    // Verify we got the address we requested
    if stack_pages.as_ptr() as u64 != 0x500000 {
        println!("WARNING: Stack allocated at unexpected address: {:p}", stack_pages);
        // Continue anyway, but the stack might not work correctly
    }
    
    // Stack grows down, so return top of allocated region
    // Virtual address is at 0xFFFF_8000_0000_0000 + 0x500000 + stack size
    let stack_top_virt = 0xFFFF_8000_0000_0000u64 + 0x500000 + (pages as u64 * 0x1000);
    Ok(VirtAddr::new(stack_top_virt))
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
