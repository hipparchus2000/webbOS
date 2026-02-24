# WebbOS Port Comparison

## Overview

| Port | Target | Files | Code Size | Status |
|------|--------|-------|-----------|--------|
| **PC** | x86_64 UEFI | 68 | ~1.05 MB | Working |
| **Pi5** | ARM64 (Pi 5) | 81 | ~1.25 MB | In Progress |
| **Pi** | ARM64 (Pi 3/4) | 88 | ~1.34 MB | Most Complete |

## Architecture Differences

### Directory Structure

**PC Port:**
```
kernel/src/
├── arch/x86_64/          # x86_64 architecture code
├── drivers/
│   ├── input/            # Keyboard/mouse
│   ├── storage/          # Disk/ATA/NVMe
│   └── vesa/             # VESA framebuffer
├── graphics/             # Graphics context
└── ...
```

**Pi/Pi5 Port:**
```
kernel/src/
├── arch/aarch64/         # ARM64 architecture code
├── drivers/
│   ├── display/          # Pi framebuffer (mailbox)
│   ├── input/            # Keyboard/mouse
│   ├── mailbox/          # VideoCore mailbox
│   ├── sdio/             # SD card I/O
│   ├── storage/          # SD card storage
│   ├── usb/              # USB controller
│   └── wifi/             # WiFi (BCM43438/BCM43455)
├── graphics/             # Graphics context
└── ...
```

## Graphics/Painting Improvements (Pi → PC Backport Needed)

### 1. Dirty Rectangle Tracking ⭐ CRITICAL

**Pi Version (Has it):**
- `DirtyRect` struct for tracking screen regions needing redraw
- `mark_dirty()` - marks region as changed
- `mark_mouse_dirty()` - optimized mouse movement tracking
- `mark_full_redraw()` - flag for complete redraw
- Partial redraw support in `draw()` - only redraws changed regions

**PC Version (Missing):**
- Always clears entire screen and redraws everything
- No dirty region tracking
- Performance issue for animations and mouse movement

**Files to Update:**
- `PC/kernel/src/desktop/ui.rs` - Add dirty rectangle system

### 2. Screen Dimension Tracking

**Pi Version:**
- Stores `screen_width` and `screen_height` in DesktopUI
- Handles resolution changes gracefully

**PC Version:**
- Queries driver every frame
- No handling for resolution changes

### 3. Graphics Primitives

Both versions have similar primitives:
- `set_pixel()` with overflow checks ✓ (both have)
- `fill_rect()`, `draw_rect()` ✓ (both have)
- `hline()`, `vline()` ✓ (both have)
- `draw_line()`, `draw_circle()` ✓ (both have)

### 4. Framebuffer Access

**Both versions have:**
- Checked arithmetic for pixel offset calculations
- Support for 2, 3, and 4 bytes-per-pixel formats
- Volatile read/write for framebuffer access
- `save_under_cursor()` for mouse cursor rendering

## Feature Comparison

| Feature | PC | Pi5 | Pi |
|---------|----|-----|-----|
| **Graphics** ||||
| VESA framebuffer | ✓ | - | - |
| Pi mailbox framebuffer | - | ✓ | ✓ |
| Dirty rectangle tracking | ✗ | ✓ | ✓ |
| Double buffering | ✗ | ✗ | ✗ |
| **Storage** ||||
| ATA/NVMe | ✓ | - | - |
| SD card (SDIO) | - | ✓ | ✓ |
| **Network** ||||
| Ethernet (RTL8139) | ✓ | - | - |
| WiFi (WPA2) | - | ⚠️ | ✓ |
| **USB** ||||
| USB controller | ✓ | - | ✓ |
| USB keyboard/mouse | ✓ | - | ✓ |
| **Input** ||||
| PS/2 keyboard | ✓ | - | - |
| PS/2 mouse | ✓ | - | - |
| USB HID | ✓ | - | ✓ |
| **Other** ||||
| PCI enumeration | ✓ | - | - |
| Real-time clock | ✓ | - | - |

Legend: ✓ Working, ✗ Missing, ⚠️ In Progress, - Not applicable

## Specific Issues in PC Port

### 1. Painting Performance
The PC port redraws the entire screen every frame, causing:
- Flickering during mouse movement
- Poor performance for paint/canvas apps
- High CPU usage for simple animations

### 2. Mouse Cursor Rendering
Both versions use save-under buffer, but:
- PC version doesn't mark dirty regions after restoring cursor
- Can leave artifacts in some cases

## Recommendations for PC Port

### High Priority
1. **Port dirty rectangle tracking from Pi**
   - Copy `DirtyRect` struct and methods
   - Modify `draw()` to use partial redraws
   - Add `mark_mouse_dirty()` call in `update_mouse()`

### Medium Priority
2. **Add hline/vline optimizations**
   - Already present in VESA driver, but could be used more

### Low Priority
3. **Screen dimension caching**
   - Store dimensions in DesktopUI instead of querying driver

## Code Changes Required

### PC/kernel/src/desktop/ui.rs Changes:

```rust
// Add to DesktopUI struct:
pub struct DesktopUI {
    // ... existing fields ...
    // Dirty rectangle tracking for performance
    dirty_rects: Vec<DirtyRect>,
    full_redraw_needed: bool,
    screen_width: u32,
    screen_height: u32,
}

// Add DirtyRect struct:
pub struct DirtyRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

// Add methods:
impl DesktopUI {
    pub fn mark_dirty(&mut self, x: i32, y: i32, width: u32, height: u32) { ... }
    pub fn mark_mouse_dirty(&mut self) { ... }
    pub fn mark_full_redraw(&mut self) { ... }
    fn draw_region(&self, driver: &mut VesaDriver, rect: &DirtyRect) { ... }
}

// Modify draw():
pub fn draw(&mut self, driver: &mut VesaDriver) {
    if self.full_redraw_needed {
        // Full redraw
        self.full_redraw_needed = false;
    } else if !self.dirty_rects.is_empty() {
        // Partial redraw
        for rect in &self.dirty_rects {
            self.draw_region(driver, rect);
        }
    }
    // Always draw cursor
    self.dirty_rects.clear();
}

// Modify update_mouse():
pub fn update_mouse(&mut self, x: i32, y: i32) {
    // ... existing code ...
    self.mark_mouse_dirty();  // Add this
}
```

## Testing After Changes

1. **Mouse movement** - Should be smooth without flickering
2. **Paint app** - Drawing should not cause full screen redraws
3. **Icon selection** - Only selected icon should redraw
4. **Browser window** - Scrolling should only redraw changed regions
