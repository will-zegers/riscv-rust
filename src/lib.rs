#![no_std]
#![feature(alloc_error_handler)]

extern crate alloc;
use alloc::{boxed::Box, string::String};

#[macro_export]
macro_rules! print {
    ($($args:tt)+) => ({
        use core::fmt::Write;
        let _ = write!(crate::uart::Uart::new(), $($args)+);
    })
}

#[macro_export]
macro_rules! println
{
    () => ({
        print!("\r\n")
    });
    ($fmt:expr) => ({
        print!(concat!($fmt, "\r\n"))
    });
    ($fmt:expr, $($args:tt)+) => ({
        print!(concat!($fmt, "\r\n"), $($args)+)
    });
}

#[unsafe(no_mangle)]
extern "C" fn eh_personality() {}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    print!("Aborting: ");
    if let Some(p) = info.location() {
        println!("line {}, file {}: {}", p.line(), p.file(), info.message(),);
    } else {
        println!("no information available.");
    }
    abort();
}

#[unsafe(no_mangle)]
extern "C" fn abort() -> ! {
    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}
unsafe extern "C" {
    static TEXT_START: usize;
    static TEXT_END: usize;
    static DATA_START: usize;
    static DATA_END: usize;
    static RODATA_START: usize;
    static RODATA_END: usize;
    static BSS_START: usize;
    static BSS_END: usize;
    static KERNEL_STACK_START: usize;
    static KERNEL_STACK_END: usize;
    static HEAP_START: usize;
    static HEAP_SIZE: usize;
}

#[unsafe(no_mangle)]
extern "C" fn kinit() -> usize {
    // Entry for hart with ID 0
    uart::Uart::new().init();
    page::init();
    kmem::init();

    let init_proc = process::init();
    println!("Init process created at address 0x{:08x}", init_proc);

    plic::set_threshold(0);
    plic::enable(uart::INT_ID);
    plic::set_priority(uart::INT_ID, 1);
    println!("UART interrupts have been enabled");

    unsafe {
        let mtimecmp = 0x0200_4000 as *mut u64;
        let mtime = 0x0200_bff8 as *const u64;
        mtimecmp.write_volatile(mtime.read_volatile() + 10_000_000);
    }
    println!("Context switch timer (1 Hz) initialized");

    init_proc
}

#[unsafe(no_mangle)]
extern "C" fn kinit_hart(hartid: usize) {
    //  Entry for all harts with ID non-zero
    unsafe {
        cpu::mscratch_write((&mut cpu::KERNEL_TRAP_FRAME[hartid] as *mut cpu::TrapFrame) as usize);
        cpu::sscratch_write(cpu::mscratch_read());
        cpu::KERNEL_TRAP_FRAME[hartid].hartid = hartid;
    }
}

pub mod cpu;
pub mod kmem;
pub mod mmu;
pub mod page;
pub mod plic;
pub mod process;
pub mod trap;
pub mod uart;
