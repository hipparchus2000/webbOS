//! ARM64 Memory Management Unit (MMU) and paging

use core::ptr;
use crate::println;

/// Page size for ARM64 (4KB)
pub const PAGE_SIZE: usize = 4096;

/// Number of entries in a page table (512 for 4KB pages with 64-bit entries)
pub const PAGE_TABLE_ENTRIES: usize = 512;

/// Page table entry flags for ARM64
#[repr(u64)]
#[derive(Debug, Clone, Copy)]
pub enum PageTableFlags {
    /// Present/Valid bit
    Valid = 1 << 0,
    /// Table descriptor (vs block descriptor)
    Table = 1 << 1,
    /// Access flag
    AF = 1 << 10,
    /// Not global
    NGNRE = 1 << 11,
    /// User accessible
    User = 1 << 6,
    /// Read/write
    RW = 1 << 7,
    /// Privileged execute never
    PXN = 1 << 53,
    /// Execute never
    XN = 1 << 54,
    /// Device memory (nGnRnE)
    Device = 0,
    /// Normal memory
    Normal = 1 << 2,
    /// Inner shareable
    InnerShareable = 3 << 8,
    /// Outer shareable
    OuterShareable = 2 << 8,
}

/// Page table entry
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct PageTableEntry(u64);

impl PageTableEntry {
    /// Create a new empty page table entry
    pub const fn empty() -> Self {
        Self(0)
    }
    
    /// Check if the entry is valid (present)
    pub fn is_valid(&self) -> bool {
        self.0 & PageTableFlags::Valid as u64 != 0
    }
    
    /// Check if this is a table descriptor (points to another page table)
    pub fn is_table(&self) -> bool {
        self.0 & PageTableFlags::Table as u64 != 0
    }
    
    /// Get the physical address from the entry
    pub fn addr(&self) -> u64 {
        self.0 & 0x0000_FFFF_FFFF_F000
    }
    
    /// Set the physical address in the entry
    pub fn set_addr(&mut self, addr: u64) {
        // Clear address bits and set new address
        self.0 = (self.0 & !0x0000_FFFF_FFFF_F000) | (addr & 0x0000_FFFF_FFFF_F000);
    }
    
    /// Set flags on the entry
    pub fn set_flags(&mut self, flags: u64) {
        // Clear flag bits (except address bits)
        self.0 = (self.0 & 0x0000_FFFF_FFFF_F000) | flags;
    }
    
    /// Create a block entry (maps 2MB block)
    pub fn new_block(phys_addr: u64, flags: u64) -> Self {
        let mut entry = Self(phys_addr & 0x0000_FFFF_FFFF_F000);
        entry.0 |= flags;
        // Block descriptor (Table bit = 0)
        entry.0 &= !(PageTableFlags::Table as u64);
        entry.0 |= PageTableFlags::Valid as u64;
        entry.0 |= PageTableFlags::AF as u64; // Set access flag
        entry
    }
    
    /// Create a table entry (points to another page table)
    pub fn new_table(table_addr: u64, flags: u64) -> Self {
        let mut entry = Self(table_addr & 0x0000_FFFF_FFFF_F000);
        entry.0 |= flags;
        entry.0 |= PageTableFlags::Table as u64;
        entry.0 |= PageTableFlags::Valid as u64;
        entry.0 |= PageTableFlags::AF as u64; // Set access flag
        entry
    }
}

/// Page table structure
#[repr(align(4096))]
pub struct PageTable {
    entries: [PageTableEntry; PAGE_TABLE_ENTRIES],
}

impl PageTable {
    /// Create a new empty page table
    pub const fn new() -> Self {
        Self {
            entries: [PageTableEntry::empty(); PAGE_TABLE_ENTRIES],
        }
    }
    
    /// Get a mutable reference to an entry
    pub fn entry_mut(&mut self, index: usize) -> &mut PageTableEntry {
        &mut self.entries[index]
    }
    
    /// Get a reference to an entry
    pub fn entry(&self, index: usize) -> &PageTableEntry {
        &self.entries[index]
    }
    
