//! Hardware Abstraction Layer (HAL)
//!
//! This module provides hardware-specific abstractions for webbOS,
//! allowing the kernel to run on different platforms (QEMU, Raspberry Pi 5, etc.)

use crate::println;

/// Platform types supported by webbOS
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformType {
    /// QEMU virt machine (ARM64)
    QemuVirt,
    /// Raspberry Pi 4
    RaspberryPi4,
    /// Raspberry Pi 5
    RaspberryPi5,
    /// Generic ARM64 (unknown platform)
    GenericArm64,
    /// x86_64 (for reference)
    X86_64,
}

/// Device tree header structure
#[repr(C)]
#[derive(Debug)]
pub struct DeviceTreeHeader {
    pub magic: u32,
    pub totalsize: u32,
    pub off_dt_struct: u32,
    pub off_dt_strings: u32,
    pub off_mem_rsvmap: u32,
    pub version: u32,
    pub last_comp_version: u32,
    pub boot_cpuid_phys: u32,
    pub size_dt_strings: u32,
    pub size_dt_struct: u32,
}

impl DeviceTreeHeader {
    /// Device tree magic number (big-endian: 0xd00dfeed)
    pub const MAGIC: u32 = 0xedfe0dd0; // Little-endian representation
    
    /// Check if the header is valid
    pub fn is_valid(&self) -> bool {
        self.magic == Self::MAGIC
    }
}

/// Platform information
#[derive(Debug, Clone)]
pub struct PlatformInfo {
    /// Platform type
    pub platform_type: PlatformType,
    /// CPU frequency in Hz
    pub cpu_freq_hz: u64,
    /// UART base clock frequency
    pub uart_clock_hz: u64,
    /// GPIO base address (physical)
    pub gpio_base: usize,
    /// UART0 (PL011) base address
    pub uart0_base: usize,
    /// Mini UART base address
    pub mini_uart_base: usize,
    /// GIC (Generic Interrupt Controller) base address
    pub gic_base: usize,
    /// Peripheral base address
    pub peripheral_base: usize,
    /// System timer base address
    pub timer_base: usize,
    /// USB XHCI base address
    pub usb_xhci_base: usize,
    /// Ethernet MAC base address
    pub ethernet_base: usize,
}

impl PlatformInfo {
    /// Create platform info for Raspberry Pi 5
    pub const fn raspberry_pi5() -> Self {
        // Raspberry Pi 5 uses RP1 I/O controller
        // Base address for peripherals is at 0x1F_0000_0000 (physical)
        // But for early boot, we use the lower mapping
        const PI5_PERIPHERAL_BASE: usize = 0x1F00000000;
        
        Self {
            platform_type: PlatformType::RaspberryPi5,
            cpu_freq_hz: 2_400_000_000, // 2.4 GHz (Cortex-A76)
            uart_clock_hz: 48_000_000,  // 48 MHz UART clock
            gpio_base: PI5_PERIPHERAL_BASE + 0xD0000,  // RP1 GPIO
            uart0_base: PI5_PERIPHERAL_BASE + 0xD_0000, // PL011 UART (via RP1)
            mini_uart_base: PI5_PERIPHERAL_BASE + 0xD_0000 + 0x1000, // Mini UART
            gic_base: 0x107FFF0000, // GIC-400 base (ARM GIC v2)
            peripheral_base: PI5_PERIPHERAL_BASE,
            timer_base: 0x107E00B000, // System timer
            usb_xhci_base: PI5_PERIPHERAL_BASE + 0x100_0000, // USB XHCI
            ethernet_base: PI5_PERIPHERAL_BASE + 0x200_0000, // Ethernet MAC
        }
    }
    
    /// Create platform info for Raspberry Pi 4
    pub const fn raspberry_pi4() -> Self {
        const PI4_PERIPHERAL_BASE: usize = 0xFE000000;
        
        Self {
            platform_type: PlatformType::RaspberryPi4,
            cpu_freq_hz: 1_500_000_000, // 1.5 GHz
            uart_clock_hz: 48_000_000,
            gpio_base: PI4_PERIPHERAL_BASE + 0x200000,
            uart0_base: PI4_PERIPHERAL_BASE + 0x201000,
            mini_uart_base: PI4_PERIPHERAL_BASE + 0x215000,
            gic_base: 0xFF840000,
            peripheral_base: PI4_PERIPHERAL_BASE,
            timer_base: PI4_PERIPHERAL_BASE + 0x3000,
            usb_xhci_base: 0, // USB 2.0 only on Pi 4
            ethernet_base: PI4_PERIPHERAL_BASE + 0x580000,
        }
    }
    
    /// Create platform info for QEMU virt machine
    pub const fn qemu_virt() -> Self {
        // QEMU virt machine uses different memory map
        Self {
            platform_type: PlatformType::QemuVirt,
            cpu_freq_hz: 1_000_000_000, // 1 GHz (configurable in QEMU)
            uart_clock_hz: 0, // Not used for virtio
            gpio_base: 0x09000000, // PL061 GPIO
            uart0_base: 0x09000000, // PL011 UART0
            mini_uart_base: 0, // No mini UART in QEMU virt
            gic_base: 0x08000000, // GIC v2/v3
            peripheral_base: 0x08000000,
            timer_base: 0x09000000, // ARM Generic Timer
            usb_xhci_base: 0, // Uses virtio
            ethernet_base: 0, // Uses virtio
        }
    }
    
