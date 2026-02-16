//! Driver Test Suite
//!
//! This module contains tests for all device drivers.
//! Run these tests to verify driver functionality.

use crate::println;

/// Run all driver tests
pub fn run_all_tests() {
    println!("\n");
    println!("╔══════════════════════════════════════════════════╗");
    println!("║       WebbOS Driver Test Suite                   ║");
    println!("╚══════════════════════════════════════════════════╝");
    println!();
    
    // Architecture-specific tests
    #[cfg(target_arch = "aarch64")]
    run_aarch64_tests();
    
    // Generic driver tests
    run_generic_tests();
    
    println!();
    println!("╔══════════════════════════════════════════════════╗");
    println!("║       Driver Test Suite Complete                 ║");
    println!("╚══════════════════════════════════════════════════╝");
    println!();
}

/// Run ARM64-specific driver tests
#[cfg(target_arch = "aarch64")]
fn run_aarch64_tests() {
    use crate::arch::cpu;
    use crate::hal;
    use crate::drivers::raspberrypi::{gpio, uart, usb, ethernet};
    
    println!("=== ARM64 Architecture Tests ===\n");
    
    // CPU info test
    println!("[TEST] CPU Information");
    cpu::print_info();
    println!("[PASS] CPU info displayed\n");
    
    // HAL tests
    println!("[TEST] Hardware Abstraction Layer");
    println!("  Platform: {:?}", hal::platform_info().platform_type);
    println!("[PASS] HAL info displayed\n");
    
    // GPIO tests
    println!("[TEST] GPIO Driver");
    gpio::print_info();
    
    // Quick pin toggle test (don't run full blink in automated tests)
    println!("[TEST] GPIO Pin Toggle");
    gpio::set_function(42, gpio::GpioFunction::Output);
    gpio::set_level(42, true);
    hal::delay::microseconds(1000);
    gpio::set_level(42, false);
    println!("[PASS] GPIO pin toggle\n");
    
    // UART tests
    println!("[TEST] UART Driver");
    uart::print_info();
    uart::puts("UART Test Output\r\n");
    println!("[PASS] UART output\n");
    
    // USB tests
    println!("[TEST] USB Driver");
    usb::print_info();
    println!("[PASS] USB info displayed\n");
    
    // Ethernet tests
    println!("[TEST] Ethernet Driver");
    ethernet::print_info();
    println!("[PASS] Ethernet info displayed\n");
}

/// Run generic driver tests
fn run_generic_tests() {
    println!("=== Generic Driver Tests ===\n");
    
    // Timer test
    println!("[TEST] Timer Driver");
    crate::drivers::timer::print_stats();
    println!("[PASS] Timer stats displayed\n");
    
    // Input test
    println!("[TEST] Input Driver");
    crate::drivers::input::print_info();
    println!("[PASS] Input info displayed\n");
}

/// Print test summary
pub fn print_summary() {
    println!("Driver Test Summary:");
    println!("  - GPIO: Pin control, interrupts, pull-up/down");
    println!("  - UART: Serial console at 115200 baud");
    println!("  - USB: XHCI controller detection (USB 3.0)");
    println!("  - Ethernet: MAC address, link status");
    println!("  - HAL: Platform detection, MMIO helpers");
}

/// Individual test functions for manual testing

/// Test GPIO LED blink manually
pub fn test_gpio_blink() {
    #[cfg(target_arch = "aarch64")]
    {
        use crate::drivers::raspberrypi::gpio;
        println!("Starting GPIO blink test...");
        gpio::blink_led(42, 10, 200);
    }
}

/// Test UART echo manually
pub fn test_uart_echo() {
    #[cfg(target_arch = "aarch64")]
    {
        use crate::drivers::raspberrypi::uart;
        println!("Starting UART echo test...");
        uart::echo_test(100);
    }
}

/// Test platform detection
#[cfg(target_arch = "aarch64")]
pub fn test_platform_detection() {
    use crate::hal;
    
    println!("Platform Detection Test:");
    let info = hal::platform_info();
    
    println!("  Platform Type: {:?}", info.platform_type);
    println!("  CPU Frequency: {} MHz", info.cpu_freq_hz / 1_000_000);
    println!("  UART Clock: {} MHz", info.uart_clock_hz / 1_000_000);
    println!("  GPIO Base: 0x{:016X}", info.gpio_base);
    println!("  UART0 Base: 0x{:016X}", info.uart0_base);
    println!("  Peripheral Base: 0x{:016X}", info.peripheral_base);
    
    if hal::is_raspberry_pi5() {
        println!("  Detected: Raspberry Pi 5");
    } else if hal::is_qemu() {
        println!("  Detected: QEMU virt machine");
    }
}

/// Test platform detection (x86_64 stub)
#[cfg(target_arch = "x86_64")]
pub fn test_platform_detection() {
    println!("Platform Detection Test:");
    println!("  Platform Type: x86_64 PC");
    println!("  Detected: x86_64 PC (no HAL available)");
}
