//! Paging implementation

use webbos_shared::types::{PhysAddr, PAGE_SIZE};

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
    pub fn set_addr(&mut self, addr: PhysAddr, flags: PageTableFlags) {
        self.0 = (addr.as_u64() & 0x000F_FFFF_FFFF_F000) | flags.bits();
    }

    /// Check if entry is present
    pub fn is_present(&self) -> bool {
        (self.0 & 1) != 0
    }

    /// Check if entry is writable
    pub fn is_writable(&self) -> bool {
        (self.0 & 2) != 0
    }

    /// Check if huge page
    pub fn is_huge_page(&self) -> bool {
        (self.0 & 0x80) != 0
    }
}

/// Page table flags
#[derive(Clone, Copy, Debug)]
pub struct PageTableFlags(u64);

impl PageTableFlags {
    /// Present flag
    pub const PRESENT: Self = Self(1 << 0);
    /// Writable flag
    pub const WRITABLE: Self = Self(1 << 1);
    /// User accessible flag
    pub const USER: Self = Self(1 << 2);
    /// Write through flag
    pub const WRITE_THROUGH: Self = Self(1 << 3);
    /// No cache flag
    pub const NO_CACHE: Self = Self(1 << 4);
    /// Accessed flag
    pub const ACCESSED: Self = Self(1 << 5);
    /// Dirty flag
    pub const DIRTY: Self = Self(1 << 6);
    /// Huge page flag
    pub const HUGE_PAGE: Self = Self(1 << 7);
    /// Global flag
    pub const GLOBAL: Self = Self(1 << 8);
    /// No execute flag (bit 63)
    pub const NO_EXECUTE: Self = Self(1 << 63);

    /// Empty flags
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Combine flags
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Get raw bits
    pub fn bits(&self) -> u64 {
        self.0
    }
}

impl core::ops::BitOr for PageTableFlags {
    type Output = Self;
    
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
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
}

/// Physical frame
#[derive(Clone, Copy, Debug)]
pub struct PhysFrame {
    addr: PhysAddr,
}

impl PhysFrame {
    /// Create a frame containing the given address
    pub fn containing_address(addr: PhysAddr) -> Self {
        Self {
            addr: addr.align_down(),
        }
    }

    /// Get frame start address
    pub fn start_address(&self) -> PhysAddr {
        self.addr
    }
}

/// A FrameAllocator that returns usable frames from the bootloader's memory map.
pub struct BootInfoFrameAllocator {
    memory_map: &'static [webbos_shared::types::MemoryRegion],
    next: usize,
}

impl BootInfoFrameAllocator {
    /// Create a FrameAllocator from the passed memory map.
    /// 
    /// # Safety
    /// This function is unsafe because the caller must guarantee that the passed
    /// memory map is valid. The main requirement is that all frames that are marked
    /// as `USABLE` in it are really unused.
    pub unsafe fn init(memory_map: &'static [webbos_shared::types::MemoryRegion]) -> Self {
        BootInfoFrameAllocator {
            memory_map,
            next: 0,
        }
    }

    /// Returns an iterator over the usable frames specified in the memory map.
    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> + '_ {
        self.memory_map
            .iter()
            .filter(|r| matches!(r.region_type, webbos_shared::types::MemoryRegionType::Available))
            .flat_map(|r| {
                let start = r.base.as_u64();
                let end = start + r.size.as_u64();
                (start..end).step_by(PAGE_SIZE).map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
            })
    }

    /// Allocate a frame
    pub fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame
    }
}

/// Mapper error
#[derive(Debug)]
pub enum MapToError {
    /// Frame allocation failed
    FrameAllocationFailed,
    /// Parent entry is huge page
    ParentEntryHugePage,
    /// Page already mapped
    PageAlreadyMapped,
}

/// Offset page table
pub struct OffsetPageTable {
    level_4_table: &'static mut PageTable,
    phys_offset: u64,
}

impl OffsetPageTable {
    /// Create a new OffsetPageTable
    /// 
    /// # Safety
    /// Caller must ensure the level_4_table is valid
    pub unsafe fn new(level_4_table: &'static mut PageTable, phys_offset: u64) -> Self {
        Self {
            level_4_table,
            phys_offset,
        }
    }

    /// Map a page to a frame
    pub unsafe fn map_to(
        &mut self,
        page: Page,
        frame: PhysFrame,
        flags: PageTableFlags,
        allocator: &mut BootInfoFrameAllocator,
    ) -> Result<(), MapToError> {
        let p4_index = page.p4_index();
        let p3_index = page.p3_index();
        let p2_index = page.p2_index();
        let p1_index = page.p1_index();

        // Get or create PDPT
        let p3 = self.get_or_create_next_level(self.level_4_table, p4_index, allocator)?;
        
        // Get or create PD
        let p2 = self.get_or_create_next_level(p3, p3_index, allocator)?;
        
        // Get or create PT
        let p1 = self.get_or_create_next_level(p2, p2_index, allocator)?;
        
        // Set page table entry
        let entry = p1.get_entry_mut(p1_index);
        if entry.is_present() {
            return Err(MapToError::PageAlreadyMapped);
        }
        entry.set_addr(frame.start_address(), flags | PageTableFlags::PRESENT);
        
        Ok(())
    }

