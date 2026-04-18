#![no_std]

mod uart;

use core::fmt::Write;

#[macro_export]
macro_rules! print {
    ($($args:tt)+) => ({
        let _ = write!(uart::Uart::new(0x1000_0000), $($args)+);
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
        println!(
            "line {}, file {}: {}",
            p.line(),
            p.file(),
            info.message(),
        );
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

#[unsafe(no_mangle)]
extern "C" fn kmain() {
    let mut my_uart = uart::Uart::new(0x1000_0000);
    my_uart.init();

    println!("This is my operating system!");
    println!("I'm so awesome. If you start typing something, I'll show you what you typed!");

    loop {
        if let Some(c) = my_uart.get() {
            match c {
                8 => {
                    print!("{}{}{}", 8 as char, ' ', 8 as char);
                },
                10 | 13 => {
                    println!();
                }
                0x1b => {
                    if let Some(next_byte) = my_uart.get() {
                        if next_byte == 91 {
                            if let Some(b) = my_uart.get() {
                                match b as char {
                                    'A' => {
                                        println!("That's the up arrow!");
                                    },
                                    'B' => {
                                        println!("That's the down arrow!");
                                    },
                                    'C' => {
                                        println!("That's the right arrow!");
                                    },
                                    'D' => {
                                        println!("That's the left arrow!");
                                    },
                                    _ => {
                                        println!("That's something else...");
                                    },
                                }
                            }
                        }
                    }
                }
                _ => {
                    print!("{}", c as char);
                }
            }
        }
    }
}
