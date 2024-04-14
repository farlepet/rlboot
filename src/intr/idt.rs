#![allow(dead_code)]

use core::arch::asm;
use core::fmt::Error;
use core::{mem, ptr};

use crate::errors::ErrorCode;

use super::InterruptID;

#[repr(C, align(8))]
struct IDTArray([IDTEntry; InterruptID::MAX as usize]);
const _IDT_ARR_SZ_TEST: [u8; InterruptID::MAX as usize * 8] = [0; mem::size_of::<IDTArray>()];

static mut IDT_ARRAY: IDTArray = IDTArray([
    IDTEntry { offset_low: 0, segment: 0, _reserved: 0, flags: 0, offset_high: 0 };
    InterruptID::MAX as usize
]);

#[no_mangle]
static mut IDTR: IDTRStruct = IDTRStruct {
    base: 0,
    limit: mem::size_of::<IDTArray>() as u16 - 1,
};

pub fn init() {
    unsafe {
        IDTR.base = ptr::addr_of!(IDT_ARRAY) as u32;

        asm!("lidt (IDTR)");
    }
}

pub fn set_entry(idx: usize, entry: &IDTEntry) -> Result<(), ErrorCode> {
    if idx >= (InterruptID::MAX as usize) {
        return Err(ErrorCode::OutOfBounds);
    }

    unsafe { IDT_ARRAY.0[idx] = *entry; }

    Ok(())
}

/// IDT entry structure
#[repr(C, packed(1))]
#[derive(Clone, Copy)]
pub struct IDTEntry {
    offset_low: u16,  //< Lower 16-bits of offset
    segment: u16,     //< Segment selector
    _reserved: u8,    //< Reserved, set to 0
    flags: u8,        //< IDT entry flags
    offset_high: u16, //< Higher 16-bits of offset
}
const _IDT_SZ_TEST: [u8; 8] = [0; mem::size_of::<IDTEntry>()];

impl IDTEntry {
    pub fn new(offset: usize, segment: u16, flags: u8) -> IDTEntry {
        IDTEntry {
            offset_low: offset as u16,
            offset_high: (offset >> 16) as u16,
            segment,
            flags,
            _reserved: 0,
        }
    }
}

/// IDT Register (IDTR) structure
#[repr(C, packed(1))]
struct IDTRStruct {
    limit: u16,
    base: u32,
}
const _IDTR_SZ_TEST: [u8; 6] = [0; mem::size_of::<IDTRStruct>()];

pub const IDT_FLAGS_TASK: u8    = 0x85;
pub const IDT_FLAGS_INTR_16: u8 = 0x86;
pub const IDT_FLAGS_TRAP_16: u8 = 0x87;
pub const IDT_FLAGS_INTR_32: u8 = 0x8e;
pub const IDT_FLAGS_TRAP_32: u8 = 0x8f;
