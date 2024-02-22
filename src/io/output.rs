extern crate alloc;

use alloc::vec::Vec;
use alloc::vec;
use core::fmt::Write;

use crate::io::vga;

pub trait IOOutput {
    fn write(self: &mut Self, data: &Vec<u8>);
}

pub static mut OUTPUT_VGA: vga::VGA = vga::VGA::new();

pub fn init() {
    unsafe {
        OUTPUT_VGA.init();
    }
}

pub fn puts(line: &str) {
    /* TODO: Support selecting output */
    unsafe {
        OUTPUT_VGA.write(&line.as_bytes().to_vec());
    }
}

pub fn putchar(ch: u8) {
    /* TODO: Support selecting output */
    unsafe {
        OUTPUT_VGA.write(&vec![ch]);
    }
}

pub fn hexdump(data: &Vec<u8>) {
    let mut cnt = 0;
    for byte in data {
        if (cnt % 16) == 0 {
            if cnt != 0 {
                putchar(b'\n');
            }
            unsafe { _ = write!(OUTPUT_VGA, "{:4x}: ", cnt); }
        }

        unsafe { _ = write!(OUTPUT_VGA, "{:2x} ", byte); }

        cnt += 1;
    }

    putchar(b'\n');
}

#[macro_export]
macro_rules! println {
    /* TODO: Make this safer and more portable */
    /* NOTE: Using core::fmt adds a lot of overhead, consider re-implementation */
    ($fmt:expr) => (
        unsafe { _ = write!(output::OUTPUT_VGA, concat!($fmt, "\n")) }
    );
    ($fmt:expr, $($arg:tt)*) => (
        unsafe { _ = write!(output::OUTPUT_VGA, concat!($fmt, "\n"), $($arg)*) }
    );
}

