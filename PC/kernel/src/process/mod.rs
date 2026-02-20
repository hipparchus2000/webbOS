//! Process and thread management
//!
//! Implements task scheduling, process creation, and context switching.

use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use spin::Mutex;
use lazy_static::lazy_static;

pub mod context;
pub mod scheduler;

use context::Context;
use webbos_shared::types::{Pid, Tid};
use crate::println;

/// Maximum number of processes
pub const MAX_PROCESSES: usize = 1024;
/// Maximum number of threads per process
pub const MAX_THREADS_PER_PROCESS: usize = 256;
/// Kernel stack size for each thread
pub const KERNEL_STACK_SIZE: usize = 128 * 1024; // 128KB
/// User stack size
pub const USER_STACK_SIZE: usize = 8 * 1024 * 1024; // 8MB

/// Process state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    /// Process is running
    Running,
    /// Process is ready to run
    Ready,
    /// Process is blocked waiting for something
    Blocked,
    /// Process is zombie (terminated but not reaped)
    Zombie,
    /// Process is being created
    Creating,
}

/// Thread state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadState {
    /// Thread is running
    Running,
    /// Thread is ready to run
    Ready,
    /// Thread is blocked
    Blocked,
    /// Thread is sleeping
    Sleeping,
    /// Thread is terminated
    Terminated,
}

/// Thread priority (0-31, higher is more important)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Priority(u8);

impl Priority {
    pub const IDLE: Self = Self(0);
    pub const LOW: Self = Self(8);
    pub const NORMAL: Self = Self(16);
    pub const HIGH: Self = Self(24);
    pub const REALTIME: Self = Self(31);

    pub fn new(value: u8) -> Self {
        Self(value.min(31))
    }

    pub fn as_u8(self) -> u8 {
        self.0
    }
}

/// Thread control block
pub struct Thread {
    /// Thread ID
    pub tid: Tid,
    /// Owning process ID
    pub pid: Pid,
    /// Thread state
    pub state: ThreadState,
    /// CPU context (registers)
    pub context: Context,
    /// Kernel stack pointer
    pub kernel_stack: u64,
    /// Thread priority
    pub priority: Priority,
    /// CPU affinity (0 = any CPU)
    pub cpu_affinity: u8,
    /// Time slice remaining (in ticks)
    pub time_slice: u64,
}

impl Thread {
    /// Create a new thread
    pub fn new(tid: Tid, pid: Pid, priority: Priority) -> Self {
        Self {
            tid,
            pid,
            state: ThreadState::Ready,
            context: Context::new(),
            kernel_stack: 0,
            priority,
            cpu_affinity: 0,
            time_slice: 0,
        }
    }

    /// Check if thread is runnable
    pub fn is_runnable(&self) -> bool {
        matches!(self.state, ThreadState::Ready | ThreadState::Running)
    }
}

/// Process control block
pub struct Process {
    /// Process ID
    pub pid: Pid,
    /// Process state
    pub state: ProcessState,
    /// Parent process ID
    pub parent: Option<Pid>,
    /// Child process IDs
    pub children: Vec<Pid>,
    /// Threads in this process
    pub threads: Vec<Tid>,
    /// Main thread ID
    pub main_thread: Tid,
    /// Process name
    pub name: [u8; 256],
    /// Exit code (if zombie)
    pub exit_code: i32,
    /// Working directory
    pub cwd: [u8; 256],
}

impl Process {
    /// Create a new process
    pub fn new(pid: Pid, parent: Option<Pid>, name: &str) -> Self {
        let mut name_buf = [0u8; 256];
        let name_bytes = name.as_bytes();
        let len = name_bytes.len().min(255);
        name_buf[..len].copy_from_slice(&name_bytes[..len]);

        Self {
            pid,
            state: ProcessState::Creating,
            parent,
            children: Vec::new(),
            threads: Vec::new(),
            main_thread: Tid::new(0),
            name: name_buf,
            exit_code: 0,
            cwd: [0u8; 256],
        }
    }

    /// Get process name as str
    pub fn name(&self) -> &str {
        let len = self.name.iter().position(|&b| b == 0).unwrap_or(256);
        core::str::from_utf8(&self.name[..len]).unwrap_or("<invalid>")
    }
}

lazy_static! {
    /// Global process table
    pub static ref PROCESSES: Mutex<BTreeMap<u64, Process>> = Mutex::new(BTreeMap::new());
    /// Global thread table
    pub static ref THREADS: Mutex<BTreeMap<u64, Thread>> = Mutex::new(BTreeMap::new());
    static ref NEXT_PID: Mutex<u64> = Mutex::new(1);
    static ref NEXT_TID: Mutex<u64> = Mutex::new(1);
}

