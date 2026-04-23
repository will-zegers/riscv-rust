use alloc::collections::vec_deque::VecDeque;

use crate::{
    cpu,
    cpu::{SatpMode, TrapFrame},
    mmu::{EntryBits, Table},
    page::{PAGE_SIZE, alloc, dealloc, zalloc},
};

static mut NEXT_PID: u16 = 1;
static mut PROCESS_LIST: Option<VecDeque<Process>> = None;

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
    frame: TrapFrame,
    stack: *mut u8,
    pc: usize,
    pid: u16,
    root: *mut Table,
    state: ProcessState,
    // data: ProcessData,
}

impl Process {
    pub fn new_default(func: fn()) -> Self {
        let mut proc = Process {
            frame: TrapFrame::zero(),
            stack: alloc(STACK_PAGES),
            pc: PROCESS_STARTING_ADDR,
            pid: unsafe { NEXT_PID },
            root: zalloc(1) as *mut Table,
            state: ProcessState::Waiting,
        };
        unsafe {
            NEXT_PID += 1;
        }

        proc.frame.regs[2] = STACK_ADDR + PAGE_SIZE * STACK_PAGES;
        let ptable;
        unsafe {
            ptable = &mut *proc.root;
        }

        let func_addr = func as usize;
        let stack_addr = proc.stack as usize;
        for i in 0..STACK_PAGES {
            let addr = i * PAGE_SIZE;
            ptable.map(
                STACK_ADDR + addr,
                stack_addr + addr,
                EntryBits::UserReadWrite.value(),
            )
        }

        // Map two pages for process instructions
        ptable.map(
            PROCESS_STARTING_ADDR,
            func_addr,
            EntryBits::UserReadExecute.value(),
        );
        ptable.map(
            PROCESS_STARTING_ADDR + 0x1001,
            func_addr + 0x1001,
            EntryBits::UserReadExecute.value(),
        );

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
        let frame = &proc as *const TrapFrame as usize;
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
    loop {}
}
