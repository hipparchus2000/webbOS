//! MMU (Memory Management Unit) for ARM64
//!
//! ARM64 uses a 4-level page table structure with 4KB pages:
//! - Level 0 (PGD): 512 entries, 512GB each
//! - Level 1 (PUD): 512 entries, 1GB each (can be 1GB block)
//! - Level 2 (PMD): 512 entries, 2MB each (can be 2MB block)
//! - Level 3 (PTE): 512 entries, 4KB each (must be page)

use webbos_shared::types::{PhysAddr, PAGE_SIZE};

/// Page table entry flags
#[derive(Clone, Copy, Debug)]
pub struct PageTableFlags(u64);

impl PageTableFlags {
    /// Valid entry
    pub const VALID: Self = Self(1 << 0);
    /// Page or table descriptor
    pub const TABLE: Self = Self(1 << 1);
    /// Memory attributes index (MAIR)
    pub const ATTR_INDEX_0: Self = Self(0b000 << 2); // Device
    pub const ATTR_INDEX_1: Self = Self(0b111 << 2); // Normal WB
    pub const ATTR_INDEX_2: Self = Self(0b001 << 2); // Normal NC
    /// Non-secure
    pub const NS: Self = Self(1 << 5);
    /// Access permissions
    pub const AP_RW: Self = Self(0b00 << 6);
    pub const AP_RO: Self = Self(0b10 << 6);
    pub const AP_USER: Self = Self(0b01 << 6);
    /// Shareability
    pub const SH_NONE: Self = Self(0b00 << 8);
    pub const SH_OUTER: Self = Self(0b10 << 8);
    pub const SH_INNER: Self = Self(0b11 << 8);
    /// Access flag
    pub const AF: Self = Self(1 << 10);
    /// Not global
    pub const NG: Self = Self(1 << 11);
    /// Contiguous hint
    pub const CONTIGUOUS: Self = Self(1 << 52);
    /// Privileged execute never
    pub const PXN: Self = Self(1 << 53);
    /// Execute never (EL0)
    pub const UXN: Self = Self(1 << 54);

    /// Empty flags
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Get raw bits
    pub const fn bits(&self) -> u64 {
        self.0
    }
}

impl core::ops::BitOr for PageTableFlags {
    type Output = Self;
    
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

/// Page table entry
#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct PageTableEntry(u64);

impl PageTableEntry {
    /// Create a new empty entry
    pub const fn new() -> Self {
        Self(0)
    }

    /// Get the physical address this entry points to
    pub fn addr(&self) -> PhysAddr {
        PhysAddr::new(self.0 & 0x0000_FFFF_FFFF_F000)
    }

    /// Set the physical address and flags
    pub fn set(&mut self, addr: PhysAddr, flags: PageTableFlags) {
        self.0 = (addr.as_u64() & 0x0000_FFFF_FFFF_F000) | flags.bits();
    }

    /// Check if entry is valid
    pub fn is_valid(&self) -> bool {
        (self.0 & 1) != 0
    }

    /// Check if this is a table entry
    pub fn is_table(&self) -> bool {
        (self.0 & 2) != 0
    }

    /// Check if this is a block entry
    pub fn is_block(&self) -> bool {
        self.is_valid() && !self.is_table()
    }
}

/// Page table (512 entries for 4KB granule)
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
    pub fn entry(&self, index: usize) -> &PageTableEntry {
        &self.entries[index]
    }

    /// Get mutable entry at index
    pub fn entry_mut(&mut self, index: usize) -> &mut PageTableEntry {
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

    /// Get PGD (Level 0) index
    pub fn l0_index(&self) -> usize {
        ((self.addr >> 39) & 0x1FF) as usize
    }

    /// Get PUD (Level 1) index
    pub fn l1_index(&self) -> usize {
        ((self.addr >> 30) & 0x1FF) as usize
    }

    /// Get PMD (Level 2) index
    pub fn l2_index(&self) -> usize {
        ((self.addr >> 21) & 0x1FF) as usize
    }

    /// Get PTE (Level 3) index
    pub fn l3_index(&self) -> usize {
        ((self.addr >> 12) & 0x1FF) as usize
    }
}

/// A FrameAllocator that returns usable frames from the bootloader's memory map.
pub struct BootInfoFrameAllocator {
    memory_map: &'static [webbos_shared::types::MemoryRegion],
    next: usize,
}

impl BootInfoFrameAllocator {
    /// Create a FrameAllocator from the passed memory map.
    pub unsafe fn init(memory_map: &'static [webbos_shared::types::MemoryRegion]) -> Self {
        BootInfoFrameAllocator {
            memory_map,
            next: 0,
        }
    }

