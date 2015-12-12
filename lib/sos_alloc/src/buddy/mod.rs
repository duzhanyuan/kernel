mod math;

use super::RawLink;
use self::math::PowersOf2Ext;

use core::mem;
use core::cmp::{max, min};

pub struct Free { next: RawLink<Free> }

// Variadic macro for taking the maximum of n > 2 numbers.
// because I'm lazy.
macro_rules! max {
    ($x:expr) => ($x);
    ($x:expr, $($xs:expr),+) => (max($x, max!($($xs),+)));
}

pub struct FreeList<'a> {
    /// A pointer to the head of the free list
    head: Option<&'a mut Free>
  , /// Number of blocks in the free list
    pub length: usize
}

impl<'a> FreeList<'a> {

    /// Create a new empty `FreeList`
    const fn new() -> FreeList<'a> {
        FreeList { head: None, length: 0 }
    }

    /// Push a new block onto the free list
    ///
    /// # Unsafe due to
    ///   - `mem::transmute()`
    ///   - Dereferencing a raw pointer
    unsafe fn push(&mut self, block: *mut u8) {
        let block_ptr = block as *mut Free;
        // be nice if rawlink was kinder to pattern-matching but whatever
        *block_ptr = if let Some(head) = self.head.take() {
            Free { next: RawLink::some(head) }
        } else {
            Free { next: RawLink::none() }
        };
        self.head = Some(mem::transmute(block_ptr));
        self.length += 1;
    }
    /// Pop the head block off of the free list.
    ///
    /// # Returns
    ///   - `Some(*mut u8)` if the free list has blocks left
    ///   - `None` if the free list is empty
    ///
    /// # Unsafe due to
    ///   - `mem::transmute()`
    ///   - Dereferencing a raw pointer
    unsafe fn pop(&mut self) -> Option<*mut u8> {
        self.head.take()
            .map(|head| {
                let popped_block
                    = mem::replace(&mut self.head, head.next.resolve_mut());
                let block_ptr: *mut u8
                    = mem::transmute(popped_block);
                block_ptr
            })
    }

    /// Attempt to remove a block from the free list.
    ///
    /// This function searches the free list for the specified block, and
    /// removes it if it is found, returning whether or not the block was
    /// removed.
    ///
    /// This is quite slow; with a worst-case time complexity of O(log n),
    /// this function is a major bottleneck in our allocator implementation.
    /// By maintaining sorted free lists, we could perhaps improve performance
    /// somewhat.
    ///
    /// # Returns
    ///   - `true` if the block was removed from the free list
    ///   - `false` if the block was not present in the free list
    unsafe fn remove(&mut self, target_block: *mut u8) -> bool {
        let target_ptr = target_block as *mut Free;
        for block in self.iter_mut() {
            let block_ptr: *mut Free = block;
            if block_ptr == target_ptr {
                *block_ptr = Free { next: block.next.take() };
                return true;
            }
        }
        false
    }

    /// Returns an iterator over the blocks in this free list
    fn iter<'b>(&'b self) -> FreeListIter<'b> {
        // FreeListIter { current: self.head.map(|c| c.borrow())
        //                             .as_ref()
        //              }
        match self.head {
            Some(ref head) => FreeListIter { current: Some(head) }
          , None           => FreeListIter { current: None }
        }
        // unimplemented!()
    }

    /// Returns a mutable iterator over the blocks in this free list.
    fn iter_mut<'b>(&'b mut self) -> FreeListIterMut<'b> {
        // FreeListIterMut { current: self.head.map(|c| *c ).as_mut() }
        match self.head {
            Some(ref mut head) => FreeListIterMut { current: Some(head) }
          , None               => FreeListIterMut { current: None }
        }
    }
}

struct FreeListIter<'a> {
    current: Option<&'a Free>
}

impl<'a> Iterator for FreeListIter<'a> {
    type Item = &'a Free;

    fn next(&mut self) -> Option<&'a Free> {
        self.current
            .map(|c| {
                self.current = unsafe { c.next.resolve() };
                c
            })
    }
}