    /// Get or create the next level page table
    fn get_or_create_next_level(
        &self,
        table: &PageTable,
        index: usize,
        allocator: &mut BootInfoFrameAllocator,
    ) -> Result<&'static mut PageTable, MapToError> {
        let entry = table.get_entry(index);
        
        if entry.is_present() {
            if entry.is_huge_page() {
                return Err(MapToError::ParentEntryHugePage);
            }
            let addr = entry.addr();
            let virt = addr.as_u64() + self.phys_offset;
            Ok(unsafe { &mut *(virt as *mut PageTable) })
        } else {
            // Allocate new table
            let frame = allocator.allocate_frame().ok_or(MapToError::FrameAllocationFailed)?;
            let phys_addr = frame.start_address();
            let virt_addr = phys_addr.as_u64() + self.phys_offset;
            
            // Zero the new table
            unsafe {
                core::ptr::write_bytes(virt_addr as *mut u8, 0, PAGE_SIZE);
            }
            
            // Set entry to point to new table using raw pointer
            unsafe {
                let table_ptr = table as *const PageTable as *mut PageTable;
                (*core::ptr::addr_of_mut!((*table_ptr).entries[index])).set_addr(
                    phys_addr,
                    PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
                );
            }
            
            Ok(unsafe { &mut *(virt_addr as *mut PageTable) })
        }
    }
}

/// Virtual page
#[derive(Clone, Copy, Debug)]
pub struct Page {
    addr: u64,
}

impl Page {
    /// Create a page containing the given address
    pub fn containing_address(addr: u64) -> Self {
        Self {
            addr: addr & !0xFFF,
        }
    }

    /// Get page address
    pub fn addr(&self) -> u64 {
        self.addr
    }

    /// Get P4 index
    fn p4_index(&self) -> usize {
        ((self.addr >> 39) & 0x1FF) as usize
    }

    /// Get P3 index
    fn p3_index(&self) -> usize {
        ((self.addr >> 30) & 0x1FF) as usize
    }

    /// Get P2 index
    fn p2_index(&self) -> usize {
        ((self.addr >> 21) & 0x1FF) as usize
    }

    /// Get P1 index
    fn p1_index(&self) -> usize {
        ((self.addr >> 12) & 0x1FF) as usize
    }
}

/// Initialize paging
/// 
/// # Safety
/// Must be called only once during kernel initialization
pub unsafe fn init(physical_memory_offset: u64) -> OffsetPageTable {
    let level_4_table = active_level_4_table(physical_memory_offset);
    OffsetPageTable::new(level_4_table, physical_memory_offset)
}

/// Get the active level 4 page table
/// 
/// # Safety
/// Caller must ensure the physical memory offset is valid
unsafe fn active_level_4_table(physical_memory_offset: u64) -> &'static mut PageTable {
    // Read the active level 4 frame from the CR3 register
    let cr3: u64;
    core::arch::asm!(
        "mov {}, cr3",
        out(reg) cr3,
        options(nomem, nostack)
    );
    
    let phys_addr = cr3 & 0x000F_FFFF_FFFF_F000;
    let virt_addr = phys_addr + physical_memory_offset;

    &mut *(virt_addr as *mut PageTable)
}

/// Translate a virtual address to a physical address
pub fn translate_addr(addr: u64, physical_memory_offset: u64) -> Option<PhysAddr> {
    translate_addr_inner(addr, physical_memory_offset)
}

fn translate_addr_inner(addr: u64, physical_memory_offset: u64) -> Option<PhysAddr> {
    // Read the active level 4 frame from the CR3 register
    let cr3: u64;
    unsafe {
        core::arch::asm!(
            "mov {}, cr3",
            out(reg) cr3,
            options(nomem, nostack)
        );
    }
    
    let phys_addr = cr3 & 0x000F_FFFF_FFFF_F000;
    let virt_addr = phys_addr + physical_memory_offset;

    let table_indexes = [
        ((addr >> 39) & 0x1FF) as usize,
        ((addr >> 30) & 0x1FF) as usize,
        ((addr >> 21) & 0x1FF) as usize,
        ((addr >> 12) & 0x1FF) as usize,
    ];

    let mut table_virt_addr = virt_addr;

    for &index in &table_indexes {
        let table = unsafe { &*(table_virt_addr as *const PageTable) };
        let entry = table.get_entry(index);
        
        if !entry.is_present() {
            return None;
        }
        
        if entry.is_huge_page() {
            panic!("huge pages not supported in translation");
        }
        
        // Convert next table's physical address to virtual
        let next_phys = entry.addr().as_u64();
        table_virt_addr = next_phys + physical_memory_offset;
    }

    // Get the physical address from the final frame
    let frame_phys = table_virt_addr - physical_memory_offset;
    // Calculate the physical address by adding the page offset
    Some(PhysAddr::new(frame_phys + (addr & 0xFFF)))
}
