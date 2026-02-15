//! ARM64 CPU-specific functions

use crate::println;

/// Initialize CPU features for ARM64
pub fn init() {
    unsafe {
        // Enable FP/NEON
        enable_fp_neon();
        
        // Enable MMU (will be done separately in paging module)
        // enable_mmu();
        
        // Configure system registers
        configure_system_registers();
    }
}

/// Enable FP/NEON (Floating Point and SIMD)
unsafe fn enable_fp_neon() {
    let mut cpacr: u64;
    core::arch::asm!(
        "mrs {}, cpacr_el1",
        out(reg) cpacr,
        options(nomem, nostack)
    );
    
    // Enable FP/NEON (bits 20-21 for EL1)
    cpacr |= (0b11 << 20);  // Set FPEN to 0b11 (no trap)
    
    core::arch::asm!(
        "msr cpacr_el1, {}",
        in(reg) cpacr,
        options(nomem, nostack)
    );
}

/// Configure system registers
unsafe fn configure_system_registers() {
    // Configure SCTLR_EL1 (System Control Register)
    let mut sctlr: u64;
    core::arch::asm!(
        "mrs {}, sctlr_el1",
        out(reg) sctlr,
        options(nomem, nostack)
    );
    
    // Clear some bits for our configuration:
    // - Clear EE (bit 25): Exception Endianness (0 = Little-endian)
    // - Clear E0E (bit 24): EL0 Endianness (0 = Little-endian)
    // - Clear SA (bit 3): Stack Alignment check (0 = Disabled for now)
    // - Clear C (bit 2): Data cache (0 = Disabled initially)
    // - Clear A (bit 1): Alignment check (0 = Disabled)
    // - Clear M (bit 0): MMU (0 = Disabled initially, will enable later)
    
    sctlr &= !((1 << 25) | (1 << 24) | (1 << 3) | (1 << 2) | (1 << 1) | (1 << 0));
    
    // Set I (bit 12): Instruction cache (1 = Enabled)
    sctlr |= 1 << 12;
    
    core::arch::asm!(
        "msr sctlr_el1, {}",
        in(reg) sctlr,
        options(nomem, nostack)
    );
}

/// Halt the CPU until next interrupt (WFI - Wait For Interrupt)
pub fn halt() {
    unsafe {
        core::arch::asm!("wfi", options(nomem, nostack));
    }
}

/// Disable interrupts (DAIF - Debug, SError, IRQ, FIQ)
pub fn disable_interrupts() {
    unsafe {
        // Set DAIF bits (D=Debug, A=SError, I=IRQ, F=FIQ)
        core::arch::asm!("msr daifset, #0b1111", options(nomem, nostack));
    }
}

/// Enable interrupts
pub fn enable_interrupts() {
    unsafe {
        // Clear DAIF bits
        core::arch::asm!("msr daifclr, #0b1111", options(nomem, nostack));
    }
}

/// Check if interrupts are enabled
pub fn interrupts_enabled() -> bool {
    let daif: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, daif",
            out(reg) daif,
            options(nomem, nostack)
        );
    }
    // Check if I (IRQ) and F (FIQ) bits are clear (0 = enabled)
    (daif & ((1 << 7) | (1 << 6))) == 0
}

/// Get CPU implementer and part number
pub fn cpu_info() -> (u32, u32) {
    let midr: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, midr_el1",
            out(reg) midr,
            options(nomem, nostack)
        );
    }
    
    // MIDR format:
    // - Bits 31:24: Implementer (0x41 = ARM, 0x42 = Broadcom, etc.)
    // - Bits 23:20: Variant
    // - Bits 19:16: Architecture
    // - Bits 15:4: Part number
    // - Bits 3:0: Revision
    
    let implementer = ((midr >> 24) & 0xFF) as u32;
    let partnum = ((midr >> 4) & 0xFFF) as u32;
    
    (implementer, partnum)
}

