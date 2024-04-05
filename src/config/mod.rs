use core::fmt::Error;

use alloc::{
    string::{String, ToString},
    vec::Vec,
};

#[derive(Default)]
pub struct ModuleConfig {
    path: String,
    name: String,
    addr: usize,
    size: usize,
}

#[derive(Default)]
pub struct Config {
    version: u8,
    kernel_path: String,
    kernel_cmdline: String,

    modules: Vec<ModuleConfig>,
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
    pub fn load(data: Vec<u8>) -> Option<Config> {
        let mut conf = Config::default();

        let cstr = match String::from_utf8(data) {
            Ok(st) => st,
            Err(_) => {
                /* TODO: display error */
                return None;
            }
        };

        for line in cstr.lines() {
            let tline = line.trim();

            match tline.find("=") {
                Some(idx) => {
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
                            match Self::parse_module(&val) {
                                Some(md) => conf.modules.push(md),
                                None => { /* TODO */ }
                            }
                        },
                        _ => {
                            /* TODO: Error */
                        }
                    }
                },
                None => {}
            }
        }

        if conf.version != 1 {
            /* TODO: Error */
            return None;
        }

        Some(conf)
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

