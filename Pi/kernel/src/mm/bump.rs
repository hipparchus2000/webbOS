//! Bump allocator
//! 
//! Simple bump allocator for early boot before the heap is set up.

#![allow(dead_code)]



/// Bump allocator
#[allow(dead_code)]
pub struct BumpAllocator {
    heap_start: usize,
    heap_end: usize,
    next: usize,
    allocations: usize,
}

impl BumpAllocator {
    /// Create a new empty bump allocator
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) -> Option<()> {
        // Validate size won't cause overflow
        self.heap_end = heap_start.checked_add(heap_size)?;
        self.heap_start = heap_start;
        self.next = heap_start;
        Some(())
    }

    /// Allocate a chunk of memory
    #[allow(dead_code)]
    pub fn alloc(&mut self, layout: core::alloc::Layout) -> Option<*mut u8> {
        // Validate layout size won't overflow
        let size = layout.size();
        if size == 0 {
            return Some(self.heap_start as *mut u8);
        }
        
        let alloc_start = align_up(self.next, layout.align())?;
        let alloc_end = alloc_start.checked_add(size)?;

        if alloc_end <= self.heap_end {
            self.next = alloc_end;
            self.allocations = self.allocations.checked_add(1)?;
            Some(alloc_start as *mut u8)
        } else {
            None
        }
    }

    /// Deallocate memory (only works for last allocation)
    #[allow(dead_code)]
    pub fn dealloc(&mut self, _ptr: *mut u8, _layout: core::alloc::Layout) {
        // Bump allocator can't really deallocate
        self.allocations -= 1;
        if self.allocations == 0 {
            self.next = self.heap_start;
        }
    }

    /// Get number of active allocations
    #[allow(dead_code)]
    pub fn allocations(&self) -> usize {
        self.allocations
    }

    /// Get used bytes
    #[allow(dead_code)]
    pub fn used(&self) -> usize {
        self.next.saturating_sub(self.heap_start)
    }

    /// Get free bytes
    #[allow(dead_code)]
    pub fn free(&self) -> usize {
        self.heap_end.saturating_sub(self.next)
    }
}

/// Align address up to alignment with overflow protection
#[allow(dead_code)]
fn align_up(addr: usize, align: usize) -> Option<usize> {
    // Validate alignment is power of 2 and non-zero
    if align == 0 || !align.is_power_of_two() {
        return None;
    }
    let remainder = addr % align;
    if remainder == 0 {
        Some(addr)
    } else {
        addr.checked_add(align.checked_sub(remainder)?)
    }
}

unsafe impl core::alloc::GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        // This is a bit of a hack - we need mutable access but GlobalAlloc requires &self
        let ptr = self as *const Self as *mut Self;
        
        // Validate layout size
        let size = layout.size();
        let align = layout.align();
        if size == 0 {
            return (*ptr).heap_start as *mut u8;
        }
        // Validate alignment is power of 2
        if align == 0 || !align.is_power_of_two() {
            return core::ptr::null_mut();
        }
        
        let addr = (*ptr).next;
        
        // Use checked arithmetic for alignment calculation
        let align_mask = align - 1;
        let aligned_addr = addr
            .checked_add(align_mask)
            .map(|v| v & !align_mask);
        
        let Some(aligned_addr) = aligned_addr else {
            return core::ptr::null_mut();
        };
        
        let new_next = aligned_addr.checked_add(size);
        
        match new_next {
            Some(new_next) if new_next <= (*ptr).heap_end => {
                (*ptr).next = new_next;
                (*ptr).allocations = (*ptr).allocations.saturating_add(1);
                aligned_addr as *mut u8
            }
            _ => core::ptr::null_mut()
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: core::alloc::Layout) {
        // Bump allocator doesn't really deallocate
        let ptr_mut = self as *const Self as *mut Self;
        (*ptr_mut).allocations = (*ptr_mut).allocations.saturating_sub(1);
        if (*ptr_mut).allocations == 0 {
            (*ptr_mut).next = (*ptr_mut).heap_start;
        }
    }
}
