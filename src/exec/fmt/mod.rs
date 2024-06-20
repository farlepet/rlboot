use alloc::boxed::Box;

use crate::config::Config;
use crate::errors::ErrorCode;
use crate::storage::fs::File;

pub mod elf;

pub const EXECFMT_INITIAL_CHUNK_SZ: usize = 512;

pub trait ExecFmt {
    fn prepare(&mut self, file: &dyn File, config: &Config) -> Result<(), ErrorCode>;

    fn load(&mut self, file: &dyn File, config: &Config) -> Result<(), ErrorCode>;

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

pub fn find_exec_fmt(chunk: &[u8]) -> Result<Box<dyn ExecFmt>, ErrorCode> {
    if elf::ExecFmtELF::test(chunk) == ExecFmtTestResult::Yes {
        return Ok(Box::new(elf::ExecFmtELF::new()));
    }

    /* TODO: Flat binary */

    Err(ErrorCode::UnsupportedExecFmt)
}
