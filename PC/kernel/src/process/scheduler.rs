//! Round-robin task scheduler
//!
//! Implements a simple preemptive round-robin scheduler with context switching.

#![allow(dead_code)]

use alloc::collections::VecDeque;
use spin::Mutex;
use lazy_static::lazy_static;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{Priority, Tid, Thread, ThreadState, THREADS};
use super::context::{Context, switch_context};
use crate::println;
use alloc::vec::Vec;

/// Time slice in timer ticks (10ms per tick, so 100ms default)
pub const DEFAULT_TIME_SLICE: u64 = 10;

/// Current running thread on each CPU (0 means None)
static CURRENT_THREADS: [AtomicU64; 8] = [
    AtomicU64::new(0), AtomicU64::new(0),
    AtomicU64::new(0), AtomicU64::new(0),
    AtomicU64::new(0), AtomicU64::new(0),
    AtomicU64::new(0), AtomicU64::new(0),
]; // Support up to 8 CPUs

/// Idle thread context (used when no other threads are runnable)
static mut IDLE_CONTEXT: Context = Context::new();

/// Scheduler state
struct Scheduler {
    /// Ready queue for each priority level
    ready_queues: [VecDeque<Tid>; 32],
    /// Current time slice remaining
    time_slice: u64,
    /// Whether scheduling is enabled
    enabled: bool,
    /// Total ticks elapsed
    ticks: u64,
    /// Sleep queue: (wakeup_tick, tid)
    sleep_queue: VecDeque<(u64, Tid)>,
}

impl Scheduler {
    const fn new() -> Self {
        const EMPTY_QUEUE: VecDeque<Tid> = VecDeque::new();
        Self {
            ready_queues: [EMPTY_QUEUE; 32],
            time_slice: DEFAULT_TIME_SLICE,
            enabled: false,
            ticks: 0,
            sleep_queue: VecDeque::new(),
        }
    }

    /// Add thread to ready queue
    fn enqueue(&mut self, tid: Tid, priority: Priority) {
        let queue_idx = priority.as_u8() as usize;
        self.ready_queues[queue_idx].push_back(tid);
    }

    /// Get next thread to run (highest priority first)
    fn dequeue(&mut self) -> Option<Tid> {
        // Check from highest priority (31) to lowest (0)
        for i in (0..32).rev() {
            if let Some(tid) = self.ready_queues[i].pop_front() {
                return Some(tid);
            }
        }
        None
    }

    /// Check if there are runnable threads
    fn has_runnable(&self) -> bool {
        for queue in &self.ready_queues {
            if !queue.is_empty() {
                return true;
            }
        }
        false
    }

    /// Add thread to sleep queue
    fn sleep(&mut self, tid: Tid, ticks: u64) {
        let wakeup_tick = self.ticks + ticks;
        // Insert in sorted order (earliest wakeup first)
        let pos = self.sleep_queue.iter()
            .position(|(t, _)| *t > wakeup_tick)
            .unwrap_or(self.sleep_queue.len());
        self.sleep_queue.insert(pos, (wakeup_tick, tid));
    }

    /// Check sleep queue for threads that should wake up
    fn check_sleepers(&mut self) {
        let current_tick = self.ticks;
        let mut woken = Vec::new();
        
        // Find threads that should wake up
        while let Some((wakeup_tick, tid)) = self.sleep_queue.front() {
            if *wakeup_tick <= current_tick {
                woken.push(*tid);
                self.sleep_queue.pop_front();
            } else {
                break;
            }
        }
        
        // Wake them up
        for tid in woken {
            let mut threads = THREADS.lock();
            if let Some(thread) = threads.get_mut(&tid.as_u64()) {
                if matches!(thread.state, ThreadState::Sleeping) {
                    thread.state = ThreadState::Ready;
                    let priority = thread.priority;
                    drop(threads);
                    self.enqueue(tid, priority);
                }
            }
        }
    }
}

lazy_static! {
    static ref SCHEDULER: Mutex<Scheduler> = Mutex::new(Scheduler::new());
}

/// Initialize the scheduler
pub fn init() {
    println!("[scheduler] Initializing round-robin scheduler...");

    let mut scheduler = SCHEDULER.lock();
    scheduler.enabled = true;

    println!("[scheduler] Scheduler initialized");
}

/// Add a thread to the scheduler
pub fn add_thread(tid: Tid) {
    let mut scheduler = SCHEDULER.lock();
    
    // Get thread priority
    let threads = THREADS.lock();
    if let Some(thread) = threads.get(&tid.as_u64()) {
        let priority = thread.priority;
        scheduler.enqueue(tid, priority);
    }
}

/// Remove a thread from the scheduler
pub fn remove_thread(tid: Tid) {
    let mut scheduler = SCHEDULER.lock();
    
    // Remove from all priority queues
    for queue in &mut scheduler.ready_queues {
        queue.retain(|&t| t.as_u64() != tid.as_u64());
    }
}

/// Get current thread ID
pub fn current_thread() -> Option<Tid> {
    let cpu_id = get_cpu_id();
    match CURRENT_THREADS[cpu_id].load(Ordering::Relaxed) {
        0 => None,
        tid => Some(Tid::new(tid)),
    }
}

