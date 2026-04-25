use alloc::collections::vec_deque::VecDeque;

use crate::{
    cpu,
    cpu::{SatpMode, TrapFrame},
    mmu::{EntryBits, Table},
    page::{PAGE_SIZE, alloc, dealloc, zalloc},
};

unsafe extern "C" {
    fn make_ecall(a: usize) -> usize;
}

static mut NEXT_PID: u16 = 1;
pub static mut PROCESS_LIST: Option<VecDeque<Process>> = None;

const PROCESS_STARTING_ADDR: usize = 0x2000_0000;
const STACK_ADDR: usize = 0x7_0000_0000;
const STACK_PAGES: usize = 2;

pub enum ProcessState {
    Dead,
    Running,
    Sleeping,
    Waiting,
}

#[repr(C)]
pub struct Process {
    frame: *mut TrapFrame,
    stack: *mut u8,
    mepc: usize,
    pid: u16,
    root: *mut Table,
    state: ProcessState,
    // data: ProcessData,
}

impl Process {
    pub fn get_frame_address(&self) -> usize {
        self.frame as usize
    }

    pub fn get_program_counter(&self) -> usize {
        self.mepc as usize
    }

    pub fn get_state(&self) ->&ProcessState {
        &self.state
    }

    pub fn get_pid(&self) -> u16 {
        self.pid
    }

    pub fn get_table_address(&self) -> usize {
        self.root as usize
    }

    pub fn new_default(func: fn()) -> Self {
        let func_addr = func as usize;
        let mut proc = Process {
            frame: zalloc(1) as *mut TrapFrame,
            stack: alloc(STACK_PAGES),
            mepc: func_addr,
            pid: unsafe { NEXT_PID },
            root: zalloc(1) as *mut Table,
            state: ProcessState::Running,
        };

        let ptable;
        unsafe {
            NEXT_PID += 1;
            (*proc.frame).regs[2] = STACK_ADDR + PAGE_SIZE * STACK_PAGES;
            ptable = &mut *proc.root;
        }

        let stack_addr = proc.stack as usize;
        for i in 0..STACK_PAGES {
            let addr = i * PAGE_SIZE;
            ptable.map(
                STACK_ADDR + addr,
                stack_addr + addr,
                EntryBits::UserReadWrite.value(),
            )
        }

        for i in 0..=100 {
            let modifier = i * PAGE_SIZE;
            ptable.map(
                func_addr + modifier,
                func_addr + modifier,
                EntryBits::UserReadWriteExecute.value()
            )
        }

        ptable.map(0x8000_0000, 0x8000_0000, EntryBits::UserReadExecute.value());

        proc
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        dealloc(self.stack);
        unsafe {
            (&mut *self.root).unmap();
        }
        dealloc(self.root as *mut u8);
    }
}

pub fn init() -> usize {
    unsafe {
        #![allow(static_mut_refs)]
        PROCESS_LIST = Some(VecDeque::with_capacity(5));
        add_process_default(init_process);

        let proc_list = PROCESS_LIST.take().unwrap();
        let proc = proc_list.front().unwrap().frame;
        let frame = proc as *const TrapFrame as usize;
        cpu::mscratch_write(frame);
        cpu::satp_write(cpu::build_satp(
            SatpMode::Sv39,
            1,
            proc_list.front().unwrap().root as usize,
        ));
        cpu::satp_fence_asid(1);
        PROCESS_LIST.replace(proc_list);
    }
    PROCESS_STARTING_ADDR
}

fn add_process_default(func: fn()) {
    #![allow(static_mut_refs)]
    unsafe {
        if let Some(mut proc_list) = PROCESS_LIST.take() {
            let proc = Process::new_default(func);
            proc_list.push_back(proc);
            PROCESS_LIST.replace(proc_list);
        }
    }
}

fn init_process() {
    let mut i: usize = 0;
    loop {
        i += 1;
        if i > 20_000_000 {
            unsafe {
                make_ecall(1);
            }
            i = 0;
        }
    }
}
