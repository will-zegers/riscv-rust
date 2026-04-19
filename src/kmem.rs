use core::ptr::null_mut;

use crate::mmu::Table;
use crate::page::{PAGE_SIZE, zalloc};

const N_KMEM_ALLOC: usize = 64;

static mut KMEM_ALLOC: usize = 0;
static mut KMEM_HEAD: *mut AllocList = null_mut();
static mut KMEM_PAGE_TABLE: *mut Table = null_mut();

pub fn get_head() -> *mut u8 {
    unsafe { KMEM_HEAD as *mut u8 }
}

pub fn get_page_table() -> *mut Table {
    unsafe { KMEM_PAGE_TABLE as *mut Table }
}

pub fn get_num_allocations() -> usize {
    unsafe { KMEM_ALLOC }
}

#[repr(usize)]
enum AllocListFlags {
    Taken = 1 << 63,
}

impl AllocListFlags {
    pub fn value(self) -> usize {
        self as usize
    }
}

struct AllocList {
    pub flags_size: usize,
}

impl AllocList {
    fn is_free(&self) -> bool {
        self.flags_size & AllocListFlags::Taken.value() != 0
    }

    fn is_taken(&self) -> bool {
        !self.is_free()
    }

    pub fn set_free(&mut self) {
        self.flags_size &= !AllocListFlags::Taken.value()
    }

    pub fn set_size(&mut self, sz: usize) {
        let k = self.is_taken();
        self.flags_size = sz & !AllocListFlags::Taken.value();
        if k {
            self.flags_size |= AllocListFlags::Taken.value();
        }
    }
}

pub fn init() {
    unsafe {
        let k_alloc = zalloc(N_KMEM_ALLOC);
        assert!(!k_alloc.is_null());
        KMEM_ALLOC = N_KMEM_ALLOC;

        KMEM_HEAD = k_alloc as *mut AllocList;
        (*KMEM_HEAD).set_free();
        (*KMEM_HEAD).set_size(KMEM_ALLOC * PAGE_SIZE);

        KMEM_PAGE_TABLE = zalloc(1) as *mut Table;
    }
}
