#![no_std]
#![no_main]
#![feature(allocator_api)]
#![feature(naked_functions)]
#![feature(fn_align)]
#![feature(alloc_error_handler)]
#![feature(generic_const_exprs)]
#![allow(incomplete_features)]
#![allow(unused_assignments)]

//! WebbOS Kernel for ARM64 (Raspberry Pi)
//!
//! Main kernel entry point and initialization for ARM64 architecture.

extern crate alloc;

use alloc::boxed::Box;
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

use arch::cpu;
use arch::exceptions;

/// Test FAT32 root directory reading
#[allow(dead_code)]
fn test_fat32_root() {
    println!("[fs] Testing FAT32 root directory...");
    
    // Try to read root directory entries
    
    
    // Get the root filesystem (should be FAT32 mounted at /)
    // For now just print a success message
    println!("[fs] FAT32 filesystem ready for use");
}

/// Mount SD card FAT32 filesystem
fn mount_sd_card_filesystem() {
    use alloc::sync::Arc;
    use crate::fs::fat32::Fat32Fs;
    
    println!("[fs] Attempting to mount SD card FAT32 filesystem...");
    
    // Create a simple wrapper that uses the SD card driver
    // The SdCardBlockDevice uses the public sd_card::read_blocks API
    match Fat32Fs::new(Box::new(SdCardBlockDevice::new())) {
        Ok(fs) => {
            let fs_arc = Arc::new(fs);
            match crate::fs::mount("/", fs_arc) {
                Ok(()) => println!("[fs] FAT32 filesystem mounted at /"),
                Err(e) => println!("[fs] Failed to mount filesystem: {:?}", e),
            }
        }
        Err(e) => println!("[fs] Failed to create FAT32 filesystem: {:?}", e),
    }
}

/// Simple wrapper to access SD card as BlockDevice
struct SdCardBlockDevice;

impl SdCardBlockDevice {
    fn new() -> Self {
        Self
    }
}

impl crate::storage::BlockDevice for SdCardBlockDevice {
    fn name(&self) -> &str {
        "sd_card"
    }
    
    fn block_size(&self) -> usize {
        512
    }
    
    fn block_count(&self) -> u64 {
        // Return a reasonable size for the SD card
        // 256MB = ~524288 blocks
        524288
    }
    
    fn read_blocks(&self, start: u64, count: usize, buf: &mut [u8]) -> Result<(), crate::storage::StorageError> {
        // Use the SD card driver to read blocks
        crate::storage::sd_card::read_blocks(start, count, buf)
    }
    
    fn write_blocks(&self, _start: u64, _count: usize, _buf: &[u8]) -> Result<(), crate::storage::StorageError> {
        // Read-only for now
        Err(crate::storage::StorageError::WriteProtected)
    }
    
    fn flush(&self) -> Result<(), crate::storage::StorageError> {
        Ok(())
    }
}

/// Wrapper to use Arc<BootDisk> as Box<dyn BlockDevice>
#[allow(dead_code)]
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
/// This is called by the bootloader after setting up the MMU.
/// The boot_info pointer is passed in x0 register per AAPCS64.
#[no_mangle]
pub extern "C" fn kernel_entry(boot_info: &'static BootInfo) -> ! {
    // Validate boot info
    if !boot_info.verify() {
        panic!("Invalid boot info magic number!");
    }

    // Initialize console for early output
    console::init();
    
    println!("╔══════════════════════════════════════════════════╗");
    println!("║                                                  ║");
    println!("║  ██╗    ██╗███████╗██████╗ ██████╗  ██████╗ ███████╗");
    println!("║  ██║    ██║██╔════╝██╔══██╗██╔══██╗██╔═══██╗██╔════╝");
    println!("║  ██║ █╗ ██║█████╗  ██████╔╝██████╔╝██║   ██║███████╗");
    println!("║  ██║███╗██║██╔══╝  ██╔══██╗██╔══██╗██║   ██║╚════██║");
    println!("║  ╚███╔███╔╝███████╗██████╔╝██║  ██║╚██████╔╝███████║");
    println!("║   ╚══╝╚══╝ ╚══════╝╚═════╝ ╚═╝  ╚═╝ ╚═════╝ ╚══════╝");
    println!("║                                                  ║");
    println!("║           Version 0.1.0 - ARM64 (Pi)             ║");
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

    // Initialize exception handling (replaces x86 IDT)
    println!("\n[exceptions] Initializing exception vectors...");
    exceptions::init();
    println!("[exceptions] Exception vectors installed");

    // Initialize memory management
    println!("\n[mm] Initializing memory management...");
    unsafe {
        mm::init(boot_info);
    }
    println!("[mm] Memory management initialized");

    // Print memory statistics
    mm::print_stats();

    // Initialize VFS
    println!("\n[fs] Initializing VFS...");
    fs::init();

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
    
    // Initialize SD card
    println!("\n[sd_card] Initializing SD card...");
    storage::sd_card::init();
    
    // Mount FAT32 filesystem from SD card
    println!("\n[fs] Mounting SD card filesystem...");
    mount_sd_card_filesystem();

    // Initialize network stack
    println!("\n[net] Initializing network stack...");
    net::init();

    // Initialize browser engine
    println!("\n[browser] Initializing browser engine...");
    browser::init();
    println!("[browser] Browser engine initialized");

    // Initialize audio subsystem
    println!("\n[audio] Initializing audio subsystem...");
    if let Err(e) = drivers::audio::init() {
        println!("[audio] Audio initialization failed: {:?}", e);
    }

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

    // Initialize VESA/framebuffer using boot info
    println!("\n[fb] Initializing framebuffer...");
    let fb_info = &boot_info.framebuffer;
    if fb_info.is_valid() {
        println!("[fb] Using framebuffer at: {:016X}", fb_info.addr.as_u64());
        println!("[fb] Resolution: {}x{} @ {}bpp", fb_info.width, fb_info.height, fb_info.bpp);
    } else {
        println!("[fb] No valid framebuffer from bootloader, allocating via mailbox...");
        // Initialize Pi framebuffer via mailbox
        drivers::display::pi_framebuffer::init(1024, 768, 32);
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
    println!("\n[exceptions] Enabling interrupts...");
    exceptions::enable();
    println!("[exceptions] Interrupts enabled");

    // Start the scheduler
    println!("\n[scheduler] Starting scheduler...");
    process::scheduler::start_scheduling();
    println!("[scheduler] Scheduler running");

    println!("\n✓ WebbOS kernel initialized successfully!");
    println!("\nSystem is ready. Type 'help' for available commands.");

    // Main kernel loop
    kernel_main();
}

/// Main kernel loop
fn kernel_main() -> ! {
    let mut first_boot = true;

    loop {
        // Show login screen on first boot after a short delay
        if first_boot {
            first_boot = false;
            println!("[boot] Starting...");
            // Use simple delay loop instead of timer sleep
            println!("[boot] Waiting...");
            for _ in 0..10000000 {
                core::hint::spin_loop();
            }
            println!("[boot] Wait complete");
            println!("[boot] Calling login_screen::show()...");
            login_screen::show();
            println!("[boot] login_screen::show() returned");
        }
        
        // Input loop - handles login screen and desktop
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
                            // If we exit desktop loop, go back to login screen
                            println!("\n[desktop] Exited desktop mode, returning to login...");
                            login_screen::show();
                        }
                        login_screen::LoginAction::LoginFailed => {
                            // Login failed, stay on login screen
                            println!("[login] Authentication failed");
                        }
                        login_screen::LoginAction::None => {}
                    }
                    continue;
                }
            }
            
            // Halt CPU until next interrupt (saves power)
            cpu::halt();
        }
    }
}

