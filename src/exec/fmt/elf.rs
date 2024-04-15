#![allow(unused)]

use core::fmt::Write;
use core::mem;

use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;

use crate::output;
use crate::errors::ErrorCode;
use crate::{config::Config, storage::fs::File};
use super::{ExecFmt, ExecFmtTestResult};

pub struct ExecFmtELF {
    ehdr: Option<ElfHeader>,
    phdr: Vec<Elf32ProgramHeader>,
    data_begin: u32,
    data_end: u32,
}

/// Size in bytes of blocks to read from file
const BLOCK_READ_SZ: usize = 1024;

impl ExecFmtELF {
    pub fn new() -> Self {
        ExecFmtELF{
            ehdr: None,
            phdr: vec!(),
            data_begin: u32::MAX,
            data_end: u32::MIN,
        }
    }

    /// Test if an executable is of this type
    ///
    /// # Arguments
    /// * chunk: Initial chunk of data to test against, must be at least of size
    ///   `EXECFMT_INITIAL_CHUNK_SZ`
    pub fn test(chunk: &Vec<u8>) -> ExecFmtTestResult {
        if u32::from_le_bytes(chunk[0..4].try_into().unwrap()) == ELF_IDENT {
            ExecFmtTestResult::Yes
        } else {
            ExecFmtTestResult::No
        }
    }

    /// Check if program header is compatible with our constreaints.
    ///
    /// Returns true if okay, else false
    fn check_phdr(&self) -> bool {
        for phent in &self.phdr {
            if phent.htype == ElfProgramHeaderType::Load {
                if phent.vaddr < 0x100000 {
                    return false;
                }
            }
        }
        true
    }

    /// Load binary using program headers
    fn load_phdr(&mut self, file: &Box<dyn File>) -> Result<(), ErrorCode> {
        for phdr in &self.phdr {
            match phdr.htype {
                ElfProgramHeaderType::Load => {
                    if phdr.paddr < self.data_begin {
                        self.data_begin = phdr.paddr;
                    }
                    if (phdr.paddr + phdr.memsz) > self.data_end {
                        self.data_end = phdr.paddr + phdr.memsz;
                    }

                    if phdr.filesz > 0 {
                        println!("  Loading {} bytes from file at {:x} into {:x}",
                                 phdr.filesz, phdr.offset, phdr.paddr);
                        for off in (0..phdr.filesz).step_by(BLOCK_READ_SZ) {
                            let read_sz = if (off - phdr.filesz) as usize > BLOCK_READ_SZ {
                                BLOCK_READ_SZ
                            } else {
                                (off - phdr.filesz) as usize
                            };

                            match file.read((phdr.offset + off) as isize, read_sz) {
                                Ok(data) => {
                                    let data = data.as_ptr();
                                    let dest = phdr.paddr as *mut u8;
                                    unsafe {
                                        //core::ptr::copy_nonoverlapping(data, dest, phdr.filesz as usize);
                                    }
                                },
                                Err(e) => return Err(e),
                            }
                        }
                                            }
                    if phdr.memsz > phdr.filesz {
                        let dest = (phdr.paddr + phdr.filesz) as *mut u8;
                        println!("  Clearing {} bytes at {:x}",
                                 phdr.memsz - phdr.filesz, phdr.paddr + phdr.filesz);
                        unsafe {
                            dest.write_bytes(0x00, (phdr.memsz - phdr.filesz) as usize);
                        }
                    }
                },
                _ => {}
            }
        }

        Ok(())
    }
}

/* Unaligned read */
macro_rules! ua_read {
    ($item:expr) => {
        { let val = $item; val }
    };
}

impl ExecFmt for ExecFmtELF {
    fn prepare(&mut self, file: &Box<dyn File>, config: &Config) -> Result<(), ErrorCode> {
        let ehdr: ElfHeader = match file.read(0, mem::size_of::<ElfHeader>()) {
            Ok(data) => unsafe {
                core::ptr::read(data.as_ptr() as *const _)
            },
            Err(e) => return Err(e),
        };

        self.ehdr = Some(ehdr);

        if (ehdr.ident.class != ElfClass::Bit32) ||
           (ehdr.ident.data  != ElfDataFormat::LittleEndian) ||
           (ua_read!(ehdr.machine)     != ElfMachine::X86) {
            return Err(ErrorCode::UnsupportedExecOptions);
        }

        if ua_read!(ehdr.etype) != ElfType::Executable {
            return Err(ErrorCode::UnsupportedExecOptions);
        }

        if unsafe { ehdr.data.e32.entry } < 0x100000 {
            /* Don't support entrypoint < 1 MiB */
            return Err(ErrorCode::UnsupportedExecOptions);
        }

        let phdr_sz = unsafe { ehdr.data.e32.phentsize * ehdr.data.e32.phnum };
        let phdr: Vec<Elf32ProgramHeader> = match file.read(unsafe { ehdr.data.e32.phoff } as isize, phdr_sz as usize) {
            Ok(data) => {
                data.chunks(unsafe { ehdr.data.e32.phentsize as usize }).map(|chunk|
                    unsafe {core::ptr::read(chunk.as_ptr() as *const _) }
                ).collect()
            },
            Err(e) => return Err(e),
        };

        self.phdr = phdr;

        if !self.check_phdr() {
            return Err(ErrorCode::UnsupportedExecOptions);
        }

        Ok(())
    }

