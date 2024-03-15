extern crate alloc;


use alloc::vec::Vec;
use alloc::vec;
use core::ptr::{addr_of, addr_of_mut};
use core::fmt::{Error,Write};
use core::usize;

use crate::storage::block::BlockDevice;
use crate::bios::{self, BiosCall};
use crate::io::output;

pub struct BiosBlockDevice {
    bios_id: u8,            //< Bios drive ID

    size: usize,            //< Size of device in bytes
    sectors_per_track: u16, //< Sectors per track
    n_heads: u16,           //< Number of heads
}

impl BiosBlockDevice {
    pub fn new(id: u8) -> Result<BiosBlockDevice, Error> {
        if id < 0x80 {
            /* INT 0x13, AH = 0x08 supposedly can report incorrect values for a
             * floppy disk. Assuming a standard 1.44MB disk geometry here. I'll
             * take any excuse to be lazy. */
            Ok(BiosBlockDevice {
                bios_id: id,
                size: 2880 * 512,
                sectors_per_track: 18,
                n_heads: 2
            })
        } else {
            /* TODO */
            Err(Error)
        }
    }

    fn floppy_reset(&self) {
        let mut bcall = bios::BiosCall {
            int_n: 0x13,
            eax: 0,
            edx: self.bios_id as u32,
            ..Default::default()
        };
        unsafe { bcall.call(); }
    }

    fn floppy_read_sector(&self, offset: isize) -> Result<Vec<u8>, Error> {
        let offset = offset / 512;

        let track: u16 = (offset / self.sectors_per_track as isize) as u16;
        let sector: u8 = ((offset % self.sectors_per_track as isize) + 1) as u8;
        let head:   u8 = (track % self.n_heads) as u8;
        let track: u16 = track / self.n_heads;

        let mut data: [u8; 512] = [0; 512];

        if addr_of!(data) as usize > 0xFE00 {
            println!("Data buffer too high!");
            return Err(Error);
        }

        let mut attempts = 4;
        while attempts > 0 {
            attempts -= 1;

            let mut bcall = BiosCall {
                int_n: 0x13,
                eax:   0x0201,
                ebx:   addr_of_mut!(data) as u32,
                ecx:   (sector as u16 | (track << 8)) as u32,
                edx:   (self.bios_id as u16 | ((head as u16) << 8)) as u32,
                ..Default::default()
            };

            unsafe { bcall.call(); }

            if (bcall.eflags & bios::EFLAGS_CF) == 0 {
                return Ok(data.to_vec());
            }

            self.floppy_reset();
        }

        println!("Read failure");
        Err(Error)
    }
}

impl BlockDevice for BiosBlockDevice {
    fn get_size(&self) -> usize {
        self.size
    }

    fn read(&self, offset: isize, size: usize) -> Result<Vec<u8>, Error> {
        if ((offset % 512) != 0) || ((size % 512) != 0) {
            /* Only sector-aligned reads are currently supported */
            return Err(Error);
        }

        let mut data: Vec<u8> = vec!();

        let mut pos: usize = 0;

        if self.bios_id < 0x80 {
            while pos < size {
                match self.floppy_read_sector(offset + pos as isize) {
                    Ok(v) => {
                        data.extend(v.iter());
                        pos += 512;
                    },
                    Err(e) => {
                        return Err(e)
                    }
                }
            }
        } else {
            /* TODO: HDD */
            return Err(Error);
        }

        Ok(data)
    }
}

