//! GPIO Driver for Raspberry Pi 5
//!
//! The Raspberry Pi 5 uses the RP1 I/O controller for GPIO access.
//! This is significantly different from previous Pi models.
//!
//! GPIO pin mappings:
//! - GPIO 2-3: I2C1 (SDA, SCL)
//! - GPIO 14-15: UART0 (TXD, RXD)
//! - GPIO 18: PCM_CLK
//! - GPIO 40-45: Various functions
//! - GPIO 42: Activity LED (on some models)

use crate::hal::{mmio, platform_info, PlatformInfo, PlatformType};
use crate::println;

/// GPIO register offsets for RP1 (Raspberry Pi 5 I/O controller)
/// These are relative to the GPIO base address
pub mod rp1_regs {
    /// GPIO status register offset
    pub const GIO_STATUS: usize = 0x00;
    /// GPIO raw interrupt status
    pub const GIO_RAW_INT: usize = 0x04;
    /// GPIO interrupt enable
    pub const GIO_INT_EN: usize = 0x08;
    /// GPIO interrupt disable
    pub const GIO_INT_DIS: usize = 0x0C;
    /// GPIO interrupt type (edge/level)
    pub const GIO_INT_TYPE: usize = 0x10;
    /// GPIO interrupt polarity
    pub const GIO_INT_POLARITY: usize = 0x14;
    /// GPIO data register (for pins 0-31)
    pub const GIO_DATA: usize = 0x18;
    /// GPIO direction register
    pub const GIO_DIR: usize = 0x1C;
    /// GPIO mode/function select
    pub const GIO_MODE: usize = 0x20;
    /// GPIO pull-up/down enable
    pub const GIO_PULL_EN: usize = 0x24;
    /// GPIO pull-up/down direction
    pub const GIO_PULL_DIR: usize = 0x28;
    /// GPIO drive strength
    pub const GIO_DRIVE: usize = 0x2C;
    /// GPIO set output (write 1 to set)
    pub const GIO_SET: usize = 0x30;
    /// GPIO clear output (write 1 to clear)
    pub const GIO_CLR: usize = 0x34;
}

/// Legacy BCM2711 GPIO offsets (for Raspberry Pi 4 compatibility)
pub mod bcm2711_regs {
    /// GPIO function select registers (GPFSEL0-GPFSEL5)
    pub const GPFSEL0: usize = 0x00;
    pub const GPFSEL1: usize = 0x04;
    pub const GPFSEL2: usize = 0x08;
    pub const GPFSEL3: usize = 0x0C;
    pub const GPFSEL4: usize = 0x10;
    pub const GPFSEL5: usize = 0x14;
    
    /// GPIO set registers
    pub const GPSET0: usize = 0x1C;
    pub const GPSET1: usize = 0x20;
    
    /// GPIO clear registers
    pub const GPCLR0: usize = 0x28;
    pub const GPCLR1: usize = 0x2C;
    
    /// GPIO level registers
    pub const GPLEV0: usize = 0x34;
    pub const GPLEV1: usize = 0x38;
    
    /// GPIO event detect status
    pub const GPEDS0: usize = 0x40;
    pub const GPEDS1: usize = 0x44;
    
    /// GPIO rising edge detect enable
    pub const GPREN0: usize = 0x4C;
    pub const GPREN1: usize = 0x50;
    
    /// GPIO falling edge detect enable
    pub const GPFEN0: usize = 0x58;
    pub const GPFEN1: usize = 0x5C;
    
    /// GPIO pull-up/down register (deprecated on Pi 4)
    pub const GPPUD: usize = 0x94;
    
    /// GPIO pull-up/down clock registers
    pub const GPPUDCLK0: usize = 0x98;
    pub const GPPUDCLK1: usize = 0x9C;
}

/// GPIO pin function selection
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpioFunction {
    /// Input
    Input = 0,
    /// Output
    Output = 1,
    /// Alternative function 0
    Alt0 = 4,
    /// Alternative function 1
    Alt1 = 5,
    /// Alternative function 2
    Alt2 = 6,
    /// Alternative function 3
    Alt3 = 7,
    /// Alternative function 4
    Alt4 = 3,
    /// Alternative function 5
    Alt5 = 2,
}

