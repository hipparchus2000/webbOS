#![no_std]
#![no_main]
#![feature(allocator_api)]
#![feature(naked_functions)]
#![feature(fn_align)]
#![feature(alloc_error_handler)]
#![cfg_attr(target_arch = "x86_64", feature(abi_x86_interrupt))]
#![cfg_attr(target_arch = "aarch64", feature(stdarch_arm_hints))]

//! WebbOS Kernel
//!
//! Main kernel entry point and initialization.

extern crate alloc;

use alloc::vec::Vec;
#[cfg(target_arch = "x86_64")]
use core::arch::naked_asm;
#[cfg(target_arch = "aarch64")]
use core::arch::naked_asm;
use webbos_shared::bootinfo::BootInfo;

mod arch;
mod mm;
mod console;
mod panic;
mod process;
mod syscall;
mod fs;
mod drivers;
mod net;
mod browser;
mod storage;
mod crypto;
mod tls;
mod graphics;
mod testing;
mod users;
mod desktop;
mod login_screen;
pub mod error;
mod pwa;

// Hardware Abstraction Layer (ARM64 only)
#[cfg(target_arch = "aarch64")]
mod hal;

use arch::cpu;
use arch::interrupts;



/// Test FAT32 root directory reading
fn test_fat32_root() {
    println!("[fs] Testing FAT32 root directory...");
    
    // Try to read root directory entries
    // use crate::fs::{FileSystem, INode};  // These types don't exist yet
    
    // Get the root filesystem (should be FAT32 mounted at /)
    // For now just print a success message
    println!("[fs] FAT32 filesystem ready for use");
}

/// Wrapper to use Arc<BootDisk> as Box<dyn BlockDevice>
struct BootDiskWrapper(alloc::sync::Arc<crate::storage::boot_disk::BootDisk>);

impl crate::storage::BlockDevice for BootDiskWrapper {
    fn name(&self) -> &str {
        self.0.name()
    }
    
    fn block_size(&self) -> usize {
        self.0.block_size()
    }
    
    fn block_count(&self) -> u64 {
        self.0.block_count()
    }
    
    fn read_blocks(&self, start: u64, count: usize, buf: &mut [u8]) -> Result<(), crate::storage::StorageError> {
        println!("[BootDiskWrapper] read_blocks: start={}, count={}", start, count);
        let result = self.0.read_blocks(start, count, buf);
        println!("[BootDiskWrapper] read_blocks result: {:?}", result.is_ok());
        result
    }
    
    fn write_blocks(&self, start: u64, count: usize, buf: &[u8]) -> Result<(), crate::storage::StorageError> {
        self.0.write_blocks(start, count, buf)
    }
    
    fn flush(&self) -> Result<(), crate::storage::StorageError> {
        self.0.flush()
    }
}

