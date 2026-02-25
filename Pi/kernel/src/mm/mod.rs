//! Memory management subsystem
//!
//! Handles physical memory allocation, virtual memory mapping,
//! and the kernel heap allocator.

use webbos_shared::bootinfo::BootInfo;
use webbos_shared::types::{MemoryRegionType, PhysAddr, VirtAddr, PAGE_SIZE, KERNEL_BASE};
use crate::arch::mmu::{BootInfoFrameAllocator, Page, PhysFrame, PageTableFlags, OffsetPageTable};
use crate::println;

pub mod allocator;
pub mod bump;

/// Physical memory offset for kernel
/// 
/// The kernel is mapped at this virtual offset from physical addresses
pub const PHYSICAL_MEMORY_OFFSET: u64 = KERNEL_BASE;

/// Kernel heap start address
pub const HEAP_START: u64 = KERNEL_BASE + 0x40000000; // 1GB after kernel base
/// Initial kernel heap size
pub const HEAP_SIZE: u64 = 8 * 1024 * 1024; // 8MB heap for browser and apps

/// Global bump allocator for early boot
use spin::Mutex;
use lazy_static::lazy_static;
lazy_static! {
    static ref BUMP_ALLOCATOR: Mutex<Option<bump::BumpAllocator>> = Mutex::new(None);
}

/// Initialize memory management
/// 
/// # Safety
/// Must be called exactly once during kernel initialization
pub unsafe fn init(boot_info: &'static BootInfo) {
    // Get memory map
    let memory_map = boot_info.memory_map();
    
    // Print memory map
    println!("  Memory map:");
    for region in memory_map {
        let size_mb = region.size.to_mb();
        let type_str = match region.region_type {
            MemoryRegionType::Available => "Available",
            MemoryRegionType::Reserved => "Reserved",
            MemoryRegionType::AcpiReclaimable => "ACPI Reclaimable",
            MemoryRegionType::AcpiNvs => "ACPI NVS",
            MemoryRegionType::Bad => "Bad",
            MemoryRegionType::Kernel => "Kernel",
            MemoryRegionType::Bootloader => "Bootloader",
            MemoryRegionType::PageTables => "Page Tables",
            MemoryRegionType::Framebuffer => "Framebuffer",
        };
        
        if size_mb > 0 {
            println!("    {:016X}-{:016X} {:6} MB {}",
                region.base.as_u64(),
                region.end().as_u64(),
                size_mb,
                type_str
            );
        }
    }
    
    // Calculate total available memory
    let total_memory: u64 = memory_map
        .iter()
        .filter(|r| matches!(r.region_type, MemoryRegionType::Available))
        .map(|r| r.size.as_u64())
        .sum();
    
    println!("  Total available memory: {} MB", total_memory / (1024 * 1024));
    
    // Initialize MMU/paging
    let mut mapper = crate::arch::mmu::init(PHYSICAL_MEMORY_OFFSET);
    
    // Initialize frame allocator
    let mut frame_allocator = BootInfoFrameAllocator::init(memory_map);
    
    // Initialize heap
    allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("heap initialization failed");
    
    println!("  Heap initialized: {} KB at {:016X}", 
        HEAP_SIZE / 1024, 
        HEAP_START
    );
    
    // Map framebuffer region if present
    map_framebuffer(&mut mapper, &mut frame_allocator, &boot_info.framebuffer);
}

/// Map the framebuffer region in page tables
/// 
/// This ensures the framebuffer is accessible at its expected virtual address
unsafe fn map_framebuffer(
    mapper: &mut OffsetPageTable,
    frame_allocator: &mut BootInfoFrameAllocator,
    fb_info: &webbos_shared::bootinfo::FramebufferInfo,
) {
    if !fb_info.is_valid() {
        println!("  No valid framebuffer to map");
        return;
    }
    
    let phys_start = fb_info.addr.as_u64();
    let size = fb_info.size() as u64;
    let virt_start = phys_start + PHYSICAL_MEMORY_OFFSET;
    
    println!("  Mapping framebuffer: {:016X} -> {:016X} ({} KB)",
        phys_start, virt_start, size / 1024);
    
    // Map each page of the framebuffer
    let page_size = PAGE_SIZE as u64;
    let num_pages = ((size + page_size - 1) / page_size) as usize;
    
    for i in 0..num_pages {
        let phys_addr = PhysAddr::new(phys_start + (i as u64) * page_size);
        let virt_addr = virt_start + (i as u64) * page_size;
        
        let page = Page::containing_address(virt_addr);
        let frame = PhysFrame::containing_address(phys_addr);
        
        // Map with device memory attributes for MMIO
        let flags = PageTableFlags::VALID 
            | PageTableFlags::ATTR_INDEX_0  // Device memory
            | PageTableFlags::AF;
        
        // Ignore errors - the page might already be mapped by bootloader
        let _ = mapper.map_to(page, frame, flags, frame_allocator);
    }
    
    println!("  Framebuffer mapped: {} pages", num_pages);
}

/// Print memory statistics
pub fn print_stats() {
    println!("Memory Statistics:");
    
    let used = allocator::used_heap();
    let free = allocator::free_heap();
    let total = HEAP_SIZE;
    
    println!("  Heap: {} KB used / {} KB total ({} KB free)",
        used / 1024,
        total / 1024,
        free / 1024
    );
}

/// Convert physical address to virtual address
pub fn phys_to_virt(addr: PhysAddr) -> VirtAddr {
    VirtAddr::new(addr.as_u64() + PHYSICAL_MEMORY_OFFSET)
}

/// Convert virtual address to physical address (if mapped)
pub fn virt_to_phys(addr: VirtAddr) -> Option<PhysAddr> {
    crate::arch::mmu::translate_addr(addr.as_u64(), PHYSICAL_MEMORY_OFFSET)
}

/// Convert virtual address (u64) to physical address (u64) for DMA
/// 
/// # Safety
/// This assumes the address is identity mapped with PHYSICAL_MEMORY_OFFSET
pub fn virt_to_phys_u64(addr: u64) -> u64 {
    if addr >= PHYSICAL_MEMORY_OFFSET {
        addr - PHYSICAL_MEMORY_OFFSET
    } else {
        addr // Already physical
    }
}
