use core::arch::asm;
use core::{mem, ptr::{self, addr_of}};

#[repr(C, align(8))]
struct GDTArray([GDTEntry; 5]);
const _GDT_ARR_SZ_TEST: [u8; 5 * 8] = [0; mem::size_of::<GDTArray>()];

static mut GDT_ARRAY: GDTArray = GDTArray([
    GDTEntry { limit_low: 0, base_low: 0, base_mid: 0, access: 0, flags_limit_high: 0, base_high: 0 };
    5
]);

#[no_mangle]
static mut GDTR: GDTRStruct = GDTRStruct {
    base: 0,
    limit: mem::size_of::<GDTArray>() as u16 - 1,
};

extern "C" {
    static mut __lboot_end: u8;
    static mut __lboot_text_begin: u8;
    static mut __lboot_text_end: u8;
}
pub fn init() {
    unsafe {
        /* 0x00: NULL entry */
        GDT_ARRAY.0[0] = GDTEntry::new(0, 0, 0, 0);
        /* 0x08: 32-bit code segment (strict) */
        let base = addr_of!(__lboot_text_begin) as u32;
        let limit = (addr_of!(__lboot_text_end) as u32 - base) - 1;
        GDT_ARRAY.0[1] = GDTEntry::new(base, limit, 0x4, 0x9b);
        /* 0x10: 32-bit data segment (lenient) - TODO: Allow access over 0xFFFF once debugging
         * complete */
        GDT_ARRAY.0[2] = GDTEntry::new(0, 0xFFFF, 0x4, 0x93);
        /* 0x18: 16-bit code segment (lenient) */
        GDT_ARRAY.0[3] = GDTEntry::new(0, 0xFFFF, 0x0, 0x9b);
        /* 0x20: 16-bit data segment (lenient) */
        GDT_ARRAY.0[4] = GDTEntry::new(0, 0xFFFF, 0x0, 0x93);
    }

    unsafe {
        GDTR.base = ptr::addr_of!(GDT_ARRAY) as u32;

        asm!("lgdt (GDTR)");
    }
}

#[repr(C, packed(1))]
#[derive(Clone, Copy, Default)]
struct GDTEntry {
    limit_low: u16,
    base_low: u16,
    base_mid: u8,
    access: u8,
    flags_limit_high: u8,
    base_high: u8,
}
const _GDT_SZ_TEST: [u8; 8] = [0; mem::size_of::<GDTEntry>()];

impl GDTEntry {
    pub fn new(base: u32, limit: u32, flags: u8, access: u8) -> GDTEntry {
        GDTEntry {
            limit_low: limit as u16,
            flags_limit_high: (((limit >> 16) & 0x0F) as u8) | flags,
            base_low: base as u16,
            base_mid: (base >> 16) as u8,
            base_high: (base >> 24) as u8,
            access
        }
    }
}

/// GDT Register (GDTR) structure
#[repr(C, packed(1))]
struct GDTRStruct {
    limit: u16,
    base: u32,
}
const _GDTR_SZ_TEST: [u8; 6] = [0; mem::size_of::<GDTRStruct>()];

