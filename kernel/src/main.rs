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

use arch::cpu;
use arch::interrupts;

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
        // Use the pre-mapped virtual address for the framebuffer
        // Bootloader mapped 0x80000000 -> 0xFFFF800080000000
        let fb_virt_addr = 0xFFFF_8000_8000_0000u64;
        drivers::vesa::init_with_virt_addr(fb_info.width, fb_info.height, fb_info.bpp as u8, fb_info.addr.as_u64(), fb_virt_addr);
        println!("[vesa] VESA: {}x{} @ {:?} (virt: {:016X})", fb_info.width, fb_info.height, fb_info.addr, fb_virt_addr);
        
        // Boot triangle skipped - will draw shapes after login instead
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

    println!("\n✓ WebbOS kernel initialized successfully!");
    println!("\nSystem is ready. Type 'help' for available commands.");

    // Main kernel loop
    kernel_main();
}

/// Draw a triangle to the VESA framebuffer
fn draw_vesa_triangle() {
    use crate::drivers::vesa::colors;
    
    // Get screen dimensions
    let info = match drivers::vesa::info() {
        Some(i) => i,
        None => return,
    };
    
    let cx = (info.width / 2) as i32;
    let cy = (info.height / 2) as i32;
    
    // Draw a simple filled rectangle as a placeholder for the triangle
    // This tests that VESA drawing works without complex algorithms
    let size = 100i32;
    drivers::vesa::fill_rect(cx - size, cy - size, 200, 200, colors::GREEN);
    drivers::vesa::draw_rect(cx - size, cy - size, 200, 200, colors::WHITE);
    
    println!("[vesa] Triangle (rectangle) drawn at ({}, {})", cx, cy);
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

/// Main kernel loop
fn kernel_main() -> ! {
    // Show VESA login screen if available
    let vesa_available = drivers::vesa::info().is_some();
    
    if vesa_available {
        println!("[main] Showing VESA login screen...");
        
        // Show login screen on VESA
        if let Some((session_id, username)) = desktop::vesa_login::show_login_screen() {
            println!("[main] User '{}' logged in with session {}", username, session_id);
            
            // Clear screen and show welcome
            desktop::vesa_login::show_welcome_message();
            
            // Draw post-login shape (circle)
            desktop::vesa_login::draw_post_login_shape();
            
            // Wait a bit so user can see the result
            println!("[main] Login complete - press any key to continue to console");
            
            // Wait for any key
            loop {
                if drivers::input::get_key().is_some() {
                    break;
                }
                cpu::halt();
            }
            
            // Clear to black for console
            drivers::vesa::clear(drivers::vesa::colors::BLACK);
            drivers::vesa::draw_text("WebbOS Console", 10, 10, drivers::vesa::colors::WHITE, 2);
        }
    }
    
    // Fall back to serial console
    let mut buffer = [0u8; 256];
    let mut pos = 0;

    loop {
        print!("$ ");
        
        // Simple command loop
        loop {
            if let Some(c) = console::getchar() {
                match c {
                    b'\n' | b'\r' => {
                        println!();
                        buffer[pos] = 0;
                        process_command(&buffer[..pos]);
                        pos = 0;
                        break;
                    }
                    8 | 127 => { // Backspace
                        if pos > 0 {
                            pos -= 1;
                            print!("\x08 \x08");
                        }
                    }
                    c if pos < buffer.len() - 1 => {
                        buffer[pos] = c;
                        pos += 1;
                        print!("{}", c as char);
                    }
                    _ => {}
                }
            }
            
            // Halt CPU until next interrupt (saves power)
            cpu::halt();
        }
    }
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
            println!("  reboot     - Reboot the system");
            println!("  shutdown   - Shutdown the system");
        }
        "info" => {
            println!("System Information:");
            println!("  OS: WebbOS v0.1.0");
            println!("  Architecture: x86_64");
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
            fs::print_stats();
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
            println!("Usage: navigate <url>");
            println!("Examples:");
            println!("  navigate file:///test.html");
            println!("  navigate http://example.com");
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
        
        // Debug: Write 'K' to VGA buffer to show we got here
        "mov byte ptr [0xFFFF8000000B8000], 0x4B",  // 'K'
        "mov byte ptr [0xFFFF8000000B8001], 0x0F",  // White on black
        
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
        
        stack_top = const 0xFFFF_8000_0000_0000u64 + 0x500000u64, // Top of 2MB stack at 3MB
        kernel_entry = sym kernel_entry,
    );
}
