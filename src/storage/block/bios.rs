extern crate alloc;


use alloc::vec::Vec;
use alloc::vec;
use core::ptr::addr_of_mut;
use core::fmt::{Error,Write};

use crate::storage::block::BlockDevice;
use crate::bios;
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
            edx: 0,
            ..Default::default()
        };
        unsafe { bcall.call(); }
    }

    fn floppy_read_sector(&self, offset: isize) -> Result<Vec<u8>, Error> {
        let mut bcall = bios::BiosCall {
            int_n: 0x13,
            eax: 0x0201,
            ..Default::default()
        };

        let track: u16 = (offset / self.sectors_per_track as isize) as u16;
        let sector: u8 = ((offset % self.sectors_per_track as isize) + 1) as u8;
        let head:   u8 = (track % self.n_heads) as u8;
        let track: u16 = track / self.n_heads;

        let mut data: [u8; 512] = [0; 512];

        let mut attempts = 4;
        while attempts > 0 {
            println!("Attempting to read T:H:S: {}:{}:{}", track, head, sector);
            attempts -= 1;

            bcall.ebx = addr_of_mut!(data) as u32;
            bcall.ecx = (sector as u16 | (track << 8)) as u32;
            bcall.edx = (self.bios_id as u16 | ((head as u16) << 8)) as u32;
            unsafe { bcall.call(); }

            if (bcall.eflags & bios::EFLAGS_CF) == 0 {
                println!("Read success");
                return Ok(data.to_vec());
            }

            println!("EFLAGS: {:x}", bcall.eflags);
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