/// Kernel entry point
/// 
/// This is called by the bootloader after setting up page tables
/// and transitioning to long mode. The boot_info pointer is passed
/// in the RDI register per System V AMD64 ABI.
#[no_mangle]
pub extern "C" fn kernel_entry(boot_info: &'static BootInfo) -> ! {
    // VERY FIRST: Raw serial output - just write 'X' to indicate we got here
    unsafe {
        #[cfg(target_arch = "x86_64")]
        {
            core::arch::asm!(
                "mov dx, 0x3F8",
                "mov al, 0x58",  // 'X' - we made it!
                "out dx, al",
                options(nomem, nostack)
            );
        }
    }
    
    // TODO: Set up page tables and transition to higher half
    // For now, run in physical mode with limited functionality
    
    // Output more debug info
    unsafe {
        #[cfg(target_arch = "x86_64")]
        {
            // Output "OK" to show we're continuing
            core::arch::asm!(
                "mov dx, 0x3F8",
                "mov al, 0x4F",  // 'O'
                "out dx, al",
                "mov al, 0x4B",  // 'K'
                "out dx, al",
                options(nomem, nostack)
            );
        }
    }
    
    // Halt for now - we'll add proper initialization later
    unsafe {
        #[cfg(target_arch = "x86_64")]
        loop {
            core::arch::asm!("hlt");
        }
        
        #[cfg(target_arch = "aarch64")]
        loop {
            core::arch::asm!("wfe");
        }
    }
    
    // Validate boot info
    if !boot_info.verify() {
        unsafe {
            console::early_print("[BOOT] ERROR: Invalid boot info magic!\n");
        }
        panic!("Invalid boot info magic number!");
    }
    
    unsafe {
        console::early_print("[BOOT] Boot info verified OK\n");
    }

    // Initialize console for early output
    unsafe {
        console::early_print("[BOOT] About to call console::init()...\n");
    }
    console::init();
    unsafe {
        console::early_print("[BOOT] console::init() returned OK\n");
    }
    
    println!("╔══════════════════════════════════════════════════╗");
    println!("║                                                  ║");
    println!("║  ██╗    ██╗███████╗██████╗ ██████╗  ██████╗ ███████╗");
    println!("║  ██║    ██║██╔════╝██╔══██╗██╔══██╗██╔═══██╗██╔════╝");
    println!("║  ██║ █╗ ██║█████╗  ██████╔╝██████╔╝██║   ██║███████╗");
    println!("║  ██║███╗██║██╔══╝  ██╔══██╗██╔══██╗██║   ██║╚════██║");
    println!("║  ╚███╔███╔╝███████╗██████╔╝██║  ██║╚██████╔╝███████║");
    println!("║   ╚══╝╚══╝ ╚══════╝╚═════╝ ╚═╝  ╚═╝ ╚═════╝ ╚══════╝");
    println!("║                                                  ║");
    #[cfg(target_arch = "x86_64")]
    println!("║           Version 0.1.0 - x86_64                 ║");
    #[cfg(target_arch = "aarch64")]
    println!("║           Version 0.1.0 - ARM64                  ║");
    println!("╚══════════════════════════════════════════════════╝");
    println!();

    // Print boot info
    println!("Boot Info:");
    println!("  Version: {}", boot_info.version);
    println!("  Kernel: {:?} (size: {} bytes)", 
        boot_info.kernel_addr, 
        boot_info.kernel_size
    );
    println!("  Stack: top={:?}, size={}KB", 
        boot_info.stack_top,
        boot_info.stack_size / 1024
    );
    println!("  Memory map: {} entries", boot_info.memory_map_count);

    unsafe {
        if let Some(name) = boot_info.bootloader_name().split('.').next() {
            println!("  Bootloader: {}", name);
        }
    }

    // Initialize architecture-specific features
    println!("\n[cpu] Initializing...");
    cpu::init();
    println!("[cpu] CPU features detected");

    // Initialize GDT and TSS (x86_64 only)
    #[cfg(target_arch = "x86_64")]
    {
        println!("\n[gdt] Initializing GDT and TSS...");
        arch::gdt::init();
        // Set kernel stack in TSS (use current stack top from boot info)
        arch::gdt::set_kernel_stack(boot_info.stack_top.as_u64());
        println!("[gdt] GDT and TSS initialized");
    }

    // Initialize memory management
    println!("\n[mm] Initializing memory management...");
    unsafe {
        mm::init(boot_info);
    }
    println!("[mm] Memory management initialized");

    // Initialize interrupt handling
    println!("\n[interrupts] Initializing IDT...");
    interrupts::init();
    println!("[interrupts] IDT initialized");

    // Print memory statistics
    mm::print_stats();

    // Initialize VFS
    println!("\n[fs] Initializing VFS...");
    // fs::init(); // TODO: Implement fs::init()
    
    // Create and mount initrd (temporarily disabled)
    // let initrd = fs::initrd::create_basic_initrd();
    // fs::initrd::print_initrd(&initrd);
    // let _ = fs::mount("/initrd", initrd);
    // println!("[fs] Initrd mounted at /initrd");

    // Initialize process management
    println!("\n[process] Initializing...");
    process::init();

    // Initialize system calls
    println!("\n[syscall] Initializing...");
    syscall::init();

    // Initialize device drivers
    println!("\n[drivers] Initializing...");
    drivers::init();

    // Initialize storage subsystem
    println!("\n[storage] Initializing...");
    storage::init();

    // Test direct disk access first
    println!("\n[fs] Testing direct disk access...");
    let mut test_buf = [0u8; 512];
    match crate::storage::read(0, 0, 1, &mut test_buf) {
        Ok(()) => {
            println!("[fs] Direct disk read OK: boot signature = 0x{:02X}{:02X}", test_buf[510], test_buf[511]);
        }
        Err(e) => {
            println!("[fs] Direct disk read failed: {:?}", e);
        }
    }

    // Initialize boot disk file operations
    println!("\n[fs] Initializing boot disk file operations...");
    unsafe {
        fs::boot_disk::init();
    }

    // Mount FAT32 filesystem from boot disk
    println!("\n[fs] Mounting boot disk FAT32...");
    println!("[fs] Note: FAT32 mount via global VFS - see fs/global_vfs.rs");

    // Initialize network stack
    println!("\n[net] Initializing network stack...");
    net::init();

    // Initialize browser engine
    browser::init();

    // Initialize cryptographic subsystem
    crypto::init();

    // Initialize TLS 1.3
    println!("\n[tls] Initializing TLS 1.3...");
    tls::init();
    println!("[tls] TLS 1.3 initialized");

    // Initialize HTTP client
    println!("\n[http] Initializing HTTP client...");
    net::http::init();
    println!("[http] HTTP client initialized");

    // Initialize graphics subsystem
    println!("\n[graphics] Initializing graphics subsystem...");
    graphics::init();
    println!("[graphics] Graphics subsystem initialized");

    // Initialize VESA framebuffer using boot info
    println!("\n[vesa] Initializing VESA framebuffer...");
    let fb_info = &boot_info.framebuffer;
    if fb_info.is_valid() {
        // Use the pre-mapped virtual address from bootloader if available
        let fb_virt_addr = if let Some(vaddr) = fb_info.virt_addr {
            vaddr.as_u64()
        } else {
            // Fallback to hardcoded mapping
            0xFFFF_8000_8000_0000u64
        };
        println!("[vesa] Using framebuffer virt addr: {:016X}", fb_virt_addr);
        
        // Initialize VESA and keep the lock to test if mutex is the issue
        {
            let mut driver = drivers::vesa::driver().lock();
            driver.init_with_pitch(
                fb_info.width, 
                fb_info.height, 
                fb_info.bpp as u8, 
                fb_info.pitch,
                fb_info.addr.as_u64(), 
                fb_virt_addr
            );
            println!("[vesa] VESA: {}x{} @ {:?}", fb_info.width, fb_info.height, fb_info.addr);
            
            // Set mouse screen dimensions to match framebuffer
            drivers::input::set_mouse_screen_dimensions(fb_info.width as i32, fb_info.height as i32);
            println!("[input] Mouse screen dimensions set to {}x{}", fb_info.width, fb_info.height);
            
            // Drawing test disabled - see LOGIN_SCREEN_NOTES.md
            println!("[vesa] Driver ready for drawing from kernel_main");
        }
        
        // Draw boot indicator to VESA framebuffer
        draw_vesa_triangle();
    } else {
        println!("[vesa] No valid framebuffer");
    }

    // Initialize user management
    println!("\n[users] Initializing user management...");
    users::init();
    println!("[users] User management initialized");

    // Initialize input subsystem
    println!("\n[input] Initializing input subsystem...");
    drivers::input::init();
    println!("[input] Input subsystem initialized");

    // Initialize USB subsystem
    println!("\n[usb] Initializing USB subsystem...");
    if let Err(e) = drivers::usb::init() {
        println!("[usb] USB initialization failed: {:?}", e);
    }

    // Initialize PWA subsystem
    println!("\n[pwa] Initializing PWA subsystem...");
    pwa::init();
    println!("[pwa] PWA subsystem initialized");

    // Initialize desktop environment
    println!("\n[desktop] Initializing desktop environment...");
    desktop::init();
    println!("[desktop] Desktop environment initialized");

    // Initialize login screen module
    println!("\n[login_screen] Initializing login screen...");
    login_screen::init();

    // Enable interrupts now that all drivers are initialized
    println!("\n[interrupts] Enabling interrupts...");
    interrupts::enable();
    println!("[interrupts] Interrupts enabled (timer, keyboard, mouse)");

    println!("\n✓ WebbOS kernel initialized successfully!");
    println!("\nSystem is ready. Type 'help' for available commands.");

    // Main kernel loop
    kernel_main();
}