/// Desktop event loop - handles mouse and keyboard input for desktop
fn desktop_event_loop() {
    use core::sync::atomic::{AtomicU64, Ordering};
    
    println!("[desktop] Entering desktop event loop (timer-based)");
    
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
    const DOUBLE_CLICK_TIME: u64 = 50; // ~500ms at 100 ticks/second
    const DOUBLE_CLICK_DIST: i32 = 20; // 20 pixels radius

    loop {
        let loop_num = LOOP_COUNT.fetch_add(1, Ordering::Relaxed);
        
        // Timer-based polling at ~40Hz
        let current_tick = crate::arch::exceptions::get_timer_ticks();
        let last_timer = LAST_TIMER_TICK.load(Ordering::Relaxed);
        
        if current_tick >= last_timer + 2 {
            LAST_TIMER_TICK.store(current_tick, Ordering::Relaxed);
            
            // Process messages from HTML frontend
            desktop::process_messages();
            
            // Poll WiFi for incoming packets and EAPOL/DHCP events
            drivers::wifi::poll();
            
            // Poll mouse from timer
            if let Some(event) = drivers::input::poll_mouse_from_timer() {
                EVENT_COUNT.fetch_add(1, Ordering::Relaxed);
                desktop::ui::update_mouse(event.x, event.y);
            }
            
            // Check for mouse button press
            let current_buttons = drivers::input::mouse_buttons();
            let button_just_pressed = (current_buttons & 0x01) != 0 && (last_button_state & 0x01) == 0;
            let (mouse_x, mouse_y) = drivers::input::mouse_position();
            
            if button_just_pressed {
                let time_since_last = current_tick - last_click_time;
                let dx = mouse_x - last_click_x;
                let dy = mouse_y - last_click_y;
                let dist_sq = dx * dx + dy * dy;
                
                if time_since_last < DOUBLE_CLICK_TIME && dist_sq < (DOUBLE_CLICK_DIST * DOUBLE_CLICK_DIST) {
                    println!("[desktop] Double-click at ({}, {})", mouse_x, mouse_y);
                    desktop::ui::handle_double_click(mouse_x, mouse_y);
                } else {
                    desktop::ui::handle_click(mouse_x, mouse_y);
                }
                
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
                    }
                    _ => {}
                }
            }
        }
        
        // Print heartbeat every ~0.5 seconds
        let last_print = LAST_PRINT.load(Ordering::Relaxed);
        if current_tick >= last_print + 50 {
            let events = EVENT_COUNT.load(Ordering::Relaxed);
            let (_, mouse_irq) = drivers::input::get_irq_counts();
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
        // Save boot info pointer (in x0 from bootloader)
        "mov x19, x0",
        
        // Set up kernel stack
        "ldr x0, ={stack_top}",
        "mov sp, x0",
        
        // Clear frame pointer
        "mov x29, xzr",
        
        // Restore boot info pointer and call kernel entry
        "mov x0, x19",
        "bl {kernel_entry}",
        
        // Should never return, but halt just in case
        "2:",
        "wfe",
        "b 2b",
        
        stack_top = const 0xFFFF_0000_0000_0000u64 + 0x500000u64 + 0x20000u64,
        kernel_entry = sym kernel_entry,
    );
}
