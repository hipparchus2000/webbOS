//! CPU-specific functions

use core::arch::x86_64::__cpuid;
use crate::println;

/// Initialize CPU features
pub fn init() {
    unsafe {
        // Enable SSE
        enable_sse();
        
        // Enable NX bit (requires EFER MSR)
        enable_nx_bit();
        
        // Enable write protect
        enable_write_protect();
    }
}

/// Enable SSE (Streaming SIMD Extensions)
unsafe fn enable_sse() {
    let mut cr0: u64;
    core::arch::asm!(
        "mov {}, cr0",
        out(reg) cr0,
        options(nomem, nostack)
    );
    // Clear EM, Set MP
    cr0 &= !(1 << 2);  // Clear EM (bit 2)
    cr0 |= 1 << 1;     // Set MP (bit 1)
    core::arch::asm!(
        "mov cr0, {}",
        in(reg) cr0,
        options(nomem, nostack)
    );
    
    let mut cr4: u64;
    core::arch::asm!(
        "mov {}, cr4",
        out(reg) cr4,
        options(nomem, nostack)
    );
    // Set OSFXSR (bit 9) and OSXMMEXCPT (bit 10)
    cr4 |= (1 << 9) | (1 << 10);
    core::arch::asm!(
        "mov cr4, {}",
        in(reg) cr4,
        options(nomem, nostack)
    );
}

/// Enable NX (No-Execute) bit
unsafe fn enable_nx_bit() {
    // Read EFER MSR (0xC0000080)
    let mut efer: u64;
    core::arch::asm!(
        "rdmsr",
        in("ecx") 0xC0000080u32,
        out("eax") efer,
        out("edx") _,
        options(nomem, nostack)
    );
    // Set NXE bit (bit 11)
    efer |= 1 << 11;
    core::arch::asm!(
        "wrmsr",
        in("ecx") 0xC0000080u32,
        in("eax") efer as u32,
        in("edx") (efer >> 32) as u32,
        options(nomem, nostack)
    );
}

/// Enable write protect (kernel can't write to read-only pages)
unsafe fn enable_write_protect() {
    let mut cr0: u64;
    core::arch::asm!(
        "mov {}, cr0",
        out(reg) cr0,
        options(nomem, nostack)
    );
    // Set WP bit (bit 16)
    cr0 |= 1 << 16;
    core::arch::asm!(
        "mov cr0, {}",
        in(reg) cr0,
        options(nomem, nostack)
    );
}

/// Halt the CPU until next interrupt
pub fn halt() {
    unsafe {
        core::arch::asm!("hlt", options(nomem, nostack));
    }
}

/// Disable interrupts
pub fn disable_interrupts() {
    unsafe {
        core::arch::asm!("cli", options(nomem, nostack));
    }
}

/// Enable interrupts
pub fn enable_interrupts() {
    unsafe {
        core::arch::asm!("sti", options(nomem, nostack));
    }
}

/// Check if interrupts are enabled
pub fn interrupts_enabled() -> bool {
    let rflags: u64;
    unsafe {
        core::arch::asm!(
            "pushfq",
            "pop {}",
            out(reg) rflags,
            options(nomem, nostack)
        );
    }
    // Interrupt flag is bit 9
    (rflags & (1 << 9)) != 0
}

/// Get CPU vendor string
pub fn vendor() -> [u8; 12] {
    let cpuid = unsafe { __cpuid(0) };
    let mut vendor = [0u8; 12];
    
    // ebx, edx, ecx contain the vendor string
    vendor[0..4].copy_from_slice(&cpuid.ebx.to_le_bytes());
    vendor[4..8].copy_from_slice(&cpuid.edx.to_le_bytes());
    vendor[8..12].copy_from_slice(&cpuid.ecx.to_le_bytes());
    
    vendor
}

/// Get CPU brand string
pub fn brand() -> [u8; 48] {
    let mut brand = [0u8; 48];
    
    for i in 0..3 {
        let cpuid = unsafe { __cpuid(0x80000002 + i as u32) };
        let offset = i * 16;
        brand[offset..offset+4].copy_from_slice(&cpuid.eax.to_le_bytes());
        brand[offset+4..offset+8].copy_from_slice(&cpuid.ebx.to_le_bytes());
        brand[offset+8..offset+12].copy_from_slice(&cpuid.ecx.to_le_bytes());
        brand[offset+12..offset+16].copy_from_slice(&cpuid.edx.to_le_bytes());
    }
    
    brand
}

/// Get CPU features
pub fn features() -> u64 {
    let cpuid = unsafe { __cpuid(1) };
    ((cpuid.edx as u64) << 32) | (cpuid.ecx as u64)
}

/// Print CPU information
pub fn print_info() {
    let vendor = vendor();
    let brand = brand();
    
    println!("  CPU Vendor: {}", core::str::from_utf8(&vendor).unwrap_or("Unknown"));
    
    let brand_str = core::str::from_utf8(&brand).unwrap_or("Unknown");
    let brand_trimmed = brand_str.trim();
    if !brand_trimmed.is_empty() {
        println!("  CPU Brand: {}", brand_trimmed);
    }
}

/// Reboot the system
pub fn reboot() -> ! {
    unsafe {
        // Try keyboard controller reset
        core::arch::asm!(
            "mov al, 0xFE",
            "out 0x64, al",
            options(nomem, nostack)
        );
        
        // If that fails, triple fault
        loop {
            core::arch::asm!("int 3", options(nomem, nostack));
        }
    }
}

/// Shutdown the system (if supported by hardware)
pub fn shutdown() -> ! {
    unsafe {
        // Try ACPI shutdown (simplified - should use proper ACPI)
        // For now, just halt
        loop {
            core::arch::asm!("cli; hlt", options(nomem, nostack));
        }
    }
}

/// Read timestamp counter
pub fn rdtsc() -> u64 {
    unsafe { core::arch::x86_64::_rdtsc() }
}
