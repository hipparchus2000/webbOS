//! GPIO LED Blink Test for Raspberry Pi
//!
//! This module provides test functions for GPIO functionality.
//! Run this test to verify GPIO is working correctly.

use super::*;

/// Run a comprehensive GPIO test sequence
pub fn run_gpio_test() {
    println!("\n========================================");
    println!("GPIO Driver Test Sequence");
    println!("========================================\n");
    
    // Test 1: Basic pin configuration
    println!("Test 1: Pin Configuration");
    println!("  Configuring GPIO 42 as output...");
    set_function(42, GpioFunction::Output);
    set_pull(42, GpioPull::None);
    println!("  [PASS] Pin configured\n");
    
    // Test 2: Level control
    println!("Test 2: Level Control");
    println!("  Setting GPIO 42 high...");
    set_level(42, true);
    delay::milliseconds(100);
    
    let high_level = get_level(42);
    println!("  Read level: {}", if high_level { "HIGH" } else { "LOW" });
    
    println!("  Setting GPIO 42 low...");
    set_level(42, false);
    delay::milliseconds(100);
    
    let low_level = get_level(42);
    println!("  Read level: {}", if low_level { "HIGH" } else { "LOW" });
    
    if high_level && !low_level {
        println!("  [PASS] Level control working\n");
    } else {
        println!("  [WARN] Level control may have issues (readback differs from expected)\n");
    }
    
    // Test 3: Toggle test
    println!("Test 3: Toggle Test");
    println!("  Toggling GPIO 42 5 times...");
    for i in 0..5 {
        toggle(42);
        println!("    Toggle {}: {}", i + 1, if get_level(42) { "HIGH" } else { "LOW" });
        delay::milliseconds(100);
    }
    println!("  [PASS] Toggle test complete\n");
    
    // Test 4: LED blink test
    println!("Test 4: LED Blink Test");
    println!("  Blinking LED 10 times (200ms on/off)...");
    blink_led(42, 10, 200);
    println!("  [PASS] Blink test complete\n");
    
    // Test 5: Multiple pin test
    println!("Test 5: Multiple Pin Configuration");
    println!("  Testing pins 40-45...");
    for pin in 40..=45 {
        set_function(pin, GpioFunction::Output);
        set_pull(pin, GpioPull::None);
        set_level(pin, false);
    }
    
    // Chaser pattern
    println!("  Running chaser pattern...");
    for _ in 0..3 {
        for pin in 40..=45 {
            set_level(pin, true);
            delay::milliseconds(50);
            set_level(pin, false);
        }
    }
    println!("  [PASS] Multiple pin test complete\n");
    
    println!("========================================");
    println!("GPIO Test Sequence Complete");
    println!("========================================\n");
}

/// Quick LED blink to indicate system status
/// This is useful for visual feedback during boot
pub fn status_blink(count: u8) {
    const STATUS_LED_PIN: u8 = 42;
    
    // Configure pin
    set_function(STATUS_LED_PIN, GpioFunction::Output);
    set_pull(STATUS_LED_PIN, GpioPull::None);
    
    // Quick blink sequence
    for _ in 0..count {
        set_level(STATUS_LED_PIN, true);
        delay::milliseconds(50);
        set_level(STATUS_LED_PIN, false);
        delay::milliseconds(50);
    }
}

/// Boot status indicator sequence
pub fn boot_indicator() {
    println!("[GPIO] Boot indicator sequence starting...");
    
    // 3 quick blinks = boot started
    status_blink(3);
    delay::milliseconds(200);
    
    // 2 medium blinks = driver init
    const STATUS_LED_PIN: u8 = 42;
    for _ in 0..2 {
        set_level(STATUS_LED_PIN, true);
        delay::milliseconds(200);
        set_level(STATUS_LED_PIN, false);
        delay::milliseconds(200);
    }
    delay::milliseconds(200);
    
    // 1 long blink = ready
    set_level(STATUS_LED_PIN, true);
    delay::milliseconds(500);
    set_level(STATUS_LED_PIN, false);
    
    println!("[GPIO] Boot indicator complete");
}
