//! Paging setup for x86_64
//!
//! Sets up page tables to transition to long mode and map the kernel
//! into higher half virtual memory.

use crate::memory::alloc_pages;
use webbos_shared::types::PhysAddr;

/// Page table entry flags
pub mod flags {
    pub const PRESENT: u64 = 1 << 0;
    pub const WRITABLE: u64 = 1 << 1;
    pub const USER: u64 = 1 << 2;
    pub const WRITE_THROUGH: u64 = 1 << 3;
    pub const CACHE_DISABLE: u64 = 1 << 4;
    pub const ACCESSED: u64 = 1 << 5;
    pub const DIRTY: u64 = 1 << 6;
    pub const HUGE_PAGE: u64 = 1 << 7;
    pub const GLOBAL: u64 = 1 << 8;
    pub const NX: u64 = 1 << 63;
}

/// Page table level
#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum PageTableLevel {
    Pml4 = 4,
    Pdpt = 3,
    Pd = 2,
    Pt = 1,
}

/// Page table entry
#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct PageTableEntry(u64);

impl PageTableEntry {
    /// Create a new page table entry
    pub const fn new() -> Self {
        Self(0)
    }

    /// Get the physical address this entry points to
    pub fn addr(&self) -> PhysAddr {
        PhysAddr::new(self.0 & 0x000F_FFFF_FFFF_F000)
    }

    /// Set the physical address
    pub fn set_addr(&mut self, addr: PhysAddr, flags: u64) {
        self.0 = (addr.as_u64() & 0x000F_FFFF_FFFF_F000) | flags;
    }

    /// Check if entry is present
    pub fn is_present(&self) -> bool {
        (self.0 & flags::PRESENT) != 0
    }

    /// Check if entry is writable
    pub fn is_writable(&self) -> bool {
        (self.0 & flags::WRITABLE) != 0
    }

    /// Check if huge page
    pub fn is_huge_page(&self) -> bool {
        (self.0 & flags::HUGE_PAGE) != 0
    }
}

/// Page table (512 entries)
#[repr(C, align(4096))]
pub struct PageTable {
    entries: [PageTableEntry; 512],
}

impl PageTable {
    /// Create a new empty page table
    pub const fn new() -> Self {
        Self {
            entries: [PageTableEntry::new(); 512],
        }
    }

    /// Get entry at index
    pub fn get_entry(&self, index: usize) -> &PageTableEntry {
        &self.entries[index]
    }

    /// Get mutable entry at index
    pub fn get_entry_mut(&mut self, index: usize) -> &mut PageTableEntry {
        &mut self.entries[index]
    }

    /// Get physical address of this page table
    pub fn as_phys_addr(&self) -> PhysAddr {
        PhysAddr::new(self as *const _ as u64)
    }
}

/// Page table manager
pub struct PageTableManager {
    pml4: &'static mut PageTable,
}

impl PageTableManager {
    /// Create new page table manager from PML4 address
    pub unsafe fn new(pml4_addr: PhysAddr) -> Self {
        let pml4 = &mut *(pml4_addr.as_mut_ptr::<PageTable>());
        Self { pml4 }
    }

    /// Map a virtual page to a physical frame
    pub fn map_page(
        &mut self,
        virt: u64,
        phys: PhysAddr,
        flags: u64,
    ) -> uefi::Result<(), ()> {
        let pml4_index = ((virt >> 39) & 0x1FF) as usize;
        let pdpt_index = ((virt >> 30) & 0x1FF) as usize;
        let pd_index = ((virt >> 21) & 0x1FF) as usize;
        let pt_index = ((virt >> 12) & 0x1FF) as usize;

        // Get or create PDPT
        let pdpt = self.get_or_create_next_level(self.pml4, pml4_index)?;
        
        // Get or create PD
        let pd = self.get_or_create_next_level(pdpt, pdpt_index)?;
        
        // Get or create PT
        let pt = self.get_or_create_next_level(pd, pd_index)?;
        
        // Set page table entry
        let entry = pt.get_entry_mut(pt_index);
        entry.set_addr(phys, flags | flags::PRESENT);
        
        Ok(())
    }

    /// Map a large page (2MB)
    pub fn map_large_page(
        &mut self,
        virt: u64,
        phys: PhysAddr,
        flags: u64,
    ) -> uefi::Result<(), ()> {
        let pml4_index = ((virt >> 39) & 0x1FF) as usize;
        let pdpt_index = ((virt >> 30) & 0x1FF) as usize;
        let pd_index = ((virt >> 21) & 0x1FF) as usize;

        // Get or create PDPT
        let pdpt = self.get_or_create_next_level(self.pml4, pml4_index)?;
        
        // Get or create PD
        let pd = self.get_or_create_next_level(pdpt, pdpt_index)?;
        
        // Set page directory entry as huge page
        let entry = pd.get_entry_mut(pd_index);
        entry.set_addr(phys, flags | flags::PRESENT | flags::HUGE_PAGE);
        
        Ok(())
    }

    /// Get or create the next level page table
    fn get_or_create_next_level(
        &self,
        table: &PageTable,
        index: usize,
    ) -> uefi::Result<&'static mut PageTable, ()> {
        let entry = table.get_entry(index);
        
