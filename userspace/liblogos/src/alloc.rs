use core::alloc::{GlobalAlloc, Layout};

/// Simple brk-based allocator for userspace.
///
/// Uses the brk syscall to grow the heap. Does not support deallocation
/// (bump allocator). Sufficient for simple programs.
pub struct BrkAllocator {
    // All state managed via brk syscall
}

// SAFETY: BrkAllocator uses brk which is process-global and single-threaded
// in our current model.
unsafe impl GlobalAlloc for BrkAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let current = crate::syscall::brk(0) as u64;
        let aligned = (current + layout.align() as u64 - 1) & !(layout.align() as u64 - 1);
        let new_brk = aligned + layout.size() as u64;

        let result = crate::syscall::brk(new_brk);
        if result < 0 {
            return core::ptr::null_mut();
        }

        aligned as *mut u8
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Bump allocator — no deallocation
    }
}

/// Global allocator instance.
#[global_allocator]
pub static ALLOCATOR: BrkAllocator = BrkAllocator {};