/// GPIO pull-up/down configuration
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpioPull {
    /// No pull-up or pull-down
    None = 0,
    /// Pull-up
    Up = 1,
    /// Pull-down
    Down = 2,
}

/// GPIO interrupt edge selection
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpioEdge {
    /// No edge detection
    None = 0,
    /// Rising edge
    Rising = 1,
    /// Falling edge
    Falling = 2,
    /// Both edges
    Both = 3,
}

/// GPIO driver state
pub struct GpioDriver {
    /// Base address of GPIO registers
    base_addr: usize,
    /// Platform type
    platform: PlatformType,
    /// Initialized flag
    initialized: bool,
}

/// Global GPIO driver instance
static mut GPIO_DRIVER: GpioDriver = GpioDriver {
    base_addr: 0,
    platform: PlatformType::GenericArm64,
    initialized: false,
};

/// Initialize the GPIO driver
pub fn init() {
    println!("[GPIO] Initializing GPIO driver...");
    
    let info = platform_info();
    let base_addr = info.gpio_base;
    
    unsafe {
        GPIO_DRIVER = GpioDriver {
            base_addr,
            platform: info.platform_type,
            initialized: true,
        };
    }
    
    println!("[GPIO] Base address: 0x{:016X}", base_addr);
    println!("[GPIO] Platform: {:?}", info.platform_type);
    
    // Configure default pin states
    match info.platform_type {
        PlatformType::RaspberryPi5 => {
            init_rp1_gpio();
        }
        PlatformType::RaspberryPi4 => {
            init_bcm2711_gpio();
        }
        _ => {
            println!("[GPIO] No GPIO initialization for this platform");
        }
    }
    
    println!("[GPIO] Initialization complete");
}

/// Initialize RP1 GPIO (Raspberry Pi 5)
fn init_rp1_gpio() {
    println!("[GPIO] Initializing RP1 GPIO controller...");
    
    // The RP1 is a separate I/O controller accessed via PCIe or dedicated bus
    // For early boot, we need to ensure the GPIO is accessible
    
    // Set activity LED pin as output if available
    // On Pi 5, activity LED is typically on GPIO 42 (through RP1)
    #[cfg(feature = "pi5_led_support")]
    {
        set_function(42, GpioFunction::Output);
        set_pull(42, GpioPull::None);
        println!("[GPIO] Activity LED (GPIO 42) configured");
    }
}

/// Initialize BCM2711 GPIO (Raspberry Pi 4)
fn init_bcm2711_gpio() {
    println!("[GPIO] Initializing BCM2711 GPIO controller...");
    
    // Set activity LED pin as output
    // On Pi 4, activity LED is on GPIO 42
    set_function(42, GpioFunction::Output);
    set_pull(42, GpioPull::None);
    println!("[GPIO] Activity LED (GPIO 42) configured");
}

/// Get the GPIO driver instance
fn driver() -> &'static mut GpioDriver {
    unsafe { &mut GPIO_DRIVER }
}

/// Set GPIO pin function
pub fn set_function(pin: u8, function: GpioFunction) {
    if pin > 53 {
        println!("[GPIO] Error: Pin {} out of range", pin);
        return;
    }
    
    let drv = driver();
    if !drv.initialized {
        println!("[GPIO] Error: Driver not initialized");
        return;
    }
    
    match drv.platform {
        PlatformType::RaspberryPi5 => {
            // RP1 uses a different register layout
            // For now, use simplified access
            set_rp1_function(pin, function);
        }
        PlatformType::RaspberryPi4 | PlatformType::GenericArm64 => {
            set_bcm2711_function(pin, function);
        }
        _ => {}
    }
}

/// Set function on RP1 (Raspberry Pi 5)
fn set_rp1_function(pin: u8, function: GpioFunction) {
    let drv = driver();
    let reg = drv.base_addr + rp1_regs::GIO_MODE;
    
    unsafe {
        // RP1 has different pin grouping
        // Each pin has its own configuration register
        let pin_reg = reg + (pin as usize) * 4;
        mmio::write32(pin_reg, function as u32);
        mmio::memory_barrier();
    }
}

