extern crate alloc;

pub mod fat;

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::any::Any;
use core::fmt::Error;

#[allow(dead_code)]
pub trait File {
    fn get_size(&self) -> usize;

    fn get_attr(&self) -> u32;

    fn read(&self, offset: isize, size: usize) -> Result<Vec<u8>, Error>;

    fn close(&self);

    fn as_any(&self) -> &dyn Any;
}

#[allow(dead_code)]
pub trait Filesystem {
    /// Get root file for this FS
    fn get_root(&self) -> &dyn File;

    /// Find a file
    ///
    /// # Arguments
    /// * dir: Directory to search from
    /// * path: Path to file
    fn find_file(&self, start_dir: Option<&dyn File>, path: &str) -> Result<Box<dyn File>, Error>;
}

#[repr(u32)]
enum FileAttribute {
    File      = (1 << 0),
    Directory = (1 << 1)
}