    /// Create platform info for generic ARM64
    pub const fn generic() -> Self {
        Self {
            platform_type: PlatformType::GenericArm64,
            cpu_freq_hz: 1_000_000_000,
            uart_clock_hz: 0,
            gpio_base: 0,
            uart0_base: 0,
            mini_uart_base: 0,
            gic_base: 0,
            peripheral_base: 0,
            timer_base: 0,
            usb_xhci_base: 0,
            ethernet_base: 0,
        }
    }
}

/// Global platform information (initialized at boot)
static mut PLATFORM_INFO: Option<PlatformInfo> = None;

/// Initialize the HAL with the detected platform
pub fn init() {
    println!("[HAL] Initializing Hardware Abstraction Layer...");
    
    // Detect platform based on device tree or CPU features
    let platform_info = detect_platform();
    
    // Print platform info before storing
    println!("[HAL] Platform detected: {:?}", platform_info.platform_type);
    println!("  CPU frequency: {} MHz", platform_info.cpu_freq_hz / 1_000_000);
    println!("  Peripheral base: 0x{:016X}", platform_info.peripheral_base);
    
    // Initialize platform-specific drivers
    init_platform_drivers(&platform_info);
    
    // Store platform info
    unsafe {
        PLATFORM_INFO = Some(platform_info);
    }
}

/// Detect the current platform
fn detect_platform() -> PlatformInfo {
    // Try to detect from device tree
    if let Some(dt_info) = detect_from_device_tree() {
        return dt_info;
    }
    
    // Try to detect from CPU features
    if let Some(platform) = detect_from_cpu() {
        return platform;
    }
    
    // Default to QEMU virt for testing
    println!("[HAL] Platform detection failed, assuming QEMU virt");
    PlatformInfo::qemu_virt()
}

/// Try to detect platform from device tree blob
fn detect_from_device_tree() -> Option<PlatformInfo> {
    // Device tree is typically passed by bootloader at a known address
    // For Raspberry Pi, it's often at 0x100 or passed in registers
    
    // Check common device tree locations
    let dt_addresses = [
        0x1000usize,      // Common bootloader location
        0x10000usize,     // Alternative location
        0x80000usize,     // Sometimes overlaps with kernel load address
    ];
    
    for &addr in &dt_addresses {
        let header = unsafe { &*(addr as *const DeviceTreeHeader) };
        
        if header.is_valid() {
            println!("[HAL] Device tree found at 0x{:016X}", addr);
            println!("  Total size: {} bytes", u32::from_be(header.totalsize));
            println!("  Version: {}", u32::from_be(header.version));
            
            // Parse the device tree to identify platform
            return parse_device_tree(addr);
        }
    }
    
    None
}

/// Parse device tree to extract platform information
fn parse_device_tree(dt_addr: usize) -> Option<PlatformInfo> {
    // Read the model property from the root node
    // This would involve walking the device tree structure
    // For now, use a simplified detection
    
    // Check for Raspberry Pi specific strings in the DTB
    let model_offset = find_property(dt_addr, b"model\0");
    
    if let Some(offset) = model_offset {
        let model_str = read_string_property(dt_addr, offset);
        println!("[HAL] Device model: {}", model_str);
        
        if model_str.contains("Raspberry Pi 5") {
            return Some(PlatformInfo::raspberry_pi5());
        } else if model_str.contains("Raspberry Pi 4") {
            return Some(PlatformInfo::raspberry_pi4());
        }
    }
    
    // Check for QEMU
    let compatible = find_property(dt_addr, b"compatible\0");
    if let Some(offset) = compatible {
        let compat_str = read_string_property(dt_addr, offset);
        if compat_str.contains("qemu") || compat_str.contains("qemu-virt") {
            return Some(PlatformInfo::qemu_virt());
        }
    }
    
    None
}

/// Find a property in the device tree (simplified)
fn find_property(_dt_addr: usize, _name: &[u8]) -> Option<usize> {
    // This would walk the device tree structure
    // For now, return None to use fallback detection
    None
}

/// Read a string property from the device tree
fn read_string_property(_dt_addr: usize, _offset: usize) -> &'static str {
    // This would read the property value
    // For now, return an empty string
    ""
}

