//! System call interface
//!
//! Implements system calls for user space programs.

use crate::println;
use crate::print;

/// System call numbers
#[repr(u64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Syscall {
    /// Exit process
    Exit = 0,
    /// Write to file descriptor
    Write = 1,
    /// Read from file descriptor
    Read = 2,
    /// Open file
    Open = 3,
    /// Close file descriptor
    Close = 4,
    /// Memory map
    Mmap = 5,
    /// Memory unmap
    Munmap = 6,
    /// Fork process
    Fork = 7,
    /// Execute program
    Exec = 8,
    /// Wait for child
    Wait = 9,
    /// Get process ID
    GetPid = 10,
    /// Get time
    GetTime = 11,
    /// Yield CPU
    Yield = 12,
    /// Sleep
    Sleep = 13,
    /// Create socket
    Socket = 14,
    /// Connect to address
    Connect = 15,
    /// Bind socket
    Bind = 16,
    /// Listen for connections
    Listen = 17,
    /// Accept connection
    Accept = 18,
    /// Send data
    Send = 19,
    /// Receive data
    Recv = 20,
    /// Device control
    Ioctl = 21,
    /// File control
    Fcntl = 22,
    /// Poll file descriptors
    Poll = 23,
    /// Set signal handler
    Sigaction = 24,
    /// Send signal
    Kill = 25,
    /// Get current directory
    GetCwd = 26,
    /// Change directory
    Chdir = 27,
    /// Create directory
    Mkdir = 28,
    /// Delete file
    Unlink = 29,
    /// Get file stats
    Stat = 30,
    /// Get thread ID
    GetTid = 31,
    /// Create thread
    CreateThread = 32,
    /// Exit thread
    ExitThread = 33,
    /// Play audio
    AudioPlay = 34,
    /// Stop audio
    AudioStop = 35,
    /// Set audio volume
    AudioSetVolume = 36,
    /// Unknown syscall
    Unknown = 0xFF,
}

impl Syscall {
    /// Convert number to syscall
    pub fn from_number(num: u64) -> Self {
        match num {
            0 => Self::Exit,
            1 => Self::Write,
            2 => Self::Read,
            3 => Self::Open,
            4 => Self::Close,
            5 => Self::Mmap,
            6 => Self::Munmap,
            7 => Self::Fork,
            8 => Self::Exec,
            9 => Self::Wait,
            10 => Self::GetPid,
            11 => Self::GetTime,
            12 => Self::Yield,
            13 => Self::Sleep,
            14 => Self::Socket,
            15 => Self::Connect,
            16 => Self::Bind,
            17 => Self::Listen,
            18 => Self::Accept,
            19 => Self::Send,
            20 => Self::Recv,
            21 => Self::Ioctl,
            22 => Self::Fcntl,
            23 => Self::Poll,
            24 => Self::Sigaction,
            25 => Self::Kill,
            26 => Self::GetCwd,
            27 => Self::Chdir,
            28 => Self::Mkdir,
            29 => Self::Unlink,
            30 => Self::Stat,
            31 => Self::GetTid,
            32 => Self::CreateThread,
            33 => Self::ExitThread,
            34 => Self::AudioPlay,
            35 => Self::AudioStop,
            36 => Self::AudioSetVolume,
            _ => Self::Unknown,
        }
    }
}

/// System call arguments
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SyscallArgs {
    pub num: u64,
    pub arg1: u64,
    pub arg2: u64,
    pub arg3: u64,
    pub arg4: u64,
    pub arg5: u64,
    pub arg6: u64,
}

/// System call return value
pub type SyscallResult = i64;

/// Initialize system call interface
pub fn init() {
    println!("[syscall] Initializing system call interface...");

    // Setup syscall MSRs (x86_64 only)
    #[cfg(target_arch = "x86_64")]
    unsafe {
        setup_syscall_msrs();
    }

    #[cfg(target_arch = "aarch64")]
    {
        println!("[syscall] ARM64 syscall support not yet implemented");
    }

    println!("[syscall] System call interface initialized");
}

