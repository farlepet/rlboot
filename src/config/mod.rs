use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use crate::{errors::ErrorCode, storage::fs::File};

#[derive(Default)]
pub struct ModuleConfig {
    pub path: String,
    pub name: String,
    pub addr: usize,
    pub size: usize,
}

#[derive(Default)]
pub struct Config {
    pub version: u8,
    pub kernel_path: String,
    pub kernel_cmdline: String,

    pub modules: Vec<ModuleConfig>,
}

impl core::fmt::Display for Config {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Config {{ version: {}, path: {}, cmdline: {}, modules: ",
               self.version, self.kernel_path, self.kernel_cmdline)?;
        for md in &self.modules {
            write!(f, "\n  {{ path: {}, name: {}, addr: 0x{:x}, size: 0x{:x} }}",
                   md.path, md.name, md.addr, md.size)?;
        }
        write!(f, "}}")?;

        Ok(())
    }
}

impl Config {
    pub fn load(file: &dyn File) -> Result<Config, ErrorCode> {
        let data = match file.read(0, file.get_size()) {
            Ok(data) => data,
            Err(e) => return Err(e)
        };

        let mut conf = Config::default();

        let cstr = match String::from_utf8(data) {
            Ok(st) => st,
            Err(_) => return Err(ErrorCode::ConfigFormatError)
        };

        for line in cstr.lines() {
            let tline = line.trim();

            if let Some(idx) = tline.find("=") {
                let (key, val) = tline.split_at(idx);
                let key = key.trim();
                let val = val[1..].trim();

                match key {
                    "CFGVER" => {
                        match str::parse(val) {
                            Ok(parsed) => conf.version = parsed,
                            Err(_) => { /* TODO */ }
                        }
                    },
                    "KERNEL" => conf.kernel_path = val.to_string(),
                    "CMDLINE" => conf.kernel_cmdline = val.to_string(),
                    "MODULE" => {
                        match Self::parse_module(val) {
                            Some(md) => conf.modules.push(md),
                            None => { /* TODO */ }
                        }
                    },
                    _ => {
                        /* TODO: Error */
                    }
                }
            }
        }

        if conf.version != 1 {
            /* TODO: Error */
            return Err(ErrorCode::UnsupportedConfig);
        }

        Ok(conf)
    }

    fn parse_module(cfg: &str) -> Option<ModuleConfig> {
        let md = ModuleConfig {
            path: cfg.to_string(),
            /* TODO: Support custom module names */
            name: cfg.to_string(),
            /* These are set at module load */
            addr: 0,
            size: 0,
        };

        Some(md)
    }
}

