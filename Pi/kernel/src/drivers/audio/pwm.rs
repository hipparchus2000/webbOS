//! PWM Audio Driver for Raspberry Pi
//!
//! Uses the BCM2837 PWM peripheral for audio output on the 3.5mm jack.
//! The PWM runs at high frequency with sigma-delta modulation for audio quality.

use core::ptr::{read_volatile, write_volatile};
use core::sync::atomic::{AtomicBool, Ordering};
use spin::Mutex;

/// PWM Base address for Pi 3 (BCM2837)
const PWM_BASE: usize = 0x3F20C000;

/// Clock manager base address
const CM_BASE: usize = 0x3F101000;

/// PWM Control register offset
const PWM_CTL: usize = 0x00;
/// PWM Status register offset
const PWM_STA: usize = 0x04;
/// PWM DMA configuration
const PWM_DMAC: usize = 0x08;
/// PWM Channel 1 range
const PWM_RNG1: usize = 0x10;
/// PWM Channel 1 data
const PWM_DAT1: usize = 0x14;
/// PWM Channel 2 range
const PWM_RNG2: usize = 0x20;
/// PWM Channel 2 data
const PWM_DAT2: usize = 0x24;

/// Clock manager PWM control
const CM_PWMCTL: usize = 0xA0;
/// Clock manager PWM divisor
const CM_PWMDIV: usize = 0xA4;

/// PWM Control bits
const PWM_CTL_PWEN1: u32 = 1 << 0;  // Channel 1 enable
const PWM_CTL_MODE1: u32 = 1 << 1;  // Channel 1 mode
const PWM_CTL_RPTL1: u32 = 1 << 2;  // Channel 1 repeat
const PWM_CTL_SBIT1: u32 = 1 << 3;  // Channel 1 silence bit
const PWM_CTL_POLA1: u32 = 1 << 4;  // Channel 1 polarity
const PWM_CTL_USEF1: u32 = 1 << 5;  // Channel 1 FIFO mode
const PWM_CTL_CLRF1: u32 = 1 << 6;  // Clear FIFO
const PWM_CTL_MSEN1: u32 = 1 << 7;  // Channel 1 M/S mode

const PWM_CTL_PWEN2: u32 = 1 << 8;  // Channel 2 enable
const PWM_CTL_MODE2: u32 = 1 << 9;  // Channel 2 mode
const PWM_CTL_RPTL2: u32 = 1 << 10; // Channel 2 repeat
const PWM_CTL_SBIT2: u32 = 1 << 11; // Channel 2 silence bit
const PWM_CTL_POLA2: u32 = 1 << 12; // Channel 2 polarity
const PWM_CTL_USEF2: u32 = 1 << 13; // Channel 2 FIFO mode
const PWM_CTL_MSEN2: u32 = 1 << 15; // Channel 2 M/S mode

/// Clock manager password (required for all CM writes)
const CM_PASSWORD: u32 = 0x5A << 24;

/// Clock source (PLLD = 500MHz)
const CM_SRC_PLLD: u32 = 6;

/// PWM clock frequency (target: ~250MHz for audio)
const PWM_CLOCK_FREQ: u32 = 250_000_000;

/// Audio sample rate
const SAMPLE_RATE: u32 = 44100;

/// PWM range for M/S algorithm (10-bit resolution)
const PWM_RANGE: u32 = 1024;

/// Audio is initialized
static INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Current volume (0-100)
static VOLUME: Mutex<u8> = Mutex::new(80);

/// Sample buffer (simple FIFO)
static SAMPLE_BUFFER: Mutex<[i16; 256]> = Mutex::new([0; 256]);
static BUFFER_HEAD: Mutex<usize> = Mutex::new(0);
static BUFFER_TAIL: Mutex<usize> = Mutex::new(0);

