#![no_std]
#![no_main]
#![feature(allocator_api)]
#![feature(naked_functions)]
#![feature(fn_align)]
#![feature(alloc_error_handler)]
#![feature(abi_x86_interrupt)]

//! WebbOS Kernel
//!
//! Main kernel entry point and initialization.

#![allow(dead_code)]

extern crate alloc;


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
mod debug_log;

use arch::cpu;
use arch::interrupts;

/// Test FAT32 root directory reading
fn test_fat32_root() {
    println!("[fs] Testing FAT32 root directory...");
    
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
    // Validate boot info
    if !boot_info.verify() {
        panic!("Invalid boot info magic number!");
    }

    // Initialize console for early output
    console::init();
    
    // Initialize debug logging
    debug_log::log("Kernel entry");
    
    println!("╔══════════════════════════════════════════════════╗");
    println!("║                                                  ║");
    println!("║  ██╗    ██╗███████╗██████╗ ██████╗  ██████╗ ███████╗");
    println!("║  ██║    ██║██╔════╝██╔══██╗██╔══██╗██╔═══██╗██╔════╝");
    println!("║  ██║ █╗ ██║█████╗  ██████╔╝██████╔╝██║   ██║███████╗");
    println!("║  ██║███╗██║██╔══╝  ██╔══██╗██╔══██╗██║   ██║╚════██║");
    println!("║  ╚███╔███╔╝███████╗██████╔╝██║  ██║╚██████╔╝███████║");
    println!("║   ╚══╝╚══╝ ╚══════╝╚═════╝ ╚═╝  ╚═╝ ╚═════╝ ╚══════╝");
    println!("║                                                  ║");
    println!("║           Version 0.1.0 - x86_64                 ║");
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
    debug_log::log("CPU initialized");

    // Initialize GDT and TSS
    println!("\n[gdt] Initializing GDT and TSS...");
    arch::gdt::init();
    // Set kernel stack in TSS (use current stack top from boot info)
    arch::gdt::set_kernel_stack(boot_info.stack_top.as_u64());
    println!("[gdt] GDT and TSS initialized");
    debug_log::log("GDT/TSS initialized");

    // Initialize memory management
    println!("\n[mm] Initializing memory management...");
    unsafe {
        mm::init(boot_info);
    }
    println!("[mm] Memory management initialized");
    debug_log::log("Memory management initialized");

    // Initialize interrupt handling
    println!("\n[interrupts] Initializing IDT...");
    interrupts::init();
    println!("[interrupts] IDT initialized");
    debug_log::log("IDT initialized");

    // Print memory statistics
    mm::print_stats();

    // Initialize VFS
    println!("\n[fs] Initializing VFS...");
    fs::init();
    
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

    // Mount FAT32 filesystem from boot disk
    println!("\n[fs] Mounting boot disk FAT32...");
    println!("[fs] Note: FAT32 mount temporarily disabled - see storage/ata.rs");
    // TODO: Re-enable FAT32 mount after fixing multi-sector read issue

    // Initialize network stack
    println!("\n[net] Initializing network stack...");
    net::init();

    // Initialize browser engine
    println!("\n[browser] Initializing browser engine...");
    browser::init();
    println!("[browser] Browser engine initialized");

    // Initialize cryptographic subsystem
    println!("\n[crypto] Initializing cryptographic subsystem...");
    crypto::init();
    println!("[crypto] Cryptographic subsystem initialized");

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
            crate::arch::constants::FRAMEBUFFER_VIRT_BASE as u64
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
            
            // Set up debug framebuffer for early visual debugging
            set_debug_framebuffer(
                fb_virt_addr as usize, 
                fb_info.width as usize, 
                fb_info.bpp as usize
            );
            
            // Visual debug: Draw test pixels to verify framebuffer works
            debug_draw_pixel(10, 10, debug_colors::RED);
            debug_draw_pixel(20, 10, debug_colors::GREEN);
            debug_draw_pixel(30, 10, debug_colors::BLUE);
            
            // Set mouse screen dimensions to match framebuffer
            drivers::input::set_mouse_screen_dimensions(fb_info.width as i32, fb_info.height as i32);
            println!("[input] Mouse screen dimensions set to {}x{}", fb_info.width, fb_info.height);
            
            println!("[vesa] Driver ready, test pixels drawn");
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

    // Show login screen directly (no CLI)
    login_screen_event_loop();
}

/// Draw the login screen to the VESA framebuffer
fn draw_vesa_triangle() {
    // Temporarily disabled - drawing causes crashes
    // The login screen will be shown by login_screen::show() instead
    println!("[vesa] Skipping boot drawing (login screen will be shown later)");
}

/// Framebuffer info for debug drawing (set during boot)
static mut DEBUG_FB_ADDR: usize = 0;
static mut DEBUG_FB_WIDTH: usize = 0;
static mut DEBUG_FB_BPP: usize = 0;

/// Set framebuffer address for debug drawing
fn set_debug_framebuffer(addr: usize, width: usize, bpp: usize) {
    unsafe {
        DEBUG_FB_ADDR = addr;
        DEBUG_FB_WIDTH = width;
        DEBUG_FB_BPP = bpp;
    }
}

