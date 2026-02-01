# Login Screen Implementation Notes

## Overview
This document tracks the implementation of a pixel-based login screen for WebbOS. The login screen appears after a short delay following the green triangle boot animation.

## Current State (2026-01-31)
- ✅ Bootloader working (UEFI)
- ✅ Kernel booting successfully
- ✅ VESA framebuffer initialized (1024x768 or higher)
- ✅ Green triangle draws at boot
- ✅ VESA driver has pixel drawing primitives (set_pixel, fill_rect, draw_text, etc.)
- ✅ Desktop module exists with login logic (HTML-based, not pixel-based)

## Goal
Replace the HTML-based login with a pixel-drawn login screen that:
1. Shows after a 3-second delay from the triangle
2. Has a clean, modern UI drawn with pixels
3. Supports username/password input fields
4. Has a login button
5. Shows default credentials hint

## Implementation Plan

### Step 1: Create Login Screen Module
Create `kernel/src/login_screen.rs` with:
- `LoginScreen` struct to manage state
- `draw()` method to render UI elements
- `handle_input()` for keyboard navigation

### Step 2: UI Elements to Draw
Using VESA driver's existing primitives:
- `fill_rect()` - Background, input fields, button
- `draw_rect()` - Borders for inputs and button
- `draw_text()` - Labels, input text, title
- `fill_circle()` or `draw_circle()` - Optional decorative elements

### Step 3: Color Scheme
```rust
const BG_TOP: u32 = colors::rgb(102, 126, 234);      // #667eea - Purple/blue gradient top
const BG_BOTTOM: u32 = colors::rgb(118, 75, 162);    // #764ba2 - Purple gradient bottom
const CARD_BG: u32 = colors::WHITE;
const CARD_BORDER: u32 = colors::LIGHT_GRAY;
const TEXT_PRIMARY: u32 = colors::rgb(51, 51, 51);   // #333333
const TEXT_SECONDARY: u32 = colors::rgb(102, 102, 102); // #666666
const INPUT_BORDER: u32 = colors::rgb(224, 224, 224); // #e0e0e0
const INPUT_BORDER_FOCUS: u32 = colors::rgb(102, 126, 234); // #667eea
const BUTTON_BG: u32 = colors::rgb(102, 126, 234);   // #667eea
const BUTTON_TEXT: u32 = colors::WHITE;
const HINT_BG: u32 = colors::rgb(245, 245, 245);     // #f5f5f5
```

### Step 4: Layout (1024x768 centered)
```
Screen: 1024x768
Card: 360x400 centered (x=332, y=184)

Card contents:
- Logo (globe icon): x=512, y=220, scale=4
- Title "WebbOS": x=512, y=260, centered, scale=2
- Subtitle: x=512, y=290, centered, scale=1
- Username label: x=352, y=320
- Username input: x=352, y=340, w=320, h=40
- Password label: x=352, y=395
- Password input: x=352, y=415, w=320, h=40
- Login button: x=352, y=480, w=320, h=50
- Hint box: x=352, y=545, w=320, h=80
```

### Step 5: Integration Points
1. In `kernel_main()` after triangle draw:
   - Add delay loop (3 seconds using timer)
   - Call `login_screen::show()`

2. Modify input handling:
   - When login screen visible, route keys to login_screen
   - Tab switches between fields
   - Enter submits or moves to next field
   - Escape cancels

### Step 6: Drawing Gradient Background
The VESA driver doesn't have gradient fill, so implement:
```rust
fn draw_gradient_background(driver: &mut VesaDriver, color_top: u32, color_bottom: u32) {
    let height = driver.info().height;
    for y in 0..height {
        let t = y as f32 / height as f32;
        let color = interpolate_color(color_top, color_bottom, t);
        driver.hline(0, y as i32, driver.info().width, color);
    }
}
```

## Critical Implementation Details

### VM Timing (VERY IMPORTANT)
- **Windows 11 PC is VERY SLOW**
- **Must wait 60 seconds for QEMU to fully boot**
- Build commands can be run immediately
- But DO NOT interrupt QEMU before 60 seconds

### Build Commands (PowerShell)
```powershell
# Build bootloader
cargo +nightly-2025-01-15 build -p bootloader --target x86_64-unknown-uefi -Z build-std=core,compiler_builtins,alloc

# Build kernel
cargo +nightly-2025-01-15 build -p kernel --target x86_64-unknown-none -Z build-std=core,compiler_builtins,alloc

# Update disk image
python update-image.py webbos.img "EFI/BOOT/BOOTX64.EFI" target/x86_64-unknown-uefi/debug/bootloader.efi
python update-image.py webbos.img kernel.elf target/x86_64-unknown-none/debug/kernel

# Run (then WAIT 60 seconds!)
qemu-system-x86_64 -bios OVMF.fd -drive format=raw,file=webbos.img -m 128M -smp 1 -nographic -serial stdio
```

### One-liner for copy-paste:
```powershell
cargo +nightly-2025-01-15 build -p bootloader --target x86_64-unknown-uefi -Z build-std=core,compiler_builtins,alloc; cargo +nightly-2025-01-15 build -p kernel --target x86_64-unknown-none -Z build-std=core,compiler_builtins,alloc; python update-image.py webbos.img "EFI/BOOT/BOOTX64.EFI" target/x86_64-unknown-uefi/debug/bootloader.efi; python update-image.py webbos.img kernel.elf target/x86_64-unknown-none/debug/kernel; qemu-system-x86_64 -bios OVMF.fd -drive format=raw,file=webbos.img -m 128M -smp 1 -nographic -serial stdio
```

