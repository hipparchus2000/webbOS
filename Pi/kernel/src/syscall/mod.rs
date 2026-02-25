//! System call interface
//!
//! Implements system calls for user space programs.

#![allow(dead_code)]

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

    // On ARM64, syscalls use the SVC instruction and are handled by the
    // exception handler in exceptions.rs. No special initialization needed.
    println!("[syscall] ARM64 uses SVC instruction for system calls");

    println!("[syscall] System call interface initialized");
}

// On ARM64, syscalls are handled via SVC exceptions in exceptions.rs
// The x86_64 SYSCALL/SYSRET mechanism is not available on ARM

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

/// Print syscall statistics
pub fn print_stats() {
    println!("System Call Statistics:");
    println!("  Implemented: 7/34");
    println!("  - exit, write, read");
    println!("  - getpid, gettid");
    println!("  - yield, sleep");
}
