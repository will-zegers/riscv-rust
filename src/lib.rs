#![no_std]

#[macro_export]
macro_rules! print {
    ($($args:tt)+) => ({
        use core::fmt::Write;
        let _ = write!(crate::uart::Uart::new(0x1000_0000), $($args)+);
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
extern "C" fn kmain() {
    page::init();

    kmem::init();
    let page_table_ptr = kmem::get_page_table();
    let page_table = unsafe { page_table_ptr.as_mut().unwrap() };
    let kheap_head = kmem::get_head() as usize;
    let total_pages = kmem::get_num_allocations();

    mmu::init(page_table_ptr as usize);

    println!();
    println!();
    unsafe {
        println!("TEXT:   0x{:x} -> 0x{:x}", TEXT_START, TEXT_END);
        println!("RODATA: 0x{:x} -> 0x{:x}", RODATA_START, RODATA_END);
        println!("DATA:   0x{:x} -> 0x{:x}", DATA_START, DATA_END);
        println!("BSS:    0x{:x} -> 0x{:x}", BSS_START, BSS_END);
        println!("STACK:  0x{:x} -> 0x{:x}", KERNEL_STACK_START, KERNEL_STACK_END);
        println!("HEAP:   0x{:x} -> 0x{:x}", kheap_head, kheap_head + total_pages * 4096);
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
        page_table.map_range(
            TEXT_START,
            TEXT_END,
            mmu::EntryBits::ReadExecute.value()
        );
        page_table.map_range(
            RODATA_START,
            RODATA_END,
            mmu::EntryBits::ReadExecute.value(),
        );
        page_table.map_range(
            DATA_START,
            DATA_END,
            mmu::EntryBits::ReadWrite.value()
        );
        page_table.map_range(
            BSS_START,
            BSS_END,
            mmu::EntryBits::ReadWrite.value()
        );
        page_table.map_range(
            KERNEL_STACK_START,
            KERNEL_STACK_END,
            mmu::EntryBits::ReadWrite.value(),
        );
    }

    //// MMIO ////
    // UART
    page_table.map(
        0x1000_0000,
        0x1000_0000,
        mmu::EntryBits::ReadWrite.value(),
    );
    // CLINT -> MSIP
    page_table.map(
        0x0200_0000,
        0x0200_0000,
        mmu::EntryBits::ReadWrite.value(),
    );
    // MTIMECMP
    page_table.map(
        0x0200_b000,
        0x0200_b000,
        mmu::EntryBits::ReadWrite.value(),
    );
    // MTIME
    page_table.map(
        0x0200_c000,
        0x0200_c000,
        mmu::EntryBits::ReadWrite.value(),
    );
    // PLIC
    page_table.map_range(
        0x0c00_0000,
        0x0c00_2000,
        mmu::EntryBits::ReadWrite.value(),
    );
    page_table.map_range(
        0x0c20_0000,
        0x0c20_8000,
        mmu::EntryBits::ReadWrite.value(),
    );

    let p = 0x8005_7000 as usize;
    let m = page_table.virt_to_phys(p).unwrap_or(0);

    page::print_page_allocations();
    println!("Walk 0x{:x} = 0x{:x}", p, m);
}

pub mod kmem;
pub mod mmu;
pub mod page;
pub mod uart;
