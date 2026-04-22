use crate::{cpu::TrapFrame, plic, uart};

#[unsafe(no_mangle)]
extern "C" fn m_trap(
    epc: usize,
    tval: usize,
    cause: usize,
    hart: usize,
    _status: usize,
    _frame: &mut TrapFrame,
) -> usize {
    let cause_code = cause & 0xfff;
    let mut return_pc = epc;

    let is_async = (cause >> 63) & 1 == 1;
    if is_async {
        match cause_code {
            3 => println!("Machine software interrupt CPU #{}", hart),
            7 => {
                let mtimecmp = 0x200_4000 as *mut u64;
                let mtime = 0x0200_bff8 as *const u64;
                unsafe {
                    // Reset the time to fire every 1 sec @ 10,000,000 Hz
                    mtimecmp.write_volatile(mtime.read_volatile() + 10_000_000);
                }
            }
            11 => {
                if let Some(interrupt) = plic::next() {
                    match interrupt {
                        uart::INT_ID => {
                            let mut uart_dev = uart::Uart::new();
                            if let Some(c) = uart_dev.get() {
                                match c {
                                    8 => println!("{} {}", c as char, c as char),
                                    10 | 13 => println!(),
                                    _ => print!("{}", c as char),
                                }
                            }
                        }
                        _ => println!(
                            "Non-UART external interrupt CPU #{}: ID {}",
                            hart, interrupt
                        ),
                    }
                    plic::complete(interrupt);
                }
            }
            _ => panic!(
                "Unhandled asynchronous trap CPU #{} -> ID {}",
                hart, cause_code
            ),
        }
    } else {
        // Synchronous interrupt
        match cause_code {
            2 => println!(
                "Illegal instruction CPU #{} -> 0x{:08x}: 0x{:08x}",
                hart, epc, tval
            ),
            8 => {
                println!(
                    "Environment call from User mode! CPU #{} -> 0x{:08x}",
                    hart, epc
                );
                return_pc += 4;
            }
            5 => {
                println!("Load access fault! CPU #{} -> 0x{:08x}", hart, epc);
                return_pc += 4;
            }
            7 => {
                println!("Store / AMO access fault! CPU #{} -> 0x{:08x}", hart, epc);
                return_pc += 4;
            }
            9 => {
                println!(
                    "Environment call from Supervisor mode! CPU #{} -> 0x{:08x}",
                    hart, epc
                );
                return_pc += 4;
            }
            11 => {
                panic!(
                    "Environment call from Machine mode! CPU #{} -> 0x{:08x}",
                    hart, epc
                );
            }
            12 => {
                println!(
                    "Instruction page fault! CPU #{} -> 0x{:08x}: 0x{:08x}",
                    hart, epc, tval
                );
                return_pc += 4;
            }
            13 => {
                println!(
                    "Load page fault! CPU #{} -> 0x{:08x}: 0x{:08x}",
                    hart, epc, tval
                );
                return_pc += 4;
            }
            15 => {
                println!(
                    "Store page fault! CPU #{} -> 0x{:08x}: 0x{:08x}",
                    hart, epc, tval
                );
                return_pc += 4;
            }
            _ => panic!(
                "Unhandled synchronous trap CPU #{} -> ID {}",
                hart, cause_code
            ),
        }
    }
    return return_pc;
}