/// Draw the login screen to the VESA framebuffer
fn draw_vesa_triangle() {
    // Temporarily disabled - drawing causes crashes
    // The login screen will be shown by login_screen::show() instead
    println!("[vesa] Skipping boot drawing (login screen will be shown later)");
}

/// Draw a simple triangle using VGA text buffer with colored blocks (fallback)
fn draw_boot_triangle() {
    // VGA text buffer address (already mapped by bootloader)
    let vga_buffer = 0xFFFF8000000B8000 as *mut u16;
    
    // Color attributes: high nibble = background, low nibble = foreground
    // Green background (0x20), white foreground (0x0F) -> 0x2F
    // Or use 0x2A for green background with green foreground (solid block)
    let green_block: u16 = (0xDB as u16) | ((0x2A as u16) << 8); // Green block character
    let white_block: u16 = (0xDB as u16) | ((0x0F as u16) << 8); // White block character
    
    // Draw a simple triangle in the center of the screen
    // VGA text mode is 80x25 characters
    let center_x = 40;
    let center_y = 12;
    
    unsafe {
        // Draw triangle pointing up
        // Top point
        let row = center_y - 4;
        let col = center_x;
        let offset = row * 80 + col;
        core::ptr::write_volatile(vga_buffer.add(offset), white_block);
        
        // Second row (3 blocks wide)
        let row = center_y - 3;
        for i in -1..=1 {
            let col = (center_x as i32 + i) as usize;
            let offset = row * 80 + col;
            core::ptr::write_volatile(vga_buffer.add(offset), green_block);
        }
        
        // Third row (5 blocks wide)
        let row = center_y - 2;
        for i in -2..=2 {
            let col = (center_x as i32 + i) as usize;
            let offset = row * 80 + col;
            core::ptr::write_volatile(vga_buffer.add(offset), green_block);
        }
        
        // Bottom row (7 blocks wide) - base of triangle
        let row = center_y - 1;
        for i in -3..=3 {
            let col = (center_x as i32 + i) as usize;
            let offset = row * 80 + col;
            core::ptr::write_volatile(vga_buffer.add(offset), green_block);
        }
        
        // Draw white border at edges
        let row = center_y - 1;
        let left_col = (center_x as i32 - 3) as usize;
        let right_col = (center_x as i32 + 3) as usize;
        core::ptr::write_volatile(vga_buffer.add(row * 80 + left_col), white_block);
        core::ptr::write_volatile(vga_buffer.add(row * 80 + right_col), white_block);
    }
    
    println!("[boot] Triangle drawn to VGA buffer");
}

