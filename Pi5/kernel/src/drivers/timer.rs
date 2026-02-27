//! ARM Generic Timer
//!
//! The ARM Generic Timer provides a standard timer interface on ARM64.
//! It uses:
//! - CNTPCT_EL0: Physical Count register (read-only)
//! - CNTVCT_EL0: Virtual Count register (read-only)
//! - CNTFRQ_EL0: Counter Frequency register
//! - CNTP_TVAL_EL0: Physical Timer Value
//! - CNTP_CTL_EL0: Physical Timer Control

use crate::println;
use core::sync::atomic::{AtomicU64, Ordering};

/// Desired timer frequency (Hz) - 1000Hz = 1ms ticks
const TIMER_FREQUENCY: u32 = 1000;

/// Counter frequency (read from CNTFRQ_EL0 at boot)
static COUNTER_FREQ: AtomicU64 = AtomicU64::new(0);

/// Timer tick counter (incremented by IRQ handler)
static TIMER_TICKS: AtomicU64 = AtomicU64::new(0);

/// Initialize the ARM Generic Timer
pub fn init() {
    println!("[timer] Initializing ARM Generic Timer...");

    // Read the counter frequency
    let freq: u64;
    unsafe {
        core::arch::asm!(
            "mrs {0}, CNTFRQ_EL0",
            out(reg) freq,
        );
    }
    COUNTER_FREQ.store(freq, Ordering::SeqCst);
    
    println!("[timer] Counter frequency: {} Hz", freq);
    
    // Calculate timer interval for desired frequency
    let interval = freq / TIMER_FREQUENCY as u64;
    
    unsafe {
        // Set the timer value (will trigger interrupt when count reaches this)
        core::arch::asm!(
            "msr CNTP_TVAL_EL0, {0}",
            in(reg) interval,
        );
        
        // Enable the timer and unmask the interrupt
        core::arch::asm!(
            "msr CNTP_CTL_EL0, {0}",
            in(reg) 1u64, // Enable = 1, IMASK = 0
        );
    }
    
    println!("[timer] Timer interval: {} ticks", interval);

    println!("[timer] ARM Generic Timer initialized at {}Hz", TIMER_FREQUENCY);
}

/// Get current tick count
pub fn ticks() -> u64 {
    TIMER_TICKS.load(Ordering::SeqCst)
}

/// Alias for ticks() - used by USB driver
pub fn get_ticks() -> u64 {
    ticks()
}

/// Get elapsed time in milliseconds
pub fn elapsed_ms() -> u64 {
    let freq = COUNTER_FREQ.load(Ordering::SeqCst);
    if freq == 0 {
        return 0;
    }
    let count = read_counter();
    (count * 1000) / freq
}

/// Get elapsed time in seconds
pub fn elapsed_sec() -> u64 {
    let freq = COUNTER_FREQ.load(Ordering::SeqCst);
    if freq == 0 {
        return 0;
    }
    let count = read_counter();
    count / freq
}

/// Read the physical counter
fn read_counter() -> u64 {
    unsafe {
        let count: u64;
        core::arch::asm!(
            "mrs {0}, CNTPCT_EL0",
            out(reg) count,
        );
        count
    }
}

/// Sleep for a number of milliseconds (busy wait)
pub fn sleep_ms(ms: u64) {
    let start = elapsed_ms();
    while elapsed_ms() < start + ms {
        core::hint::spin_loop();
    }
}

/// Sleep for a number of seconds (busy wait)
pub fn sleep_sec(sec: u64) {
    sleep_ms(sec * 1000);
}

/// Timer interrupt handler - called from exceptions.rs
///
/// # Safety
/// This is called from interrupt context.
pub unsafe fn timer_interrupt() {
    // Increment tick counter
    TIMER_TICKS.fetch_add(1, Ordering::SeqCst);
    
    // Reload timer for next interrupt
    let interval = COUNTER_FREQ.load(Ordering::SeqCst) / TIMER_FREQUENCY as u64;
    core::arch::asm!(
        "msr CNTP_TVAL_EL0, {0}",
        in(reg) interval,
    );
    
    // Call scheduler tick
    crate::process::scheduler::timer_tick();
}

/// Read current time (stub - Pi doesn't have a battery-backed RTC)
pub fn read_rtc() -> RtcTime {
    // On Pi, we'd need to get time from the VideoCore via mailbox
    // For now, return a placeholder
    RtcTime {
        second: 0,
        minute: 0,
        hour: 0,
        day: 1,
        month: 1,
        year: 2024,
    }
}

/// RTC time structure
#[derive(Debug, Clone, Copy)]
pub struct RtcTime {
    pub second: u8,
    pub minute: u8,
    pub hour: u8,
    pub day: u8,
    pub month: u8,
    pub year: u16,
}

impl RtcTime {
    /// Format as string
    pub fn format(&self) -> [u8; 20] {
        let mut buf = [0u8; 20];
        
        fn write_num(buf: &mut [u8], pos: usize, num: u16, width: usize) {
            let s = format_num(num, width);
            buf[pos..pos+width].copy_from_slice(&s[..width]);
        }
        
        write_num(&mut buf, 0, self.year as u16, 4);
        buf[4] = b'-';
        write_num(&mut buf, 5, self.month as u16, 2);
        buf[7] = b'-';
        write_num(&mut buf, 8, self.day as u16, 2);
        buf[10] = b' ';
        write_num(&mut buf, 11, self.hour as u16, 2);
        buf[13] = b':';
        write_num(&mut buf, 14, self.minute as u16, 2);
        buf[16] = b':';
        write_num(&mut buf, 17, self.second as u16, 2);
        
        buf
    }
}

/// Format number as fixed-width decimal
fn format_num(num: u16, width: usize) -> [u8; 4] {
    let mut buf = [b'0'; 4];
    let mut n = num;
    
    for i in (0..width).rev() {
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
    }
    
    buf
}

/// Print timer statistics
pub fn print_stats() {
    println!("Timer Statistics:");
    println!("  Ticks: {}", ticks());
    println!("  Elapsed: {}s", elapsed_sec());
    println!("  Frequency: {}Hz", TIMER_FREQUENCY);
    println!("  Counter Freq: {} Hz", COUNTER_FREQ.load(Ordering::SeqCst));
    
    let rtc = read_rtc();
    let formatted = rtc.format();
    if let Ok(time_str) = core::str::from_utf8(&formatted) {
        println!("  RTC: {}", time_str);
    }
}
