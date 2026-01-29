//! Memory management subsystem
//!
//! Handles physical memory allocation, virtual memory mapping,
//! and the kernel heap allocator.

use webbos_shared::bootinfo::BootInfo;
use webbos_shared::types::{MemoryRegionType, PhysAddr, VirtAddr, KERNEL_BASE};
use crate::arch::paging::BootInfoFrameAllocator;
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
pub const HEAP_SIZE: u64 = 1024 * 1024; // 1MB initial heap

/// Global bump allocator for early boot
static mut BUMP_ALLOCATOR: Option<bump::BumpAllocator> = None;

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
    
    // Initialize paging
    let mut mapper = crate::arch::paging::init(PHYSICAL_MEMORY_OFFSET);
    
    // Initialize frame allocator
    let mut frame_allocator = BootInfoFrameAllocator::init(memory_map);
    
    // Initialize heap
    allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("heap initialization failed");
    
    println!("  Heap initialized: {} KB at {:016X}", 
        HEAP_SIZE / 1024, 
        HEAP_START
    );
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
    crate::arch::paging::translate_addr(addr.as_u64(), PHYSICAL_MEMORY_OFFSET)
        .map(|p| PhysAddr::new(p.as_u64()))
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
