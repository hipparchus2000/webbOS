# Raspberry Pi Display Driver Documentation

## Overview

This directory contains the Raspberry Pi display drivers that use the VideoCore GPU
mailbox interface for framebuffer allocation. These drivers replace the x86_64
VESA drivers while maintaining API compatibility.

## Modules

### `mailbox/mod.rs`

VideoCore Mailbox driver for CPU-GPU communication.

**Base Addresses:**
- Pi 3 (BCM2837): `0x3F00B880`
- Pi 4 (BCM2711): `0xFE00B880`

**Registers:**
- `0x00` - Read register
- `0x18` - Status register
- `0x20` - Write register

**Channels:**
- Channel 8: Property interface (ARM -> VC) - used for framebuffer config

**Key Functions:**
```rust
mailbox::init()                          // Initialize mailbox
mailbox::set_base_address(addr)          // Set Pi 3 or Pi 4 base
mailbox::print_info()                    // Print board info
```

### `display/pi_framebuffer.rs`

Pi Framebuffer driver using the mailbox property interface.

**Framebuffer Configuration Tags:**
- `0x48003` - Set physical size (display resolution)
- `0x48004` - Set virtual size (buffer size)
- `0x48005` - Set depth (bits per pixel)
- `0x48006` - Set pixel order (0=BGR, 1=RGB)
- `0x40001` - Allocate framebuffer
- `0x40008` - Get pitch (bytes per scanline)

**Key Functions:**
```rust
pi_framebuffer::init(width, height, bpp)     // Initialize framebuffer
pi_framebuffer::clear(color)                 // Clear screen
pi_framebuffer::set_pixel(x, y, color)       // Draw pixel
pi_framebuffer::fill_rect(x, y, w, h, color) // Draw rectangle
pi_framebuffer::draw_text(text, x, y, color, scale) // Draw text
```

### `display/mod.rs`

Display module that exports the Pi framebuffer with a clean API.

### `vesa` (Compatibility Layer)

The `drivers::vesa` module is now a compatibility layer that re-exports
`pi_framebuffer` with a VESA-compatible API. This allows existing x86_64 code
to work with minimal changes.

## Usage Example

```rust
use crate::drivers::display;

// Initialize display (1024x768 @ 32bpp)
if display::init(1024, 768, 32) {
    // Clear to black
    display::clear(0xFF000000);
    
    // Draw a red rectangle
    display::fill_rect(100, 100, 200, 150, 0xFFFF0000);
    
    // Draw text
    display::draw_text("Hello Pi!", 150, 150, 0xFFFFFFFF, 2);
}
```

## Compatibility with x86_64 VESA Code

Existing code using `drivers::vesa` will continue to work:

```rust
// This works on both x86_64 and Pi
drivers::vesa::init_with_pitch(width, height, bpp, pitch, phys_addr, virt_addr);
drivers::vesa::clear(color);
drivers::vesa::set_pixel(x, y, color);
```

On Pi, the `phys_addr` and `virt_addr` parameters are ignored since the
framebuffer is allocated by the GPU via the mailbox interface.

## Memory Layout

The framebuffer is allocated by the VideoCore GPU and returned as a bus address
(converted to physical address by masking with `0x3FFFFFFF`). The kernel must
map this physical address to a virtual address for CPU access.

```
GPU Allocates -> Bus Address -> Physical Address -> Virtual Address
```

The framebuffer is uncached, allowing direct pixel manipulation without
cache coherency issues.