    /// Clear all entries
    pub fn clear(&mut self) {
        for entry in self.entries.iter_mut() {
            *entry = PageTableEntry::empty();
        }
    }
}

/// Translation table base register 0 (TTBR0) - for user space
pub struct TTBR0;

impl TTBR0 {
    /// Write to TTBR0_EL1
    pub unsafe fn write(addr: u64) {
        core::arch::asm!(
            "msr ttbr0_el1, {}",
            in(reg) addr,
            options(nomem, nostack)
        );
    }
    
    /// Read from TTBR0_EL1
    pub unsafe fn read() -> u64 {
        let value: u64;
        core::arch::asm!(
            "mrs {}, ttbr0_el1",
            out(reg) value,
            options(nomem, nostack)
        );
        value
    }
}

/// Translation table base register 1 (TTBR1) - for kernel space
pub struct TTBR1;

impl TTBR1 {
    /// Write to TTBR1_EL1
    pub unsafe fn write(addr: u64) {
        core::arch::asm!(
            "msr ttbr1_el1, {}",
            in(reg) addr,
            options(nomem, nostack)
        );
    }
    
    /// Read from TTBR1_EL1
    pub unsafe fn read() -> u64 {
        let value: u64;
        core::arch::asm!(
            "mrs {}, ttbr1_el1",
            out(reg) value,
            options(nomem, nostack)
        );
        value
    }
}

/// Initialize the MMU
pub unsafe fn init() {
    println!("[MMU] Initializing ARM64 MMU...");
    
    // Create page tables
    let ttbr1_addr = create_kernel_page_tables();
    
    // Set TTBR1 (kernel space)
    TTBR1::write(ttbr1_addr);
    
    // Configure TCR_EL1 (Translation Control Register)
    configure_tcr();
    
    // Configure MAIR_EL1 (Memory Attribute Indirection Register)
    configure_mair();
    
    // Enable MMU
    enable_mmu();
    
    println!("[MMU] MMU enabled");
}

/// Create kernel page tables
unsafe fn create_kernel_page_tables() -> u64 {
    // Allocate page tables
    // For simplicity, we'll use static allocation for now
    static mut L0_TABLE: PageTable = PageTable::new();
    static mut L1_TABLE: PageTable = PageTable::new();
    static mut L2_TABLE: PageTable = PageTable::new();
    
    // Clear tables
    L0_TABLE.clear();
    L1_TABLE.clear();
    L2_TABLE.clear();
    
    // Kernel virtual base (0xFFFF800000000000)
    // We need to map this to physical memory
    
    // For now, create a simple 1:1 mapping of the first 2MB
    let flags = PageTableFlags::Valid as u64
        | PageTableFlags::AF as u64
        | PageTableFlags::RW as u64
        | PageTableFlags::Normal as u64
        | PageTableFlags::InnerShareable as u64;
    
    // Create a 2MB block mapping at physical address 0
    L2_TABLE.entry_mut(0).set_flags(flags);
    L2_TABLE.entry_mut(0).set_addr(0);
    
    // Link L1 to L2
    let l2_addr = &L2_TABLE as *const _ as u64;
    L1_TABLE.entry_mut(0).set_flags(PageTableFlags::Table as u64 | PageTableFlags::Valid as u64);
    L1_TABLE.entry_mut(0).set_addr(l2_addr);
    
    // Link L0 to L1
    let l1_addr = &L1_TABLE as *const _ as u64;
    L0_TABLE.entry_mut(0).set_flags(PageTableFlags::Table as u64 | PageTableFlags::Valid as u64);
    L0_TABLE.entry_mut(0).set_addr(l1_addr);
    
    // Return address of L0 table
    &L0_TABLE as *const _ as u64
}