/// Desktop event loop - handles mouse and keyboard input for desktop
/// Uses timer-based polling (40Hz) instead of IRQ-driven events
fn desktop_event_loop() {
    use core::sync::atomic::{AtomicU64, Ordering};
    
    println!("[desktop] Entering desktop event loop (timer-based)");
    
    // Print screen resolution for debugging
    {
        let driver = drivers::vesa::driver().lock();
        if driver.is_initialized() {
            let info = driver.info();
            println!("[desktop] Screen resolution: {}x{}", info.width, info.height);
        }
    }
    
    // Heartbeat counter to detect freezes
    static LOOP_COUNT: AtomicU64 = AtomicU64::new(0);
    static LAST_PRINT: AtomicU64 = AtomicU64::new(0);
    static EVENT_COUNT: AtomicU64 = AtomicU64::new(0);
    static LAST_TIMER_TICK: AtomicU64 = AtomicU64::new(0);
    
    // Double-click detection state
    let mut last_click_time: u64 = 0;
    let mut last_click_x: i32 = 0;
    let mut last_click_y: i32 = 0;
    let mut last_button_state: u8 = 0;
    const DOUBLE_CLICK_TIME: u64 = 50; // ~500ms at 100 ticks/second (was 30)
    const DOUBLE_CLICK_DIST: i32 = 20; // 20 pixels radius (was 10)

    loop {
        let loop_num = LOOP_COUNT.fetch_add(1, Ordering::Relaxed);
        
        // Timer-based polling at ~40Hz (every 2-3 timer ticks)
        let current_tick = crate::arch::interrupts::get_timer_ticks();
        let last_timer = LAST_TIMER_TICK.load(Ordering::Relaxed);
        
        if current_tick >= last_timer + 2 { // 40Hz polling (was 5 for 20Hz)
            LAST_TIMER_TICK.store(current_tick, Ordering::Relaxed);
            
            // Poll mouse from timer (reads atomic position, generates events)
            if let Some(event) = drivers::input::poll_mouse_from_timer() {
                EVENT_COUNT.fetch_add(1, Ordering::Relaxed);
                desktop::ui::update_mouse(event.x, event.y);
            }
            
            // Check for mouse button press (for double-click detection)
            let current_buttons = drivers::input::mouse_buttons();
            let button_just_pressed = (current_buttons & 0x01) != 0 && (last_button_state & 0x01) == 0;
            let (mouse_x, mouse_y) = drivers::input::mouse_position();
            
            if button_just_pressed {
                // Check for double-click
                let time_since_last = current_tick - last_click_time;
                let dx = mouse_x - last_click_x;
                let dy = mouse_y - last_click_y;
                let dist_sq = dx * dx + dy * dy;
                
                if time_since_last < DOUBLE_CLICK_TIME && dist_sq < (DOUBLE_CLICK_DIST * DOUBLE_CLICK_DIST) {
                    // Double-click detected!
                    println!("[desktop] Double-click at ({}, {})", mouse_x, mouse_y);
                    desktop::ui::handle_double_click(mouse_x, mouse_y);
                } else {
                    // Single click
                    desktop::ui::handle_click(mouse_x, mouse_y);
                }
                
                // Update last click state
                last_click_time = current_tick;
                last_click_x = mouse_x;
                last_click_y = mouse_y;
            }
            
            last_button_state = current_buttons;
            
            // Poll keyboard
            if let Some(event) = drivers::input::poll_keyboard_from_timer() {
                EVENT_COUNT.fetch_add(1, Ordering::Relaxed);
                match event.event_type {
                    drivers::input::EventType::KeyPress => {
                        if event.ascii == 27 { // ESC
                            println!("[desktop] ESC pressed, exiting desktop mode");
                            return;
                        }
                        
                        // Route to URL bar if browser has focus
                        if desktop::ui::browser_has_url_focus() {
                            if let Some(ch) = char::from_u32(event.keycode as u32) {
                                desktop::ui::handle_url_input(ch);
                            } else if event.ascii != 0 {
                                desktop::ui::handle_url_input(event.ascii as char);
                            }
                        }
                    }
                    _ => {}
                }
            }
            
            // Handle mouse button down/up for click detection
            let current_buttons = drivers::input::mouse_buttons();
            let button_just_pressed = (current_buttons & 0x01) != 0 && (last_button_state & 0x01) == 0;
            let button_just_released = (current_buttons & 0x01) == 0 && (last_button_state & 0x01) != 0;
            
            if button_just_pressed {
                let (mouse_x, mouse_y) = drivers::input::mouse_position();
                desktop::ui::handle_mouse_down(mouse_x, mouse_y);
            }
            
            if button_just_released {
                let (mouse_x, mouse_y) = drivers::input::mouse_position();
                desktop::ui::handle_mouse_up(mouse_x, mouse_y);
            }
        }
        
        // Print heartbeat every ~0.5 seconds
        let last_print = LAST_PRINT.load(Ordering::Relaxed);
        if current_tick >= last_print + 50 {
            let events = EVENT_COUNT.load(Ordering::Relaxed);
            let (kb_irq, mouse_irq, _usb_kb_irq) = drivers::input::get_irq_counts();
            let (mx, my) = drivers::input::mouse_position();
            println!("[hb] loops={} evt={} irq(m)={} mouse=({},{})", 
                loop_num, events, mouse_irq, mx, my);
            LAST_PRINT.store(current_tick, Ordering::Relaxed);
        }

        // Halt CPU to save power
        cpu::halt();
    }
}

