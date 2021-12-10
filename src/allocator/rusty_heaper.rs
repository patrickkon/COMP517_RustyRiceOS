use super::Locked;
use alloc::alloc::{GlobalAlloc, Layout};
use core::{
    mem,
    ptr::{self, NonNull},
};

/// The block sizes to use. By construction, the first 64 bins will contain allocations of exact amounts.
/// Any bin thereafter ([1024,65536]) will contain a varying linked list, NOT a fixed block

const MAX_FIXED: usize = 504;
const MAX_BIN: usize = 65536;
const NUM_VAR_LISTS: usize = 4;
const BLOCK_SIZES: &[usize] = &[8, 16, 24, 32, 40, 48, 56, 64, 72, 80, 88, 96, 104, 112, 120, 128,
                            136, 144, 152, 160, 168, 176, 184, 192, 200, 208, 216, 224, 232, 240,
                            248, 256, 264, 272, 280, 288, 296, 304, 312, 320, 328, 336, 344, 352,
                            360, 368, 376, 384, 392, 400, 408, 416, 424, 432, 440, 448, 456, 464,
                            472, 480, 488, 496, 504, 1024, 4096, 16384, 65536];
// const BLOCK_ALIGNS: &[usize] = &[8, 16, 32, 64, 128, 256, 512, 1024, 2048];

/// Choose an appropriate block size for the given layout.
///
/// Returns an index into the `BLOCK_SIZES` array.
fn size_index(layout: &Layout) -> Option<usize> {
    let required_block_size = layout.size().max(layout.align());
    BLOCK_SIZES.iter().position(|&s| s >= required_block_size)
}

fn get_alignment(size: usize) -> usize {
    let mut two_power = 8;
    while two_power < size {
        two_power *= 2
    }
    return two_power
}

struct ListNode {
    next: Option<&'static mut ListNode>,
}

pub struct RustyHeapAllocator {
    list_heads: [Option<&'static mut ListNode>; BLOCK_SIZES.len()-NUM_VAR_LISTS],
    var_lists: [linked_list_allocator::Heap; NUM_VAR_LISTS],
    fallback_allocator: linked_list_allocator::Heap,
}

impl RustyHeapAllocator {
    /// Creates an empty RustyHeapAllocator.
    pub const fn new() -> Self {
        const EMPTY: Option<&'static mut ListNode> = None;
        const EMPTY_HEAP_SEG_1:  linked_list_allocator::Heap = linked_list_allocator::Heap::empty();
        const EMPTY_HEAP_SEG_2:  linked_list_allocator::Heap = linked_list_allocator::Heap::empty();
        const EMPTY_HEAP_SEG_3:  linked_list_allocator::Heap = linked_list_allocator::Heap::empty();
        const EMPTY_HEAP_SEG_4:  linked_list_allocator::Heap = linked_list_allocator::Heap::empty();
        RustyHeapAllocator {
            list_heads: [EMPTY; BLOCK_SIZES.len()-NUM_VAR_LISTS],
            var_lists: [EMPTY_HEAP_SEG_1, EMPTY_HEAP_SEG_2, EMPTY_HEAP_SEG_3, EMPTY_HEAP_SEG_4],
            fallback_allocator: linked_list_allocator::Heap::empty(),
        }
    }

    /// Initialize the allocator with the given heap bounds.
    ///
    /// This function is unsafe because the caller must guarantee that the given
    /// heap bounds are valid and that the heap is unused. This method must be
    /// called only once.
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        //self.fallback_allocator.init(heap_start, heap_size);
        self.fallback_allocator.init(heap_start, 3*(heap_size/4));
        //Currently, variable blocks are assigned into subheaps of size 1/16 of total heap.
        self.var_lists[0].init(heap_start+3*(heap_size/4), heap_size/16);
        self.var_lists[1].init(heap_start+13*(heap_size/16), heap_size/16);
        self.var_lists[2].init(heap_start+14*(heap_size/16), heap_size/16);
        self.var_lists[3].init(heap_start+15*(heap_size/16), heap_size/16);
    }

