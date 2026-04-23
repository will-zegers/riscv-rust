use core::{mem::size_of, ptr::null_mut};

unsafe extern "C" {
    pub static HEAP_START: usize;
    pub static HEAP_SIZE: usize;
}

static mut ALLOC_START: usize = 0;
pub const PAGE_ORDER: usize = 12;
pub const PAGE_SIZE: usize = 1 << PAGE_ORDER;

pub const fn align_val(val: usize, order: usize) -> usize {
    let o = (1usize << order) - 1;
    (val + o) & !o
}

#[repr(u8)]
pub enum PageDescriptorBits {
    Empty = 0,
    Taken = 1 << 0,
    Last = 1 << 1,
}

impl PageDescriptorBits {
    pub fn val(self) -> u8 {
        self as u8
    }
}

pub struct PageDescriptor {
    flags: u8,
}

impl PageDescriptor {
    pub fn is_last(&self) -> bool {
        if self.flags & PageDescriptorBits::Last.val() != 0 {
            true
        } else {
            false
        }
    }

    pub fn is_taken(&self) -> bool {
        if self.flags & PageDescriptorBits::Taken.val() != 0 {
            true
        } else {
            false
        }
    }

    pub fn is_free(&self) -> bool {
        !self.is_taken()
    }

    pub fn clear(&mut self) {
        self.flags = PageDescriptorBits::Empty.val();
    }

    pub fn set_flag(&mut self, flag: PageDescriptorBits) {
        self.flags |= flag.val();
    }
}

pub fn init() {
    unsafe {
        // The start of heap memory will contain PageDescriptors for each
        // page to be allocated in memory
        let num_pages = HEAP_SIZE / PAGE_SIZE;
        let ptr = HEAP_START as *mut PageDescriptor;

        for i in 0..num_pages {
            (*ptr.add(i)).clear();
        }

        // Where allocated memory will start after the offset of PageDescriptors
        ALLOC_START = align_val(
            HEAP_START + num_pages * size_of::<PageDescriptor>(),
            PAGE_ORDER,
        );
    }
}

pub fn alloc(pages: usize) -> *mut u8 {
    assert!(pages > 0);
    unsafe {
        let num_pages = HEAP_SIZE / PAGE_SIZE;
        let ptr = HEAP_START as *mut PageDescriptor;
        for i in 0..num_pages - pages {
            let mut found = false;

            // Iterate through PageDescriptors to find allocatable memory
            if (*ptr.add(i)).is_free() {
                found = true;
                for j in i..i + pages {
                    if (*ptr.add(j)).is_taken() {
                        // Requested pages will not fit in this chunk.
                        // Restart the search
                        found = false;
                        break;
                    }
                }
            }

            if found {
                // If we found a chunk of allocatable memory that can hold all
                // requested pages, mark them as taken and return the address
                // of the start of the allocation
                for k in i..i + pages - 1 {
                    (*ptr.add(k)).set_flag(PageDescriptorBits::Taken)
                }

                // Mark the last page as Taken and Last
                (*ptr.add(i + pages - 1)).set_flag(PageDescriptorBits::Taken);
                (*ptr.add(i + pages - 1)).set_flag(PageDescriptorBits::Last);

                return (ALLOC_START + PAGE_SIZE * i) as *mut u8;
            }
        }
    }
    null_mut()
}

pub fn zalloc(pages: usize) -> *mut u8 {
    // Allocate pages and zero out all bytes of memory
    let allocated = alloc(pages);
    if !allocated.is_null() {
        let size = (PAGE_SIZE * pages) / 8;
        let big_ptr = allocated as *mut u64;
        for i in 0..size {
            unsafe {
                (*big_ptr.add(i)) = 0;
            }
        }
    }
    allocated
}

pub fn dealloc(ptr: *mut u8) {
    assert!(!ptr.is_null());
    unsafe {
        // Calculate the starting address of memory to be deallocated
        let addr = HEAP_START + (ptr as usize - ALLOC_START) / PAGE_SIZE;
        assert!(addr >= HEAP_START && addr < HEAP_START + HEAP_SIZE);

        let mut p = addr as *mut PageDescriptor;

        // Set all PageDescriptors up until the last of this allocation
        // to be empty.
        while (*p).is_taken() && !(*p).is_last() {
            (*p).clear();
            p = p.add(1);
        }

        assert!((*p).is_last() == true, "Possible double-free detected!");
        (*p).clear();
    }
}

pub fn print_page_allocations() {
    unsafe {
        let num_pages = HEAP_SIZE / PAGE_SIZE;
        let mut beg = HEAP_START as *const PageDescriptor;
        let end = beg.add(num_pages);
        let alloc_beg = ALLOC_START;
        let alloc_end = ALLOC_START + num_pages * PAGE_SIZE;
        println!();
        println!(
            "PAGE ALLOCATION TABLE\nMETA: {:p} -> {:p}\nPHYS: \
                  0x{:x} -> 0x{:x}",
            beg, end, alloc_beg, alloc_end
        );
        println!("~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~");
        let mut num = 0;
        while beg < end {
            if (*beg).is_taken() {
                let start = beg as usize;
                let memaddr = ALLOC_START + (start - HEAP_START) * PAGE_SIZE;
                print!("0x{:x} => ", memaddr);
                loop {
                    num += 1;
                    if (*beg).is_last() {
                        let end = beg as usize;
                        let memaddr = ALLOC_START + (end - HEAP_START) * PAGE_SIZE + PAGE_SIZE - 1;
                        print!("0x{:x}: {:>3} page(s)", memaddr, (end - start + 1));
                        println!(".");
                        break;
                    }
                    beg = beg.add(1);
                }
            }
            beg = beg.add(1);
        }
        println!("~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~");
        println!(
            "Allocated: {:>5} pages ({:>9} bytes).",
            num,
            num * PAGE_SIZE
        );
        println!(
            "Free     : {:>5} pages ({:>9} bytes).",
            num_pages - num,
            (num_pages - num) * PAGE_SIZE
        );
        println!();
    }
}
