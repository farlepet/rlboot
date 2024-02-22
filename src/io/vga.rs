extern crate alloc;

use core::ptr;

pub struct VGA {
    vidmem: *mut u16,

    color_fg: u8,
    color_bg: u8,

    res_x: u8,
    res_y: u8,

    pos_x: u8,
    pos_y: u8
}

macro_rules! vga_char {
    ($ch:expr, $fg:expr, $bg:expr) => {
        $ch as u16 | (($fg as u16) << 8) | (($bg as u16) << 12)
    };
}

impl VGA {
    pub const fn new() -> Self {
        Self {
            vidmem: 0xB8000 as *mut u16,
            color_fg: 7,
            color_bg: 0,
            res_x: 80,
            /* TODO: Account for statusbar */
            res_y: 25,
            pos_x: 0,
            pos_y: 0
        }
    }

    pub fn init(&mut self) {
        self.color_fg = 7;
        self.color_bg = 0;
        self.pos_x    = 0;
        self.pos_y    = 0;

        self.clear();
    }

    fn clear(&self) {
        unsafe {
            self.vidmem.write_bytes(0, (self.res_x as usize * 2) * self.res_y as usize);
        }
    }

    fn place_char(&self, x: u8, y: u8, ch: u8) {
        unsafe {
            *self.vidmem.add((y as usize * self.res_x as usize) + x as usize) = vga_char!(ch, self.color_fg, self.color_bg);
        }
    }

    fn put_char(&mut self, ch: u8) {
        match ch {
            b'\n' => {
                self.pos_x = 0;
                self.pos_y += 1;
            },
            _ => {
                self.place_char(self.pos_x, self.pos_y, ch);
                self.pos_x += 1;
            }
        }

        if self.pos_x >= self.res_x {
            self.pos_x = 0;
            self.pos_y += 1;
        }
        if self.pos_y >= self.res_y {
            self.pos_y = self.res_y - 1;
            self.scroll();
        }
    }

    fn scroll(&mut self) {
        unsafe {
            ptr::copy(self.vidmem.offset(self.res_x as isize), self.vidmem, (self.res_x as usize) * (self.res_y as usize - 1));
            self.vidmem.offset(self.res_x as isize * (self.res_y as isize - 1)).write_bytes(0, self.res_x as usize * 2)
        }
    }
}

impl crate::io::output::IOOutput for VGA {
    fn write(self: &mut Self, data: &alloc::vec::Vec<u8>) {
        for ch in data {
            self.put_char(*ch);
        }
    }
}

impl core::fmt::Write for VGA {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for ch in s.bytes() {
            self.put_char(ch);
        }
        Ok(())
    }
}

