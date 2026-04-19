const TABLE_SIZE: usize = 512;

use crate::page::{PAGE_ORDER, PAGE_SIZE, align_val, dealloc, zalloc};

#[repr(i64)]
#[derive(Copy, Clone)]
pub enum EntryBits {
    None = 0,
    Valid = 1 << 0,
    Read = 1 << 1,
    Write = 1 << 2,
    Exectue = 1 << 3,
    User = 1 << 4,
    Global = 1 << 5,
    Access = 1 << 6,
    Dirty = 1 << 7,

    ReadWrite = (1 << 1) | (1 << 2),
    ReadExecute = (1 << 1) | (1 << 3),
    ReadWriteExecute = (1 << 1) | (1 << 2) | (1 << 3),
}

impl EntryBits {
    pub fn value(self) -> i64 {
        self as i64
    }
}

pub struct Entry {
    pub entry: i64,
}

impl Entry {
    pub fn is_valid(&self) -> bool {
        self.entry & EntryBits::Valid.value() != 0
    }

    pub fn is_invalid(&self) -> bool {
        !self.is_valid()
    }

    pub fn is_leaf(&self) -> bool {
        self.entry & EntryBits::ReadWriteExecute.value() != 0
    }

    pub fn is_branch(&self) -> bool {
        !self.is_leaf()
    }

    pub fn get_entry(&self) -> i64 {
        self.entry
    }

    pub fn set_entry(&mut self, val: i64) {
        self.entry = val;
    }
}

pub struct Table {
    pub entries: [Entry; TABLE_SIZE],
}

fn vpn_from_address(addr: usize) -> [usize; 3] {
    [
        (addr >> 12) & 0x1ff, // VPN[0] = vaddr[20:12]
        (addr >> 21) & 0x1ff, // VPN[1] = vaddr[29:21]
        (addr >> 30) & 0x1ff, // VPN[2] = vaddr[38:30]
    ]
}

fn ppn_from_address(addr: usize) -> [usize; 3] {
    [
        (addr >> 12) & 0x1ff,      // PPN[0] = paddr[20:12]
        (addr >> 21) & 0x1ff,      // PPN[1] = paddr[29:21]
        (addr >> 30) & 0x3ff_ffff, // PPN[2] = paddr[55:30]
    ]
}

impl Table {
    pub fn len() -> usize {
        TABLE_SIZE
    }

    pub fn map(&mut self, vaddr: usize, paddr: usize, bits: i64, level: usize) {
        assert!(bits & EntryBits::ReadWriteExecute.value() != 0);

        let vpn = vpn_from_address(vaddr);
        let ppn = ppn_from_address(paddr);

        let mut v = &mut self.entries[vpn[2]];

        for i in (level..2).rev() {
            if !v.is_valid() {
                let page = zalloc(1);

                v.entry = (page as i64 >> 2) | EntryBits::Valid.value();
            }
            let entry = ((v.entry & !0x3ff) << 2) as *mut Entry;
            v = unsafe { entry.add(vpn[i]).as_mut().unwrap() };
        }

        v.entry = (ppn[2] << 28) as i64
            | (ppn[1] << 19) as i64
            | (ppn[0] << 10) as i64
            | bits
            | EntryBits::Valid.value();
    }

    pub fn unmap(&mut self) {
        for i in 0..Table::len() {
            let ref entry_lv2 = self.entries[i];
            if entry_lv2.is_valid() && entry_lv2.is_branch() {
                // Valid entry, so continue down to level 1 and free
                let memaddr_lv1 = (entry_lv2.entry & !0x3ff) << 2;
                let table_lv1 = unsafe { (memaddr_lv1 as *mut Table).as_mut().unwrap() };
                for lv1 in 0..Table::len() {
                    let ref entry_lv1 = table_lv1.entries[lv1];
                    if entry_lv1.is_valid() && entry_lv1.is_branch() {
                        let memaddr_lv0 = (entry_lv1.entry & !0x3ff) << 2;
                        dealloc(memaddr_lv0 as *mut u8);
                    }
                }
                dealloc(memaddr_lv1 as *mut u8);
            }
        }
    }

    pub fn virt_to_phys(&self, vaddr: usize, level: usize) -> Option<usize> {
        let vpn = vpn_from_address(vaddr);
        let mut v = &self.entries[vpn[level]];

        for i in (0..=2).rev() {
            if v.is_invalid() {
                break; // page fault
            }
            if v.is_leaf() {
                let offset_mask = (1 << (12 + i * 9)) - 1;
                let vaddr_pgoff = vaddr & offset_mask;
                let addr = ((v.entry << 2) as usize) & !offset_mask;
                return Some(addr | vaddr_pgoff);
            }

            let entry = ((v.entry & !0x3ff) << 2) as *const Entry;
            v = unsafe { entry.add(vpn[i - 1]).as_ref().unwrap() };
        }
        None
    }

    pub fn map_range(&mut self, start: usize, end: usize, bits: i64) {
        let mut memaddr = start & !(PAGE_SIZE - 1);
        let num_kb_pages = (align_val(end, PAGE_ORDER) - memaddr) / PAGE_SIZE;

        for _ in 0..num_kb_pages {
            self.map(memaddr, memaddr, bits, 0);
            memaddr += PAGE_SIZE;
        }
    }
}
