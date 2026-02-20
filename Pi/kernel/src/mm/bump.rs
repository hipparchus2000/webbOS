//! Bump allocator
//! 
//! Simple bump allocator for early boot before the heap is set up.



/// Bump allocator
pub struct BumpAllocator {
    heap_start: usize,
    heap_end: usize,
    next: usize,
    allocations: usize,
}

impl BumpAllocator {
    /// Create a new empty bump allocator
    pub const fn new() -> Self {
        BumpAllocator {
            heap_start: 0,
            heap_end: 0,
            next: 0,
            allocations: 0,
        }
    }

    /// Initialize the bump allocator with a memory region
    /// 
    /// # Safety
    /// The caller must ensure that the given memory range is valid and unused
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.heap_start = heap_start;
        self.heap_end = heap_start + heap_size;
        self.next = heap_start;
    }

    /// Allocate a chunk of memory
    pub fn alloc(&mut self, layout: core::alloc::Layout) -> Option<*mut u8> {
        let alloc_start = align_up(self.next, layout.align());
        let alloc_end = alloc_start.checked_add(layout.size())?;

        if alloc_end <= self.heap_end {
            self.next = alloc_end;
            self.allocations += 1;
            Some(alloc_start as *mut u8)
        } else {
            None
        }
    }

    /// Deallocate memory (only works for last allocation)
    pub fn dealloc(&mut self, _ptr: *mut u8, _layout: core::alloc::Layout) {
        // Bump allocator can't really deallocate
        self.allocations -= 1;
        if self.allocations == 0 {
            self.next = self.heap_start;
        }
    }

    /// Get number of active allocations
    pub fn allocations(&self) -> usize {
        self.allocations
    }

    /// Get used bytes
    pub fn used(&self) -> usize {
        self.next - self.heap_start
    }

    /// Get free bytes
    pub fn free(&self) -> usize {
        self.heap_end - self.next
    }
}

/// Align address up to alignment
fn align_up(addr: usize, align: usize) -> usize {
    let remainder = addr % align;
    if remainder == 0 {
        addr
    } else {
        addr + (align - remainder)
    }
}

unsafe impl core::alloc::GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        // This is a bit of a hack - we need mutable access but GlobalAlloc requires &self
        let ptr = self as *const Self as *mut Self;
        // Call the BumpAllocator's alloc method (which returns Option<usize>)
        let addr = (*ptr).next;
        let aligned_addr = (addr + layout.align() - 1) & !(layout.align() - 1);
        let new_next = aligned_addr + layout.size();
        
        if new_next <= (*ptr).heap_end {
            (*ptr).next = new_next;
            (*ptr).allocations += 1;
            aligned_addr as *mut u8
        } else {
            core::ptr::null_mut()
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: core::alloc::Layout) {
        // Bump allocator doesn't really deallocate
        let ptr_mut = self as *const Self as *mut Self;
        (*ptr_mut).allocations -= 1;
        if (*ptr_mut).allocations == 0 {
            (*ptr_mut).next = (*ptr_mut).heap_start;
        }
    }
}