/// Setup syscall MSRs (x86_64 only)
///
/// # Safety
/// This function is unsafe because it writes to MSRs.
#[cfg(target_arch = "x86_64")]
unsafe fn setup_syscall_msrs() {
    use crate::arch::gdt::KERNEL_CODE_SELECTOR;

    // IA32_STAR - Ring 0 and Ring 3 segment bases
    // Bits 32-47: Kernel CS (also determines DS = CS + 8)
    // Bits 48-63: User CS (also determines DS = CS + 8)
    let star = ((KERNEL_CODE_SELECTOR as u64) << 32) | 
               ((0x18 | 3) << 48); // User CS = 0x18 | RPL 3

    // IA32_LSTAR - syscall entry point
    let lstar = syscall_entry as u64;

    // IA32_FMASK - RFLAGS mask (clear interrupt flag)
    let fmask = 0x200; // Clear IF

    // Write MSRs
    core::arch::asm!(
        "wrmsr",
        in("ecx") 0xC0000081u32, // IA32_STAR
        in("eax") star as u32,
        in("edx") (star >> 32) as u32,
    );

    core::arch::asm!(
        "wrmsr",
        in("ecx") 0xC0000082u32, // IA32_LSTAR
        in("eax") lstar as u32,
        in("edx") (lstar >> 32) as u32,
    );

    core::arch::asm!(
        "wrmsr",
        in("ecx") 0xC0000084u32, // IA32_FMASK
        in("eax") fmask as u32,
        in("edx") 0u32,
    );

    // Enable syscall instruction in EFER MSR
    let mut efer: u64;
    core::arch::asm!(
        "rdmsr",
        in("ecx") 0xC0000080u32, // EFER
        out("eax") efer,
        out("edx") _,
    );

    efer |= 1; // Set SCE (System Call Extensions) bit

    unsafe {
        core::arch::asm!(
            "wrmsr",
            in("ecx") 0xC0000080u32, // EFER
            in("eax") efer as u32,
            in("edx") 0u32,
        );
    }
}

/// System call entry point (x86_64 only)
///
/// This is called by the SYSCALL instruction.
#[cfg(target_arch = "x86_64")]
#[naked]
unsafe extern "C" fn syscall_entry() {
    core::arch::naked_asm!(
        // Swap GS to get kernel stack
        "swapgs",
        
        // Save user stack and load kernel stack
        "mov gs:[0], rsp",      // Save user RSP
        "mov rsp, gs:[8]",      // Load kernel RSP
        
        // Push user state
        "push r11",             // Save RFLAGS
        "push rcx",             // Save RIP (return address)
        
        // Save remaining registers
        "push rax",
        "push rdx",
        "push rsi",
        "push rdi",
        "push r8",
        "push r9",
        "push r10",
        
        // Call handler
        "mov rdi, rax",         // Syscall number
        "mov rsi, rdi",         // Arg1
        "mov rdx, rsi",         // Arg2
        "mov rcx, rdx",         // Arg3
        "mov r8, r10",          // Arg4
        "mov r9, r8",           // Arg5
        "call {handler}",
        
        // Restore registers
        "pop r10",
        "pop r9",
        "pop r8",
        "pop rdi",
        "pop rsi",
        "pop rdx",
        "add rsp, 8",           // Skip rax (return value is in rax)
        
        // Restore user state
        "pop rcx",              // Restore RIP
        "pop r11",              // Restore RFLAGS
        
        // Restore user stack
        "mov rsp, gs:[0]",
        
        // Return to user space
        "swapgs",
        "sysretq",
        handler = sym syscall_handler,
    );
}

/// System call handler
extern "C" fn syscall_handler(
    num: u64,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    _arg4: u64,
    _arg5: u64,
) -> i64 {
    let syscall = Syscall::from_number(num);

    match syscall {
        Syscall::Exit => sys_exit(arg1 as i32),
        Syscall::Write => sys_write(arg1 as i32, arg2 as *const u8, arg3 as usize),
        Syscall::Read => sys_read(arg1 as i32, arg2 as *mut u8, arg3 as usize),
        Syscall::GetPid => sys_getpid(),
        Syscall::GetTid => sys_gettid(),
        Syscall::Yield => sys_yield(),
        Syscall::Sleep => sys_sleep(arg1),
        Syscall::AudioPlay => sys_audio_play(arg1 as *const u8, arg2 as usize, arg3 as u64),
        Syscall::AudioStop => sys_audio_stop(),
        Syscall::AudioSetVolume => sys_audio_set_volume(arg1 as u8),
        _ => {
            println!("[syscall] Unimplemented syscall: {:?}({})", syscall, num);
            -1
        }
    }
}

