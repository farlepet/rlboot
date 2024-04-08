pub mod fmt;

use alloc::boxed::Box;

use crate::{config::Config, storage::fs::File};

use self::fmt::{ExecFmt, EXECFMT_INITIAL_CHUNK_SZ};

#[allow(unused)]
#[derive(Debug)]
pub enum ExecError {
    Generic,
    NoSuitableFormat,
    UnsupportedOptions,
    FileReadError,
    WrongArchitecture,
}

pub struct ExecFile {
    file: Box<dyn File>,
    fmt: Box<dyn ExecFmt>,
}
impl ExecFile {
    pub fn open(file: Box<dyn File>) -> Result<Self, ExecError> {
        let chunk = match file.read(0, EXECFMT_INITIAL_CHUNK_SZ) {
            Ok(data) => data,
            Err(_) => return Err(ExecError::FileReadError)
        };

        Ok(Self {
            file,
            fmt: fmt::find_exec_fmt(&chunk)?
        })
    }

    pub fn prepare(&mut self, config: &Config) -> Result<(), ExecError> {
        self.fmt.prepare(&self.file, config)
    }

    pub fn load(&mut self, config: &Config) -> Result<(), ExecError> {
        self.fmt.load(&self.file, config)
    }
}

