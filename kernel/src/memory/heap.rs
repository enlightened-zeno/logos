use crate::memory::addr::VirtAddr;
use crate::memory::paging::PageFlags;
use crate::memory::vmm::{layout, Vmm};
use core::alloc::{GlobalAlloc, Layout};
use core::ptr;
use spin::Mutex;

/// Linked-list heap allocator for bootstrap use.
///
/// Simple first-fit allocator. Replaced by the slab allocator after
/// early boot, but remains as the fallback for large allocations.
pub struct LinkedListHeap {
    inner: Mutex<LinkedListHeapInner>,
}

struct LinkedListHeapInner {
    head: *mut FreeBlock,
    heap_start: u64,
    heap_end: u64,
    heap_max: u64,
    allocated_bytes: u64,
}

// SAFETY: The heap is protected by a Mutex.
unsafe impl Send for LinkedListHeapInner {}

#[repr(C)]
struct FreeBlock {
    size: u64,
    next: *mut FreeBlock,
}

impl FreeBlock {
    const MIN_SIZE: u64 = core::mem::size_of::<FreeBlock>() as u64;
}

impl LinkedListHeap {
    pub const fn new() -> Self {
        Self {
            inner: Mutex::new(LinkedListHeapInner {
                head: ptr::null_mut(),
                heap_start: 0,
                heap_end: 0,
                heap_max: 0,
                allocated_bytes: 0,
            }),
        }
    }

    /// Initialize the heap by mapping initial pages.
    ///
    /// # Safety
    /// Must be called exactly once after VMM and PMM are initialized.
    pub unsafe fn init(&self) {
        let vmm = Vmm::get();
        let start = layout::KERNEL_HEAP_START;
        let initial_pages = layout::KERNEL_HEAP_INITIAL_SIZE / 4096;

        vmm.alloc_and_map(
            start,
            initial_pages,
            PageFlags::WRITABLE | PageFlags::NO_EXECUTE,
        )
        .expect("Failed to map initial kernel heap");

        let mut inner = self.inner.lock();
        inner.heap_start = start.as_u64();
        inner.heap_end = start.as_u64() + layout::KERNEL_HEAP_INITIAL_SIZE;
        inner.heap_max = start.as_u64() + layout::KERNEL_HEAP_MAX_SIZE;

        // Create one large free block spanning the entire heap
        let block = start.as_u64() as *mut FreeBlock;
        // SAFETY: We just mapped this memory and it's zeroed.
        unsafe {
            (*block).size = layout::KERNEL_HEAP_INITIAL_SIZE;
            (*block).next = ptr::null_mut();
        }
        inner.head = block;
    }

    /// Try to grow the heap by mapping more pages.
    fn grow(inner: &mut LinkedListHeapInner, min_bytes: u64) -> bool {
        let grow_size = min_bytes.next_multiple_of(4096).max(1024 * 1024); // Grow by at least 1 MiB

        let new_end = inner.heap_end + grow_size;
        if new_end > inner.heap_max {
            return false;
        }

        let vmm = Vmm::get();
        let start = VirtAddr::new_canonicalize(inner.heap_end);
        let pages = grow_size / 4096;

        if vmm
            .alloc_and_map(start, pages, PageFlags::WRITABLE | PageFlags::NO_EXECUTE)
            .is_err()
        {
            return false;
        }

        // Add the new region as a free block
        let block = inner.heap_end as *mut FreeBlock;
        // SAFETY: We just mapped this memory.
        unsafe {
            (*block).size = grow_size;
            (*block).next = inner.head;
        }
        inner.head = block;
        inner.heap_end = new_end;

        true
    }

    pub fn allocated_bytes(&self) -> u64 {
        self.inner.lock().allocated_bytes
    }
}

// SAFETY: We implement a proper allocator with correct alignment handling,
// size tracking (stored before the returned pointer), and a free list.
unsafe impl GlobalAlloc for LinkedListHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut inner = self.inner.lock();

        let align = layout.align().max(8) as u64;
        // We store the block size in 8 bytes before the returned pointer
        let header_size = 8u64;
        let total_size = (header_size + layout.size() as u64)
            .next_multiple_of(align)
            .max(FreeBlock::MIN_SIZE);

        // First-fit search
        let mut prev: *mut FreeBlock = ptr::null_mut();
        let mut current = inner.head;

        loop {
            if current.is_null() {
                // Try to grow the heap
                if !Self::grow(&mut inner, total_size) {
                    return ptr::null_mut();
                }
                // Restart search from head after growing
                prev = ptr::null_mut();
                current = inner.head;
                continue;
            }

            // SAFETY: current is a valid free block in mapped heap memory.
            let block_size = unsafe { (*current).size };

            if block_size >= total_size {
                // Can we split this block?
                let remainder = block_size - total_size;
                if remainder >= FreeBlock::MIN_SIZE {
                    // Split: create a new free block after our allocation
                    let new_block = (current as u64 + total_size) as *mut FreeBlock;
                    // SAFETY: new_block is within the original block's bounds.
                    unsafe {
                        (*new_block).size = remainder;
                        (*new_block).next = (*current).next;
                    }

                    // Remove current from free list, insert new_block
                    if prev.is_null() {
                        inner.head = new_block;
                    } else {
                        // SAFETY: prev is a valid free block.
                        unsafe {
                            (*prev).next = new_block;
                        }
                    }

                    // Store actual allocated size in the block
                    // SAFETY: current points to valid mapped memory.
                    unsafe {
                        *(current as *mut u64) = total_size;
                    }
                } else {
                    // Use the entire block
                    // SAFETY: Updating the free list pointers.
                    unsafe {
                        if prev.is_null() {
                            inner.head = (*current).next;
                        } else {
                            (*prev).next = (*current).next;
                        }
                        *(current as *mut u64) = block_size;
                    }
                }

                inner.allocated_bytes += total_size;

                // Return pointer after the size header
                return (current as u64 + header_size) as *mut u8;
            }

            prev = current;
            // SAFETY: current is a valid free block.
            current = unsafe { (*current).next };
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        if ptr.is_null() {
            return;
        }

        let mut inner = self.inner.lock();

        // Read the block size from the header
        let block_ptr = (ptr as u64 - 8) as *mut FreeBlock;
        // SAFETY: We stored the size at this location during alloc.
        let block_size = unsafe { *(block_ptr as *const u64) };

        inner.allocated_bytes = inner.allocated_bytes.saturating_sub(block_size);

        // Add block back to free list (insert at head for O(1))
        // SAFETY: The block is being freed and its memory is valid.
        unsafe {
            (*block_ptr).size = block_size;
            (*block_ptr).next = inner.head;
        }
        inner.head = block_ptr;
    }
}

/// Initialize the kernel heap. Must be called after PMM and VMM.
///
/// # Safety
/// Must be called exactly once during single-threaded boot.
pub unsafe fn init() {
    // SAFETY: Caller guarantees single call after PMM/VMM init.
    unsafe {
        KERNEL_HEAP.init();
    }
}

/// The kernel's linked-list heap, used directly as the global allocator
/// during early boot and as the fallback for large allocations once the
/// slab allocator is active.
pub static KERNEL_HEAP: LinkedListHeap = LinkedListHeap::new();

/// Get current heap allocation statistics.
pub fn allocated_bytes() -> u64 {
    KERNEL_HEAP.allocated_bytes()
}
