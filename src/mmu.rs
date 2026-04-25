use crate::page::{PAGE_ORDER, PAGE_SIZE, align_val, dealloc, zalloc};

const TABLE_SIZE: usize = 512; // Number of virtual address per table
const N_LEVELS: usize = 2; // Number of virtual redirects until physical address

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

    UserReadWrite = (1 << 1) | (1 << 2) | (1 << 4),
    UserReadExecute = (1 << 1) | (1 << 3) | (1 << 4),
    UserReadWriteExecute = (1 << 1) | (1 << 2) | (1 << 3) | (1 << 4),
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
    // Virtual page number (VPN) is a 27-bit number given in bits 12 to 38 of
    // the virtual address, followed by the 12-bit offset
    [
        (addr >> 12) & 0x1ff, // VPN[0] = vaddr[20:12]
        (addr >> 21) & 0x1ff, // VPN[1] = vaddr[29:21]
        (addr >> 30) & 0x1ff, // VPN[2] = vaddr[38:30]
    ]
}

fn ppn_from_address(addr: usize) -> [usize; 3] {
    // Phyiscal page number (PPN) is a 44-bit number given in bits 12 to 55
    // of a page table entry
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

    pub fn map(&mut self, vaddr: usize, paddr: usize, bits: i64) {
        assert!(bits & EntryBits::ReadWriteExecute.value() != 0);

        let vpn = vpn_from_address(vaddr);

        let entry = &mut self.entries[vpn[N_LEVELS]];
        Table::map_rec(entry, vaddr, paddr, bits, N_LEVELS);
    }

    fn map_rec(current: &mut Entry, vaddr: usize, paddr: usize, bits: i64, level: usize) {
        if level == 0 {
            let ppn = ppn_from_address(paddr);
            current.entry = (ppn[2] << 28) as i64
                | (ppn[1] << 19) as i64
                | (ppn[0] << 10) as i64
                | bits
                | EntryBits::Valid.value();
            return;
        }

        if !current.is_valid() {
            let page = zalloc(1);

            current.entry = (page as i64 >> 2) | EntryBits::Valid.value();
        }
        let vpn = vpn_from_address(vaddr);
        let entry = ((current.entry & !0x3ff) << 2) as *mut Entry;
        let v = unsafe { entry.add(vpn[level - 1]).as_mut().unwrap() };

        Table::map_rec(v, vaddr, paddr, bits, level - 1);
    }

    pub fn map_range(&mut self, start: usize, end: usize, bits: i64) {
        let mut memaddr = start & !(PAGE_SIZE - 1);
        let num_kb_pages = (align_val(end, PAGE_ORDER) - memaddr) / PAGE_SIZE;

        for _ in 0..num_kb_pages {
            self.map(memaddr, memaddr, bits);
            memaddr += PAGE_SIZE;
        }
    }

    pub fn unmap(&mut self) {
        self.unmap_rec();
    }

    fn unmap_rec(&mut self) {
        for i in 0..Table::len() {
            let ref entry = self.entries[i];
            if entry.is_valid() && entry.is_branch() {
                let memaddr = (entry.entry & !0x3ff) << 2;
                let table = unsafe { (memaddr as *mut Table).as_mut().unwrap() };
                table.unmap_rec();
                dealloc(memaddr as *mut u8);
            }
        }
    }

    pub fn virt_to_phys(&self, vaddr: usize) -> Option<usize> {
        let vpn = vpn_from_address(vaddr);
        let entry = &self.entries[vpn[N_LEVELS]];
        Table::virt_to_phys_rec(entry, vaddr, N_LEVELS)
    }

    fn virt_to_phys_rec(current: &Entry, vaddr: usize, level: usize) -> Option<usize> {
        if current.is_leaf() {
            let offset_mask = (1 << (12 + level * 9)) - 1;
            let vaddr_pgoff = vaddr & offset_mask;
            let addr = ((current.entry << 2) as usize) & !offset_mask;
            return Some(addr | vaddr_pgoff);
        }

        if current.is_invalid() || level == 0 {
            return None; // page fault
        }

        let vpn = vpn_from_address(vaddr);
        let entry = ((current.entry & !0x3ff) << 2) as *const Entry;
        let next = unsafe { entry.add(vpn[level - 1]).as_ref().unwrap() };

        Table::virt_to_phys_rec(next, vaddr, level - 1)
    }
}
