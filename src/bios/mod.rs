#![allow(dead_code)]

use core::fmt::Write;

use crate::io::output;

pub const EFLAGS_CF: u32 = 1 <<  0; //< Carry flag
pub const EFLAGS_PF: u32 = 1 <<  2; //< Parity flag
pub const EFLAGS_AF: u32 = 1 <<  4; //< Auxiliary flag
pub const EFLAGS_ZF: u32 = 1 <<  6; //< Zero flag
pub const EFLAGS_SF: u32 = 1 <<  7; //< Sign flag
pub const EFLAGS_TF: u32 = 1 <<  8; //< Trap flag
pub const EFLAGS_IF: u32 = 1 <<  9; //< Interrupt enable flag
pub const EFLAGS_DF: u32 = 1 << 10; //< Direction flag
pub const EFLAGS_OF: u32 = 1 << 11; //< Overflow flag
pub const EFLAGS_NT: u32 = 1 << 14; //< Nested task flag

#[repr(C)]
pub struct BiosCall {
    pub int_n: u8,          /// Interrupt ID
    pub _padding: [u8; 3],

    pub eax: u32,
    pub ebx: u32,
    pub ecx: u32,
    pub edx: u32,
    pub esi: u32,
    pub edi: u32,

    pub eflags: u32,
}

impl Default for BiosCall {
    fn default() -> BiosCall {
        BiosCall {
            int_n: 0,
            _padding: [ 0, 0, 0 ],
            eax: 0,
            ebx: 0,
            ecx: 0,
            edx: 0,
            esi: 0,
            edi: 0,
            eflags: 0,
        }
    }
}

extern "C" {
    fn bios_call_asm(bcall: *mut BiosCall);
}

impl BiosCall {
    pub unsafe fn call(&mut self) {
        bios_call_asm(self);
    }

    pub fn print(&self) {
        println!("BiosCall {:02x}:", self.int_n);
        println!("  EAX: {:08x} EBX: {:08x}", self.eax, self.ebx);
        println!("  ECX: {:08x} EDX: {:08x}", self.ecx, self.edx);
        println!("  ESI: {:08x} EDI: {:08x}", self.esi, self.edi);
        println!("  EFLAGS: {:08x}", self.eflags);
    }
}

