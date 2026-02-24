//! VGA text mode driver

use core::fmt;

/// VGA buffer width
const BUFFER_WIDTH: usize = 80;
/// VGA buffer height
const BUFFER_HEIGHT: usize = 25;
/// VGA buffer address
const BUFFER_ADDR: u64 = 0xB8000;

/// Volatile wrapper for VGA buffer
#[repr(transparent)]
struct VolatileCell(u16);

impl VolatileCell {
    fn read(&self) -> u16 {
        unsafe { core::ptr::read_volatile(&self.0) }
    }
    
    fn write(&mut self, val: u16) {
        unsafe { core::ptr::write_volatile(&mut self.0, val) }
    }
}

/// VGA color codes
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

/// Color code (foreground + background)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    /// Create a new color code
    fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

/// Screen character
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

impl Default for ScreenChar {
    fn default() -> Self {
        Self {
            ascii_character: b' ',
            color_code: ColorCode::new(Color::Black, Color::Black),
        }
    }
}

/// VGA text buffer
#[repr(transparent)]
struct Buffer {
    chars: [[VolatileCell; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

/// VGA writer
pub struct Writer {
    column_position: usize,
    color_code: ColorCode,
    buffer: &'static mut Buffer,
}

impl Writer {
    /// Create a new writer
    pub fn new() -> Self {
        Self {
            column_position: 0,
            color_code: ColorCode::new(Color::White, Color::Black),
            buffer: unsafe { &mut *(BUFFER_ADDR as *mut Buffer) },
        }
    }
    
    /// Write a screen character
    fn write_char(&mut self, row: usize, col: usize, ch: ScreenChar) {
        let value = (ch.color_code.0 as u16) << 8 | (ch.ascii_character as u16);
        self.buffer.chars[row][col].write(value);
    }
    
    /// Read a screen character
    fn read_char(&self, row: usize, col: usize) -> ScreenChar {
        let value = self.buffer.chars[row][col].read();
        ScreenChar {
            ascii_character: (value & 0xFF) as u8,
            color_code: ColorCode((value >> 8) as u8),
        }
    }

    /// Write a byte to the buffer
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }

                let row = BUFFER_HEIGHT - 1;
                let col = self.column_position;

                let color_code = self.color_code;
                self.write_char(row, col, ScreenChar {
                    ascii_character: byte,
                    color_code,
                });
                self.column_position += 1;
            }
        }
    }

    /// Move to new line
    fn new_line(&mut self) {
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let character = self.read_char(row, col);
                self.write_char(row - 1, col, character);
            }
        }
        self.clear_row(BUFFER_HEIGHT - 1);
        self.column_position = 0;
    }

    /// Clear a row
    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };
        for col in 0..BUFFER_WIDTH {
            self.write_char(row, col, blank);
        }
    }

    /// Set color
    #[allow(dead_code)]
    pub fn set_color(&mut self, foreground: Color, background: Color) {
        self.color_code = ColorCode::new(foreground, background);
    }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            match byte {
                // printable ASCII byte or newline
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                // not part of printable ASCII range
                _ => self.write_byte(0xfe),
            }
        }
        Ok(())
    }
}