/// Initialize PWM audio hardware
pub fn init() -> Result<(), super::AudioError> {
    if INITIALIZED.load(Ordering::SeqCst) {
        return Ok(());
    }
    
    crate::println!("[pwm_audio] Initializing PWM audio...");
    
    unsafe {
        // 1. Configure GPIO pins for PWM audio (GPIO 40/41 for Pi 3)
        // GPIO 40 = PWM0 (left channel), GPIO 41 = PWM1 (right channel)
        configure_gpio();
        
        // 2. Stop PWM before configuring
        write_volatile((PWM_BASE + PWM_CTL) as *mut u32, 0);
        
        // 3. Configure PWM clock
        configure_clock()?;
        
        // 4. Wait for clock to stabilize
        for _ in 0..1000 {
            core::arch::asm!("nop");
        }
        
        // 5. Configure PWM for audio
        // Use M/S mode for better audio quality
        write_volatile((PWM_BASE + PWM_RNG1) as *mut u32, PWM_RANGE);
        write_volatile((PWM_BASE + PWM_RNG2) as *mut u32, PWM_RANGE);
        
        // Clear FIFO and status
        write_volatile((PWM_BASE + PWM_CTL) as *mut u32, PWM_CTL_CLRF1);
        
        // Read status to clear errors
        let _ = read_volatile((PWM_BASE + PWM_STA) as *const u32);
        
        crate::println!("[pwm_audio] PWM configured");
    }
    
    INITIALIZED.store(true, Ordering::SeqCst);
    crate::println!("[pwm_audio] PWM audio initialized successfully");
    Ok(())
}

/// Configure GPIO pins for PWM audio
unsafe fn configure_gpio() {
    // GPIO 40 and 41 are on GPFSEL4
    const GPIO_BASE: usize = 0x3F200000;
    const GPFSEL4: usize = 0x10;
    
    let gpfsel4 = read_volatile((GPIO_BASE + GPFSEL4) as *const u32);
    
    // Set GPIO 40 (bits 0-2) to Alt0 (PWM0) - value 4
    // Set GPIO 41 (bits 3-5) to Alt0 (PWM1) - value 4
    let new_gpfsel4 = (gpfsel4 & !0x3F) | (4 << 0) | (4 << 3);
    
    write_volatile((GPIO_BASE + GPFSEL4) as *mut u32, new_gpfsel4);
    
    crate::println!("[pwm_audio] GPIO 40/41 configured for PWM");
}

/// Configure PWM clock
unsafe fn configure_clock() -> Result<(), super::AudioError> {
    // Stop the clock
    write_volatile((CM_BASE + CM_PWMCTL) as *mut u32, 
                   CM_PASSWORD | (1 << 5)); // Kill clock
    
    // Wait for clock to stop
    while (read_volatile((CM_BASE + CM_PWMCTL) as *const u32) & (1 << 7)) != 0 {}
    
    // Set divisor for desired frequency
    // Target: 250MHz from 500MHz PLLD
    // Divisor = 500MHz / 250MHz = 2
    let div = 2 << 12; // Integer part in bits 12-23
    write_volatile((CM_BASE + CM_PWMDIV) as *mut u32, CM_PASSWORD | div);
    
    // Start clock with PLLD source
    write_volatile((CM_BASE + CM_PWMCTL) as *mut u32,
                   CM_PASSWORD | (CM_SRC_PLLD << 0) | (1 << 4)); // Enable
    
    // Wait for clock to be ready
    while (read_volatile((CM_BASE + CM_PWMCTL) as *const u32) & (1 << 7)) == 0 {}
    
    crate::println!("[pwm_audio] Clock configured to 250MHz");
    Ok(())
}

/// Set volume (0-100)
pub fn set_volume(volume: u8) {
    *VOLUME.lock() = volume.min(100);
}

/// Start PWM audio output
pub fn start() -> Result<(), super::AudioError> {
    if !INITIALIZED.load(Ordering::SeqCst) {
        init()?;
    }
    
    unsafe {
        // Enable both PWM channels in M/S mode
        let ctl = PWM_CTL_PWEN1 | PWM_CTL_MSEN1 | PWM_CTL_PWEN2 | PWM_CTL_MSEN2;
        write_volatile((PWM_BASE + PWM_CTL) as *mut u32, ctl);
    }
    
    crate::println!("[pwm_audio] PWM audio started");
    Ok(())
}