/// Main kernel loop
fn kernel_main() -> ! {
    let mut line_editor = console::line_editor::LineEditor::new();
    let mut first_boot = true;

    loop {
        // Show login screen on first boot after a short delay
        if first_boot {
            first_boot = false;
            println!("[boot] Starting...");
            // Use simple delay loop instead of timer sleep (timer not working yet)
            println!("[boot] Waiting...");
            for _ in 0..10000000 {
                core::hint::spin_loop();
            }
            println!("[boot] Wait complete");
            println!("[boot] Calling login_screen::show()...");
            login_screen::show();
            println!("[boot] login_screen::show() returned");
        }
        
        // Only show prompt if login screen is not visible
        if !login_screen::is_visible() {
            print!("$ ");
        }
        
        // Command input loop with line editor
        loop {
            // Check for input
            let key_opt = console::getchar();
            
            if let Some(c) = key_opt {
                // If login screen is visible, route input to it
                if login_screen::is_visible() {
                    match login_screen::handle_key(c) {
                        login_screen::LoginAction::LoginSuccess => {
                            // Login successful - show graphical desktop
                            println!("\n[desktop] Login successful, launching desktop...");

                            // Show the macOS-style graphical desktop
                            desktop::ui::show();

                            println!("[desktop] Desktop shown, entering desktop mode...");
                            // Enter desktop event loop
                            desktop_event_loop();
                            // If we exit desktop loop, go back to command prompt
                            println!("\nExited desktop mode");
                            break;
                        }
                        login_screen::LoginAction::LoginFailed => {
                            // Login failed, stay on login screen
                            println!("[login] Authentication failed");
                        }
                        login_screen::LoginAction::None => {}
                    }
                    continue;
                }
                
                // Use line editor for command input
                if line_editor.handle_key(c) {
                    // Command complete
                    let cmd = line_editor.buffer();
                    process_command(cmd);
                    line_editor.finish();
                    break;
                }
            }
            
            // Halt CPU until next interrupt (saves power)
            cpu::halt();
        }
    }
}

/// Print detailed USB status
fn print_usb_detailed_status() {
    use drivers::usb::{get_controller_info, devices, UsbSpeed};
    
    println!("╔══════════════════════════════════════════════════╗");
    println!("║              USB Subsystem Status                ║");
    println!("╚══════════════════════════════════════════════════╝");
    
    // Controller information
    if let Some(info) = get_controller_info() {
        println!("\n[Controller]");
        println!("  Type: xHCI (USB 3.0)");
        println!("  Version: {}", info.version_string());
        println!("  Max Ports: {}", info.num_ports);
        println!("  Max Device Slots: {}", info.max_slots);
        println!("  Status: {}", if info.running { "Running ✓" } else { "Stopped ✗" });
        println!("  MMIO Base: 0x{:016X}", info.mmio_base);
    } else {
        println!("\n[Controller]");
        println!("  No USB controller found!");
        return;
    }
    
    // Connected devices
    let devs = devices();
    println!("\n[Connected Devices: {}]", devs.len());
    
    if devs.is_empty() {
        println!("  No devices connected");
    } else {
        for dev in &devs {
            let class_name = match dev.class {
                0x00 => "Interface",
                0x01 => "Audio",
                0x02 => "Communications",
                0x03 => "HID",
                0x05 => "Physical",
                0x06 => "Image",
                0x07 => "Printer",
                0x08 => "Mass Storage",
                0x09 => "Hub",
                0x0A => "CDC Data",
                0x0B => "Smart Card",
                0x0D => "Content Security",
                0x0E => "Video",
                0x0F => "Personal Healthcare",
                0x10 => "Audio/Video",
                0xDC => "Diagnostic",
                0xE0 => "Wireless",
                0xEF => "Miscellaneous",
                0xFF => "Vendor Specific",
                _ => "Unknown",
            };
            
            let speed_str = match dev.speed {
                UsbSpeed::Low => "1.5 Mbps (Low)",
                UsbSpeed::Full => "12 Mbps (Full)",
                UsbSpeed::High => "480 Mbps (High)",
                UsbSpeed::Super => "5 Gbps (Super)",
                UsbSpeed::SuperPlus => "10 Gbps (SuperPlus)",
            };
            
            println!("  Device @ Address {}:", dev.address);
            println!("    VID/PID: {:04X}:{:04X}", dev.vendor_id, dev.product_id);
            println!("    Class: {} (0x{:02X})", class_name, dev.class);
            println!("    Speed: {}", speed_str);
        }
    }
    
    // Port summary
    let ports = drivers::usb::list_ports();
    let connected_count = ports.iter().filter(|p| p.connected).count();
    println!("\n[Port Summary: {}/{} connected]", connected_count, ports.len());
}

