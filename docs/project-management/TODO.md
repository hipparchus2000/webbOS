# WebbOS TODO List

**Last Updated:** 2026-02-16

**Status:** x86_64 & aarch64 building and booting ✅

---

## Currently In Progress

- [x] **PNG Icon Display** ✅ COMPLETE
  - [x] Icons added to FAT32 image (system/icons/*.png) - 8 files
  - [x] Icon paths configured in desktop UI code
  - [x] PNG decoder implemented (no_std compatible) - RGB/RGBA, 8-bit
  - [x] Icon cache system with 1MB limit
  - [x] Integrated PNG loading in dock/desktop icon rendering
  - [x] Global VFS with FAT32 filesystem integration

---

## Critical Issues (Must Fix First)

- [x] **Mouse Cursor Freezing** 🔥 FIXED
  - Migrated from IRQ-driven events to timer-based atomic polling
  - Mouse IRQ handler now only updates AtomicI32 position values
  - Main loop polls at 40Hz timer interval
  - Status: RESOLVED

---

## High Priority

- [ ] **PNG Icon Display**
  - [x] Icons added to FAT32 image (system/icons/*.png) - 8 files
  - [x] Icon paths configured in desktop UI code
  - [x] Created add-files-to-image.py script
  - [ ] Implement PNG decoder (no_std compatible)
  - [ ] Load icon files from FAT32 filesystem
  - [ ] Render PNG icons in icon drawing functions
  - Currently using character fallback display

- [x] **FAT32 Desktop Integration** ✅ COMPLETE
  - [x] Show `/Desktop` folder from FAT32 image as the desktop
  - [x] Display files and folders from `/Desktop` on desktop
  - [x] File saving API (`save_to_desktop`, `write_file`)
  - [x] Shell `save` command for saving to Desktop
  - [x] Global VFS with FAT32 write support

- [x] **Browser Improvements** ✅ COMPLETE
  - [x] Browser launches from desktop icon click
  - [x] Browser address bar clickable for URL entry
  - [x] Keyboard input for typing URLs
  - [x] Text cursor/caret in address bar (blinks at ~500ms)
  - [x] URL navigation (Enter key loads page - logs URL)
  - [x] **Optimized browser window redraw** 🔥 
  - [x] Browser testing infrastructure (`browsertest` command)
  - [x] HTML file loading (`loadhtml` command)
  - [x] Improved navigate command with URL support
  - [ ] Test navigation (back/forward buttons - deferred)
  - [x] Test saving downloaded files to Desktop (via `save` command)

- [x] **App Store & PWA System** ✅ COMPLETE
  - [x] Open App Store when App Store icon clicked
  - [x] Created `/system/apps/` folder with PWA apps
  - [x] Added PWA apps: calculator, notepad, paint, settings
  - [x] PWA manifest parsing (JSON)
  - [x] PWA registry with install/uninstall support
  - [x] PWA launcher with browser integration
  - [x] Kernel shell commands: pwa, apps, install, appstore

---

## Medium Priority

- [x] **File Manager (Files App)** ✅ COMPLETE
  - [x] Open File Manager window when Files icon clicked
  - [x] Basic window with title bar, path bar, file list, status bar
  - [x] Display files with icons (📁 folder vs 📄 file)
  - [x] Show file sizes in human-readable format (B, KB, MB, GB)
  - [x] Click to select files (highlight in blue)
  - [x] Double-click folders to navigate
  - [x] Added `fs::read_dir()` API for directory listing
  - [x] Block device wrapper for FAT32 (`block_wrapper.rs`)
  - [x] Boot disk auto-mount at startup
  - [x] Navigate up to parent folder ("..")
  - [x] Delete files with Delete key (mock)
  - [x] Open files with Enter key
  - [x] File type detection (.txt → Notepad, .html → Browser)
  - [x] Fixed no_std compatibility (no `format!` macro, no `std::`)
  - [ ] Actual FAT32 filesystem integration (falls back to mock data if disk read fails)
  - [ ] Create/rename files and folders
  - [ ] Copy/paste/move operations
  - [ ] File properties dialog

- [ ] **Code Cleanup**
  - [ ] Fix duplicate initialization messages (browser, crypto print twice)
  - [ ] Auto-detect kernel entry point instead of hardcoding
  - [ ] Remove debug print statements from browser init
  - [ ] Consistent logging format across modules

- [x] **Desktop Wallpaper** ✅ COMPLETE
  - [x] Add wallpaper background to desktop
  - [x] Load wallpaper from FAT32 filesystem (BMP and PPM formats)
  - [x] Gradient fallback when no wallpaper found

- [ ] **Desktop Polish**
  - [ ] Window manager improvements (dragging, resizing)
  - [ ] Better app launching feedback

- [x] **Command Line Improvements** ✅ COMPLETE
  - [x] Tab completion for commands
  - [x] Command history (up/down arrows)
  - [x] `history` command to show command history
  - [ ] Better `help` output with categories (deferred)

---

## Low Priority / Nice to Have

- [ ] **Performance Optimizations**
  - [ ] **Browser window redraw is slow** 🔥 
    - Drawing entire browser window every frame is painful
    - Need dirty rectangle tracking (only redraw changed regions)
    - Consider double buffering for smoother rendering
  - [ ] Reduce binary size
  - [ ] Faster heap allocation
  - [ ] Optimize graphics rendering globally (dirty rectangle tracking)
  - [ ] Implement double buffering for smoother rendering

- [ ] **Real Hardware Testing**
  - [ ] Boot on actual hardware
  - [ ] USB boot instructions
  - [ ] Hardware compatibility testing

- [ ] **Additional Features**
  - [ ] Sound support (PC speaker or Intel HD Audio)
  - [ ] More file system drivers (NTFS read-only?)
  - [ ] USB mass storage support

---

## Completed ✅

- [x] UEFI Bootloader
- [x] Kernel boots successfully
- [x] Memory management (8MB heap)
- [x] Interrupt handling
- [x] Network stack (TCP/IP, TLS 1.3, HTTP)
- [x] Browser engine parsers (HTML, CSS, JS)
- [x] Desktop environment (7 apps)
- [x] User management (2 users)
- [x] Graphics (VESA framebuffer)
- [x] Input (keyboard, mouse)
- [x] Build system (Windows 11 native)
- [x] Disk image update script (Python, no WSL)
- [x] Login screen working
- [x] Icon infrastructure (FAT32 storage, icon paths in code) (2026-02-03)
- [x] Add-files-to-image.py script for FAT32 file additions (2026-02-03)

---

## Out of Scope / Deferred

- **WebAssembly Runtime** - Not needed for this project phase. Parser exists but execution engine is deferred to future work.

---

## Questions / Research

1. Should we support service workers for PWAs? (Complex)
2. How to implement efficient dirty rectangle tracking for mouse cursor?
3. Should browser tabs be separate processes? (Currently single-process)
4. Network security - certificate validation? (TLS 1.3 implemented but basic)

---

## Blocked / Waiting

- None - Mouse refresh issue resolved!
