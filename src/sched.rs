use crate::process::{PROCESS_LIST, ProcessState};

pub fn schedule() -> (usize, usize, usize) {
    unsafe {
        #[allow(static_mut_refs)]
        if let Some(mut plist) = PROCESS_LIST.take() {
            plist.rotate_left(1);
            let mut frame_addr: usize = 0;
            let mut mepc: usize = 0;
            let mut satp: usize = 0;
            let mut pid: usize = 0;
            if let Some(proc) = plist.front() {
                match proc.get_state() {
                    ProcessState::Running => {
                        frame_addr = proc.get_frame_address();
                        mepc = proc.get_program_counter();
                        satp = proc.get_table_address() >> 12;
                        pid = proc.get_pid() as usize;
                    },
                    _ => {}

                }
                println!("Scheduling {} {:x} {:x}", pid, frame_addr, mepc);
                PROCESS_LIST.replace(plist);
                if frame_addr != 0 {
                    return (frame_addr, mepc, (8 << 60) | (pid << 44) | satp);
                } else {
                    return (frame_addr, mepc, 0);
                }
            }
        }
    }
    (0, 0, 0)
}
