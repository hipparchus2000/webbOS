//! Console output
//!
//! Provides VGA text mode and serial port output.

use core::fmt;
use spin::Mutex;

mod vga;
mod serial;

/// Global writer for console output
static WRITER: Mutex<ConsoleWriter> = Mutex::new(ConsoleWriter::new());

/// Console writer that outputs to both VGA and serial
struct ConsoleWriter {
    vga: Option<vga::Writer>,
    serial: Option<serial::SerialPort>,
}

impl ConsoleWriter {
    const fn new() -> Self {
        Self {
            vga: None,
            serial: None,
        }
    }

    fn init(&mut self) {
        self.vga = Some(vga::Writer::new());
        self.serial = Some(serial::SerialPort::new(serial::COM1));
    }
}

impl fmt::Write for ConsoleWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        // Write to VGA
        if let Some(ref mut vga) = self.vga {
            vga.write_str(s)?;
        }
        
        // Write to serial
        if let Some(ref mut serial) = self.serial {
            serial.write_str(s)?;
        }
        
        Ok(())
    }
}

/// Initialize console output
pub fn init() {
    WRITER.lock().init();
}

/// Get a character from input
pub fn getchar() -> Option<u8> {
    // Try serial first, then keyboard
    if let Some(c) = serial::try_receive() {
        return Some(c);
    }
    
    // TODO: Add PS/2 keyboard support
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
