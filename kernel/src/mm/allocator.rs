//! Kernel heap allocator

use linked_list_allocator::LockedHeap;
use crate::arch::paging::{Page, PageTableFlags, BootInfoFrameAllocator, OffsetPageTable, MapToError};
use super::{HEAP_SIZE, HEAP_START};

/// Global heap allocator
#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

/// Initialize the kernel heap
/// 
/// # Safety
/// Must be called exactly once during kernel initialization
pub fn init_heap(
    mapper: &mut OffsetPageTable,
    frame_allocator: &mut BootInfoFrameAllocator,
) -> Result<(), MapToError> {
    let heap_start = HEAP_START;
    let heap_end = heap_start + HEAP_SIZE - 1;
    let heap_start_page = Page::containing_address(heap_start);
    let heap_end_page = Page::containing_address(heap_end);
    
    // Calculate number of pages
    let start_idx = heap_start_page.addr() >> 12;
    let end_idx = heap_end_page.addr() >> 12;
    let num_pages = end_idx - start_idx + 1;

    for i in 0..num_pages {
        let page_addr = heap_start + (i << 12);
        let page = Page::containing_address(page_addr);
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;
        let flags = PageTableFlags::PRESENT
            .union(PageTableFlags::WRITABLE);
        unsafe {
            mapper.map_to(page, frame, flags, frame_allocator)?;
        }
    }

    unsafe {
        ALLOCATOR.lock().init(HEAP_START as *mut u8, HEAP_SIZE as usize);
    }

    Ok(())
}

/// Get used heap bytes
pub fn used_heap() -> u64 {
    ALLOCATOR.lock().used() as u64
}

/// Get free heap bytes
pub fn free_heap() -> u64 {
    ALLOCATOR.lock().free() as u64
}

/// Allocation error handler
#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}
