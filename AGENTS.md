# WebbOS Agent Guidelines

This document contains important guidelines for agents working on the WebbOS codebase. **Do not ignore these requirements.**

## Critical Requirements

### 0. Zero Warnings Policy ⚠️ MANDATORY
- **ALL warnings must be fixed before submitting code**
- Treat warnings as errors - no exceptions
- If a warning cannot be fixed, use `#[allow(...)]` with a comment explaining why
- Common warning fixes:
  - `unused_imports` - Remove the import
  - `dead_code` - Either use the code or remove it, or `#[allow(dead_code)]` if intentionally kept
  - `unused_variables` - Prefix with `_` or remove
  - `unused_mut` - Remove `mut` keyword
  - `unused_doc_comments` - Move doc comments to proper location

### 1. Debug Output
- **PRIMARY**: Use debug buffer system (see below) - stores messages for bootloader to display
- **SECONDARY**: Framebuffer/VGA pixels for visual feedback
- **NEVER** use serial port debugging
- Serial debugging is prohibited - remove all serial port references

#### Debug Buffer System (Preferred)
The kernel stores debug messages in a circular buffer. If the kernel crashes or returns to bootloader, the bootloader can display these messages using UEFI console services.

```rust
// In kernel code:
use crate::debug_log;

debug_log!("Initializing subsystem...");
debug_log!("Value: {}", some_value);
```

Benefits:
- Works even if framebuffer isn't initialized yet
- Captures full debug history
- Bootloader displays messages using working UEFI console
- No magic numbers or hardware dependencies

See: `kernel/src/debug_log.rs` for implementation details.

### 2. User Interface
- **NO command line interface** - WebbOS boots directly to GUI
- Remove all CLI code, command prompts, and shell functionality
- System should boot to graphical desktop/login screen immediately

### 3. Code Quality Standards

#### No Magic Numbers
- **ALL** numeric offsets must be defined as named constants
- Hardware addresses, offsets, and sizes go in `src/arch/constants.rs` or `src/drivers/constants.rs`
- Example: Instead of `0xB8000`, use `VGA_TEXT_BUFFER_ADDR`

#### Example - Bad:
```rust
// BAD - Magic numbers!
unsafe {
    core::ptr::write_volatile(0xB8000 as *mut u8, 0x4B);
    core::ptr::write_volatile(0xB8001 as *mut u8, 0x0F);
    let fb = 0xFFFF800000000000 as *mut u32;
    fb.offset(100).write(0xFF00FF00);
}
```

#### Example - Good:
```rust
// GOOD - Named constants
use crate::arch::constants::{VGA_TEXT_BUFFER_ADDR, FRAMEBUFFER_BASE};
use crate::drivers::vesa::colors::{GREEN, BLACK};

unsafe {
    write_vga_char(0, 0, 'K', GREEN, BLACK);
    write_framebuffer_pixel(100, 0, COLOR_GREEN);
}
```

### 4. File Organization
- Hardware constants: `src/arch/constants.rs` or `src/drivers/constants.rs`
- Driver-specific constants: In driver's module (e.g., `src/drivers/pci/constants.rs`)
- No hardcoded addresses in main code

### 5. Boot Flow
1. UEFI bootloader loads kernel
2. Kernel initializes graphics immediately
3. Show login screen (GUI, not CLI)
4. After login, show desktop environment
5. No text mode shell ever appears

## Checklist Before Submitting Changes

- [ ] No serial port code (`0x3F8`, `COM1`, `serial::`)
- [ ] No command line interface code
- [ ] No magic numbers (all numeric constants named)
- [ ] All debug output goes to framebuffer
- [ ] Code follows Rust naming conventions
- [ ] Hardware addresses in constants files
- [ ] **ZERO warnings** - Treat warnings as errors
- [ ] Build passes with `cargo build --release` without any warnings

## Current Project Status

### Ports
- **PC**: x86_64 UEFI - **ZERO WARNINGS** ✅, boots to kernel
- **Pi**: ARM64 Pi 3/4 - Most complete
- **Pi5**: ARM64 Pi 5 - In progress

### Recent Changes (2026-02-25)
- ✅ **ZERO WARNINGS** - Kernel and bootloader now build with 0 warnings
- ✅ Removed all serial port debugging code
- ✅ Removed CLI (kernel boots directly to GUI login)
- ✅ Created hardware constants file (no magic numbers)
- ✅ Added AGENTS.md with coding standards
- ✅ Added debug buffer system for bootloader communication
- ✅ Warning count: 474 → 0 (kernel + bootloader)

### Known Issues
- PC port: Boot to kernel transition needs testing
- All ports: Continue auditing for magic numbers

### Build Commands
```powershell
cd PC
./build.bat release
./run.bat
```

## Coding Standards

### Rust Conventions
- Use `snake_case` for functions and variables
- Use `CamelCase` for types and traits
- Use `SCREAMING_SNAKE_CASE` for constants
- No `unwrap()` in production code
- Proper error handling with `Result<T, E>`

### Documentation
- All public functions must have doc comments
- All modules must have module-level documentation
- Constants must document their purpose

### Safety
- Minimize `unsafe` blocks
- Document safety invariants for each unsafe block
- Use safe abstractions where possible

---

**Last Updated**: 2026-02-25
**Author**: Project Lead
