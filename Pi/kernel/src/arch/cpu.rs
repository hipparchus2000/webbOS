//! CPU-specific functions for ARM64

#![allow(dead_code)]

use crate::println;

/// Initialize CPU features
pub fn init() {
    unsafe {
        // Enable FP/SIMD (required on ARM64)
        enable_fp_simd();
        
        // Set up CPU features
        println!("[cpu] ARM64 processor initialized");
    }
}

/// Enable FP/SIMD (Floating Point / SIMD)
/// 
/// On ARM64, FP/SIMD is disabled at reset and must be explicitly enabled
/// by clearing the CPACR.FPEN bits.
unsafe fn enable_fp_simd() {
    // Read CPACR_EL1
    let mut cpacr: u64;
    core::arch::asm!(
        "mrs {0}, CPACR_EL1",
        out(reg) cpacr,
    );
    
    // Clear FPEN bits (bits 20-21) to enable FP/SIMD for EL0 and EL1
    cpacr |= 3 << 20;
    
    // Write CPACR_EL1
    core::arch::asm!(
        "msr CPACR_EL1, {0}",
        in(reg) cpacr,
    );
    
    // Instruction barrier
    core::arch::asm!("isb");
}

/// Halt the CPU until next interrupt
pub fn halt() {
    unsafe {
        // WFE = Wait For Event (low power state)
        core::arch::asm!("wfe", options(nomem, nostack));
    }
}

/// Disable interrupts
pub fn disable_interrupts() {
    unsafe {
        // Set DAIF bits (Debug, SError, IRQ, FIQ)
        core::arch::asm!(
            "msr DAIFSet, #0xF",
            options(nomem, nostack)
        );
    }
}

/// Enable interrupts
pub fn enable_interrupts() {
    unsafe {
        // Clear DAIF bits
        core::arch::asm!(
            "msr DAIFClr, #0xF",
            options(nomem, nostack)
        );
    }
}

/// Check if interrupts are enabled
pub fn interrupts_enabled() -> bool {
    let daif: u64;
    unsafe {
        core::arch::asm!(
            "mrs {0}, DAIF",
            out(reg) daif,
        );
    }
    // DAIF bits are at positions 9-6 in PSTATE
    // If any are set, interrupts are masked (disabled)
    (daif & 0xF) == 0
}

/// Get CPU vendor string (MIDR_EL1)
pub fn vendor() -> [u8; 12] {
    let midr: u64;
    unsafe {
        core::arch::asm!(
            "mrs {0}, MIDR_EL1",
            out(reg) midr,
        );
    }
    
    // MIDR_EL1 contains implementer, variant, architecture, etc.
    // Implementer is bits 31:24
    let implementer = (midr >> 24) as u8;
    
    let mut vendor = [0u8; 12];
    
    // Decode implementer
    match implementer {
        0x41 => vendor[0..6].copy_from_slice(b"ARM   "),
        0x42 => vendor[0..6].copy_from_slice(b"Broadcom"),
        0x43 => vendor[0..6].copy_from_slice(b"Cavium"),
        0x44 => vendor[0..6].copy_from_slice(b"DEC   "),
        0x4E => vendor[0..6].copy_from_slice(b"NVIDIA"),
        0x50 => vendor[0..6].copy_from_slice(b"APM   "),
        0x51 => vendor[0..6].copy_from_slice(b"Qualcom"),
        _ => vendor[0..7].copy_from_slice(b"Unknown"),
    }
    
    vendor
}

/// Get CPU part number
pub fn part_number() -> u16 {
    let midr: u64;
    unsafe {
        core::arch::asm!(
            "mrs {0}, MIDR_EL1",
            out(reg) midr,
        );
    }
    
    // Part number is bits 15:4
    ((midr >> 4) & 0xFFF) as u16
}

/// Get CPU revision
pub fn revision() -> u8 {
    let midr: u64;
    unsafe {
        core::arch::asm!(
            "mrs {0}, MIDR_EL1",
            out(reg) midr,
        );
    }
    
    // Revision is bits 3:0
    (midr & 0xF) as u8
}

/// Print CPU information
pub fn print_info() {
    let vendor_str = vendor();
    let part = part_number();
    let rev = revision();
    
    println!("  CPU Vendor: {}", core::str::from_utf8(&vendor_str).unwrap_or("Unknown"));
    println!("  Part Number: {:#x}", part);
    println!("  Revision: {}", rev);
    
    // Get CurrentEL
    let current_el: u64;
    unsafe {
        core::arch::asm!(
            "mrs {0}, CurrentEL",
            out(reg) current_el,
        );
    }
    let el = (current_el >> 2) & 3;
    println!("  Exception Level: EL{}", el);
}

/// Reboot the system
pub fn reboot() -> ! {
    // On Raspberry Pi, we can reboot by writing to the watchdog
    // or by triggering a system reset via the mailbox
    unsafe {
        // PM_RSTC and PM_WDOG registers
        const PM_RSTC: *mut u32 = 0x3F10001C as *mut u32;
        const PM_WDOG: *mut u32 = 0x3F100024 as *mut u32;
        const PM_PASSWORD: u32 = 0x5A000000;
        
        // Timeout in 10 ticks (about 100ms)
        core::ptr::write_volatile(PM_WDOG, PM_PASSWORD | 10);
        // Reset
        core::ptr::write_volatile(PM_RSTC, PM_PASSWORD | 0x20);
    }
    
    loop {
        halt();
    }
}

/// Shutdown the system
pub fn shutdown() -> ! {
    // On Pi, shutdown typically requires power management
    // For now, just halt
    loop {
        halt();
    }
}

/// Read CNTPCT_EL0 (physical counter) - 64-bit timer
pub fn read_cntpct() -> u64 {
    let cnt: u64;
    unsafe {
        core::arch::asm!(
            "mrs {0}, CNTPCT_EL0",
            out(reg) cnt,
        );
    }
    cnt
}

/// Get timer frequency (CNTFRQ_EL0)
pub fn timer_frequency() -> u64 {
    let freq: u64;
    unsafe {
        core::arch::asm!(
            "mrs {0}, CNTFRQ_EL0",
            out(reg) freq,
        );
    }
    freq
}

/// Read timestamp counter (similar to x86 RDTSC)
pub fn rdtsc() -> u64 {
    read_cntpct()
}
