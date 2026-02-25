//! ARM64 MMU (Memory Management Unit) Setup
//!
//! Sets up 4-level page tables for AArch64 with 4KB granules.
//! 
//! ARM64 uses:
//! - Level 0: PGD (Page Global Directory) - 512GB per entry
//! - Level 1: PUD (Page Upper Directory) - 1GB per entry  
//! - Level 2: PMD (Page Middle Directory) - 2MB per entry (can be block)
//! - Level 3: PTE (Page Table Entry) - 4KB per entry
//!
//! Virtual address layout (48-bit with 4KB pages):
//! - Bits 47-39: Level 0 index
//! - Bits 38-30: Level 1 index
//! - Bits 29-21: Level 2 index
//! - Bits 20-12: Level 3 index
//! - Bits 11-0: Page offset

use crate::alloc_pages;
use webbos_shared::types::PhysAddr;

/// Page table entry flags
#[allow(dead_code)]
pub mod flags {
    /// Valid entry
    pub const VALID: u64 = 1 << 0;
    /// Page or block (1=page/block, 0=table)
    pub const TABLE: u64 = 1 << 1;
    /// Block entry at level 1/2
    pub const BLOCK: u64 = 1 << 1;
    /// Memory attributes index (MAIR)
    pub const ATTR_INDEX_MASK: u64 = 0b111 << 2;
    /// Non-secure
    pub const NS: u64 = 1 << 5;
    /// Access permission: EL0 no access
    pub const AP_EL0_NONE: u64 = 0b00 << 6;
    /// Access permission: read/write
    pub const AP_RW: u64 = 0b00 << 6;
    /// Access permission: read-only
    pub const AP_RO: u64 = 0b10 << 6;
    /// Shareability: non-shareable
    pub const SH_NONE: u64 = 0b00 << 8;
    /// Shareability: outer shareable
    pub const SH_OUTER: u64 = 0b10 << 8;
    /// Shareability: inner shareable
    pub const SH_INNER: u64 = 0b11 << 8;
    /// Access flag
    pub const AF: u64 = 1 << 10;
    /// Not global (ASID-specific)
    pub const NG: u64 = 1 << 11;
    /// Contiguous hint
    pub const CONTIGUOUS: u64 = 1 << 52;
    /// Privileged execute never
    pub const PXN: u64 = 1 << 53;
    /// Execute never (EL0)
    pub const UXN: u64 = 1 << 54;

    /// Device nGnRnE memory (strongly ordered, non-cacheable)
    pub const ATTR_DEVICE: u64 = 0b000 << 2;
    /// Normal memory, non-cacheable
    pub const ATTR_NORMAL_NC: u64 = 0b001 << 2;
    /// Normal memory, write-through cacheable
    pub const ATTR_NORMAL_WT: u64 = 0b010 << 2;
    /// Normal memory, write-back cacheable (inner and outer)
    pub const ATTR_NORMAL_WB: u64 = 0b111 << 2;
}

/// MAIR (Memory Attribute Indirection Register) values
const MAIR_VALUE: u64 = 
    // Attr0: Device nGnRnE (strongly ordered)
    (0x00 << 0) |
    // Attr1: Normal, inner/outer write-back read/write-allocate
    (0xFF << 8) |
    // Attr2: Normal, inner/outer non-cacheable
    (0x44 << 16);

/// TCR (Translation Control Register) value
/// - T0SZ = 16 (48-bit address space)
/// - TG0 = 0 (4KB granule for TTBR0)
/// - TG1 = 2 (4KB granule for TTBR1)
/// - IPS = 3 (40-bit physical address space)
const TCR_VALUE: u64 = 
    (16 << 0) |      // T0SZ = 16 (2^(64-16) = 2^48 address space)
    (0 << 6) |       // RES0
    (0 << 7) |       // EP0 = 0 (lower EL0/1)
    (0 << 8) |       // SH0 = 0 (non-shareable for TTBR0)
    (0 << 10) |      // ORGN0 = 0 (normal memory, outer non-cacheable)
    (0 << 12) |      // IRGN0 = 0 (normal memory, inner non-cacheable)
    (0 << 14) |      // T1SZ = 16 (same for TTBR1)
    (2 << 30) |      // TG1 = 2 (4KB granule for TTBR1)
    (3 << 32);       // IPS = 3 (40-bit physical address)