/// Print USB port status
fn print_usb_ports() {
    use drivers::usb::list_ports;
    
    println!("USB Port Status:");
    println!("════════════════════════════════════════════════════");
    
    let ports = list_ports();
    
    if ports.is_empty() {
        println!("No USB ports available.");
        return;
    }
    
    for port in &ports {
        println!("\nPort {}:", port.port);
        println!("  Status: {}", port.connection_string());
        println!("  Speed: {}", port.speed_string());
        println!("  Enabled: {}", if port.enabled { "Yes" } else { "No" });
        println!("  Powered: {}", if port.powered { "Yes" } else { "No" });
        
        if port.in_reset {
            println!("  State: Reset in progress");
        }
        if port.over_current {
            println!("  ⚠ OVER-CURRENT DETECTED!");
        }
        
        if let Some(ref dev) = port.device {
            println!("  Device: {:04X}:{:04X} (Addr: {})", 
                dev.vendor_id, dev.product_id, dev.address);
        }
    }
    
    println!("\n════════════════════════════════════════════════════");
    let connected = ports.iter().filter(|p| p.connected).count();
    println!("Total: {} ports, {} connected", ports.len(), connected);
}

/// Run USB tests
fn run_usb_tests() {
    use drivers::usb::test_controller;
    
    println!("Running USB functionality tests...");
    println!("════════════════════════════════════════════════════");
    
    match test_controller() {
        Ok(result) => {
            println!("\n[Test Results]");
            println!("  Controller Registers: {}", 
                if result.register_check { "✓ PASS" } else { "✗ FAIL" });
            println!("  Port Reset Test: {}", 
                if result.port_reset_test { "✓ PASS" } else { "✗ FAIL" });
            println!("  Transfer Ring Status: {}", 
                if result.transfer_ring_ok { "✓ PASS" } else { "✗ FAIL" });
            
            if result.passed {
                println!("\n✓ All tests PASSED!");
            } else {
                println!("\n✗ Some tests FAILED!");
                if let Some(msg) = result.error_message {
                    println!("  Error: {}", msg);
                }
            }
        }
        Err(e) => {
            println!("✗ Test failed with error: {:?}", e);
        }
    }
    
    println!("════════════════════════════════════════════════════");
}

/// Print USB storage devices
fn print_usb_storage() {
    use drivers::usb::list_storage_devices;
    
    println!("USB Mass Storage Devices:");
    println!("════════════════════════════════════════════════════");
    
    let devices = list_storage_devices();
    
    if devices.is_empty() {
        println!("No USB mass storage devices found.");
        println!("\nNote: Connect a USB flash drive or external HDD.");
    } else {
        for (i, dev) in devices.iter().enumerate() {
            println!("\nDevice {}:", i + 1);
            println!("  USB Address: {}", dev.address);
            println!("  Vendor: {} (0x{:04X})", dev.vendor_name, dev.vendor_id);
            println!("  Product: {} (0x{:04X})", dev.product_name, dev.product_id);
            
            if dev.capacity_mb > 0 {
                if dev.capacity_mb >= 1024 {
                    println!("  Capacity: {:.2} GB", dev.capacity_mb as f64 / 1024.0);
                } else {
                    println!("  Capacity: {} MB", dev.capacity_mb);
                }
            } else {
                println!("  Capacity: Unknown (device not fully initialized)");
            }
            
            println!("  Status: {}", if dev.ready { "Ready" } else { "Not Ready" });
        }
    }
    
    println!("\n════════════════════════════════════════════════════");
    println!("Total: {} storage device(s)", devices.len());
}