        if entry.is_present() {
            // Table already exists
            let addr = entry.addr();
            Ok(unsafe { &mut *(addr.as_mut_ptr::<PageTable>()) })
        } else {
            // Allocate new table
            let new_table = allocate_page_table()?;
            let phys_addr = PhysAddr::new(new_table as *mut _ as u64);
            
            // Set entry to point to new table
            unsafe {
                let table_ptr = table as *const _ as *mut PageTable;
                (*table_ptr).get_entry_mut(index).set_addr(
                    phys_addr,
                    flags::PRESENT | flags::WRITABLE,
                );
            }
            
            Ok(new_table)
        }
    }

    /// Get PML4 physical address
    pub fn pml4_addr(&self) -> PhysAddr {
        PhysAddr::new(self.pml4 as *const _ as u64)
    }
}

/// Allocate a new page table
fn allocate_page_table() -> uefi::Result<&'static mut PageTable, ()> {
    let phys_addr = alloc_pages(1)?;
    
    unsafe {
        core::ptr::write_bytes(phys_addr.as_mut_ptr::<u8>(), 0, 0x1000);
        Ok(&mut *(phys_addr.as_mut_ptr::<PageTable>()))
    }
}

/// Setup kernel paging
/// 
/// This creates page tables that map:
/// - Identity mapping for first 4MB (contains kernel and VGA)
/// - Identity mapping for bootloader code region
/// - Higher half mapping for kernel at 0xFFFF_8000_0000_0000
/// 
/// The kernel has three segments that need to be mapped:
/// - 0xFFFF_8000_0010_0000 -> 0x100000 (rodata)
/// - 0xFFFF_8000_0012_14f0 -> 0x1214f0 (text/code - entry point)
/// - 0xFFFF_8000_0022_a3dd -> 0x22a3dd (data)
/// 
/// We map the entire region from 0xFFFF_8000_0010_0000 to cover all segments
pub fn setup_kernel_paging(_kernel_size: usize) -> uefi::Result<PhysAddr, ()> {
    // Allocate PML4
    let pml4 = allocate_page_table()?;
    
    unsafe {
        let mut manager = PageTableManager::new(PhysAddr::new(pml4 as *mut _ as u64));
        
        // Map first 8MB at identity (0x000000-0x800000)
        // This includes:
        // - VGA buffer at 0xB8000
        // - Kernel at 0x100000-0x300000
        // - Stack at 0x500000
        for i in 0..4u64 {
            manager.map_large_page(
                i * 0x200000,
                PhysAddr::new(i * 0x200000),
                flags::PRESENT | flags::WRITABLE,
            )?;
        }
        
        // Map VGA buffer at higher half (0xFFFF8000000B8000 -> 0xB8000)
        // Use 4KB page since VGA buffer is not 2MB aligned
        manager.map_page(
            0xFFFF_8000_000B_8000,
            PhysAddr::new(0xB8000),
            flags::PRESENT | flags::WRITABLE,
        )?;
        
        // Map higher half kernel region (0xFFFF800000100000 -> 0x100000)
        // Kernel is at 0xFFFF800000100000, needs to be mapped with 4KB pages
        // because it's not 2MB aligned. Map 4MB to cover the kernel.
        for i in 0..1024u64 { // 1024 * 4KB = 4MB
            let phys_addr = 0x100000 + i * 0x1000;
            let virt_addr = 0xFFFF_8000_0010_0000 + i * 0x1000;
            manager.map_page(
                virt_addr,
                PhysAddr::new(phys_addr),
                flags::PRESENT | flags::WRITABLE,
            )?;
        }
        
        // Map kernel stack at 0xFFFF_8000_0050_0000 (5MB in higher half)
        // The kernel expects the stack at this virtual address
        // Stack is 128KB, map it with 4KB pages for flexibility
        // Physical stack is allocated at 0x500000 (5MB physical)
        for i in 0..32u64 { // 32 * 4KB = 128KB
            let phys_addr = 0x500000 + i * 0x1000;
            let virt_addr = 0xFFFF_8000_0050_0000 + i * 0x1000;
            manager.map_page(
                virt_addr,
                PhysAddr::new(phys_addr),
                flags::PRESENT | flags::WRITABLE,
            )?;
        }
        
        // Map a large region of physical memory to higher half
        // This covers 0-512MB mapped at 0xFFFF800000000000
        // Use 2MB large pages for efficiency
        for i in 0..256u64 { // 256 * 2MB = 512MB
            let phys = i * 0x200000;
            let virt = 0xFFFF_8000_0000_0000 + phys;
            manager.map_large_page(
                virt,
                PhysAddr::new(phys),
                flags::PRESENT | flags::WRITABLE,
            )?;
        }
        
        // Also identity map the same 512MB region
        // This ensures the bootloader can continue executing after page table switch
        for i in 0..256u64 {
            let phys = i * 0x200000;
            manager.map_large_page(
                phys,
                PhysAddr::new(phys),
                flags::PRESENT | flags::WRITABLE,
            )?;
        }
        
        // Map framebuffer at 0x80000000 (2GB) - used by QEMU for VESA
        // Map just one 2MB page for now
        manager.map_large_page(
            0xFFFF_8000_8000_0000u64,  // Virtual: 0xFFFF800080000000
            PhysAddr::new(0x80000000),  // Physical: 0x80000000
            flags::PRESENT | flags::WRITABLE,
        )?;
        
        Ok(manager.pml4_addr())
    }
}