/// Get CPU implementer name
pub fn implementer_name(implementer: u32) -> &'static str {
    match implementer {
        0x41 => "ARM",
        0x42 => "Broadcom",
        0x43 => "Cavium",
        0x44 => "DEC",
        0x46 => "Fujitsu",
        0x48 => "HiSilicon",
        0x49 => "Infineon",
        0x4D => "Motorola/Freescale",
        0x4E => "NVIDIA",
        0x50 => "APM",
        0x51 => "Qualcomm",
        0x53 => "Samsung",
        0x56 => "Marvell",
        0x61 => "Apple",
        0x66 => "Faraday",
        0x69 => "Intel",
        0x70 => "Phytium",
        0xC0 => "Ampere",
        _ => "Unknown",
    }
}

/// Get CPU part name
pub fn part_name(partnum: u32) -> &'static str {
    match partnum {
        0xD03 => "Cortex-A53",
        0xD04 => "Cortex-A35",
        0xD05 => "Cortex-A55",
        0xD06 => "Cortex-A65",
        0xD07 => "Cortex-A57",
        0xD08 => "Cortex-A72",
        0xD09 => "Cortex-A73",
        0xD0A => "Cortex-A75",
        0xD0B => "Cortex-A76",
        0xD0C => "Neoverse N1",
        0xD0D => "Cortex-A77",
        0xD0E => "Cortex-A76AE",
        0xD40 => "Neoverse V1",
        0xD41 => "Cortex-A78",
        0xD42 => "Cortex-A78AE",
        0xD43 => "Cortex-A65AE",
        0xD44 => "Cortex-X1",
        0xD46 => "Cortex-A510",
        0xD47 => "Cortex-A710",
        0xD48 => "Cortex-X2",
        0xD49 => "Neoverse N2",
        0xD4A => "Neoverse E1",
        0xD4B => "Cortex-A78C",
        0xD4C => "Cortex-X1C",
        0xD4D => "Cortex-A715",
        0xD4E => "Cortex-X3",
        0xD80 => "Cortex-A520",
        0xD81 => "Cortex-A720",
        0xD82 => "Cortex-X4",
        0xD83 => "Cortex-A530",
        0xD84 => "Cortex-A730",
        0xD85 => "Cortex-X5",
        _ => "Unknown ARM Core",
    }
}

/// Print CPU information
pub fn print_info() {
    let (implementer, partnum) = cpu_info();
    let implementer_name = implementer_name(implementer);
    let part_name = part_name(partnum);
    
    println!("  CPU Implementer: {} (0x{:02X})", implementer_name, implementer);
    println!("  CPU Part: {} (0x{:03X})", part_name, partnum);
    
    // Read revision
    let midr: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, midr_el1",
            out(reg) midr,
            options(nomem, nostack)
        );
    }
    let revision = (midr & 0xF) as u32;
    println!("  CPU Revision: r{}p{}", (revision >> 4) & 0xF, revision & 0xF);
}

/// Reboot the system (system reset)
pub fn reboot() -> ! {
    // For ARM, we need to use PSCI (Power State Coordination Interface)
    // For now, we'll try to trigger a reset via the system registers
    unsafe {
        // Try to write to reset register (simplified - actual implementation
        // would use PSCI or platform-specific reset controller)
        println!("System reboot requested - halting");
        loop {
            halt();
        }
    }
}

/// Shutdown the system
pub fn shutdown() -> ! {
    unsafe {
        println!("System shutdown requested - halting");
        loop {
            disable_interrupts();
            halt();
        }
    }
}

/// Read system counter (similar to x86 RDTSC)
pub fn read_system_counter() -> u64 {
    let cntvct: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, cntvct_el0",
            out(reg) cntvct,
            options(nomem, nostack)
        );
    }
    cntvct
}

/// Get current exception level
pub fn current_el() -> u32 {
    let currentel: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, currentel",
            out(reg) currentel,
            options(nomem, nostack)
        );
    }
    ((currentel >> 2) & 0x3) as u32
}

/// Check if we're running at EL1 (kernel mode)
pub fn is_el1() -> bool {
    current_el() == 1
}

/// Check if we're running at EL2 (hypervisor mode)
pub fn is_el2() -> bool {
    current_el() == 2
}

/// Check if we're running at EL3 (secure monitor mode)
pub fn is_el3() -> bool {
    current_el() == 3
}