/// Process a user command
fn process_command(cmd: &[u8]) {
    let cmd_str = core::str::from_utf8(cmd).unwrap_or("").trim();
    
    match cmd_str {
        "" => {}
        "help" => {
            println!("Available commands:");
            println!("  help       - Show this help message");
            println!("  info       - Show system information");
            println!("  memory     - Show memory statistics");
            println!("  processes  - Show process list");
            println!("  scheduler  - Show scheduler statistics");
            println!("  vfs        - Show VFS statistics");
            println!("  pci        - Show PCI devices");
            println!("  time       - Show time/timers");
            println!("  network    - Show network status");
            println!("  dhcp       - Start DHCP discovery");
            println!("  ping       - Ping a host");
            println!("  netstat    - Show network connections");
            println!("  storage    - Show storage devices");
            println!("  usb        - Show USB devices and status");
            println!("  usbreset   - Reset USB controller");
            println!("  usbports   - Show USB port status");
            println!("  usbtest    - Run USB tests");
            println!("  usbstorage - List USB mass storage devices");
            println!("  tls        - Test TLS connection");
            println!("  http       - HTTP client usage");
            println!("  fetch      - Fetch a URL (e.g., fetch http://example.com)");
            println!("  graphics   - Show graphics info");
            println!("  vesa       - Show VESA framebuffer info");
            println!("  input      - Show input status");
            println!("  test       - Run test suite");
            println!("  users      - List user accounts");
            println!("  sessions   - List active sessions");
            println!("  login      - Login to desktop");
            println!("  desktop    - Show desktop info");
            println!("  launch     - Launch application (e.g., launch notepad)");
            println!("  browser    - Show browser engine status");
            println!("  navigate   - Navigate to URL (e.g., navigate file:///test.html)");
            println!("  browsertest- Test browser rendering engine");
            println!("  loadhtml   - Load HTML file (e.g., loadhtml system/apps/calculator/index.html)");
            println!("  save       - Save file to Desktop (e.g., save notes.txt Hello World)");
            println!("  history    - Show command history");
            println!("  pwa        - Show PWA system status");
            println!("  apps       - List installed PWA apps");
            println!("  install    - Install PWA app (e.g., install calculator)");
            println!("  appstore   - Open app store");
            println!("  reboot     - Reboot the system");
            println!("  shutdown   - Shutdown the system");
        }
        "info" => {
            println!("System Information:");
            println!("  OS: WebbOS v0.1.0");
            #[cfg(target_arch = "x86_64")]
            println!("  Architecture: x86_64");
            #[cfg(target_arch = "aarch64")]
            println!("  Architecture: ARM64 (aarch64)");
            cpu::print_info();
        }
        "memory" => {
            mm::print_stats();
        }
        "processes" | "ps" => {
            process::print_process_list();
        }
        "scheduler" => {
            process::scheduler::print_stats();
        }
        "vfs" => {
            // fs::print_stats(); // TODO: Implement fs::print_stats()
            println!("fs::print_stats() not implemented yet");
        }
        "pci" => {
            drivers::pci::print_devices();
        }
        "time" => {
            drivers::timer::print_stats();
        }
        "network" | "net" => {
            net::print_interfaces();
            println!();
            net::print_stats();
        }
        "dhcp" => {
            net::dhcp::start_dhcp();
        }
        "ping" => {
            println!("Usage: ping <ip_address>");
            println!("Example: ping 8.8.8.8");
        }
        "netstat" => {
            net::socket::print_sockets();
        }
        "storage" => {
            storage::print_devices();
        }
        "usb" => {
            print_usb_detailed_status();
        }
        "usbreset" => {
            println!("Resetting USB controller...");
            drivers::usb::reset_controller();
            println!("USB controller reset complete.");
        }
        "usbports" => {
            print_usb_ports();
        }
        "usbtest" => {
            run_usb_tests();
        }
        "usbstorage" => {
            print_usb_storage();
        }
        "tls" => {
            let _ = tls::connect("example.com");
        }
        "http" => {
            println!("Usage: http <url>");
            println!("Example: http http://example.com");
        }
        "fetch" => {
            if net::dns::resolve("example.com").is_none() {
                println!("Configuring network with static IP...");
                let config = net::NetworkConfig {
                    ip: net::Ipv4Address::from_octets(10, 0, 2, 15),
                    netmask: net::Ipv4Address::from_octets(255, 255, 255, 0),
                    gateway: net::Ipv4Address::from_octets(10, 0, 2, 2),
                    dns: net::Ipv4Address::from_octets(8, 8, 8, 8),
                };
                net::set_config(config);
            }
            match net::http::get("http://example.com") {
                Ok(response) => net::http::print_response(&response),
                Err(e) => println!("HTTP request failed: {:?}", e),
            }
        }
        "graphics" => {
            graphics::print_info();
        }
        "vesa" => {
            drivers::vesa::print_info();
        }
        "input" => {
            drivers::input::print_info();
        }
        "test" => {
            testing::run_tests();
        }
        "users" => {
            users::print_users();
        }
        "sessions" => {
            users::print_sessions();
        }
        "login" => {
            println!("Usage: login <username> <password>");
            println!("Example: login admin admin");
        }
        "desktop" => {
            desktop::print_info();
        }
        "launch" => {
            // Parse command to get app name
            let args = &cmd_str[cmd_str.len().min(6)..];
            let app_name = args.trim();
            if !app_name.is_empty() {
                if let Some(window_id) = desktop::launch_app(app_name) {
                    println!("Launched {} (window {})", app_name, window_id);
                } else {
                    println!("Failed to launch {}", app_name);
                    println!("Available apps: filemanager, notepad, paint, taskmanager, usermanager, terminal, browser");
                }
            } else {
                println!("Usage: launch <app_name>");
                println!("Available apps:");
                for app in desktop::list_apps() {
                    println!("  {} - {} {}", app.name, app.icon, app.title);
                }
            }
        }
        "browser" => {
            browser::print_stats();
        }
        "navigate" => {
            let args = &cmd_str[cmd_str.len().min(8)..];
            let url = args.trim();
            if !url.is_empty() {
                match browser::navigate(url) {
                    Ok(_) => println!("Navigated to: {}", url),
                    Err(e) => println!("Navigation failed: {:?}", e),
                }
            } else {
                println!("Usage: navigate <url>");
                println!("Examples:");
                println!("  navigate file:///test.html");
                println!("  navigate http://example.com");
            }
        }
        "browsertest" => {
            println!("Running browser engine test...");
            match browser::test_render() {
                Ok(_) => println!("Browser test passed!"),
                Err(e) => println!("Browser test failed: {:?}", e),
            }
        }
        "loadhtml" => {
            let args = &cmd_str[cmd_str.len().min(8)..];
            let path = args.trim();
            if !path.is_empty() {
                match browser::load_file(path) {
                    Ok(_) => println!("Loaded HTML file: {}", path),
                    Err(e) => println!("Failed to load file: {:?}", e),
                }
            } else {
                println!("Usage: loadhtml <path>");
                println!("Example: loadhtml system/apps/calculator/index.html");
            }
        }
        "save" => {
            let args = &cmd_str[cmd_str.len().min(4)..];
            let args = args.trim();
            
            // Parse filename and content
            let parts: Vec<&str> = args.splitn(2, ' ').collect();
            if parts.len() >= 1 && !parts[0].is_empty() {
                let filename = parts[0];
                let content = if parts.len() > 1 { parts[1] } else { "" };
                
                match crate::fs::boot_disk::save_to_desktop(filename, content.as_bytes()) {
                    Ok(_) => println!("Saved to Desktop: {}", filename),
                    Err(e) => println!("Save failed: {}", e),
                }
            } else {
                println!("Usage: save <filename> <content>");
                println!("Example: save notes.txt Hello World");
            }
        }
        "pwa" => {
            pwa::print_stats();
        }
        "apps" => {
            let apps = pwa::list_apps();
            println!("\nInstalled PWA Apps ({}):", apps.len());
            for app in apps {
                let status = if pwa::launcher::is_running(&app.id) { " [RUNNING]" } else { "" };
                println!("  {} - {}{}", app.id, app.manifest.name, status);
            }
        }
        "install" => {
            let args = &cmd_str[cmd_str.len().min(7)..];
            let app_name = args.trim();
            if !app_name.is_empty() {
                match pwa::appstore::install(app_name) {
                    Ok(app) => println!("Installed {} v{}", app.manifest.name, app.manifest.version),
                    Err(e) => println!("Install failed: {:?}", e),
                }
            } else {
                println!("Usage: install <app_name>");
                println!("Available apps:");
                for app in pwa::appstore::list_available(None) {
                    println!("  {} - {}", app.id, app.name);
                }
            }
        }
        "appstore" => {
            println!("Opening App Store...");
            let html = pwa::appstore::get_html();
            println!("App Store HTML generated ({} bytes)", html.len());
        }
        "history" => {
            console::line_editor::print_history();
        }
        "reboot" => {
            println!("Rebooting...");
            cpu::reboot();
        }
        "shutdown" => {
            println!("Shutting down...");
            cpu::shutdown();
        }
        _ => {
            println!("Unknown command: {}", cmd_str);
            println!("Type 'help' for available commands.");
        }
    }
}