/// Set function on BCM2711 (Raspberry Pi 4)
fn set_bcm2711_function(pin: u8, function: GpioFunction) {
    let drv = driver();
    
    // BCM2711 has 6 function select registers (10 pins each)
    let reg_index = (pin / 10) as usize;
    let bit_offset = ((pin % 10) * 3) as usize;
    
    let reg_addr = drv.base_addr + bcm2711_regs::GPFSEL0 + (reg_index * 4);
    
    unsafe {
        let mut value = mmio::read32(reg_addr);
        // Clear the 3 bits for this pin
        value &= !(0b111 << bit_offset);
        // Set the new function
        value |= (function as u32) << bit_offset;
        mmio::write32(reg_addr, value);
        mmio::memory_barrier();
    }
}

/// Set GPIO pin direction
pub fn set_direction(pin: u8, is_output: bool) {
    let function = if is_output {
        GpioFunction::Output
    } else {
        GpioFunction::Input
    };
    set_function(pin, function);
}

/// Set GPIO pin output level
pub fn set_level(pin: u8, high: bool) {
    if pin > 53 {
        return;
    }
    
    let drv = driver();
    if !drv.initialized {
        return;
    }
    
    match drv.platform {
        PlatformType::RaspberryPi5 => {
            set_rp1_level(pin, high);
        }
        _ => {
            set_bcm2711_level(pin, high);
        }
    }
}

/// Set level on RP1
fn set_rp1_level(pin: u8, high: bool) {
    let drv = driver();
    let reg_offset = if high {
        rp1_regs::GIO_SET
    } else {
        rp1_regs::GIO_CLR
    };
    
    unsafe {
        let reg = drv.base_addr + reg_offset;
        mmio::write32(reg, 1u32 << (pin % 32));
        mmio::memory_barrier();
    }
}

/// Set level on BCM2711
fn set_bcm2711_level(pin: u8, high: bool) {
    let drv = driver();
    let reg_offset = if high {
        if pin < 32 { bcm2711_regs::GPSET0 } else { bcm2711_regs::GPSET1 }
    } else {
        if pin < 32 { bcm2711_regs::GPCLR0 } else { bcm2711_regs::GPCLR1 }
    };
    
    let pin_bit = if pin < 32 { pin } else { pin - 32 };
    
    unsafe {
        let reg = drv.base_addr + reg_offset;
        mmio::write32(reg, 1u32 << pin_bit);
        mmio::memory_barrier();
    }
}

/// Read GPIO pin level
pub fn get_level(pin: u8) -> bool {
    if pin > 53 {
        return false;
    }
    
    let drv = driver();
    if !drv.initialized {
        return false;
    }
    
    match drv.platform {
        PlatformType::RaspberryPi5 => {
            get_rp1_level(pin)
        }
        _ => {
            get_bcm2711_level(pin)
        }
    }
}

/// Get level on RP1
fn get_rp1_level(pin: u8) -> bool {
    let drv = driver();
    
    unsafe {
        let reg = drv.base_addr + rp1_regs::GIO_DATA;
        let value = mmio::read32(reg);
        (value & (1u32 << (pin % 32))) != 0
    }
}

/// Get level on BCM2711
fn get_bcm2711_level(pin: u8) -> bool {
    let drv = driver();
    let reg_offset = if pin < 32 { bcm2711_regs::GPLEV0 } else { bcm2711_regs::GPLEV1 };
    let pin_bit = if pin < 32 { pin } else { pin - 32 };
    
    unsafe {
        let reg = drv.base_addr + reg_offset;
        let value = mmio::read32(reg);
        (value & (1u32 << pin_bit)) != 0
    }
}

/// Set pull-up/down resistor
pub fn set_pull(pin: u8, pull: GpioPull) {
    if pin > 53 {
        return;
    }
    
    let drv = driver();
    if !drv.initialized {
        return;
    }
    
    match drv.platform {
        PlatformType::RaspberryPi5 => {
            set_rp1_pull(pin, pull);
        }
        _ => {
            set_bcm2711_pull(pin, pull);
        }
    }
}

