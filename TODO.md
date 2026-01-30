# WebbOS TODO List

**Last Updated:** 2026-01-30

---

## Active Decisions to Make

- [ ] **WebAssembly Runtime** - Parser exists but execution is stubbed
  - Option A: Implement full WASM VM (complex, large effort)
  - Option B: Remove WASM, focus on JavaScript only
  - **Decision needed:** Do we need WASM for a browser OS?

- [ ] **App Store Architecture** 
  - Current thought: Progressive Web Apps (PWA) instead of native packages
  - PWA manifest parsing (JSON)
  - Service worker support (simplified)
  - Install to desktop from URLs

---

## High Priority

- [ ] **App Store - PWA Support**
  - [ ] Parse PWA manifest files
  - [ ] Install web apps to desktop
  - [ ] Add installed PWAs to start menu
  - [ ] Basic offline caching (simplified)
  - [ ] Command: `pwa install <url>`

- [ ] **Browser Integration**
  - [ ] Connect browser engine to desktop
  - [ ] Launch browser from desktop icon
  - [ ] Navigate to URLs from command line
  - [ ] Render basic HTML pages

---

## Medium Priority

- [ ] **Code Cleanup**
  - [ ] Fix duplicate initialization messages (browser, crypto print twice)
  - [ ] Auto-detect kernel entry point instead of hardcoding
  - [ ] Remove debug print statements from browser init
  - [ ] Consistent logging format across modules

- [ ] **Desktop Polish**
  - [ ] Login screen actually requires credentials
  - [ ] Window manager improvements (dragging, resizing)
  - [ ] Better app launching feedback
  - [ ] Desktop wallpaper/background

- [ ] **Command Line Improvements**
  - [ ] Tab completion for commands
  - [ ] Command history (up/down arrows)
  - [ ] Better `help` output with categories

---

## Low Priority / Nice to Have

- [ ] **Real Hardware Testing**
  - [ ] Boot on actual hardware
  - [ ] USB boot instructions
  - [ ] Hardware compatibility testing

- [ ] **Performance Optimizations**
  - [ ] Reduce binary size
  - [ ] Faster heap allocation
  - [ ] Optimize graphics rendering

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

---

## Questions / Research

1. Should we support service workers for PWAs? (Complex)
2. Do we need a real JavaScript engine or just parsing? (Currently just parsing)
3. Should browser tabs be separate processes? (Currently single-process)
4. Network security - certificate validation? (TLS 1.3 implemented but basic)

---

## Blocked / Waiting

- None currently