/// Exit system call
fn sys_exit(code: i32) -> i64 {
    use crate::process;
    use crate::process::scheduler;

    let pid = scheduler::current_thread()
        .and_then(|tid| {
            let threads = process::THREADS.lock();
            threads.get(&tid.as_u64()).map(|t| t.pid)
        });

    if let Some(_pid) = pid {
        // Process exit - just print for now
        println!("[syscall] Process exit with code {}", code);
    }

    0
}

/// Write system call
fn sys_write(fd: i32, buf: *const u8, count: usize) -> i64 {
    // For now, just write to console
    if fd == 1 || fd == 2 { // stdout or stderr
        unsafe {
            let slice = core::slice::from_raw_parts(buf, count);
            if let Ok(s) = core::str::from_utf8(slice) {
                print!("{}", s);
            }
        }
        count as i64
    } else {
        -1
    }
}

/// Read system call
fn sys_read(_fd: i32, _buf: *mut u8, _count: usize) -> i64 {
    // TODO: Implement proper file reading
    -1
}

/// Get process ID
fn sys_getpid() -> i64 {
    use crate::process::scheduler;
    
    scheduler::current_thread()
        .map(|tid| {
            let threads = crate::process::THREADS.lock();
            threads.get(&tid.as_u64()).map(|t| t.pid.as_u64() as i64)
                .unwrap_or(-1)
        })
        .unwrap_or(-1)
}

/// Get thread ID
fn sys_gettid() -> i64 {
    use crate::process::scheduler;
    
    scheduler::current_thread()
        .map(|tid| tid.as_u64() as i64)
        .unwrap_or(-1)
}

/// Yield system call
fn sys_yield() -> i64 {
    unsafe {
        crate::process::scheduler::yield_current();
    }
    0
}

/// Sleep system call
fn sys_sleep(ticks: u64) -> i64 {
    unsafe {
        crate::process::scheduler::sleep_current(ticks);
    }
    0
}

/// Audio play system call
/// 
/// Arguments:
/// - buffer: pointer to audio data
/// - length: length of audio data in bytes
/// - format: packed audio format (sample_rate << 16 | channels << 8 | bits_per_sample)
fn sys_audio_play(buffer: *const u8, length: usize, format: u64) -> i64 {
    use crate::drivers::audio::{play, AudioFormat};
    
    if buffer.is_null() || length == 0 {
        return -1;
    }
    
    // Unpack format
    let sample_rate = ((format >> 16) & 0xFFFFFFFF) as u32;
    let channels = ((format >> 8) & 0xFF) as u8;
    let bits_per_sample = (format & 0xFF) as u8;
    
    let audio_format = AudioFormat {
        sample_rate,
        channels,
        bits_per_sample,
    };
    
    unsafe {
        let data = core::slice::from_raw_parts(buffer, length);
        match play(data, &audio_format) {
            Ok(()) => 0,
            Err(_) => -1,
        }
    }
}

/// Audio stop system call
fn sys_audio_stop() -> i64 {
    use crate::drivers::audio::stop;
    stop();
    0
}

/// Audio set volume system call
/// 
/// Argument:
/// - volume: volume level (0-100)
fn sys_audio_set_volume(volume: u8) -> i64 {
    use crate::drivers::audio::set_volume;
    set_volume(volume.min(100));
    0
}

/// Print syscall statistics
pub fn print_stats() {
    println!("System Call Statistics:");
    println!("  Implemented: 10/37");
    println!("  - exit, write, read");
    println!("  - getpid, gettid");
    println!("  - yield, sleep");
    println!("  - audio_play, audio_stop, audio_set_volume");
}