/// Set pull on RP1
fn set_rp1_pull(pin: u8, pull: GpioPull) {
    let drv = driver();
    
    unsafe {
        // Enable pull
        let en_reg = drv.base_addr + rp1_regs::GIO_PULL_EN;
        let mut en_value = mmio::read32(en_reg);
        
        // Set pull direction
        let dir_reg = drv.base_addr + rp1_regs::GIO_PULL_DIR;
        let mut dir_value = mmio::read32(dir_reg);
        
        match pull {
            GpioPull::None => {
                en_value &= !(1u32 << pin);
            }
            GpioPull::Up => {
                en_value |= 1u32 << pin;
                dir_value |= 1u32 << pin;
            }
            GpioPull::Down => {
                en_value |= 1u32 << pin;
                dir_value &= !(1u32 << pin);
            }
        }
        
        mmio::write32(en_reg, en_value);
        mmio::write32(dir_reg, dir_value);
        mmio::memory_barrier();
    }
}

/// Set pull on BCM2711 (using legacy method for compatibility)
fn set_bcm2711_pull(pin: u8, pull: GpioPull) {
    let drv = driver();
    
    // BCM2711 uses a different pull-up/down mechanism than earlier Pi models
    // It has dedicated pull-up/down registers per pin
    // This is a simplified implementation
    
    unsafe {
        // Use GPPUD method for older Pi models
        // First, write to GPPUD
        let pud_value = match pull {
            GpioPull::None => 0,
            GpioPull::Down => 1,
            GpioPull::Up => 2,
        };
        
        mmio::write32(drv.base_addr + bcm2711_regs::GPPUD, pud_value);
        
        // Wait 150 cycles
        for _ in 0..150 {
            core::arch::asm!("nop", options(nomem, nostack));
        }
        
        // Clock the pull resistor
        let clk_reg = if pin < 32 {
            bcm2711_regs::GPPUDCLK0
        } else {
            bcm2711_regs::GPPUDCLK1
        };
        let pin_bit = if pin < 32 { pin } else { pin - 32 };
        
        mmio::write32(drv.base_addr + clk_reg, 1u32 << pin_bit);
        
        // Wait 150 cycles
        for _ in 0..150 {
            core::arch::asm!("nop", options(nomem, nostack));
        }
        
        // Clear GPPUD and clock
        mmio::write32(drv.base_addr + bcm2711_regs::GPPUD, 0);
        mmio::write32(drv.base_addr + clk_reg, 0);
        mmio::memory_barrier();
    }
}

/// Enable GPIO interrupt
pub fn enable_interrupt(pin: u8, edge: GpioEdge) {
    if pin > 53 {
        return;
    }
    
    let drv = driver();
    if !drv.initialized {
        return;
    }
    
    match drv.platform {
        PlatformType::RaspberryPi5 => {
            enable_rp1_interrupt(pin, edge);
        }
        _ => {
            enable_bcm2711_interrupt(pin, edge);
        }
    }
}

/// Enable interrupt on RP1
fn enable_rp1_interrupt(pin: u8, edge: GpioEdge) {
    let drv = driver();
    
    unsafe {
        let type_reg = drv.base_addr + rp1_regs::GIO_INT_TYPE;
        let polarity_reg = drv.base_addr + rp1_regs::GIO_INT_POLARITY;
        let en_reg = drv.base_addr + rp1_regs::GIO_INT_EN;
        
        let pin_mask = 1u32 << pin;
        
        // Configure edge/level detection
        match edge {
            GpioEdge::None => {
                // Disable interrupt
                mmio::write32(en_reg, pin_mask);
            }
            GpioEdge::Rising => {
                let mut type_val = mmio::read32(type_reg);
                type_val |= pin_mask; // Edge triggered
                mmio::write32(type_reg, type_val);
                
                let mut pol_val = mmio::read32(polarity_reg);
                pol_val |= pin_mask; // Rising edge (high)
                mmio::write32(polarity_reg, pol_val);
                
                mmio::write32(en_reg, pin_mask);
            }
            GpioEdge::Falling => {
                let mut type_val = mmio::read32(type_reg);
                type_val |= pin_mask; // Edge triggered
                mmio::write32(type_reg, type_val);
                
                let mut pol_val = mmio::read32(polarity_reg);
                pol_val &= !pin_mask; // Falling edge (low)
                mmio::write32(polarity_reg, pol_val);
                
                mmio::write32(en_reg, pin_mask);
            }
            GpioEdge::Both => {
                let mut type_val = mmio::read32(type_reg);
                type_val |= pin_mask; // Edge triggered
                mmio::write32(type_reg, type_val);
                
                // Both edges - enable both rising and falling
                // RP1 may need special handling for both edges
                mmio::write32(en_reg, pin_mask);
            }
        }
        
        mmio::memory_barrier();
    }
}

