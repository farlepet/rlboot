extern crate alloc;

pub mod fat;

use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::vec::Vec;
use core::any::Any;
use core::cell::RefCell;
use core::fmt::Error;

use super::block::BlockDevice;

type FilesystemIdent = u8;

pub trait File {
    fn get_size(&self) -> usize;

    fn get_name(&self) -> &str;

    fn get_attr(&self) -> u32;

    fn read(&self, offset: isize, size: usize) -> Result<&Vec<u8>, Error>;

    fn close(&self);

    fn get_fs_ident(&self) -> FilesystemIdent;

    fn as_any(&self) -> &dyn Any;
}

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

