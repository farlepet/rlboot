pub mod bios;

extern crate alloc;

use core::fmt::Error;

use alloc::vec::Vec;

pub trait BlockDevice {
    fn get_size(&self) -> usize;

    /// Read bytes from storage device
    ///
    /// # Arguments
    /// * offset - Offset at which to start reading
    /// * size - How many bytes to read
    fn read(&self, offset: isize, size: usize) -> Result<Vec<u8>, Error>;
}