/// Page table entry
#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct PageTableEntry(u64);

impl PageTableEntry {
    /// Create a new empty entry
    #[allow(dead_code)]
    pub const fn new() -> Self {
        Self(0)
    }

    /// Get the physical address this entry points to
    #[allow(dead_code)]
    pub fn addr(&self) -> PhysAddr {
        PhysAddr::new(self.0 & 0x0000_FFFF_FFFF_F000)
    }

    /// Set the physical address and flags
    pub fn set(&mut self, addr: PhysAddr, flags: u64) {
        self.0 = (addr.as_u64() & 0x0000_FFFF_FFFF_F000) | flags;
    }

    /// Check if entry is valid
    #[allow(dead_code)]
    pub fn is_valid(&self) -> bool {
        (self.0 & flags::VALID) != 0
    }

    /// Check if this is a table entry (not a block/page)
    #[allow(dead_code)]
    pub fn is_table(&self) -> bool {
        (self.0 & flags::TABLE) == flags::TABLE
    }
}

/// Page table (512 entries for 4KB granule)
#[repr(C, align(4096))]
pub struct PageTable {
    entries: [PageTableEntry; 512],
}

impl PageTable {
    /// Create a new empty page table
    #[allow(dead_code)]
    pub const fn new() -> Self {
        Self {
            entries: [PageTableEntry::new(); 512],
        }
    }

    /// Get entry at index
    #[allow(dead_code)]
    pub fn entry(&self, index: usize) -> &PageTableEntry {
        &self.entries[index]
    }

    /// Get mutable entry at index
    pub fn entry_mut(&mut self, index: usize) -> &mut PageTableEntry {
        &mut self.entries[index]
    }
}