    fn load(&mut self, file: &Box<dyn File>, config: &Config) -> Result<(), ErrorCode> {
        self.load_phdr(file)
    }

    fn get_entrypoint(&self) -> Option<usize> {
        match self.ehdr {
            Some(ehdr) => Some(unsafe { ehdr.data.e32.entry } as usize),
            None => None,
        }
    }
}

/*
 * ELF definitions
 */
const ELF_IDENT: u32 = 0x464c457f;

#[derive(Clone, Copy)]
#[repr(C, packed(1))]
struct ElfHeaderIdent {
    magic: u32,          //< 32-bit magic number, see `ELF_IDENT`
    class: ElfClass,     //< Class/bittiness
    data: ElfDataFormat, //< Data format/endianness
    version: u8,         //< Version
    osabi: u8,           //< Target ABI
    abiversion: u8,      //< Target ABI version
    _reserved: [u8; 7],
}

#[derive(Clone, Copy)]
#[repr(C, packed(1))]
struct ElfHeaderData32 {
    entry: u32,     //< Entrypoint
    phoff: u32,     //< Program header table offset
    shoff: u32,     //< Section header table offset
    flags: u32,     //< Target-dependant flags
    ehsize: u16,    //< Size of this header (52 bytes)
    phentsize: u16, //< Size of a program header table entry
    phnum: u16,     //< Number of program header table entries
    shentsize: u16, //< Size of a section header table entry
    shnum: u16,     //< Number of section header table entries
    shstrndx: u16,  //< Section index that contains section names
}

#[derive(Clone, Copy)]
#[repr(C, packed(1))]
struct ElfHeaderData64 {
    entry: u64,     //< Entrypoint
    phoff: u64,     //< Program header table offset
    shoff: u64,     //< Section header table offset
    flags: u32,     //< Target-dependant flags
    ehsize: u16,    //< Size of this header (64 bytes)
    phentsize: u16, //< Size of a program header table entry
    phnum: u16,     //< Number of program header table entries
    shentsize: u16, //< Size of a section header table entry
    shnum: u16,     //< Number of section header table entries
    shstrndx: u16,  //< Section index that contains section names
}

#[derive(Clone, Copy)]
#[repr(C, packed(1))]
union ElfHeaderData {
    e32: ElfHeaderData32,
    e64: ElfHeaderData64,
}

#[derive(Clone, Copy)]
#[repr(C, packed(1))]
struct ElfHeader {
    ident: ElfHeaderIdent,
    etype: ElfType,      //< ELF type
    machine: ElfMachine, //< Target machine/ISA
    version: u32,        //< ELF version,
    data: ElfHeaderData
}

#[derive(Clone, Copy, PartialEq)]
#[repr(u8)]
enum ElfClass {
    None  = 0x00,
    Bit32 = 0x01,
    Bit64 = 0x02,
}

#[derive(Clone, Copy, PartialEq)]
#[repr(u8)]
enum ElfDataFormat {
    None         = 0x00,
    LittleEndian = 0x01,
    BigEndian    = 0x02,
}

#[derive(Clone, Copy, PartialEq)]
#[repr(u16)]
enum ElfType {
    None        = 0x0000,
    Relocatable = 0x0001,
    Executable  = 0x0002,
    Dynamic     = 0x0003,
    Core        = 0x0004,
}

#[derive(Clone, Copy, PartialEq)]
#[repr(u16)]
enum ElfMachine {
    None   = 0x0000,
    X86    = 0x0003,
    ARM32  = 0x0028,
    X86_64 = 0x003e,
    ARM64  = 0x00b7,
    RISCV  = 0x00f3,
}

#[derive(Clone, Copy, PartialEq)]
#[repr(u32)]
enum ElfProgramHeaderType {
    Null                       = 0x00000000,
    Load                       = 0x00000001,
    Dynamic                    = 0x00000002,
    InterpreterInfo            = 0x00000003,
    Note                       = 0x00000004,
    SharedLibrary              = 0x00000005,
    ProgramHeaderTable         = 0x00000006,
    ThreadLocalStorageTemplate = 0x00000007
}

#[derive(Clone, Copy)]
#[repr(C)]
struct Elf32ProgramHeader {
    htype:  ElfProgramHeaderType,
    offset: u32, //< Offset of segment into file
    vaddr:  u32, //< Virtual address of segment
    paddr:  u32, //< Physical address of segment, if relevant
    filesz: u32, //< Size of segment data within file, in bytes
    memsz:  u32, //< Size of segment within memory, in bytes
    flags:  u32, //< Segment flags
    align:  u32, //< Required alignment of section
}