/// Stop PWM audio output
pub fn stop() {
    unsafe {
        // Disable both channels
        write_volatile((PWM_BASE + PWM_CTL) as *mut u32, 0);
        
        // Set both channels to silence (midpoint)
        write_volatile((PWM_BASE + PWM_DAT1) as *mut u32, PWM_RANGE / 2);
        write_volatile((PWM_BASE + PWM_DAT2) as *mut u32, PWM_RANGE / 2);
    }
}

/// Write a single sample to PWM
/// Sample should be 16-bit signed (-32768 to 32767)
pub fn write_sample(sample: i16) -> Result<(), super::AudioError> {
    if !INITIALIZED.load(Ordering::SeqCst) {
        return Err(super::AudioError::NotFound);
    }
    
    // Convert 16-bit signed to PWM range
    // -32768 -> 0, 0 -> PWM_RANGE/2, 32767 -> PWM_RANGE
    let normalized = (sample as i32 + 32768) as u32;
    let pwm_value = (normalized * PWM_RANGE / 65536) as u32;
    
    unsafe {
        // Write to both channels (mono)
        write_volatile((PWM_BASE + PWM_DAT1) as *mut u32, pwm_value);
        write_volatile((PWM_BASE + PWM_DAT2) as *mut u32, pwm_value);
    }
    
    Ok(())
}

/// Write stereo samples
pub fn write_stereo(left: i16, right: i16) -> Result<(), super::AudioError> {
    if !INITIALIZED.load(Ordering::SeqCst) {
        return Err(super::AudioError::NotFound);
    }
    
    let left_norm = ((left as i32 + 32768) as u32 * PWM_RANGE / 65536) as u32;
    let right_norm = ((right as i32 + 32768) as u32 * PWM_RANGE / 65536) as u32;
    
    unsafe {
        write_volatile((PWM_BASE + PWM_DAT1) as *mut u32, left_norm);
        write_volatile((PWM_BASE + PWM_DAT2) as *mut u32, right_norm);
    }
    
    Ok(())
}

/// Get FIFO status
pub fn fifo_status() -> u32 {
    unsafe {
        read_volatile((PWM_BASE + PWM_STA) as *const u32)
    }
}

/// Check if FIFO is full
pub fn fifo_full() -> bool {
    let status = fifo_status();
    (status & (1 << 1)) != 0 // FIFO full flag
}

/// Check if FIFO is empty
pub fn fifo_empty() -> bool {
    let status = fifo_status();
    (status & (1 << 2)) != 0 // FIFO empty flag
}

/// Push sample to buffer
pub fn push_sample(sample: i16) -> Result<(), super::AudioError> {
    let mut buffer = SAMPLE_BUFFER.lock();
    let mut head = BUFFER_HEAD.lock();
    let tail = *BUFFER_TAIL.lock();
    
    let next_head = (*head + 1) % buffer.len();
    if next_head == tail {
        // Buffer full
        return Err(super::AudioError::Underrun);
    }
    
    buffer[*head] = sample;
    *head = next_head;
    
    Ok(())
}

/// Pop sample from buffer
pub fn pop_sample() -> Option<i16> {
    let mut buffer = SAMPLE_BUFFER.lock();
    let head = *BUFFER_HEAD.lock();
    let mut tail = BUFFER_TAIL.lock();
    
    if head == *tail {
        // Buffer empty
        return None;
    }
    
    let sample = buffer[*tail];
    *tail = (*tail + 1) % buffer.len();
    
    Some(sample)
}

/// Get buffer fill level (0-100%)
pub fn buffer_level() -> u8 {
    let buffer = SAMPLE_BUFFER.lock();
    let head = *BUFFER_HEAD.lock();
    let tail = *BUFFER_TAIL.lock();
    
    let count = if head >= tail {
        head - tail
    } else {
        buffer.len() - tail + head
    };
    
    (count * 100 / buffer.len()) as u8
}