/// Configure TCR_EL1 (Translation Control Register)
unsafe fn configure_tcr() {
    let mut tcr: u64;
    
    // Read current TCR
    core::arch::asm!(
        "mrs {}, tcr_el1",
        out(reg) tcr,
        options(nomem, nostack)
    );
    
    // Configure for 48-bit address space with 4KB granules
    // T0SZ = 64 - 48 = 16 (bits [5:0])
    // T1SZ = 64 - 48 = 16 (bits [21:16])
    // TG0 = 0 (4KB granules for TTBR0)
    // TG1 = 2 (4KB granules for TTBR1)
    // SH0 = 3 (Inner Shareable)
    // SH1 = 3 (Inner Shareable)
    // ORGN0 = 1 (Normal Outer Write-Back Read-Allocate Write-Allocate)
    // ORGN1 = 1
    // IRGN0 = 1 (Normal Inner Write-Back Read-Allocate Write-Allocate)
    // IRGN1 = 1
    // EPD0 = 0 (Use TTBR0)
    // EPD1 = 0 (Use TTBR1)
    // IPS = 2 (40-bit physical address)
    
    tcr = 0;
    tcr |= 16 << 0;    // T0SZ = 16
    tcr |= 16 << 16;   // T1SZ = 16
    tcr |= 0b10 << 30; // TG1 = 2 (4KB)
    tcr |= 0b11 << 12; // SH0 = 3
    tcr |= 0b11 << 28; // SH1 = 3
    tcr |= 0b01 << 10; // ORGN0 = 1
    tcr |= 0b01 << 26; // ORGN1 = 1
    tcr |= 0b01 << 8;  // IRGN0 = 1
    tcr |= 0b01 << 24; // IRGN1 = 1
    tcr |= 0b010 << 32; // IPS = 2 (40-bit)
    
    core::arch::asm!(
        "msr tcr_el1, {}",
        in(reg) tcr,
        options(nomem, nostack)
    );
}

/// Configure MAIR_EL1 (Memory Attribute Indirection Register)
unsafe fn configure_mair() {
    // MAIR format:
    // - Attr0: Device memory (nGnRnE)
    // - Attr1: Normal memory (Write-Back, Read/Write Allocate)
    
    let mair: u64 = 0;
    let mair = mair
        | (0x00 << 0)   // Attr0: Device nGnRnE
        | (0xFF << 8);  // Attr1: Normal Write-Back
    
    core::arch::asm!(
        "msr mair_el1, {}",
        in(reg) mair,
        options(nomem, nostack)
    );
}

/// Enable the MMU
unsafe fn enable_mmu() {
    let mut sctlr: u64;
    
    // Read SCTLR_EL1
    core::arch::asm!(
        "mrs {}, sctlr_el1",
        out(reg) sctlr,
        options(nomem, nostack)
    );
    
    // Enable MMU (M bit = 1)
    sctlr |= 1 << 0;
    
    // Enable instruction cache (I bit = 1)
    sctlr |= 1 << 12;
    
    // Enable data cache (C bit = 1)
    sctlr |= 1 << 2;
    
    // Write back SCTLR_EL1
    core::arch::asm!(
        "msr sctlr_el1, {}",
        in(reg) sctlr,
        options(nomem, nostack)
    );
    
    // Ensure all changes are visible
    core::arch::asm!("isb", options(nomem, nostack));
}

/// Map a virtual address to a physical address
pub unsafe fn map_page(virt_addr: u64, phys_addr: u64, flags: u64) {
    // TODO: Implement page mapping
    // This would walk the page tables and set up the mapping
    println!("[MMU] Mapping {:016x} -> {:016x}", virt_addr, phys_addr);
}

/// Unmap a virtual address
pub unsafe fn unmap_page(virt_addr: u64) {
    // TODO: Implement page unmapping
    println!("[MMU] Unmapping {:016x}", virt_addr);
}

/// Get physical address from virtual address
pub unsafe fn virt_to_phys(virt_addr: u64) -> Option<u64> {
    // TODO: Implement address translation
    // For now, assume 1:1 mapping for kernel
    Some(virt_addr)
}

/// Initialize heap memory
pub fn init_heap() {
    // TODO: Implement heap initialization
    println!("[MMU] Heap initialization not yet implemented for ARM64");
}