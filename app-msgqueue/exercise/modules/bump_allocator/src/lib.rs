#![no_std]

use allocator::{AllocResult, BaseAllocator, ByteAllocator, PageAllocator};
use core::alloc::Layout;
use core::ptr::NonNull;

/// Early memory allocator using a bump (linear) allocation strategy.
///
/// Memory layout:
/// ```
/// [ bytes-used | avail-area | pages-used ]
/// |            | -->    <-- |            |
/// start       b_pos        p_pos       end
/// ```
///
/// - Bytes are allocated forward from `b_pos`
/// - Pages are allocated backward from `p_pos`
/// - `b_count` tracks number of byte allocations; when it reaches 0, bytes area is freed
pub struct EarlyAllocator<const PAGE_SIZE: usize> {
    start: usize,
    end: usize,
    b_pos: usize,   // next byte allocation position (grows forward)
    p_pos: usize,   // next page allocation position (grows backward)
    b_count: usize, // number of active byte allocations
}

impl<const PAGE_SIZE: usize> EarlyAllocator<PAGE_SIZE> {
    pub const fn new() -> Self {
        Self {
            start: 0,
            end: 0,
            b_pos: 0,
            p_pos: 0,
            b_count: 0,
        }
    }
}

impl<const PAGE_SIZE: usize> BaseAllocator for EarlyAllocator<PAGE_SIZE> {
    fn init(&mut self, start: usize, size: usize) {
        self.start = start;
        self.end = start + size;
        self.b_pos = start;
        self.p_pos = start + size;
        self.b_count = 0;
    }

    fn add_memory(&mut self, _start: usize, _size: usize) -> AllocResult {
        // Bump allocator doesn't support adding more memory regions
        Err(allocator::AllocError::NoMemory)
    }
}

impl<const PAGE_SIZE: usize> ByteAllocator for EarlyAllocator<PAGE_SIZE> {
    fn alloc(&mut self, layout: Layout) -> AllocResult<NonNull<u8>> {
        let align = layout.align();
        let size = layout.size();

        // Align b_pos up to the required alignment
        let aligned = (self.b_pos + align - 1) & !(align - 1);
        let new_pos = aligned + size;

        if new_pos > self.p_pos {
            return Err(allocator::AllocError::NoMemory);
        }

        self.b_pos = new_pos;
        self.b_count += 1;

        // Safety: aligned is within [start, p_pos), which is valid memory
        Ok(unsafe { NonNull::new_unchecked(aligned as *mut u8) })
    }

    fn dealloc(&mut self, _pos: NonNull<u8>, _layout: Layout) {
        // Decrement count; when it reaches zero, reset the bytes area
        if self.b_count > 0 {
            self.b_count -= 1;
        }
        if self.b_count == 0 {
            self.b_pos = self.start;
        }
    }

    fn total_bytes(&self) -> usize {
        self.end - self.start
    }

    fn used_bytes(&self) -> usize {
        self.b_pos - self.start
    }

    fn available_bytes(&self) -> usize {
        self.p_pos - self.b_pos
    }
}

impl<const PAGE_SIZE: usize> PageAllocator for EarlyAllocator<PAGE_SIZE> {
    const PAGE_SIZE: usize = PAGE_SIZE;

    fn alloc_pages(&mut self, num_pages: usize, align_pow2: usize) -> AllocResult<usize> {
        let size = num_pages * PAGE_SIZE;
        let align = (1 << align_pow2) * PAGE_SIZE;

        // Allocate pages backward from p_pos
        // Align p_pos down to the required alignment
        let new_pos = self.p_pos.checked_sub(size).ok_or(allocator::AllocError::NoMemory)?;
        let aligned = new_pos & !(align - 1);

        if aligned < self.b_pos {
            return Err(allocator::AllocError::NoMemory);
        }

        self.p_pos = aligned;
        Ok(aligned)
    }

    fn dealloc_pages(&mut self, _pos: usize, _num_pages: usize) {
        // Pages are never freed in the bump allocator
    }

    fn total_pages(&self) -> usize {
        (self.end - self.start) / PAGE_SIZE
    }

    fn used_pages(&self) -> usize {
        (self.end - self.p_pos) / PAGE_SIZE
    }

    fn available_pages(&self) -> usize {
        (self.p_pos - self.b_pos) / PAGE_SIZE
    }
}
