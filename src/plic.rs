const PRIORITY_REG: usize = 0x0c00_0000;
const PENDING_REG: usize = 0x0c00_1000;
const ENABLE_REG: usize = 0x0c00_2000;
const THRESHOLD_REG: usize = 0x0c20_0000;
const CLAIM_REG: usize = 0x0c20_0004;
const COMPLETE_REG: usize = 0x0c20_0004;

pub fn enable(id: u32) {
    let enable_reg = ENABLE_REG as *mut u32;
    unsafe {
        (enable_reg as *mut u32).write_volatile(enable_reg.read_volatile() | (1 << id));
    }
}

pub fn has_pending(id: u32) -> bool {
    let pending;
    unsafe {
        pending = (PENDING_REG as *const u32).read_volatile();
    }
    (id << 1) & pending != 0
}

pub fn set_priority(id: u32, priority: u8) {
    unsafe {
        (PRIORITY_REG as *mut u32)
            .add(id as usize)
            .write_volatile(priority as u32 & 0x7);
    }
}

pub fn set_threshold(threshold: u8) {
    unsafe {
        (THRESHOLD_REG as *mut u32).write_volatile(threshold as u32 & 0x7);
    }
}

pub fn next() -> Option<u32> {
    let claim_no;
    unsafe {
        claim_no = (CLAIM_REG as *const u32).read_volatile();
    }

    if claim_no == 0 {
        return None;
    }

    Some(claim_no)
}

pub fn complete(id: u32) {
    unsafe {
        (COMPLETE_REG as *mut u32).write_volatile(id);
    }
}
