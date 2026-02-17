//! Command Line Editor
//!
//! Provides command history, tab completion, and line editing for the shell.

use alloc::vec::Vec;
use alloc::string::String;
use spin::Mutex;
use lazy_static::lazy_static;

use crate::println;
use crate::print;

/// Maximum command history size
const MAX_HISTORY: usize = 20;

/// Maximum line length
const MAX_LINE_LEN: usize = 256;

/// Command history
pub struct CommandHistory {
    entries: Vec<String>,
    current_index: usize,
    temp_buffer: String,
}

impl CommandHistory {
    /// Create new empty history
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
            current_index: 0,
            temp_buffer: String::new(),
        }
    }

    /// Add a command to history
    pub fn add(&mut self, cmd: &str) {
        if cmd.trim().is_empty() {
            return;
        }

        // Don't add duplicate of most recent entry
        if let Some(last) = self.entries.last() {
            if last == cmd {
                return;
            }
        }

        // Add to history
        self.entries.push(String::from(cmd));

        // Trim history if too large
        if self.entries.len() > MAX_HISTORY {
            self.entries.remove(0);
        }

        // Reset index
        self.current_index = self.entries.len();
    }

    /// Get previous command (up arrow)
    pub fn prev(&mut self, current: &str) -> Option<&str> {
        if self.entries.is_empty() {
            return None;
        }

        // Save current buffer if at end
        if self.current_index == self.entries.len() {
            self.temp_buffer = String::from(current);
        }

        if self.current_index > 0 {
            self.current_index -= 1;
            Some(&self.entries[self.current_index])
        } else {
            None
        }
    }

    /// Get next command (down arrow)
    pub fn next(&mut self) -> Option<&str> {
        if self.entries.is_empty() {
            return None;
        }

        if self.current_index < self.entries.len() - 1 {
            self.current_index += 1;
            Some(&self.entries[self.current_index])
        } else if self.current_index == self.entries.len() - 1 {
            // Return to temp buffer
            self.current_index = self.entries.len();
            Some(&self.temp_buffer)
        } else {
            None
        }
    }

    /// Reset index to end
    pub fn reset_index(&mut self) {
        self.current_index = self.entries.len();
        self.temp_buffer.clear();
    }
}

/// Global command history
lazy_static! {
    static ref HISTORY: Mutex<CommandHistory> = Mutex::new(CommandHistory::new());
}

/// Available commands for tab completion
const COMMANDS: &[&str] = &[
    "help", "info", "memory", "processes", "network", "users",
    "launch", "browser", "navigate", "browsertest", "loadhtml", "save",
    "pwa", "apps", "install", "appstore",
    "clear", "test", "reboot", "shutdown",
];

/// Line editor state
pub struct LineEditor {
    buffer: [u8; MAX_LINE_LEN],
    pos: usize,
    escape_state: EscapeState,
}

#[derive(Clone, Copy, PartialEq)]
enum EscapeState {
    Normal,
    Escape,      // Received ESC
    EscapeBracket, // Received ESC[
}

impl LineEditor {
    /// Create new line editor
    pub const fn new() -> Self {
        Self {
            buffer: [0; MAX_LINE_LEN],
            pos: 0,
            escape_state: EscapeState::Normal,
        }
    }

    /// Get current buffer content
    pub fn buffer(&self) -> &[u8] {
        &self.buffer[..self.pos]
    }

    /// Clear the buffer
    fn clear(&mut self) {
        self.pos = 0;
        self.buffer[0] = 0;
    }

    /// Set buffer content
    fn set(&mut self, text: &str) {
        self.clear();
        for c in text.bytes() {
            if self.pos < MAX_LINE_LEN - 1 {
                self.buffer[self.pos] = c;
                self.pos += 1;
            }
        }
        self.buffer[self.pos] = 0;
    }

    /// Print current buffer (for redrawing)
    fn redraw(&self) {
        for i in 0..self.pos {
            print!("{}", self.buffer[i] as char);
        }
    }

    /// Erase current line on screen
    fn erase_line(&self) {
        // Move cursor to beginning of line
        for _ in 0..self.pos {
            print!("\x08");
        }
        // Clear to end of line
        print!("\x1B[K");
    }

    /// Handle a single key input
    /// Returns true if line is complete (Enter pressed)
    pub fn handle_key(&mut self, c: u8) -> bool {
        match self.escape_state {
            EscapeState::Normal => {
                match c {
                    b'\n' | b'\r' => {
                        println!();
                        self.buffer[self.pos] = 0;
                        return true;
                    }
                    8 | 127 => { // Backspace
                        if self.pos > 0 {
                            self.pos -= 1;
                            print!("\x08 \x08");
                        }
                    }
                    9 => { // Tab
                        self.handle_tab();
                    }
                    27 => { // ESC
                        self.escape_state = EscapeState::Escape;
                    }
                    c if self.pos < MAX_LINE_LEN - 1 => {
                        self.buffer[self.pos] = c;
                        self.pos += 1;
                        print!("{}", c as char);
                    }
                    _ => {}
                }
            }
            EscapeState::Escape => {
                if c == b'[' {
                    self.escape_state = EscapeState::EscapeBracket;
                } else {
                    self.escape_state = EscapeState::Normal;
                }
            }
            EscapeState::EscapeBracket => {
                // Handle arrow keys
                match c {
                    b'A' => { // Up arrow
                        self.handle_history_prev();
                    }
                    b'B' => { // Down arrow
                        self.handle_history_next();
                    }
                    b'C' => { // Right arrow (ignore for now)
                    }
                    b'D' => { // Left arrow (ignore for now)
                    }
                    _ => {}
                }
                self.escape_state = EscapeState::Normal;
            }
        }
        false
    }

    /// Handle tab completion
    fn handle_tab(&mut self) {
        let current = core::str::from_utf8(&self.buffer[..self.pos])
            .unwrap_or("")
            .trim();

        if current.is_empty() {
            return;
        }

        // Find matching commands
        let matches: Vec<&str> = COMMANDS
            .iter()
            .filter(|cmd| cmd.starts_with(current))
            .copied()
            .collect();

        if matches.len() == 1 {
            // Single match - complete it
            let completion = matches[0];
            self.erase_line();
            self.set(completion);
            self.redraw();
        } else if matches.len() > 1 {
            // Multiple matches - show them
            println!();
            for m in &matches {
                print!("{}  ", m);
            }
            println!();
            print!("$ ");
            self.redraw();
        }
    }

    /// Handle up arrow (previous history)
    fn handle_history_prev(&mut self) {
        let current = core::str::from_utf8(&self.buffer[..self.pos])
            .unwrap_or("");

        if let Some(cmd) = HISTORY.lock().prev(current) {
            self.erase_line();
            self.set(cmd);
            self.redraw();
        }
    }

    /// Handle down arrow (next history)
    fn handle_history_next(&mut self) {
        if let Some(cmd) = HISTORY.lock().next() {
            self.erase_line();
            self.set(cmd);
            self.redraw();
        }
    }

    /// Add current buffer to history and reset
    pub fn finish(&mut self) {
        if let Ok(cmd) = core::str::from_utf8(&self.buffer[..self.pos]) {
            HISTORY.lock().add(cmd.trim());
        }
        HISTORY.lock().reset_index();
        self.clear();
    }
}

/// Get command history reference (for display)
pub fn history() -> &'static Mutex<CommandHistory> {
    &HISTORY
}

/// Print command history
pub fn print_history() {
    let history = HISTORY.lock();
    println!("Command history:");
    for (i, cmd) in history.entries.iter().enumerate() {
        println!("  {}: {}", i + 1, cmd);
    }
}
