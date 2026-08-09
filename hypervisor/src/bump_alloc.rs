//! Static bump allocator for Gate C MVP hypervisor bring-up.

use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::mem::MaybeUninit;

const HEAP_SIZE: usize = 64 * 1024;

static mut HEAP: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];

struct BumpInner {
    next: usize,
}

/// Simple bump allocator backed by a fixed static buffer.
pub struct BumpAllocator(UnsafeCell<BumpInner>);

unsafe impl Sync for BumpAllocator {}

impl BumpAllocator {
    /// Creates a new bump allocator in its initial state.
    pub const fn new() -> Self {
        Self(UnsafeCell::new(BumpInner { next: 0 }))
    }
}

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let inner = &mut *self.0.get();
        let align = layout.align();
        let aligned = (inner.next + align - 1) & !(align - 1);
        let end = aligned
            .checked_add(layout.size())
            .unwrap_or(HEAP_SIZE + 1);
        if end > HEAP_SIZE {
            return core::ptr::null_mut();
        }
        inner.next = end;
        HEAP.as_mut_ptr().cast::<u8>().add(aligned)
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

#[global_allocator]
static ALLOCATOR: BumpAllocator = BumpAllocator::new();
