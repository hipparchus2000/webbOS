# WebbOS TODO List

**Last Updated:** 2026-02-03

---

## Critical Issues (Must Fix First)

- [x] **Mouse Pointer Refresh Issue** 🔥 FIXED!
  - ~~Mouse movement causes complete screen refresh~~
  - ~~This blocks desktop usability~~
  - Implemented cursor-only redrawing with background save/restore
  - Performance: ~4,551x improvement (1,024,000 pixels → 225 pixels per movement)
  - Status: COMPLETED 2026-02-03

---

## High Priority

- [ ] **FAT32 Desktop Integration**
  - [ ] Show `/Desktop` folder from FAT32 image as the desktop
  - [ ] Display files and folders from `/Desktop` on desktop
  - [ ] Enable file creation/saving to Desktop from apps
  - [ ] Enable saving to Desktop subfolders

- [ ] **Browser Testing & Verification**
  - [ ] Test browser launch from desktop
  - [ ] Verify browser can display web pages
  - [ ] Test navigation (back/forward, URL bar)
  - [ ] Test saving downloaded files to Desktop

- [ ] **App Store Architecture**
  - Current thought: Progressive Web Apps (PWA) instead of native packages
  - PWA manifest parsing (JSON)
  - Service worker support (simplified)
  - Install to desktop from URLs

---

## Medium Priority

- [ ] **File Manager Integration**
  - [ ] Browse FAT32 filesystem
  - [ ] Create/delete/rename files and folders
  - [ ] Copy/paste/move operations
  - [ ] File properties dialog

- [ ] **Code Cleanup**
  - [ ] Fix duplicate initialization messages (browser, crypto print twice)
  - [ ] Auto-detect kernel entry point instead of hardcoding
  - [ ] Remove debug print statements from browser init
  - [ ] Consistent logging format across modules

- [ ] **Desktop Polish**
  - [ ] Window manager improvements (dragging, resizing)
  - [ ] Better app launching feedback
  - [ ] Desktop wallpaper/background

- [ ] **Command Line Improvements**
  - [ ] Tab completion for commands
  - [ ] Command history (up/down arrows)
  - [ ] Better `help` output with categories

---

## Low Priority / Nice to Have

- [ ] **Performance Optimizations**
  - [ ] Reduce binary size
  - [ ] Faster heap allocation
  - [ ] Optimize graphics rendering (dirty rectangle tracking)
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
- [x] Mouse pointer refresh fix (2026-02-03)

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
