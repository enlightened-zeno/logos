use crate::memory::addr::{VirtAddr, PAGE_SIZE};
use crate::memory::heap::KERNEL_HEAP;
use crate::memory::paging::PageFlags;
use crate::memory::vmm::Vmm;
use core::alloc::{GlobalAlloc, Layout};
use core::ptr;
use core::sync::atomic::{AtomicBool, Ordering};
use spin::Mutex;

/// Size classes for the slab allocator.
const SIZE_CLASSES: [usize; 8] = [32, 64, 128, 256, 512, 1024, 2048, 4096];

/// Number of empty slabs to keep per cache before returning pages to VMM.
const HYSTERESIS_EMPTY_SLABS: usize = 2;

/// Virtual address region for slab pages. Each size class gets 4 GiB.
const SLAB_REGION_START: u64 = 0xFFFF_D000_0000_0000;
const SLAB_REGION_PER_CLASS: u64 = 4 * 1024 * 1024 * 1024;

/// Per-object free list node, stored in-place in free objects.
#[repr(C)]
struct FreeObject {
    next: *mut FreeObject,
}

/// Slab metadata, stored at the start of each slab's memory region.
#[repr(C)]
struct Slab {
    free_head: *mut FreeObject,
    allocated: u32,
    total: u32,
    base: u64,
    page_count: u64,
    next: *mut Slab,
}

// SAFETY: Protected by Mutex.
unsafe impl Send for Slab {}

/// Per-size-class cache.
struct SlabCache {
    object_size: usize,
    partial_head: *mut Slab,
    empty_count: usize,
    next_vaddr: u64,
}

// SAFETY: Protected by Mutex.
unsafe impl Send for SlabCache {}

impl SlabCache {
    const fn new(object_size: usize, base_vaddr: u64) -> Self {
        Self {
            object_size,
            partial_head: ptr::null_mut(),
            empty_count: 0,
            next_vaddr: base_vaddr,
        }
    }

    fn alloc(&mut self) -> *mut u8 {
        // Ensure we have a slab with free objects
        let needs_grow =
            self.partial_head.is_null() || unsafe { (*self.partial_head).free_head.is_null() };
        if needs_grow && !self.grow() {
            return ptr::null_mut();
        }

        // SAFETY: We just ensured partial_head is non-null with free objects.
        let slab = unsafe { &mut *self.partial_head };
        let obj = slab.free_head;
        // SAFETY: obj is a valid free object in mapped slab memory.
        slab.free_head = unsafe { (*obj).next };
        slab.allocated += 1;

        if slab.allocated == 1 && self.empty_count > 0 {
            self.empty_count -= 1;
        }

        // If slab is now full, remove from partial list
        if slab.free_head.is_null() {
            self.partial_head = slab.next;
            slab.next = ptr::null_mut();
        }

        obj as *mut u8
    }

    fn dealloc(&mut self, ptr: *mut u8) {
        // Find which slab owns this pointer by computing the slab base
        let addr = ptr as u64;
        let pages = self.pages_per_slab();
        let slab_size = pages * PAGE_SIZE;
        // Compute which slab this belongs to
        let class_start = SLAB_REGION_START + self.class_index() as u64 * SLAB_REGION_PER_CLASS;
        let offset_in_class = addr - class_start;
        let slab_index = offset_in_class / slab_size;
        let slab_base = class_start + slab_index * slab_size;
        let slab_ptr = slab_base as *mut Slab;

        // SAFETY: slab_ptr points to valid slab metadata at the base of the slab.
        let slab = unsafe { &mut *slab_ptr };
        let was_full = slab.free_head.is_null() && slab.allocated > 0;

        // Push object onto free list
        let obj = ptr as *mut FreeObject;
        // SAFETY: ptr was allocated from this slab.
        unsafe { (*obj).next = slab.free_head };
        slab.free_head = obj;
        slab.allocated -= 1;

        // If slab was full, re-add to partial list
        if was_full {
            slab.next = self.partial_head;
            self.partial_head = slab_ptr;
        }

        // Hysteresis: free empty slabs beyond the threshold
        if slab.allocated == 0 {
            self.empty_count += 1;
            if self.empty_count > HYSTERESIS_EMPTY_SLABS {
                self.remove_and_free_slab(slab_ptr);
            }
        }
    }

    fn grow(&mut self) -> bool {
        let vmm = Vmm::get();
        let pages = self.pages_per_slab();
        let vaddr = VirtAddr::new_canonicalize(self.next_vaddr);

        if vmm
            .alloc_and_map(vaddr, pages, PageFlags::WRITABLE | PageFlags::NO_EXECUTE)
            .is_err()
        {
            return false;
        }

        let slab_ptr = self.next_vaddr as *mut Slab;
        self.next_vaddr += pages * PAGE_SIZE;

        // Initialize slab: metadata at start, objects after
        let metadata_size = core::mem::size_of::<Slab>().next_multiple_of(self.object_size);
        let data_start = slab_ptr as u64 + metadata_size as u64;
        let data_end = slab_ptr as u64 + pages * PAGE_SIZE;
        let object_count = ((data_end - data_start) / self.object_size as u64) as u32;

        // Build free list in reverse order so first alloc gets lowest address
        let mut head: *mut FreeObject = ptr::null_mut();
        for i in (0..object_count).rev() {
            let obj = (data_start + i as u64 * self.object_size as u64) as *mut FreeObject;
            // SAFETY: obj is within freshly mapped, zeroed slab memory.
            unsafe { (*obj).next = head };
            head = obj;
        }

        // SAFETY: slab_ptr points to freshly mapped, zeroed memory.
        unsafe {
            (*slab_ptr).free_head = head;
            (*slab_ptr).allocated = 0;
            (*slab_ptr).total = object_count;
            (*slab_ptr).base = slab_ptr as u64;
            (*slab_ptr).page_count = pages;
            (*slab_ptr).next = self.partial_head;
        }

        self.partial_head = slab_ptr;
        self.empty_count += 1;
        true
    }

