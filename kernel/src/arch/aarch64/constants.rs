//! AArch64 architecture constants

/// UART (PL011) constants for QEMU virt platform
pub mod uart {
    /// UART0 base address on QEMU virt platform
    pub const UART0_BASE: u64 = 0x09000000;
    
    /// UART data register offset
    pub const DATA_OFFSET: u64 = 0x00;
    
    /// UART flag register offset
    pub const FLAG_OFFSET: u64 = 0x18;
    
    /// ASCII characters for boot messages
    pub mod ascii {
        pub const A: u8 = b'A';
        pub const K: u8 = b'K';
        pub const E: u8 = b'E';
        pub const R: u8 = b'R';
        pub const N: u8 = b'N';
        pub const L: u8 = b'L';
        pub const S: u8 = b'S';
        pub const X: u8 = b'X';
        pub const C: u8 = b'C';
    }
    
    /// Build UART address using movz immediate value
    /// movz xN, 0x0900, lsl 16 produces 0x09000000
    pub const MOVZ_UART_IMM: u16 = 0x0900;
    pub const MOVZ_UART_SHIFT: u8 = 16;
}

/// Memory layout constants
pub mod memory {
    /// Physical stack top address (5MB)
    pub const PHYS_STACK_TOP: u64 = 0x500000;
    
    /// Page size (4KB)
    pub const PAGE_SIZE: u64 = 4096;
    
    /// Size of a large page (2MB)
    pub const LARGE_PAGE_SIZE: u64 = 2 * 1024 * 1024;
}

/// CPU register and system constants
pub mod cpu {
    /// Current Exception Level 1 (EL1)
    pub const EL1: u8 = 1;
    
    /// CPACR_EL1 register: FP/NEON enable bits
    pub const CPACR_FPEN_SHIFT: u8 = 20;
    pub const CPACR_FPEN_ENABLE: u64 = 0b11;
}

/// BootInfo field offsets
pub mod bootinfo {
    // aarch64 uses the same BootInfo structure
    use webbos_shared::bootinfo::BOOTINFO_PAGE_TABLE_OFFSET;
    
    /// Offset to page_table_addr field in BootInfo
    pub const PAGE_TABLE_OFFSET: usize = BOOTINFO_PAGE_TABLE_OFFSET;
}

/// Move immediate constants for inline assembly
pub mod asm {
    /// movz xN, 0x50, lsl 16 produces 0x500000
    pub const STACK_TOP_IMM: u16 = 0x50;
    pub const STACK_TOP_SHIFT: u8 = 16;
}
