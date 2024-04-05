use core::arch::asm;

#[inline(always)]
pub fn outb(port: u16, value: u8) {
    unsafe {
        asm!("out dx, al",
             in("dx") port,
             in("al") value
        );
    }
}

#[inline(always)]
pub fn inb(port: u16) -> u8 {
    let mut value: u8;
    unsafe {
        asm!("in al, dx",
             in("dx") port,
             out("al") value
        );
    }
    value
}

