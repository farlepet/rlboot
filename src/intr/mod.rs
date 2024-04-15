#![allow(dead_code)]

use core::arch::asm;
use core::fmt::Error;
use core::fmt::Write;
use alloc::boxed::Box;

pub mod pic;
pub mod idt;

use crate::bios::EFLAGS_IF;
use crate::io::output;
use crate::errors::ErrorCode;

use self::{
    idt::{IDTEntry, IDT_FLAGS_INTR_32, IDT_FLAGS_TRAP_32},
    pic::{PIC_OFFSET_MASTER, PIC_OFFSET_SLAVE}
};


#[inline(always)]
pub fn interrupts_enable() {
    unsafe { asm!("sti"); }
}

#[inline(always)]
pub fn interrupts_disable() {
    unsafe { asm!("cli"); }
}

#[inline(always)]
pub fn interrupts_enabled() -> bool {
    let mut eflags: u32;
    unsafe {
        asm!(
            "pushfd",
            "pop eax",
            out("eax") eflags
        );
    };
    return (eflags & EFLAGS_IF) != 0;
}

pub fn interrupt_enable(id: InterruptID) {
    let id: u8 = id as u8;
    if (id >= PIC_OFFSET_MASTER) &&
       (id < (PIC_OFFSET_MASTER + 8)) {
        pic::unmask(id - PIC_OFFSET_MASTER);
    } else if (id >= PIC_OFFSET_SLAVE) &&
              (id < (PIC_OFFSET_SLAVE + 8)) {
        pic::unmask(id - PIC_OFFSET_SLAVE + 8);
    }
}

pub fn interrupt_disable(id: InterruptID) {
    let id: u8 = id as u8;
    if (id >= PIC_OFFSET_MASTER) &&
       (id < (PIC_OFFSET_MASTER + 8)) {
        pic::mask(id - PIC_OFFSET_MASTER);
    } else if (id >= PIC_OFFSET_SLAVE) &&
              (id < (PIC_OFFSET_SLAVE + 8)) {
        pic::mask(id - PIC_OFFSET_SLAVE + 8);
    }
}


pub fn init() {
    interrupts_disable();

    isr_wrappers_init();

    idt::init();
    pic::remap();

    interrupts_enable();
}

pub fn interrupt_register(id: InterruptID, handler: impl Fn (u8, u32) + 'static) -> Result<(), ErrorCode> {
    if id >= InterruptID::MAX {
        return Err(ErrorCode::OutOfBounds);
    }
    let id = id as usize;

    unsafe {
        ISR_ENTRIES[id].handler = Some(Box::new(handler));
    }

    Ok(())
}

#[no_mangle]
extern "C" fn interrupt_wrapper(int_id: u32, pusha: PUSHARegs, errcode: u32, iret: IRETRegs) {
    let int_id = int_id as u8;

    if int_id >= InterruptID::MAX as u8 {
        return;
    }

    match unsafe { &ISR_ENTRIES[int_id as usize].handler } {
        Some(handler) => {
            handler(int_id, errcode); },
        None => {
            if int_id < 32 {
                exception_handler(int_id, errcode, &pusha, &iret);
            } else if int_id != 32 {
                println!("Unhandled interrupt: {}", int_id);
            }
        }
    }

    let int_id = int_id as u8;
    if (int_id >= PIC_OFFSET_MASTER) && (int_id <= (PIC_OFFSET_MASTER + 8)) {
        pic::eoi(int_id - PIC_OFFSET_MASTER);
    } else if (int_id >= PIC_OFFSET_SLAVE) && (int_id <= (PIC_OFFSET_SLAVE + 8)) {
        pic::eoi((int_id - PIC_OFFSET_SLAVE) + 8);
    }
}

fn exception_handler(int_id: u8, errcode: u32, pusha: &PUSHARegs, iret: &IRETRegs) {
    println!("exception_handler({}, {:x})", int_id, errcode);

    {
        let eip    = iret.eip;
        let esp    = iret.esp;
        println!("eip: {:08x} esp: {:08x}", eip, esp);
        let cs     = iret.cs;
        let ds     = iret.ds;
        let eflags = iret.eflags;
        println!("cs: {:x} ds: {:x} eflags: {:x}", cs, ds, eflags);
    }
    {
        let eax = pusha.eax;
        let ebx = pusha.ebx;
        let ecx = pusha.ecx;
        let edx = pusha.edx;
        println!("eax: {:08x} ebx: {:08x} ecx: {:08x} edx: {:08x}", eax, ebx, ecx, edx);
        let edi = pusha.edi;
        let esi = pusha.esi;
        let ebp = pusha.ebp;
        let esp = pusha.esp;
        println!("edi: {:08x} esi: {:08x} ebp: {:08x} esp: {:08x}", edi, esi, ebp, esp);
    }

    /* This isn't working currently */
    //panic!("Unhandled exception: {}, {:08x}", int_id, errcode);
    loop {}
}