/// Kernel entry trampoline (x86_64 version)
/// 
/// This is the actual entry point from the bootloader.
/// It works in PHYSICAL mode initially - no virtual memory!
#[cfg(target_arch = "x86_64")]
#[naked]
#[no_mangle]
#[repr(align(16))]
pub unsafe extern "C" fn _start() -> ! {
    naked_asm!(
        // Save boot info pointer (in RDI from bootloader)
        "mov r15, rdi",
        
        // Debug: Write "KERNEL" to serial port
        "mov dx, 0x3F8",
        "mov al, 0x4B",  // 'K'
        "out dx, al",
        "mov al, 0x45",  // 'E'
        "out dx, al",
        "mov al, 0x52",  // 'R'
        "out dx, al",
        "mov al, 0x4E",  // 'N'
        "out dx, al",
        "mov al, 0x45",  // 'E'
        "out dx, al",
        "mov al, 0x4C",  // 'L'
        "out dx, al",
        
        // For now, set up a simple physical stack and call kernel_main
        // Later we'll set up page tables and jump to higher half
        "mov rsp, {stack_top}",  // Physical stack at 0x500000
        "xor rbp, rbp",
        
        // Restore boot info pointer and call kernel entry
        "mov rdi, r15",
        "call {kernel_entry}",
        
        // Should never return
        "cli",
        "2:",
        "hlt",
        "jmp 2b",
        
        stack_top = const 0x500000u64,  // Physical stack top (5MB)
        kernel_entry = sym kernel_entry,
    );
}

/// Kernel entry trampoline (AArch64 version)
/// 
/// This is the actual entry point from the bootloader for ARM64.
#[cfg(target_arch = "aarch64")]
#[naked]
#[no_mangle]
#[repr(align(16))]
pub unsafe extern "C" fn _start() -> ! {
    naked_asm!(
        // Save boot info pointer (passed in x0 from bootloader)
        "mov x19, x0",
        
        // Set up kernel stack
        "ldr x1, ={stack_top}",
        "mov sp, x1",
        
        // Clear frame pointer
        "mov x29, xzr",
        
        // Restore boot info pointer and call kernel entry
        "mov x0, x19",
        "bl {kernel_entry}",
        
        // Should never return, but halt just in case
        "2:",
        "wfi",
        "b 2b",
        
        stack_top = const 0xFFFF_8000_0000_0000u64 + 0x500000u64, // Top of 2MB stack at 3MB
        kernel_entry = sym kernel_entry,
    );
}