    /// Returns an iterator over the usable frames
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
    FrameAllocationFailed,
    ParentEntryHugePage,
    PageAlreadyMapped,
}

/// Offset page table
pub struct OffsetPageTable {
    level_0_table: &'static mut PageTable,
    phys_offset: u64,
}

impl OffsetPageTable {
    /// Create a new OffsetPageTable
    pub unsafe fn new(level_0_table: &'static mut PageTable, phys_offset: u64) -> Self {
        Self {
            level_0_table,
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
        let l0_index = page.l0_index();
        let l1_index = page.l1_index();
        let l2_index = page.l2_index();
        let l3_index = page.l3_index();

        // Get or create PUD
        let l1 = self.get_or_create_next_level(self.level_0_table, l0_index, allocator)?;
        
        // Get or create PMD
        let l2 = self.get_or_create_next_level(l1, l1_index, allocator)?;
        
        // Get or create PTE
        let l3 = self.get_or_create_next_level(l2, l2_index, allocator)?;
        
        // Set page table entry
        let entry = l3.entry_mut(l3_index);
        if entry.is_valid() {
            return Err(MapToError::PageAlreadyMapped);
        }
        entry.set(frame.start_address(), flags | PageTableFlags::VALID);
        
        Ok(())
    }

    /// Get or create the next level page table
    fn get_or_create_next_level(
        &self,
        table: &PageTable,
        index: usize,
        allocator: &mut BootInfoFrameAllocator,
    ) -> Result<&'static mut PageTable, MapToError> {
        let entry = table.entry(index);
        
        if entry.is_valid() {
            if entry.is_block() {
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
            
            // Set entry to point to new table
            unsafe {
                let table_ptr = table as *const PageTable as *mut PageTable;
                (*core::ptr::addr_of_mut!((*table_ptr).entries[index])).set(
                    phys_addr,
                    PageTableFlags::VALID | PageTableFlags::TABLE,
                );
            }
            
            Ok(unsafe { &mut *(virt_addr as *mut PageTable) })
        }
    }
}

/// Initialize paging
pub unsafe fn init(physical_memory_offset: u64) -> OffsetPageTable {
    let level_0_table = active_level_0_table(physical_memory_offset);
    OffsetPageTable::new(level_0_table, physical_memory_offset)
}

/// Get the active level 0 page table
unsafe fn active_level_0_table(physical_memory_offset: u64) -> &'static mut PageTable {
    // Read TTBR1_EL1 for the kernel page table
    let ttbr1: u64;
    core::arch::asm!(
        "mrs {0}, TTBR1_EL1",
        out(reg) ttbr1,
    );
    
    // Extract physical address (bits 47:12)
    let phys_addr = ttbr1 & 0x0000_FFFF_FFFF_F000;
    let virt_addr = phys_addr + physical_memory_offset;

    &mut *(virt_addr as *mut PageTable)
}

/// Translate a virtual address to a physical address
pub fn translate_addr(addr: u64, physical_memory_offset: u64) -> Option<PhysAddr> {
    translate_addr_inner(addr, physical_memory_offset)
}

fn translate_addr_inner(addr: u64, physical_memory_offset: u64) -> Option<PhysAddr> {
    // Read TTBR1_EL1
    let ttbr1: u64;
    unsafe {
        core::arch::asm!(
            "mrs {0}, TTBR1_EL1",
            out(reg) ttbr1,
        );
    }
    
    let phys_addr = ttbr1 & 0x0000_FFFF_FFFF_F000;
    let virt_addr = phys_addr + physical_memory_offset;

    let table_indexes = [
        ((addr >> 39) & 0x1FF) as usize,
        ((addr >> 30) & 0x1FF) as usize,
        ((addr >> 21) & 0x1FF) as usize,
        ((addr >> 12) & 0x1FF) as usize,
    ];

    let mut table_virt_addr = virt_addr;

    for (level, &index) in table_indexes.iter().enumerate() {
        let table = unsafe { &*(table_virt_addr as *const PageTable) };
        let entry = table.entry(index);
        
        if !entry.is_valid() {
            return None;
        }
        
        if entry.is_block() {
            // Block entry - calculate physical address
            let block_size = match level {
                1 => 0x40000000, // 1GB
                2 => 0x200000,   // 2MB
                _ => panic!("Invalid block level"),
            };
            let block_mask = block_size - 1;
            let block_addr = entry.addr().as_u64();
            return Some(PhysAddr::new(block_addr + (addr & block_mask)));
        }
        
        // Convert next table's physical address to virtual
        let next_phys = entry.addr().as_u64();
        table_virt_addr = next_phys + physical_memory_offset;
    }

    // Get the physical address from the final frame
    let frame_phys = table_virt_addr - physical_memory_offset;
    Some(PhysAddr::new(frame_phys + (addr & 0xFFF)))
}

/// Invalidate TLB entry for address
pub fn invalidate_tlb(addr: u64) {
    unsafe {
        core::arch::asm!(
            "tlbi VAAE1, {0}",
            "dsb ish",
            "isb",
            in(reg) (addr >> 12),
        );
    }
}

/// Invalidate entire TLB
pub fn invalidate_tlb_all() {
    unsafe {
        core::arch::asm!(
            "tlbi VMALLE1",
            "dsb ish",
            "isb",
        );
    }
}
