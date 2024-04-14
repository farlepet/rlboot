pub mod bios;

extern crate alloc;

use alloc::vec::Vec;

use crate::errors::ErrorCode;

#[allow(dead_code)]
pub trait BlockDevice {
    fn get_size(&self) -> usize;

    /// Read bytes from storage device
    ///
    /// # Arguments
    /// * offset - Offset at which to start reading
    /// * size - How many bytes to read
    fn read(&self, offset: isize, size: usize) -> Result<Vec<u8>, ErrorCode>;
}