/// Get CPU ID (simplified - always returns 0 for single-core)
fn get_cpu_id() -> usize {
    // On single-core PC, always return 0
    // TODO: Read APIC ID for multi-core support
    0
}

/// Disable interrupts
fn disable_interrupts() {
    unsafe {
        core::arch::asm!("cli", options(nomem, nostack));
    }
}

/// Enable interrupts
fn enable_interrupts() {
    unsafe {
        core::arch::asm!("sti", options(nomem, nostack));
    }
}

/// Schedule next thread to run
/// 
/// # Safety
/// This function is unsafe because it performs a context switch.
pub unsafe fn schedule_next() {
    // Disable interrupts during context switch
    disable_interrupts();
    
    let mut scheduler = SCHEDULER.lock();

    if !scheduler.enabled {
        enable_interrupts();
        return;
    }

    // Check for threads that should wake up from sleep
    scheduler.check_sleepers();

    // Get current thread
    let cpu_id = get_cpu_id();
    let current_tid = match CURRENT_THREADS[cpu_id].load(Ordering::Relaxed) {
        0 => None,
        tid => Some(Tid::new(tid)),
    };

    // Get next thread from ready queue
    let next_tid = scheduler.dequeue();
    
    // If no runnable threads, use idle thread
    let next_tid = match next_tid {
        Some(tid) => tid,
        None => {
            // No threads to run, use idle
            if current_tid.is_some() {
                // Current thread continues running
                scheduler.time_slice = DEFAULT_TIME_SLICE;
                enable_interrupts();
                return;
            }
            // Switch to idle
            Tid::new(0)
        }
    };

    // If same thread, just reset time slice and return
    if Some(next_tid) == current_tid {
        scheduler.time_slice = DEFAULT_TIME_SLICE;
        enable_interrupts();
        return;
    }

    // Get contexts for context switch
    let (old_ctx_ptr, new_ctx_ptr) = {
        let mut threads = THREADS.lock();
        
        // Save current thread's context pointer and mark as ready
        let old_ctx = if let Some(tid) = current_tid {
            if let Some(thread) = threads.get_mut(&tid.as_u64()) {
                if thread.state == ThreadState::Running {
                    thread.state = ThreadState::Ready;
                }
                &mut thread.context as *mut Context
            } else {
                &raw mut IDLE_CONTEXT as *mut Context
            }
        } else {
            &raw mut IDLE_CONTEXT as *mut Context
        };
        
        // Get next thread's context pointer and mark as running
        let new_ctx = if next_tid.as_u64() == 0 {
            // Idle thread
            &raw const IDLE_CONTEXT as *const Context
        } else {
            if let Some(thread) = threads.get_mut(&next_tid.as_u64()) {
                thread.state = ThreadState::Running;
                &thread.context as *const Context
            } else {
                // Thread not found, use idle
                &raw const IDLE_CONTEXT as *const Context
            }
        };
        
        (old_ctx, new_ctx)
    };

    // Update current thread
    CURRENT_THREADS[cpu_id].store(next_tid.as_u64(), Ordering::Relaxed);
    scheduler.time_slice = DEFAULT_TIME_SLICE;

    // Release scheduler lock before context switch
    drop(scheduler);

    // Perform context switch
    switch_context(old_ctx_ptr, new_ctx_ptr);
    
    // Re-enable interrupts after context switch
    enable_interrupts();
}

/// Called on every timer tick
/// 
/// # Safety
/// This function is unsafe because it may trigger a context switch.
pub unsafe fn timer_tick() {
    let mut scheduler = SCHEDULER.lock();

    scheduler.ticks += 1;

    if !scheduler.enabled {
        return;
    }

    // Check for sleeping threads that should wake up
    scheduler.check_sleepers();

    // Decrement time slice
    if scheduler.time_slice > 0 {
        scheduler.time_slice -= 1;
    }

    // If time slice expired and we have other runnable threads, schedule next
    if scheduler.time_slice == 0 && scheduler.has_runnable() {
        drop(scheduler);
        schedule_next();
    }
}

/// Yield the current thread
/// 
/// # Safety
/// This function is unsafe because it triggers a context switch.
pub unsafe fn yield_current() {
    // Put current thread back in queue
    if let Some(tid) = current_thread() {
        let threads = THREADS.lock();
        if let Some(thread) = threads.get(&tid.as_u64()) {
            let priority = thread.priority;
            drop(threads);
            SCHEDULER.lock().enqueue(tid, priority);
        }
    }
    
    schedule_next();
}

/// Get scheduler statistics
pub fn print_stats() {
    let scheduler = SCHEDULER.lock();

    println!("Scheduler Statistics:");
    println!("  Ticks: {}", scheduler.ticks);
    println!("  Enabled: {}", scheduler.enabled);
    println!("  Time slice remaining: {}", scheduler.time_slice);

    // Count threads in each priority queue
    for (i, queue) in scheduler.ready_queues.iter().enumerate() {
        if !queue.is_empty() {
            println!("  Priority {}: {} threads", i, queue.len());
        }
    }
    
    // Show sleepers
    if !scheduler.sleep_queue.is_empty() {
        println!("  Sleeping: {} threads", scheduler.sleep_queue.len());
    }

    if let Some(tid) = current_thread() {
        println!("  Current thread: {}", tid.as_u64());
    }
}

