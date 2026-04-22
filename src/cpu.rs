use core::ptr::null_mut;

#[repr(usize)]
pub enum SatpMode {
    Off = 0,
    Sv39 = 8,
    Sv48 = 9,
}

#[derive(Copy, Clone)]
pub struct TrapFrame {
    pub regs: [usize; 32],  // GP registers
    pub fregs: [usize; 32], // Floating point registers
    pub satp: usize,        // MMU
    pub trap_stack: *mut u8,
    pub hartid: usize, // hardware ID thread
}

impl TrapFrame {
    pub const fn zero() -> Self {
        TrapFrame {
            regs: [0; 32],
            fregs: [0; 32],
            satp: 0,
            trap_stack: null_mut(),
            hartid: 0,
        }
    }
}

pub static mut KERNEL_TRAP_FRAME: [TrapFrame; 8] = [TrapFrame::zero(); 8];

pub fn build_satp(mode: SatpMode, asid: usize, addr: usize) -> usize {
    (mode as usize) << 60 | (asid & 0xfff) << 44 | (addr >> 12) & 0xff_ffff_ffff
}

pub fn mscratch_read() -> usize {
    let val: usize;
    unsafe {
        core::arch::asm!("csrr {}, mscratch", out(reg) val);
    }
    val
}

pub fn mscratch_write(val: usize) {
    unsafe {
        core::arch::asm!("csrw mscratch, {}", in(reg) val);
    }
}

pub fn satp_fence_asid(asid: usize) {
    unsafe { core::arch::asm!("sfence.vma zero, {}", in(reg) asid) }
}

pub fn satp_write(val: usize) {
    unsafe {
        core::arch::asm!("csrw satp, {}", in(reg) val);
    }
}

pub fn sscratch_write(val: usize) {
    unsafe {
        core::arch::asm!("csrw sscratch, {}", in(reg) val);
    }
}