fn idt_set_wrapper(idx: usize, wrapper: usize) {
    /* Exceptions are traps, all others interrupts */
    let flags = if idx < 32 { IDT_FLAGS_TRAP_32 } else { IDT_FLAGS_INTR_32 };
    let entry = IDTEntry::new(wrapper, 0x0008, flags);
    let _ = idt::set_entry(idx, &entry);
}

extern "C" {
    fn isr_wrapper_0();
    fn isr_wrapper_1();
    fn isr_wrapper_2();
    fn isr_wrapper_3();
    fn isr_wrapper_4();
    fn isr_wrapper_5();
    fn isr_wrapper_6();
    fn isr_wrapper_7();
    fn isr_wrapper_8();
    fn isr_wrapper_9();
    fn isr_wrapper_10();
    fn isr_wrapper_11();
    fn isr_wrapper_12();
    fn isr_wrapper_13();
    fn isr_wrapper_14();
    fn isr_wrapper_15();
    fn isr_wrapper_16();
    fn isr_wrapper_17();
    fn isr_wrapper_18();
    fn isr_wrapper_19();
    fn isr_wrapper_20();
    fn isr_wrapper_21();
    fn isr_wrapper_22();
    fn isr_wrapper_23();
    fn isr_wrapper_24();
    fn isr_wrapper_25();
    fn isr_wrapper_26();
    fn isr_wrapper_27();
    fn isr_wrapper_28();
    fn isr_wrapper_29();
    fn isr_wrapper_30();
    fn isr_wrapper_31();
    fn isr_wrapper_32();
    fn isr_wrapper_33();
    fn isr_wrapper_34();
    fn isr_wrapper_35();
    fn isr_wrapper_36();
    fn isr_wrapper_37();
    fn isr_wrapper_38();
    fn isr_wrapper_39();
    fn isr_wrapper_40();
    fn isr_wrapper_41();
    fn isr_wrapper_42();
    fn isr_wrapper_43();
    fn isr_wrapper_44();
    fn isr_wrapper_45();
    fn isr_wrapper_46();
    fn isr_wrapper_47();
}

