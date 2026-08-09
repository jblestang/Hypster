//! Static bump allocator for Gate C MVP hypervisor bring-up.

use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::ptr::NonNull;

const HEAP_SIZE: usize = 64 * 1024;

struct HeapStorage(UnsafeCell<[MaybeUninit<u8>; HEAP_SIZE]>);

unsafe impl Sync for HeapStorage {}

static HEAP: HeapStorage = HeapStorage(UnsafeCell::new([MaybeUninit::uninit(); HEAP_SIZE]));

struct BumpInner {
    next: usize,
}

/// Simple bump allocator backed by a fixed static buffer.
pub struct BumpAllocator(UnsafeCell<BumpInner>);

unsafe impl Sync for BumpAllocator {}

impl BumpAllocator {
    /// Creates a new bump allocator in its initial state.
    #[must_use]
    pub const fn new() -> Self {
        Self(UnsafeCell::new(BumpInner { next: 0 }))
    }
}

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let inner = &mut *self.0.get();
        let align = layout.align();
        let aligned = (inner.next + align - 1) & !(align - 1);
        let end = aligned.checked_add(layout.size()).unwrap_or(HEAP_SIZE + 1);
        if end > HEAP_SIZE {
            return core::ptr::null_mut();
        }
        inner.next = end;
        // SAFETY: `HEAP` is only accessed through this single-threaded bump allocator.
        let heap = NonNull::new_unchecked(HEAP.0.get().cast::<u8>());
        heap.as_ptr().add(aligned)
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}