/// Initialize process management
pub fn init() {
    println!("[process] Initializing process management...");

    // Create idle process (PID 0)
    let idle_process = Process::new(Pid::new(0), None, "idle");
    let idle_thread = Thread::new(Tid::new(0), Pid::new(0), Priority::IDLE);

    {
        let mut processes = PROCESSES.lock();
        let mut threads = THREADS.lock();
        processes.insert(0, idle_process);
        threads.insert(0, idle_thread);
    }

    // Initialize scheduler
    scheduler::init();

    println!("[process] Process management initialized");
}

/// Allocate a new process ID
fn alloc_pid() -> Pid {
    let mut next = NEXT_PID.lock();
    let pid = *next;
    *next += 1;
    Pid::new(pid)
}

/// Allocate a new thread ID
fn alloc_tid() -> Tid {
    let mut next = NEXT_TID.lock();
    let tid = *next;
    *next += 1;
    Tid::new(tid)
}

/// Create a new process
pub fn create_process(name: &str, parent: Option<Pid>) -> Result<Pid, ProcessError> {
    let pid = alloc_pid();
    let tid = alloc_tid();

    let mut process = Process::new(pid, parent, name);
    process.main_thread = tid;
    process.threads.push(tid);
    process.state = ProcessState::Ready;

    let thread = Thread::new(tid, pid, Priority::NORMAL);

    {
        let mut processes = PROCESSES.lock();
        let mut threads = THREADS.lock();

        // Add to parent's children if parent exists
        if let Some(parent_pid) = parent {
            if let Some(parent) = processes.get_mut(&parent_pid.as_u64()) {
                parent.children.push(pid);
            }
        }

        processes.insert(pid.as_u64(), process);
        threads.insert(tid.as_u64(), thread);
    }

    // Add to scheduler
    scheduler::add_thread(tid);

    println!("[process] Created process {}:{} ({})", pid.as_u64(), tid.as_u64(), name);
    Ok(pid)
}

/// Get process by PID
pub fn get_process(pid: Pid) -> Option<spin::MutexGuard<'static, BTreeMap<u64, Process>>> {
    let processes = PROCESSES.lock();
    if processes.contains_key(&pid.as_u64()) {
        Some(processes)
    } else {
        None
    }
}

/// Get thread by TID
pub fn get_thread(tid: Tid) -> Option<spin::MutexGuard<'static, BTreeMap<u64, Thread>>> {
    let threads = THREADS.lock();
    if threads.contains_key(&tid.as_u64()) {
        Some(threads)
    } else {
        None
    }
}

/// Exit current process
pub fn exit_process(pid: Pid, exit_code: i32) {
    println!("[process] Process {} exiting with code {}", pid.as_u64(), exit_code);

    let mut processes = PROCESSES.lock();
    
    if let Some(process) = processes.get_mut(&pid.as_u64()) {
        process.state = ProcessState::Zombie;
        process.exit_code = exit_code;

        // Clean up threads
        let mut threads = THREADS.lock();
        for tid in &process.threads {
            if let Some(thread) = threads.get_mut(&tid.as_u64()) {
                thread.state = ThreadState::Terminated;
            }
        }
    }

    // Schedule next process
    unsafe {
        scheduler::schedule_next();
    }
}

/// Get current process info
pub fn print_process_list() {
    let processes = PROCESSES.lock();
    let threads = THREADS.lock();

    println!("PID  State    Name         Threads");
    println!("---  -----    ----         -------");

    for (pid, process) in processes.iter() {
        let state_str = match process.state {
            ProcessState::Running => "RUN",
            ProcessState::Ready => "RDY",
            ProcessState::Blocked => "BLK",
            ProcessState::Zombie => "ZMB",
            ProcessState::Creating => "NEW",
        };
        println!("{:>3}  {:<8} {:<12} {}", 
            pid, state_str, process.name(), process.threads.len());
    }

    println!("\nTID  PID  State    Priority");
    println!("---  ---  -----    --------");

    for (tid, thread) in threads.iter() {
        let state_str = match thread.state {
            ThreadState::Running => "RUN",
            ThreadState::Ready => "RDY",
            ThreadState::Blocked => "BLK",
            ThreadState::Sleeping => "SLP",
            ThreadState::Terminated => "TRM",
        };
        println!("{:>3}  {:>3}  {:<8} {}", 
            tid, thread.pid.as_u64(), state_str, thread.priority.as_u8());
    }
}

/// Process errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessError {
    /// Process table full
    ProcessTableFull,
    /// Thread table full
    ThreadTableFull,
    /// Process not found
    ProcessNotFound,
    /// Thread not found
    ThreadNotFound,
    /// Invalid operation
    InvalidOperation,
}
