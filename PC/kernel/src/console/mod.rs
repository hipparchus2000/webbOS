//! Console output
//!
//! Provides VGA text mode output.

use core::fmt;
use spin::Mutex;

mod vga;

/// Global writer for console output
static WRITER: Mutex<ConsoleWriter> = Mutex::new(ConsoleWriter::new());

/// Console writer that outputs to VGA
struct ConsoleWriter {
    vga: Option<vga::Writer>,
}

impl ConsoleWriter {
    const fn new() -> Self {
        Self {
            vga: None,
        }
    }

    fn init(&mut self) {
        self.vga = Some(vga::Writer::new());
    }
}

impl fmt::Write for ConsoleWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        // Write to VGA only
        if let Some(ref mut vga) = self.vga {
            vga.write_str(s)?;
        }
        
        Ok(())
    }
}

/// Initialize console output
pub fn init() {
    WRITER.lock().init();
}

/// Get a character from input (keyboard only)
pub fn getchar() -> Option<u8> {
    // Check keyboard input via interrupt-driven input system
    if let Some(event) = crate::drivers::input::poll_event() {
        if event.event_type == crate::drivers::input::EventType::KeyPress {
            if event.ascii != 0 {
                return Some(event.ascii);
            }
        }
    }

    None
}

/// Print to console
#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    WRITER.lock().write_fmt(args).unwrap();
}

/// Print macro
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::console::_print(format_args!($($arg)*)));
}

/// Print with newline macro
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}
