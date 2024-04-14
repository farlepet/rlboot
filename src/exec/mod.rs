pub mod fmt;

use alloc::boxed::Box;

use crate::{config::Config, errors::ErrorCode, storage::fs::File};

use self::fmt::{ExecFmt, EXECFMT_INITIAL_CHUNK_SZ};

pub struct ExecFile {
    file: Box<dyn File>,
    fmt: Box<dyn ExecFmt>,
}
impl ExecFile {
    pub fn open(file: Box<dyn File>) -> Result<Self, ErrorCode> {
        let chunk = match file.read(0, EXECFMT_INITIAL_CHUNK_SZ) {
            Ok(data) => data,
            Err(e) => return Err(e)
        };

        Ok(Self {
            file,
            fmt: fmt::find_exec_fmt(&chunk)?
        })
    }

    pub fn prepare(&mut self, config: &Config) -> Result<(), ErrorCode> {
        self.fmt.prepare(&self.file, config)
    }

    pub fn load(&mut self, config: &Config) -> Result<(), ErrorCode> {
        self.fmt.load(&self.file, config)
    }
}