/// Block current thread (e.g., waiting for I/O)
/// 
/// # Safety
/// This function is unsafe because it triggers a context switch.
pub unsafe fn block_current() {
    use super::ThreadState;

    if let Some(tid) = current_thread() {
        let mut threads = THREADS.lock();
        if let Some(thread) = threads.get_mut(&tid.as_u64()) {
            thread.state = ThreadState::Blocked;
        }
    }

    schedule_next();
}

/// Unblock a thread
pub fn unblock_thread(tid: Tid) {
    use super::ThreadState;

    let mut threads = THREADS.lock();
    if let Some(thread) = threads.get_mut(&tid.as_u64()) {
        if matches!(thread.state, ThreadState::Blocked) {
            thread.state = ThreadState::Ready;
            let priority = thread.priority;
            drop(threads);
            SCHEDULER.lock().enqueue(tid, priority);
        }
    }
}

/// Sleep current thread for N ticks
/// 
/// # Safety
/// This function is unsafe because it triggers a context switch.
pub unsafe fn sleep_current(ticks: u64) {
    use super::ThreadState;

    if let Some(tid) = current_thread() {
        let mut threads = THREADS.lock();
        if let Some(thread) = threads.get_mut(&tid.as_u64()) {
            thread.state = ThreadState::Sleeping;
        }
        drop(threads);
        
        // Add to sleep queue
        SCHEDULER.lock().sleep(tid, ticks);
    }

    schedule_next();
}

/// Spawn a new kernel thread
/// 
/// Creates a new thread with the given entry point and adds it to the scheduler.
/// # Safety
/// The entry point must be a valid function that never returns.
pub unsafe fn spawn_kernel_thread(entry: fn() -> !, name: &str) -> Result<Tid, ()> {
    use super::{ProcessState, PROCESSES, KERNEL_STACK_SIZE};
    use alloc::alloc::{alloc, Layout};
    use webbos_shared::types::Pid;
    
    // Allocate a kernel stack
    let stack_layout = Layout::from_size_align(KERNEL_STACK_SIZE, 16)
        .map_err(|_| ())?;
    let stack_bottom = alloc(stack_layout) as u64;
    if stack_bottom == 0 {
        return Err(());
    }
    let stack_top = stack_bottom + KERNEL_STACK_SIZE as u64;
    
    // Create a dummy process for this thread (kernel threads have no user process)
    let pid = {
        let mut processes = PROCESSES.lock();
        static mut NEXT_KERNEL_PID: u64 = 0x10000; // Start kernel PIDs high
        let pid = Pid::new(NEXT_KERNEL_PID);
        NEXT_KERNEL_PID += 1;
        
        let mut process = super::Process::new(pid, None, name);
        process.state = ProcessState::Running;
        processes.insert(pid.as_u64(), process);
        pid
    };
    
    // Create the thread
    let tid = {
        let mut threads = THREADS.lock();
        static mut NEXT_KERNEL_TID: u64 = 0x10000;
        let tid = Tid::new(NEXT_KERNEL_TID);
        NEXT_KERNEL_TID += 1;
        
        let mut thread = Thread::new(tid, pid, Priority::NORMAL);
        thread.kernel_stack = stack_bottom;
        thread.state = ThreadState::Ready;
        
        // Initialize context for the new thread
        thread.context = Context::new_kernel_thread(entry, stack_top);
        
        threads.insert(tid.as_u64(), thread);
        tid
    };
    
    // Add to scheduler
    add_thread(tid);
    
    println!("[scheduler] Spawned kernel thread {} ({})", tid.as_u64(), name);
    Ok(tid)
}

/// Initialize and start the idle thread
/// 
/// This should be called once the system is initialized.
pub fn start_idle_thread() {
    // The idle thread (TID 0) is already created during process init
    // Just set it as the current thread and start the timer
    CURRENT_THREADS[0].store(0, Ordering::Relaxed);
    
    println!("[scheduler] Idle thread started");
}

/// Idle thread entry point
fn idle_thread_entry() -> ! {
    loop {
        // Halt until next interrupt (low power mode)
        unsafe {
            core::arch::asm!("hlt", options(nomem, nostack));
        }
    }
}

/// Initialize the scheduler and start the first thread
/// 
/// This is called after kernel initialization is complete.
pub fn start_scheduling() {
    println!("[scheduler] Starting scheduler...");
    
    // Enable scheduler
    SCHEDULER.lock().enabled = true;
    
    // If no threads are ready, create the idle thread
    if !SCHEDULER.lock().has_runnable() {
        // Add idle thread to scheduler
        add_thread(Tid::new(0));
    }
    
    println!("[scheduler] Scheduling started");
    
    // Trigger first context switch
    unsafe {
        schedule_next();
    }
}
