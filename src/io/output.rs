extern crate alloc;

use alloc::vec::Vec;
use alloc::vec;

use crate::io::vga;

pub trait IOOutput {
    fn write(self: &mut Self, data: &Vec<u8>);
}

static mut OUTPUT_VGA: vga::VGA = vga::VGA::new();

pub fn init() {
    unsafe {
        OUTPUT_VGA.init();
    }
}

fn _puts(out: &mut dyn IOOutput, line: &str) {
    out.write(&line.as_bytes().to_vec());
}

pub fn puts(line: &str) {
    /* TODO: Support selecting output (and fix the warning) */
    unsafe {
        _puts(&mut OUTPUT_VGA, line);
    }
}

fn _putchar(out: &mut dyn IOOutput, ch: u8) {
    out.write(&vec![ch]);
}

pub fn putchar(ch: u8) {
    /* TODO: Support selecting output (and fix the warning) */
    unsafe {
        _putchar(&mut OUTPUT_VGA, ch);
    }
}