    fn remove_and_free_slab(&mut self, slab_ptr: *mut Slab) {
        // Remove from partial list
        let mut prev: *mut Slab = ptr::null_mut();
        let mut current = self.partial_head;
        while !current.is_null() {
            if current == slab_ptr {
                // SAFETY: Updating linked list pointers of valid slabs.
                unsafe {
                    if prev.is_null() {
                        self.partial_head = (*current).next;
                    } else {
                        (*prev).next = (*current).next;
                    }
                }
                break;
            }
            prev = current;
            // SAFETY: Traversing valid slab list.
            current = unsafe { (*current).next };
        }

        self.empty_count -= 1;

        // SAFETY: Reading valid slab metadata before freeing.
        let (base, pages) = unsafe { ((*slab_ptr).base, (*slab_ptr).page_count) };
        Vmm::get().unmap_and_free(VirtAddr::new_canonicalize(base), pages);
    }

    fn pages_per_slab(&self) -> u64 {
        let min_data = self.object_size * 8 + core::mem::size_of::<Slab>();
        (min_data as u64).div_ceil(PAGE_SIZE).max(1)
    }

    fn class_index(&self) -> usize {
        SIZE_CLASSES
            .iter()
            .position(|&s| s == self.object_size)
            .unwrap_or(0)
    }
}

/// Unified kernel allocator: slab caches for small sizes, linked-list heap for large.
pub struct KernelAllocator {
    caches: [Mutex<SlabCache>; 8],
    slab_active: AtomicBool,
}

impl KernelAllocator {
    const fn new() -> Self {
        Self {
            #[allow(clippy::erasing_op, clippy::identity_op)]
            caches: [
                Mutex::new(SlabCache::new(SIZE_CLASSES[0], SLAB_REGION_START)),
                Mutex::new(SlabCache::new(
                    SIZE_CLASSES[1],
                    SLAB_REGION_START + SLAB_REGION_PER_CLASS,
                )),
                Mutex::new(SlabCache::new(
                    SIZE_CLASSES[2],
                    SLAB_REGION_START + SLAB_REGION_PER_CLASS * 2,
                )),
                Mutex::new(SlabCache::new(
                    SIZE_CLASSES[3],
                    SLAB_REGION_START + SLAB_REGION_PER_CLASS * 3,
                )),
                Mutex::new(SlabCache::new(
                    SIZE_CLASSES[4],
                    SLAB_REGION_START + SLAB_REGION_PER_CLASS * 4,
                )),
                Mutex::new(SlabCache::new(
                    SIZE_CLASSES[5],
                    SLAB_REGION_START + SLAB_REGION_PER_CLASS * 5,
                )),
                Mutex::new(SlabCache::new(
                    SIZE_CLASSES[6],
                    SLAB_REGION_START + SLAB_REGION_PER_CLASS * 6,
                )),
                Mutex::new(SlabCache::new(
                    SIZE_CLASSES[7],
                    SLAB_REGION_START + SLAB_REGION_PER_CLASS * 7,
                )),
            ],
            slab_active: AtomicBool::new(false),
        }
    }

    /// Activate the slab caches. Before this, all allocations go to the heap.
    pub fn activate(&self) {
        self.slab_active.store(true, Ordering::Release);
    }

    fn cache_index(size: usize) -> Option<usize> {
        SIZE_CLASSES.iter().position(|&s| size <= s)
    }

    fn is_in_slab_region(addr: u64) -> bool {
        addr >= SLAB_REGION_START
            && addr < SLAB_REGION_START + SLAB_REGION_PER_CLASS * SIZE_CLASSES.len() as u64
    }
}

// SAFETY: Small allocations go to per-size-class slab caches (Mutex-protected).
// Large allocations (>4096) and pre-init allocations go to the linked-list heap.
unsafe impl GlobalAlloc for KernelAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size().max(layout.align());

        // Use slab for small allocations when active
        if self.slab_active.load(Ordering::Acquire) {
            if let Some(idx) = Self::cache_index(size) {
                let ptr = self.caches[idx].lock().alloc();
                if !ptr.is_null() {
                    return ptr;
                }
            }
        }

        // Fall back to linked-list heap
        // SAFETY: Delegating to the heap allocator.
        unsafe { KERNEL_HEAP.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let addr = ptr as u64;

        // If pointer is in the slab region, free via slab
        if Self::is_in_slab_region(addr) {
            let size = layout.size().max(layout.align());
            if let Some(idx) = Self::cache_index(size) {
                self.caches[idx].lock().dealloc(ptr);
                return;
            }
        }

        // Otherwise, free via linked-list heap
        // SAFETY: Delegating to the heap allocator.
        unsafe { KERNEL_HEAP.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: KernelAllocator = KernelAllocator::new();

/// Activate the slab allocator. Call after heap is initialized.
pub fn activate() {
    ALLOCATOR.activate();
    crate::serial_println!("Slab: activated (size classes: 32..4096)");
}