    /// Allocates using the fallback allocator.
    fn fallback_alloc(&mut self, layout: Layout) -> *mut u8 {
        match self.fallback_allocator.allocate_first_fit(layout) {
            Ok(ptr) => ptr.as_ptr(),
            Err(_) => ptr::null_mut(),
        }
    }
    fn var_list_alloc(&mut self, layout: Layout, size_index: Option<usize>) -> *mut u8 {
        match size_index {
            Some(index) => {
                if BLOCK_SIZES[index] == 1024 {
                    match self.var_lists[0].allocate_first_fit(layout) {
                        Ok(ptr) => ptr.as_ptr(),
                        Err(_) => ptr::null_mut(),
                    }
                }
                else if BLOCK_SIZES[index] == 4096 {
                    match self.var_lists[1].allocate_first_fit(layout) {
                        Ok(ptr) => ptr.as_ptr(),
                        Err(_) => ptr::null_mut(),
                    }
                }
                else if BLOCK_SIZES[index] == 16384 {
                    match self.var_lists[2].allocate_first_fit(layout) {
                        Ok(ptr) => ptr.as_ptr(),
                        Err(_) => ptr::null_mut(),
                    }
                }
                else if BLOCK_SIZES[index] == 65536 {
                    match self.var_lists[3].allocate_first_fit(layout) {
                        Ok(ptr) => ptr.as_ptr(),
                        Err(_) => ptr::null_mut(),
                    }
                }
                else {
                    ptr::null_mut()
                }
            }
            None => ptr::null_mut()
        }
    }
}

unsafe impl GlobalAlloc for Locked<RustyHeapAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut allocator = self.lock();
        let mut size_bin = 0;
        let size_ind = size_index(&layout);
        //if the malloced size is within fixed bin size:
        match size_ind {
            Some(index) => {
                size_bin = BLOCK_SIZES[index];
                if (size_bin <= MAX_FIXED) || (size_bin > MAX_BIN) {
                    match allocator.list_heads[index].take() {
                        Some(node) => { //List not empty
                            allocator.list_heads[index] = node.next.take(); //head now is pointing to successor
                            return node as *mut ListNode as *mut u8; //return node
                        }
                        None => {
                            // no block exists in list => allocate new block
                            let block_size = BLOCK_SIZES[index];
                            let block_align = get_alignment(block_size);
                            let layout = Layout::from_size_align(block_size, block_align).unwrap();
                            return allocator.fallback_alloc(layout);
                        }
                    }
                }
                else {
                    assert!(true);
                }
            }
            None => {return allocator.fallback_alloc(layout);}
        }
        //Else malloced size is within range of variable bins, assign to a variable bin
        if (size_bin > MAX_FIXED) && (size_bin <= MAX_BIN) {
            return allocator.var_list_alloc(layout, size_ind);
        }
        else {
            return ptr::null_mut();
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut allocator = self.lock();
        let mut size_bin = 0;
        let size_ind = size_index(&layout);
        match size_ind {
            Some(index) => {
                size_bin = BLOCK_SIZES[index];
                if (size_bin <= MAX_FIXED) || (size_bin > MAX_BIN) {
                    let new_node = ListNode {
                        next: allocator.list_heads[index].take(),
                    };
                    // verify that block has size and alignment required for storing node
                    assert!(mem::size_of::<ListNode>() <= BLOCK_SIZES[index]);
                    assert!(mem::align_of::<ListNode>() <= get_alignment(BLOCK_SIZES[index])); //align_of gets number of bytes needed for type
                    let new_node_ptr = ptr as *mut ListNode;
                    new_node_ptr.write(new_node);
                    allocator.list_heads[index] = Some(&mut *new_node_ptr);
                }
                else{
                    assert!(true);
                }
            }
            None => {
                let ptr = NonNull::new(ptr).unwrap();
                allocator.fallback_allocator.deallocate(ptr, layout);
            }
        }
        if (size_bin > MAX_FIXED) && (size_bin <= MAX_BIN) {
            let ptr = NonNull::new(ptr).unwrap();
            match size_ind {
                Some(index) => {
                    if BLOCK_SIZES[index] == 1024 {
                        allocator.var_lists[0].deallocate(ptr, layout);
                    }
                    else if BLOCK_SIZES[index] == 4096 {
                        allocator.var_lists[1].deallocate(ptr, layout);
                    }
                    else if BLOCK_SIZES[index] == 16384 {
                        allocator.var_lists[2].deallocate(ptr, layout);
                    }
                    else if BLOCK_SIZES[index] == 65536 {
                        allocator.var_lists[3].deallocate(ptr, layout);
                    }
                    else {
                        assert!(true);
                    }
                }
                None => {
                    assert!(true);
                }
            }
        }
        else{
            assert!(true);
        }
    }
}
