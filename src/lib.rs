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
extern "C" fn kinit() {
    // Entry for hart with ID 0
    uart::Uart::new().init();
    page::init();
    kmem::init();

    let page_table_ptr = kmem::get_page_table();
    let page_table = unsafe { page_table_ptr.as_mut().unwrap() };
    let kheap_head = kmem::get_head() as usize;
    let total_pages = kmem::get_num_allocations();

    println!();
    println!();
    unsafe {
        println!("TEXT:   0x{:x} -> 0x{:x}", TEXT_START, TEXT_END);
        println!("RODATA: 0x{:x} -> 0x{:x}", RODATA_START, RODATA_END);
        println!("DATA:   0x{:x} -> 0x{:x}", DATA_START, DATA_END);
        println!("BSS:    0x{:x} -> 0x{:x}", BSS_START, BSS_END);
        println!(
            "STACK:  0x{:x} -> 0x{:x}",
            KERNEL_STACK_START, KERNEL_STACK_END
        );
        println!(
            "HEAP:   0x{:x} -> 0x{:x}",
            kheap_head,
            kheap_head + total_pages * 4096
        );
    }

    page_table.map_range(
        kheap_head,
        kheap_head + total_pages * 4096,
        mmu::EntryBits::ReadWrite.value(),
    );

    unsafe {
        page_table.map_range(
            HEAP_START,
            HEAP_START + (HEAP_SIZE / page::PAGE_SIZE),
            mmu::EntryBits::ReadWrite.value(),
        );
        page_table.map_range(TEXT_START, TEXT_END, mmu::EntryBits::ReadExecute.value());
        page_table.map_range(
            RODATA_START,
            RODATA_END,
            mmu::EntryBits::ReadExecute.value(),
        );
        page_table.map_range(DATA_START, DATA_END, mmu::EntryBits::ReadWrite.value());
        page_table.map_range(BSS_START, BSS_END, mmu::EntryBits::ReadWrite.value());
        page_table.map_range(
            KERNEL_STACK_START,
            KERNEL_STACK_END,
            mmu::EntryBits::ReadWrite.value(),
        );
    }

    //// MMIO ////
    // CLINT -> MSIP
    page_table.map_range(0x0200_0000, 0x0200_ffff, mmu::EntryBits::ReadWrite.value());
    // PLIC
    page_table.map_range(0x0c00_0000, 0x0c00_2000, mmu::EntryBits::ReadWrite.value());
    page_table.map_range(0x0c20_0000, 0x0c20_8000, mmu::EntryBits::ReadWrite.value());
    // UART
    page_table.map_range(0x1000_0000, 0x1000_0100, mmu::EntryBits::ReadWrite.value());

    let satp_value = cpu::build_satp(cpu::SatpMode::Sv39, 0, page_table_ptr as usize);
    unsafe {
        cpu::mscratch_write((&mut cpu::KERNEL_TRAP_FRAME[0] as *mut cpu::TrapFrame) as usize);
        cpu::sscratch_write(cpu::mscratch_read());
        cpu::KERNEL_TRAP_FRAME[0].satp = satp_value;

        cpu::KERNEL_TRAP_FRAME[0].trap_stack = page::zalloc(1).add(page::PAGE_SIZE);
        page_table.map_range(
            cpu::KERNEL_TRAP_FRAME[0].trap_stack.sub(page::PAGE_SIZE) as usize,
            cpu::KERNEL_TRAP_FRAME[0].trap_stack as usize,
            mmu::EntryBits::ReadWrite.value(),
        );
        page_table.map_range(
            cpu::mscratch_read(),
            cpu::mscratch_read() + core::mem::size_of::<cpu::TrapFrame>(),
            mmu::EntryBits::ReadWrite.value(),
        );
        let p = cpu::KERNEL_TRAP_FRAME[0].trap_stack as usize - 1;
        let m = page_table.virt_to_phys(p).unwrap_or(0);
        println!();
        println!("Walk 0x{:x} = 0x{:x}", p, m);
    }
    page::print_page_allocations();

    println!("Setting 0x{:x}", satp_value);
    println!("Scratch reg = 0x{:x}", cpu::mscratch_read());
    cpu::satp_write(satp_value);
    cpu::satp_fence_asid(0);
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

#[unsafe(no_mangle)]
extern "C" fn kmain() {
    let _uart_dev = uart::Uart::new().init();

    {
        let box1 = Box::<u32>::new(100);
        println!("Boxed 1 value = {}", *box1);

        let long_vec = alloc::vec![0; 256];
        let sparkle_heart = alloc::vec![240, 159, 146, 150];
        let sparkle_heart = String::from_utf8(sparkle_heart).unwrap();
        println!("Long vec length = {}", long_vec.len());
        println!("String = {}", sparkle_heart);
        println!("\n\nAllocations of a box, vector, and string");
        kmem::print_table();
    }
    println!("\nEverything should now be free:");
    kmem::print_table();

    println!("\n\nTesting the timer...");
    unsafe {
        let mtimecmp = 0x0200_4000 as *mut u64;
        let mtime = 0x0200_bff8 as *const u64;
        mtimecmp.write_volatile(mtime.read_volatile() + 10_000_000);
    }

    println!("\n\nTesting access faults...");
    unsafe {
        let v = 0x0 as *mut u64;
        v.write_volatile(1);

        let _ = v.read_volatile();
    }

    println!("\n\nEnabling UART interrupts in the PLIC");
    plic::set_threshold(0);
    plic::enable(uart::INT_ID);
    plic::set_priority(uart::INT_ID, 1)
}

pub mod cpu;
pub mod kmem;
pub mod mmu;
pub mod page;
pub mod plic;
pub mod trap;
pub mod uart;
