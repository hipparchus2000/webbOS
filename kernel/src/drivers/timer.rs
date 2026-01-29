//! Programmable Interval Timer (PIT) and APIC Timer
//!
//! Provides timing services and preemptive scheduling.

use crate::println;

/// PIT frequency (Hz)
const PIT_FREQUENCY: u32 = 1193182;
/// Desired timer frequency (Hz) - 1000Hz = 1ms ticks
const TIMER_FREQUENCY: u32 = 1000;

/// Number of ticks since boot
static mut TICKS: u64 = 0;

/// Initialize the timer
pub fn init() {
    println!("[timer] Initializing PIT timer at {}Hz...", TIMER_FREQUENCY);

    unsafe {
        // Calculate PIT divisor
        let divisor = PIT_FREQUENCY / TIMER_FREQUENCY;
        
        // Set PIT channel 0 to mode 3 (square wave generator)
        // Command: 0b00110110 = Channel 0, Access mode: lobyte/hibyte, Mode 3, Binary
        core::arch::asm!(
            "mov al, 0x36",
            "out 0x43, al",
            options(nomem, nostack)
        );

        // Set divisor
        let low = (divisor & 0xFF) as u8;
        let high = ((divisor >> 8) & 0xFF) as u8;
        
        core::arch::asm!(
            "out 0x40, al",
            in("al") low,
            options(nomem, nostack)
        );
        
        core::arch::asm!(
            "out 0x40, al",
            in("al") high,
            options(nomem, nostack)
        );
    }

    println!("[timer] PIT timer initialized");
}

/// Get current tick count
pub fn ticks() -> u64 {
    unsafe { TICKS }
}

/// Get elapsed time in milliseconds
pub fn elapsed_ms() -> u64 {
    unsafe { TICKS * 1000 / TIMER_FREQUENCY as u64 }
}

/// Get elapsed time in seconds
pub fn elapsed_sec() -> u64 {
    unsafe { TICKS / TIMER_FREQUENCY as u64 }
}

/// Sleep for a number of milliseconds (busy wait)
pub fn sleep_ms(ms: u64) {
    let target = elapsed_ms() + ms;
    while elapsed_ms() < target {
        core::hint::spin_loop();
    }
}

/// Sleep for a number of seconds (busy wait)
pub fn sleep_sec(sec: u64) {
    sleep_ms(sec * 1000);
}

/// Timer interrupt handler
///
/// # Safety
/// This is called from interrupt context.
pub unsafe fn timer_interrupt() {
    TICKS += 1;
    
    // Call scheduler tick
    crate::process::scheduler::timer_tick();
}

/// Read current time from CMOS RTC
pub fn read_rtc() -> RtcTime {
    unsafe {
        // Read CMOS registers
        let second = read_cmos(0x00);
        let minute = read_cmos(0x02);
        let hour = read_cmos(0x04);
        let day = read_cmos(0x07);
        let month = read_cmos(0x08);
        let year = read_cmos(0x09);

        RtcTime {
            second: bcd_to_binary(second),
            minute: bcd_to_binary(minute),
            hour: bcd_to_binary(hour),
            day: bcd_to_binary(day),
            month: bcd_to_binary(month),
            year: 2000 + bcd_to_binary(year) as u16,
        }
    }
}

/// Read CMOS register
unsafe fn read_cmos(reg: u8) -> u8 {
    // Select register
    core::arch::asm!(
        "out 0x70, al",
        in("al") reg,
        options(nomem, nostack)
    );
    
    // Small delay
    for _ in 0..100 {
        core::arch::asm!("nop", options(nomem, nostack));
    }
    
    // Read value
    let val: u8;
    core::arch::asm!(
        "in al, 0x71",
        out("al") val,
        options(nomem, nostack)
    );
    
    val
}

/// Convert BCD to binary
fn bcd_to_binary(bcd: u8) -> u8 {
    ((bcd >> 4) * 10) + (bcd & 0x0F)
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
    
    let rtc = read_rtc();
    let formatted = rtc.format();
    if let Ok(time_str) = core::str::from_utf8(&formatted) {
        println!("  RTC: {}", time_str);
    }
}