/// Set up page tables for kernel
/// 
/// Creates mappings for:
/// - Identity mapping for first 1GB (bootloader, kernel load address)
/// - Higher half mapping at 0xFFFF_0000_0000_0000 for kernel
pub fn setup_page_tables(kernel_size: usize) -> Result<PhysAddr, ()> {
    // Allocate top-level page table (PGD)
    let pgd = unsafe {
        let addr = alloc_pages(1).ok_or(())?;
        let ptr = addr.as_mut_ptr::<PageTable>();
        core::ptr::write_bytes(ptr as *mut u8, 0, 4096);
        &mut *ptr
    };

    // Map first 1GB with 1GB blocks (identity mapping)
    // This covers 0x0000_0000 to 0x3FFF_FFFF
    // Entry 0 of PGD -> PUD -> 1GB block at level 1
    let pud = allocate_table()?;
    pgd.entry_mut(0).set(pud, flags::VALID | flags::TABLE);

    // Map 0-1GB as device memory (for peripherals)
    // Map 0x40000000-0x80000000 as normal memory
    for i in 0..4 {
        let phys = (i as u64) * 0x40000000; // 1GB blocks
        let flags = if i == 0 {
            // First 1GB - mix of device and normal
            flags::VALID | flags::BLOCK | flags::AF | flags::ATTR_NORMAL_WB | flags::SH_INNER
        } else {
            // Normal memory
            flags::VALID | flags::BLOCK | flags::AF | flags::ATTR_NORMAL_WB | flags::SH_INNER
        };
        
        // Use level 1 block entries (1GB each)
        // For simplicity, we're mapping with 1GB blocks at PUD level
        unsafe {
            let pud_ptr = pud.as_u64() as *mut PageTable;
            (*pud_ptr).entry_mut(i).set(PhysAddr::new(phys), flags);
        }
    }

    // Set up higher half mapping at 0xFFFF_0000_0000_0000
    // This is entry 511 in the PGD (top of address space)
    let kernel_pud = allocate_table()?;
    pgd.entry_mut(511).set(kernel_pud, flags::VALID | flags::TABLE);

    // Map kernel at 0xFFFF_0000_0000_0000 + 0x100000
    // Use 2MB blocks for kernel code/data
    let kernel_pmd = allocate_table()?;
    unsafe {
        let pud_ptr = kernel_pud.as_u64() as *mut PageTable;
        (*pud_ptr).entry_mut(0).set(kernel_pmd, flags::VALID | flags::TABLE);
    }

    // Map kernel with 2MB blocks (simpler than 4KB pages)
    // Kernel is at physical 0x100000, map to virtual 0xFFFF_0000_0010_0000
    let kernel_pages = (kernel_size + 0x1FFFFF) / 0x200000;
    for i in 0..kernel_pages.max(16) { // At least 32MB
        let phys = 0x100000 + (i as u64) * 0x200000;
        let flags = flags::VALID | flags::BLOCK | flags::AF | 
                    flags::ATTR_NORMAL_WB | flags::SH_INNER;
        unsafe {
            let pmd_ptr = kernel_pmd.as_u64() as *mut PageTable;
            (*pmd_ptr).entry_mut(i).set(PhysAddr::new(phys), flags);
        }
    }

    // Map stack area
    let stack_pmd = allocate_table()?;
    unsafe {
        let pud_ptr = kernel_pud.as_u64() as *mut PageTable;
        (*pud_ptr).entry_mut(1).set(stack_pmd, flags::VALID | flags::TABLE);
    }
    // Stack at 0xFFFF_0000_0050_0000 (5MB in higher half)
    for i in 0..64 { // 128MB of stack space
        let phys = 0x500000 + (i as u64) * 0x200000;
        let flags = flags::VALID | flags::BLOCK | flags::AF | 
                    flags::ATTR_NORMAL_WB | flags::SH_INNER;
        unsafe {
            let pmd_ptr = stack_pmd.as_u64() as *mut PageTable;
            (*pmd_ptr).entry_mut(i).set(PhysAddr::new(phys), flags);
        }
    }

    Ok(PhysAddr::new(pgd as *const _ as u64))
}

/// Allocate a new page table
fn allocate_table() -> Result<PhysAddr, ()> {
    unsafe {
        alloc_pages(1).ok_or(())
    }
}

/// Enable MMU and jump to kernel
/// 
/// # Safety
/// This function does not return. It enables the MMU and jumps to the kernel.
pub unsafe fn enable_mmu_and_jump(
    page_tables: PhysAddr,
    boot_info: PhysAddr,
    kernel_entry: u64,
) -> ! {
    // Set up MAIR
    core::arch::asm!(
        "msr MAIR_EL1, {0}",
        in(reg) MAIR_VALUE,
    );

    // Set up TCR
    core::arch::asm!(
        "msr TCR_EL1, {0}",
        in(reg) TCR_VALUE,
    );

    // Set up TTBR0 and TTBR1
    core::arch::asm!(
        "msr TTBR0_EL1, {0}",
        "msr TTBR1_EL1, {0}",
        in(reg) page_tables.as_u64(),
    );

    // Invalidate TLB
    core::arch::asm!(
        "tlbi vmalle1is",
        "dsb ish",
        "isb",
    );

    // Enable MMU
    // SCTLR_EL1 bits:
    // M[0] = 1 (MMU enable)
    // A[1] = 0 (Alignment check disabled)
    // C[2] = 1 (Data cache enable)
    // SA[3] = 0 (Stack alignment check disabled)
    // I[12] = 1 (Instruction cache enable)
    let sctlr_value: u64 = 0x00000001 | (1 << 2) | (1 << 12);
    
    core::arch::asm!(
        "msr SCTLR_EL1, {0}",
        "isb",
        in(reg) sctlr_value,
    );

    // Jump to kernel
    // x0 = boot_info pointer
    let entry_fn: extern "C" fn(u64) -> ! = core::mem::transmute(kernel_entry);
    entry_fn(boot_info.as_u64());
}
