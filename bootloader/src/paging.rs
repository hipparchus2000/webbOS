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
/// - Identity mapping for first 4GB (for bootloader transition)
/// - Higher half mapping for kernel at 0xFFFF_8000_0000_0000
/// 
/// The kernel has three segments that need to be mapped:
/// - 0xFFFF_8000_0010_0000 -> 0x100000 (rodata)
/// - 0xFFFF_8000_0012_14f0 -> 0x1214f0 (text/code - entry point)
/// - 0xFFFF_8000_0022_a3dd -> 0x22a3dd (data)
/// 
/// We map the entire region from 0xFFFF_8000_0010_0000 to cover all segments
pub fn setup_kernel_paging(kernel_size: usize) -> uefi::Result<PhysAddr, ()> {
    // Allocate PML4
    let pml4 = allocate_page_table()?;
    
    unsafe {
        let mut manager = PageTableManager::new(PhysAddr::new(pml4 as *mut _ as u64));
        
        // Identity map first 4GB using large pages (2MB each)
        for i in 0..2048u64 {
            let phys = PhysAddr::new(i * 0x200000);
            let virt = i * 0x200000;
            manager.map_large_page(
                virt,
                phys,
                flags::PRESENT | flags::WRITABLE,
            )?;
        }
        
        // Map kernel segments to higher half
        // Base virtual address: 0xFFFF_8000_0010_0000
        // We need to map this to the actual physical addresses in the ELF
        // The ELF has physical addresses like 0x100000, 0x1214f0, 0x22a3dd
        // We map the range 0xFFFF_8000_0010_0000 -> 0x100000 and so on
        
        // Find the highest physical address we need to map
        let kernel_pages = (kernel_size + 0xFFF) / 0x1000;
        
        // Map from virtual 0xFFFF_8000_0010_0000 to physical 0x100000
        // This covers all segments since they're within ~12MB of 0x100000
        for i in 0..kernel_pages as u64 {
            let phys_addr = 0x100000 + i * 0x1000;
            let virt_addr = 0xFFFF_8000_0010_0000 + i * 0x1000;
            
            // Map as present and writable (no NX so code can execute)
            manager.map_page(
                virt_addr,
                PhysAddr::new(phys_addr),
                flags::PRESENT | flags::WRITABLE,
            )?;
        }
        
        // Map kernel stack at 0xFFFF_8000_0040_0000 (4MB in higher half)
        // Stack is 128KB and grows DOWN from 0xFFFF_8000_0040_0000
        // So we need to map from 0xFFFF_8000_003E_0000 to 0xFFFF_8000_0040_0000
        const STACK_TOP_VIRT: u64 = 0xFFFF_8000_0000_0000 + 0x400000;
        const STACK_PHYS_BASE: u64 = 0x400000 - 0x20000; // 4MB - 128KB
        const STACK_PAGES: u64 = 32; // 128KB stack
        
        for i in 0..STACK_PAGES {
            manager.map_page(
                STACK_TOP_VIRT - (STACK_PAGES - i) * 0x1000,
                PhysAddr::new(STACK_PHYS_BASE + i * 0x1000),
                flags::PRESENT | flags::WRITABLE,
            )?;
        }
        
        Ok(manager.pml4_addr())
    }
}