## Files to Modify
1. `kernel/src/main.rs` - Add login screen call after triangle, add delay
2. `kernel/src/lib.rs` (or add module) - Create `kernel/src/login_screen.rs`
3. `kernel/src/drivers/vesa/mod.rs` - May need to add gradient fill helper

## Testing Checklist
- [x] Build succeeds without errors
- [ ] QEMU boots (wait 60s!)
- [ ] Green triangle appears
- [ ] After 3 seconds, login screen appears
- [ ] Login screen has gradient background
- [ ] Login card is centered with white background
- [ ] Title "WebbOS" is visible
- [ ] Username/password fields visible
- [ ] Login button visible
- [ ] Hint box shows default credentials

## Implementation Status (2026-01-31)
- ✅ Created `kernel/src/login_screen.rs` with pixel-based UI
- ✅ Added login_screen module to main.rs
- ✅ Integrated with VESA driver for drawing
- ✅ Added 3-second delay before showing login screen (in kernel_main loop)
- ✅ Implemented keyboard input handling (Tab, Enter, Escape, Backspace)
- ✅ Integrated with desktop::login() for authentication
- ✅ Fixed String::repeat() issue for no_std
- ✅ Fixed timing - login screen now shows in kernel_main loop
- ⚠️ VESA drawing issue: System crashes when drawing after init

## Current Issue (CRITICAL)
The system crashes/reboots when trying to draw to the VESA framebuffer after initialization.

### Symptoms:
- Kernel boots successfully through all initialization
- VESA framebuffer initializes successfully
- "[vesa] Drawing triangle..." message prints
- System immediately reboots (no panic message shown)

### What Works:
- VESA driver initialization (including clear())
- All kernel subsystems (network, browser, crypto, etc.)
- Build process completes without errors

### What Crashes:
- fill_triangle() - original triangle drawing
- fill_rect() - even simple rectangle drawing
- Any VESA drawing after init phase

### Suspected Causes:
1. **Mutex deadlock**: Possible issue with locking VESA driver twice
2. **Framebuffer memory mapping**: Virtual address may be incorrect
3. **Page fault**: Writing to unmapped framebuffer memory
4. **Stack overflow**: Drawing functions use too much stack

### Files Modified:
- `kernel/src/login_screen.rs` - New login screen module
- `kernel/src/main.rs` - Added login_screen module and integration

### Code Structure (Ready to work once VESA issue fixed):
```rust
// In kernel_main() - shows login screen on first boot
if first_boot {
    first_boot = false;
    println!("[boot] Starting in 3 seconds...");
    drivers::timer::sleep_sec(3);
    login_screen::show();  // Draws pixel-based UI
}

// Input routing when login screen visible
if login_screen::is_visible() {
    match login_screen::handle_key(c) {
        LoginAction::LoginSuccess => { /* proceed to shell */ }
        LoginAction::LoginFailed => { /* stay on login */ }
        LoginAction::None => {}
    }
}
```

## Critical Finding - VESA Drawing Issue

The VESA framebuffer can be cleared during initialization (`clear()` works in `init()`), but ANY drawing after init causes a crash/reboot.

### Tested approaches that all fail:
1. Direct volatile writes to framebuffer memory
2. Using VESA driver's `set_pixel()` 
3. Using VESA driver's `fill_rect()`
4. Using VESA driver's `clear()` AFTER init
5. Small loops (50x50 pixels) vs large loops

### What works:
- `drivers::vesa::clear(0)` during VESA initialization
- Serial console output
- Everything else in the kernel

### Hypothesis:
Something changes after VESA init that makes framebuffer writes dangerous:
- Page table permissions change
- Interrupt handling changes  
- Memory mapping becomes invalid
- Stack overflow (unlikely - small loops crash too)

### Next Steps:
1. Compare working `clear()` (during init) vs non-working (after init)
2. Check if interrupts need to be disabled during drawing
3. Verify page table mappings for framebuffer region
4. Try using the driver's internal buffer then blitting

## Current Workaround:
The login screen module is implemented but drawing is disabled. The system boots successfully and shows the command prompt. To complete:
1. Fix VESA drawing issue
2. Enable login_screen::show() to draw to framebuffer
3. Test keyboard input routing

## VM Timing:
Remember to wait 60 seconds for QEMU on Windows 11

## Past Attempts (What NOT to do)
- ❌ Don't try to use HTML/CSS rendering - use pixel drawing
- ❌ Don't create complex widget systems - keep it simple
- ❌ Don't forget the 60 second wait time
- ❌ Don't use large allocations at boot time
- ❌ Don't modify the triangle drawing code - add AFTER it

## Notes for Future Agents
1. Always check this file before modifying login screen code
2. The VESA driver is already initialized when login screen shows
3. Use the existing `drivers::vesa::driver()` to get the mutex
4. Keep UI simple - rectangle-based with text
5. Test with `login admin admin` to verify it works
