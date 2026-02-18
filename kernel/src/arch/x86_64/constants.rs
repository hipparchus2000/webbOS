//! x86_64 architecture constants

/// Serial port (COM1) constants
pub mod serial {
    /// COM1 base I/O port
    pub const COM1_PORT: u16 = 0x3F8;
    
    /// COM1 data port (offset 0)
    pub const DATA_PORT: u16 = COM1_PORT;
    
    /// ASCII characters for boot messages
    pub mod ascii {
        pub const K: u8 = b'K';
        pub const E: u8 = b'E';
        pub const R: u8 = b'R';
        pub const N: u8 = b'N';
        pub const L: u8 = b'L';
        pub const X: u8 = b'X';
        pub const O: u8 = b'O';
        pub const P: u8 = b'P';
        pub const ZERO: u8 = b'0';
        pub const ONE: u8 = b'1';
        pub const TWO: u8 = b'2';
        pub const THREE: u8 = b'3';
        pub const FOUR: u8 = b'4';
        pub const FIVE: u8 = b'5';
        pub const SIX: u8 = b'6';
        pub const EXCLAMATION: u8 = b'!';
    }
}

/// Memory layout constants
pub mod memory {
    /// Physical address where kernel is loaded (1MB)
    pub const KERNEL_PHYS_BASE: u64 = 0x100000;
    
    /// Physical stack top address (5MB)
    pub const PHYS_STACK_TOP: u64 = 0x500000;
    
    /// Higher half kernel virtual base address
    pub const KERNEL_VIRT_BASE: u64 = 0xFFFF_8000_0000_0000;
    
    /// Virtual stack top address in higher half
    pub const VIRT_STACK_TOP: u64 = KERNEL_VIRT_BASE + PHYS_STACK_TOP;
    
    /// Page size (4KB)
    pub const PAGE_SIZE: u64 = 4096;
    
    /// Size of a large page (2MB)
    pub const LARGE_PAGE_SIZE: u64 = 2 * 1024 * 1024;
}

/// CPU control register constants
pub mod cr {
    /// CR0: Paging enable bit
    pub const CR0_PG: u64 = 1 << 31;
    
    /// CR4: Physical Address Extension bit
    pub const CR4_PAE: u64 = 1 << 5;
    
    /// EFER MSR address
    pub const EFER_MSR: u32 = 0xC0000080;
    
    /// EFER: Long Mode Enable bit
    pub const EFER_LME: u64 = 1 << 8;
    
    /// EFER_LME low 32 bits (for use with eax)
    pub const EFER_LME_LO: u32 = (EFER_LME & 0xFFFFFFFF) as u32;
}

/// BootInfo field offsets
pub mod bootinfo {
    use webbos_shared::bootinfo::BOOTINFO_PAGE_TABLE_OFFSET;
    
    /// Offset to page_table_addr field in BootInfo
    pub const PAGE_TABLE_OFFSET: usize = BOOTINFO_PAGE_TABLE_OFFSET;
    
    /// Offset to PhysAddr value inside Option<PhysAddr>
    pub const OPTION_PHYSADDR_VALUE_OFFSET: usize = 8;
}
