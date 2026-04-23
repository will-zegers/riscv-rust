use crate::cpu::TrapFrame;

pub fn do_ecall(mepc: usize, frame: *mut TrapFrame) -> usize {
    let ecall_number;
    unsafe {
        ecall_number = (*frame).regs[17]; // ecall number in a7
    }
    match ecall_number {
        0 => println!("Exit"),
        1 => println!("Environment call test"),
        _ => println!("Unknown ecall number: {}", ecall_number),
    }
    mepc + 4
}