/// Enable interrupt on BCM2711
fn enable_bcm2711_interrupt(pin: u8, edge: GpioEdge) {
    let drv = driver();
    
    unsafe {
        let pin_bit = if pin < 32 {
            1u32 << pin
        } else {
            1u32 << (pin - 32)
        };
        
        match edge {
            GpioEdge::Rising => {
                let reg = drv.base_addr + if pin < 32 {
                    bcm2711_regs::GPREN0
                } else {
                    bcm2711_regs::GPREN1
                };
                let mut value = mmio::read32(reg);
                value |= pin_bit;
                mmio::write32(reg, value);
            }
            GpioEdge::Falling => {
                let reg = drv.base_addr + if pin < 32 {
                    bcm2711_regs::GPFEN0
                } else {
                    bcm2711_regs::GPFEN1
                };
                let mut value = mmio::read32(reg);
                value |= pin_bit;
                mmio::write32(reg, value);
            }
            GpioEdge::Both => {
                // Enable both rising and falling
                let ren_reg = drv.base_addr + if pin < 32 {
                    bcm2711_regs::GPREN0
                } else {
                    bcm2711_regs::GPREN1
                };
                let fen_reg = drv.base_addr + if pin < 32 {
                    bcm2711_regs::GPFEN0
                } else {
                    bcm2711_regs::GPFEN1
                };
                
                let mut ren_value = mmio::read32(ren_reg);
                ren_value |= pin_bit;
                mmio::write32(ren_reg, ren_value);
                
                let mut fen_value = mmio::read32(fen_reg);
                fen_value |= pin_bit;
                mmio::write32(fen_reg, fen_value);
            }
            GpioEdge::None => {}
        }
        
        mmio::memory_barrier();
    }
}

/// Disable GPIO interrupt
pub fn disable_interrupt(pin: u8) {
    enable_interrupt(pin, GpioEdge::None);
}

/// Toggle GPIO pin output
pub fn toggle(pin: u8) {
    let current = get_level(pin);
    set_level(pin, !current);
}

/// Blink an LED on the specified pin
/// This is a blocking function - it will not return until the specified number of blinks
pub fn blink_led(pin: u8, count: u32, delay_ms: u32) {
    println!("[GPIO] Blinking LED on pin {} ({} blinks, {}ms delay)", pin, count, delay_ms);
    
    // Configure pin as output
    set_function(pin, GpioFunction::Output);
    set_pull(pin, GpioPull::None);
    
    // Perform blinks
    for i in 0..count {
        // LED on
        set_level(pin, true);
        crate::hal::delay::milliseconds(delay_ms);
        
        // LED off
        set_level(pin, false);
        crate::hal::delay::milliseconds(delay_ms);
        
        if (i + 1) % 5 == 0 {
            println!("[GPIO] Completed {}/{} blinks", i + 1, count);
        }
    }
    
    println!("[GPIO] Blink test complete");
}

/// Test function to blink the activity LED
pub fn test_activity_led() {
    // GPIO 42 is typically the activity LED on Raspberry Pi 4/5
    // Note: On Pi 5, the LED is controlled through RP1 and may be on a different pin
    let led_pin = 42u8;
    
    println!("[GPIO] Testing activity LED on pin {}", led_pin);
    blink_led(led_pin, 10, 200);
}

/// Print GPIO driver information
pub fn print_info() {
    let drv = driver();
    
    println!("GPIO Driver Information:");
    println!("  Initialized: {}", drv.initialized);
    println!("  Platform: {:?}", drv.platform);
    println!("  Base Address: 0x{:016X}", drv.base_addr);
}