fn isr_wrappers_init() {
    idt_set_wrapper(0, isr_wrapper_0 as *const () as usize);
    idt_set_wrapper(1, isr_wrapper_1 as *const () as usize);
    idt_set_wrapper(2, isr_wrapper_2 as *const () as usize);
    idt_set_wrapper(3, isr_wrapper_3 as *const () as usize);
    idt_set_wrapper(4, isr_wrapper_4 as *const () as usize);
    idt_set_wrapper(5, isr_wrapper_5 as *const () as usize);
    idt_set_wrapper(6, isr_wrapper_6 as *const () as usize);
    idt_set_wrapper(7, isr_wrapper_7 as *const () as usize);
    idt_set_wrapper(8, isr_wrapper_8 as *const () as usize);
    idt_set_wrapper(9, isr_wrapper_9 as *const () as usize);
    idt_set_wrapper(10, isr_wrapper_10 as *const () as usize);
    idt_set_wrapper(11, isr_wrapper_11 as *const () as usize);
    idt_set_wrapper(12, isr_wrapper_12 as *const () as usize);
    idt_set_wrapper(13, isr_wrapper_13 as *const () as usize);
    idt_set_wrapper(14, isr_wrapper_14 as *const () as usize);
    idt_set_wrapper(15, isr_wrapper_15 as *const () as usize);
    idt_set_wrapper(16, isr_wrapper_16 as *const () as usize);
    idt_set_wrapper(17, isr_wrapper_17 as *const () as usize);
    idt_set_wrapper(18, isr_wrapper_18 as *const () as usize);
    idt_set_wrapper(19, isr_wrapper_19 as *const () as usize);
    idt_set_wrapper(20, isr_wrapper_20 as *const () as usize);
    idt_set_wrapper(21, isr_wrapper_21 as *const () as usize);
    idt_set_wrapper(22, isr_wrapper_22 as *const () as usize);
    idt_set_wrapper(23, isr_wrapper_23 as *const () as usize);
    idt_set_wrapper(24, isr_wrapper_24 as *const () as usize);
    idt_set_wrapper(25, isr_wrapper_25 as *const () as usize);
    idt_set_wrapper(26, isr_wrapper_26 as *const () as usize);
    idt_set_wrapper(27, isr_wrapper_27 as *const () as usize);
    idt_set_wrapper(28, isr_wrapper_28 as *const () as usize);
    idt_set_wrapper(29, isr_wrapper_29 as *const () as usize);
    idt_set_wrapper(30, isr_wrapper_30 as *const () as usize);
    idt_set_wrapper(31, isr_wrapper_31 as *const () as usize);
    idt_set_wrapper(32, isr_wrapper_32 as *const () as usize);
    idt_set_wrapper(33, isr_wrapper_33 as *const () as usize);
    idt_set_wrapper(34, isr_wrapper_34 as *const () as usize);
    idt_set_wrapper(35, isr_wrapper_35 as *const () as usize);
    idt_set_wrapper(36, isr_wrapper_36 as *const () as usize);
    idt_set_wrapper(37, isr_wrapper_37 as *const () as usize);
    idt_set_wrapper(38, isr_wrapper_38 as *const () as usize);
    idt_set_wrapper(39, isr_wrapper_39 as *const () as usize);
    idt_set_wrapper(40, isr_wrapper_40 as *const () as usize);
    idt_set_wrapper(41, isr_wrapper_41 as *const () as usize);
    idt_set_wrapper(42, isr_wrapper_42 as *const () as usize);
    idt_set_wrapper(43, isr_wrapper_43 as *const () as usize);
    idt_set_wrapper(44, isr_wrapper_44 as *const () as usize);
    idt_set_wrapper(45, isr_wrapper_45 as *const () as usize);
    idt_set_wrapper(46, isr_wrapper_46 as *const () as usize);
    idt_set_wrapper(47, isr_wrapper_47 as *const () as usize);
}

struct ISREntry {
    handler: Option<Box<dyn Fn (u8, u32)>>,
}

const EMPTY_ISRENTRY: ISREntry = ISREntry { handler: None, };
static mut ISR_ENTRIES: [ISREntry; InterruptID::MAX as usize] = [
    EMPTY_ISRENTRY;
    InterruptID::MAX as usize
];

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub enum InterruptID {
    /* Exceptions */
    DivideByZero                =  0,
    Debug                       =  1,
    NonMaskableInterrupt        =  2,
    Breakpoint                  =  3,
    Overflow                    =  4,
    BoundRangeExceeded          =  5,
    InvalidOpcode               =  6,
    DeviceNotAvailable          =  7,
    DoubleFault                 =  8,
    Reserved9                   =  9,
    InvalidTSS                  = 10,
    SegmentNotPresent           = 11,
    StackSegmentFault           = 12,
    GeneralProtectionFault      = 13,
    PageFault                   = 14,
    Reserved15                  = 15,
    FloatingPointFault          = 16,
    AlignmentCheck              = 17,
    MachineCheck                = 18,
    SIMDFloatingPointException  = 19,
    VirtualizationException     = 20,
    ControlProtectionException  = 21,
    /* Master PIC */
    PIT = pic::PIC_OFFSET_MASTER,
    Keyboard,
    Cascade,
    COM2,
    COM1,
    LPT2,
    Floppy,
    LPT1,
    /* Slave PIC */
    CMOSClock = pic::PIC_OFFSET_SLAVE,
    IRQ9,
    IRQ10,
    IRQ11,
    PS2Mouse,
    Coprocessor,
    ATAPrimary,
    ATASecondary,

    MAX
}

#[repr(C, packed(1))]
struct PUSHARegs {
    edi: u32,
    esi: u32,
    ebp: u32,
    esp: u32,
    ebx: u32,
    edx: u32,
    ecx: u32,
    eax: u32,
}

#[repr(C, packed(1))]
struct IRETRegs {
    eip: u32,
    cs : u32,
    eflags: u32,
    esp: u32,
    ds: u32,
}

