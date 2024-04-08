use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::config::Config;
use crate::storage::fs::File;

use super::ExecError;

pub mod elf;

pub const EXECFMT_INITIAL_CHUNK_SZ: usize = 512;

pub trait ExecFmt {
    fn prepare(&mut self, file: &Box<dyn File>, config: &Config) -> Result<(), ExecError>;

    fn load(&mut self, file: &Box<dyn File>, config: &Config) -> Result<(), ExecError>;

    /// Get executable's entrypoint
    fn get_entrypoint(&self) -> Option<usize>;
}

/// Result of ExecFmt::test
#[allow(unused)]
#[derive(PartialEq)]
pub enum ExecFmtTestResult {
    /// The executable is almost definitely of this type
    Yes,
    /// The executable is definitely not of this type
    No,
    /// The executable could be of this type
    Maybe
}

pub fn find_exec_fmt(chunk: &Vec<u8>) -> Result<Box<dyn ExecFmt>, ExecError> {
    if elf::ExecFmtELF::test(chunk) == ExecFmtTestResult::Yes {
        return Ok(Box::new(elf::ExecFmtELF::new()));
    }

    /* TODO: Flat binary */

    Err(ExecError::NoSuitableFormat)
}