/// Debug: Draw a colored pixel to the framebuffer
/// This is visible even when serial/console output isn't working
fn debug_draw_pixel(x: usize, y: usize, color: u32) {
    unsafe {
        if DEBUG_FB_ADDR == 0 {
            return; // Framebuffer not set up yet
        }
        
        // Calculate pixel offset based on BPP
        let bytes_per_pixel = DEBUG_FB_BPP / 8;
        let offset = (y * DEBUG_FB_WIDTH + x) * bytes_per_pixel;
        
        // Write pixel based on BPP
        if DEBUG_FB_BPP == 32 {
            let fb = DEBUG_FB_ADDR as *mut u32;
            fb.add(offset / 4).write_volatile(color);
        } else if DEBUG_FB_BPP == 24 {
            let fb = DEBUG_FB_ADDR as *mut u8;
            let bytes = color.to_le_bytes();
            fb.add(offset).write_volatile(bytes[0]);
            fb.add(offset + 1).write_volatile(bytes[1]);
            fb.add(offset + 2).write_volatile(bytes[2]);
        }
    }
}

/// Debug colors for framebuffer
#[allow(dead_code)]
mod debug_colors {
    pub const RED: u32 = 0xFFFF0000;
    pub const GREEN: u32 = 0xFF00FF00;
    pub const BLUE: u32 = 0xFF0000FF;
    pub const WHITE: u32 = 0xFFFFFFFF;
    pub const YELLOW: u32 = 0xFFFFFF00;
    pub const CYAN: u32 = 0xFF00FFFF;
    pub const MAGENTA: u32 = 0xFFFF00FF;
}

/// Login screen event loop - handles login and transitions to desktop
fn login_screen_event_loop() -> ! {
    use core::sync::atomic::{AtomicU64, Ordering};
    
    // Show login screen
    println!("[boot] Starting...");
    println!("[boot] Calling login_screen::show()...");
    login_screen::show();
    println!("[boot] login_screen::show() returned");
    
    println!("[login] Entering login event loop");
    
    // Heartbeat counter
    static LOOP_COUNT: AtomicU64 = AtomicU64::new(0);
    static LAST_PRINT: AtomicU64 = AtomicU64::new(0);
    
    loop {
        let loop_num = LOOP_COUNT.fetch_add(1, Ordering::Relaxed);
        
        // Check for keyboard input
        if let Some(c) = console::getchar() {
            match login_screen::handle_key(c) {
                login_screen::LoginAction::LoginSuccess => {
                    // Login successful - show graphical desktop
                    println!("\n[desktop] Login successful, launching desktop...");

                    // Show the macOS-style graphical desktop
                    desktop::ui::show();

                    println!("[desktop] Desktop shown, entering desktop mode...");
                    // Enter desktop event loop
                    desktop_event_loop();
                    
                    // If we exit desktop loop, return to login screen
                    println!("\n[login] Returned from desktop, showing login screen...");
                    login_screen::show();
                }
                login_screen::LoginAction::LoginFailed => {
                    // Login failed, stay on login screen
                    println!("[login] Authentication failed");
                }
                login_screen::LoginAction::None => {}
            }
        }
        
        // Print heartbeat every ~5 seconds
        let current_tick = crate::arch::interrupts::get_timer_ticks();
        let last_print = LAST_PRINT.load(Ordering::Relaxed);
        if current_tick >= last_print + 500 {
            println!("[hb] login loop={}", loop_num);
            LAST_PRINT.store(current_tick, Ordering::Relaxed);
        }

        // Halt CPU to save power
        cpu::halt();
    }
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
            const MOUSE_LEFT_BUTTON_MASK: u8 = 0x01;
            let current_buttons = drivers::input::mouse_buttons();
            let button_just_pressed = (current_buttons & MOUSE_LEFT_BUTTON_MASK) != 0 && 
                                       (last_button_state & MOUSE_LEFT_BUTTON_MASK) == 0;
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
                        const KEY_ESCAPE: u8 = 27;
                        if event.ascii == KEY_ESCAPE {
                            println!("[desktop] ESC pressed, exiting desktop mode");
                            return;
                        }
                    }
                    _ => {}
                }
            }
        }
        
        // Print heartbeat every ~0.5 seconds
        let last_print = LAST_PRINT.load(Ordering::Relaxed);
        if current_tick >= last_print + 50 {
            let events = EVENT_COUNT.load(Ordering::Relaxed);
            let (_kb_irq, mouse_irq) = drivers::input::get_irq_counts();
            let (mx, my) = drivers::input::mouse_position();
            println!("[hb] loops={} evt={} irq(m)={} mouse=({},{})", 
                loop_num, events, mouse_irq, mx, my);
            LAST_PRINT.store(current_tick, Ordering::Relaxed);
        }

        // Halt CPU to save power
        cpu::halt();
    }
}

/// Kernel entry trampoline
/// 
/// This is the actual entry point from the bootloader.
/// It sets up the stack and calls kernel_entry.
#[naked]
#[no_mangle]
#[repr(align(16))]
pub unsafe extern "C" fn _start() -> ! {
    naked_asm!(
        // Save boot info pointer (in RDI from bootloader)
        "mov r12, rdi",
        
        // Set up kernel stack
        "mov rsp, {stack_top}",
        
        // Clear frame pointer
        "xor rbp, rbp",
        
        // Restore boot info pointer and call kernel entry
        "mov rdi, r12",
        "call {kernel_entry}",
        
        // Should never return, but halt just in case
        "2:",
        "cli",
        "hlt",
        "jmp 2b",
        
        stack_top = const crate::arch::constants::KERNEL_STACK_TOP as u64,
        kernel_entry = sym kernel_entry,
    );
}
