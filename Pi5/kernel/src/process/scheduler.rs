//! Round-robin task scheduler
//!
//! Implements a simple preemptive round-robin scheduler.

use alloc::collections::VecDeque;
use spin::Mutex;
use lazy_static::lazy_static;

use super::{Priority, Tid};
use crate::println;

/// Time slice in timer ticks (10ms per tick, so 100ms default)
pub const DEFAULT_TIME_SLICE: u64 = 10;

/// Current running thread on each CPU
static mut CURRENT_THREADS: [Option<Tid>; 8] = [None; 8]; // Support up to 8 CPUs

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
}

impl Scheduler {
    const fn new() -> Self {
        const EMPTY_QUEUE: VecDeque<Tid> = VecDeque::new();
        Self {
            ready_queues: [EMPTY_QUEUE; 32],
            time_slice: DEFAULT_TIME_SLICE,
            enabled: false,
            ticks: 0,
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
    use super::THREADS;

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

/// Schedule next thread to run
/// 
/// # Safety
/// This function is unsafe because it performs a context switch.
pub unsafe fn schedule_next() {
    let mut scheduler = SCHEDULER.lock();

    if !scheduler.enabled {
        return;
    }

    // Get current thread
    let cpu_id = 0; // TODO: Get actual CPU ID
    let current_tid = CURRENT_THREADS[cpu_id];

    // Get next thread from ready queue
    let next_tid = scheduler.dequeue()
        .or(current_tid)
        .unwrap_or(Tid::new(0)); // Idle thread

    // If same thread, just reset time slice and return
    if Some(next_tid) == current_tid {
        scheduler.time_slice = DEFAULT_TIME_SLICE;
        return;
    }

    // Put current thread back in queue if it's still runnable
    if let Some(tid) = current_tid {
        use super::THREADS;
        let threads = THREADS.lock();
        if let Some(thread) = threads.get(&tid.as_u64()) {
            if thread.is_runnable() {
                let priority = thread.priority;
                // Need to reacquire scheduler lock
                drop(scheduler);
                SCHEDULER.lock().enqueue(tid, priority);
                
                // Reacquire for the rest of the function
                scheduler = SCHEDULER.lock();
            }
        }
    }

    // Update current thread
    CURRENT_THREADS[cpu_id] = Some(next_tid);
    scheduler.time_slice = DEFAULT_TIME_SLICE;

    // Perform context switch
    // Note: This is a simplified version - real implementation needs more care
    drop(scheduler); // Release lock before context switch

    // TODO: Actually perform the context switch
    // switch_context(old_context, new_context);
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

    // Decrement time slice
    if scheduler.time_slice > 0 {
        scheduler.time_slice -= 1;
    }

    // If time slice expired, schedule next thread
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
    schedule_next();
}

/// Get current thread ID
pub fn current_thread() -> Option<Tid> {
    let cpu_id = 0; // TODO: Get actual CPU ID
    unsafe { CURRENT_THREADS[cpu_id] }
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

    if let Some(tid) = current_thread() {
        println!("  Current thread: {}", tid.as_u64());
    }
}

/// Block current thread (e.g., waiting for I/O)
/// 
/// # Safety
/// This function is unsafe because it triggers a context switch.
pub unsafe fn block_current() {
    use super::{THREADS, ThreadState};

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
    use super::{THREADS, ThreadState};

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
pub unsafe fn sleep_current(_ticks: u64) {
    use super::{THREADS, ThreadState};

    if let Some(tid) = current_thread() {
        let mut threads = THREADS.lock();
        if let Some(thread) = threads.get_mut(&tid.as_u64()) {
            thread.state = ThreadState::Sleeping;
            // TODO: Add to sleep queue
        }
    }

    schedule_next();
}