/// Detect platform from CPU features
fn detect_from_cpu() -> Option<PlatformInfo> {
    // Read MIDR_EL1 to identify CPU
    let midr: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, midr_el1",
            out(reg) midr,
            options(nomem, nostack)
        );
    }
    
    let implementer = ((midr >> 24) & 0xFF) as u32;
    let partnum = ((midr >> 4) & 0xFFF) as u32;
    
    println!("[HAL] CPU Implementer: 0x{:02X}, Part: 0x{:03X}", implementer, partnum);
    
    // Cortex-A76 is used in Raspberry Pi 5
    if implementer == 0x41 && partnum == 0xD0B {
        println!("[HAL] Detected Cortex-A76 (Raspberry Pi 5)");
        return Some(PlatformInfo::raspberry_pi5());
    }
    
    // Cortex-A72 is used in Raspberry Pi 4
    if implementer == 0x41 && partnum == 0xD08 {
        println!("[HAL] Detected Cortex-A72 (Raspberry Pi 4)");
        return Some(PlatformInfo::raspberry_pi4());
    }
    
    None
}

/// Initialize platform-specific drivers
fn init_platform_drivers(info: &PlatformInfo) {
    match info.platform_type {
        PlatformType::RaspberryPi5 => {
            println!("[HAL] Initializing Raspberry Pi 5 specific drivers...");
            // GPIO and UART will be initialized separately
        }
        PlatformType::RaspberryPi4 => {
            println!("[HAL] Initializing Raspberry Pi 4 specific drivers...");
        }
        PlatformType::QemuVirt => {
            println!("[HAL] Initializing QEMU virt specific drivers...");
        }
        _ => {
            println!("[HAL] No specific drivers for generic platform");
        }
    }
}

/// Get the current platform information
pub fn platform_info() -> &'static PlatformInfo {
    unsafe {
        PLATFORM_INFO.as_ref().expect("HAL not initialized")
    }
}

/// Check if running on Raspberry Pi 5
pub fn is_raspberry_pi5() -> bool {
    platform_info().platform_type == PlatformType::RaspberryPi5
}

/// Check if running on QEMU
pub fn is_qemu() -> bool {
    platform_info().platform_type == PlatformType::QemuVirt
}

/// Memory-mapped I/O helper functions
pub mod mmio {
    /// Read an 8-bit value from a memory-mapped register
    #[inline]
    pub unsafe fn read8(addr: usize) -> u8 {
        core::ptr::read_volatile(addr as *const u8)
    }
    
    /// Read a 16-bit value from a memory-mapped register
    #[inline]
    pub unsafe fn read16(addr: usize) -> u16 {
        core::ptr::read_volatile(addr as *const u16)
    }
    
    /// Read a 32-bit value from a memory-mapped register
    #[inline]
    pub unsafe fn read32(addr: usize) -> u32 {
        core::ptr::read_volatile(addr as *const u32)
    }
    
    /// Read a 64-bit value from a memory-mapped register
    #[inline]
    pub unsafe fn read64(addr: usize) -> u64 {
        core::ptr::read_volatile(addr as *const u64)
    }
    
    /// Write an 8-bit value to a memory-mapped register
    #[inline]
    pub unsafe fn write8(addr: usize, value: u8) {
        core::ptr::write_volatile(addr as *mut u8, value);
    }
    
    /// Write a 16-bit value to a memory-mapped register
    #[inline]
    pub unsafe fn write16(addr: usize, value: u16) {
        core::ptr::write_volatile(addr as *mut u16, value);
    }
    
    /// Write a 32-bit value to a memory-mapped register
    #[inline]
    pub unsafe fn write32(addr: usize, value: u32) {
        core::ptr::write_volatile(addr as *mut u32, value);
    }
    
    /// Write a 64-bit value to a memory-mapped register
    #[inline]
    pub unsafe fn write64(addr: usize, value: u64) {
        core::ptr::write_volatile(addr as *mut u64, value);
    }
    
    /// Memory barrier to ensure write completion
    #[inline]
    pub fn memory_barrier() {
        unsafe {
            core::arch::asm!("dsb sy", options(nomem, nostack));
            core::arch::asm!("isb", options(nomem, nostack));
        }
    }
    
    /// Data synchronization barrier
    #[inline]
    pub fn dsb() {
        unsafe {
            core::arch::asm!("dsb sy", options(nomem, nostack));
        }
    }
    
    /// Instruction synchronization barrier
    #[inline]
    pub fn isb() {
        unsafe {
            core::arch::asm!("isb", options(nomem, nostack));
        }
    }
}

/// Delay functions
pub mod delay {
    use super::platform_info;
    
    /// Simple busy-wait delay (approximately microseconds)
    /// Note: This is approximate and depends on CPU frequency
    pub fn microseconds(us: u32) {
        // Calculate loop iterations based on CPU frequency
        // Assuming roughly 4 cycles per iteration
        let freq_hz = platform_info().cpu_freq_hz;
        let iterations = (us as u64 * freq_hz) / 4_000_000;
        
        for _ in 0..iterations {
            unsafe { core::arch::asm!("nop", options(nomem, nostack)); }
        }
    }
    
    /// Simple busy-wait delay (approximately milliseconds)
    pub fn milliseconds(ms: u32) {
        for _ in 0..ms {
            microseconds(1000);
        }
    }
    
    /// Simple busy-wait delay (approximately seconds)
    pub fn seconds(s: u32) {
        for _ in 0..s {
            milliseconds(1000);
        }
    }
}