struct FreeListIterMut<'a> {
    current: Option<&'a mut Free>
}

impl<'a> Iterator for FreeListIterMut<'a> {
    type Item = &'a mut Free;

    fn next(&mut self) -> Option<&'a mut Free> {
        self.current.take()
            .map(|c| {
                self.current = unsafe { c.next.resolve_mut() };
                c
            })
    }
}

pub struct BuddyHeapAllocator<'a> {
    /// Address of the base of the heap. This must be aligned
    /// on a `MIN_ALIGN` boundary.
    start_addr: *mut u8
  , /// The allocator's free list
    free_lists: &'a mut [FreeList<'a>]
  , /// Number of blocks in the heap (must be a power of 2)
    heap_size: usize
  , /// Minimum block size
    min_block_size: usize
}

impl<'a> BuddyHeapAllocator<'a> {
    pub unsafe fn new( start_addr: *mut u8
                     , free_lists: &'a mut [FreeList<'a>]
                     , heap_size: usize) -> BuddyHeapAllocator<'a> {
        let n_free_lists = free_lists.len();

        assert!( !start_addr.is_null()
                , "Heap start address cannot be null." );
        assert!( n_free_lists > 0
               , "Allocator must have at least one free list.");
        assert!( start_addr as usize & (::PAGE_SIZE-1) == 0
               , "Heap start address must be aligned on a 4k boundary.");

        let min_block_size = heap_size >> (n_free_lists - 1);

        assert!( heap_size >= min_block_size
               , "Heap must be large enough to contain at least one block.");
        assert!( min_block_size >= mem::size_of::<Free>()
               , "Minimum block size must be large enough to contain \
                  the free block header.");

        let mut heap
            = BuddyHeapAllocator { start_addr: start_addr
                                 , free_lists: free_lists
                                 , heap_size: heap_size
                                 , min_block_size: min_block_size
                                 };
        // TODO: put first head block on appropriately-sized freelist
        heap
    }

    /// Computes the size of an allocation request.
    ///
    /// # Arguments
    ///   - `size`: A `usize` containing the size of the request
    ///   - `align`: A `usize` containing the alignment of the request
    ///
    /// # Returns
    ///   - `None` if the request is invalid
    ///   - `Some(usize)` containing the size needed if the request is valid
    #[inline]
    pub fn alloc_size(&self, size: usize, align: usize) -> Option<usize> {
        // Pre-check if this is a valid allocation request:
        //  - allocations must be aligned on power of 2 boundaries
        //  - we cannot allocate requests with alignments greater than the
        //    base alignment of the heap without jumping through a bunch of
        //    hoops.
        if !align.is_pow2() || align > ::PAGE_SIZE {
            None
        // If the request is valid, compute the size we need to allocate
        } else {
            let alloc_size
                // the allocation size for the request is the next power of 2
                // after the size of the request, the alignment of the request,
                // or the minimum block size (whichever is greatest).
                = max!( size
                        // we can't allocate less than the minimum block size
                      , self.min_block_size
                        // we can't allocate less than the alignment, either
                      , align )
                    .next_pow2();

            if alloc_size > self.heap_size {
                // if the calculated size is greater than the size of the heap,
                // we (obviously) cannot allocate this request.
                None
            } else {
                // otherwise, return the calculated size.
                Some(alloc_size)
            }
        }
    }

    /// Computes the order of an allocation request.
    ///
    /// The "order" of an allocation refers to the number of times we need to
    /// double the minimum block size to get a large enough block for that
    /// allocation.
    #[inline]
    pub fn alloc_order(&self, size: usize, align: usize) -> Option<usize> {
        self.alloc_size(size, align)
            .map(|s| // the order of the allocation is the base-2 log of the
                     // allocation size minus the base-2 log of the minimum
                     // block size
                s.log2() - self.min_block_size.log2() // TODO: cache this?
            )
    }

    // pub unsafe fn allocate
}
