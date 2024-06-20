#![allow(dead_code)]

use crate::io::ioport::{inb, outb};

fn remap_pic(master: u8, slave: u8) {
    let m_mask = inb(PIC1_DATA);
    let s_mask = inb(PIC2_DATA);

    /* ICW 1 */
    outb(PIC1_COMMAND, PIC_ICW1_IC4 | PIC_ICW1_1);
    outb(PIC2_COMMAND, PIC_ICW1_IC4 | PIC_ICW1_1);
    /* ICW 2 - vector offset*/
    outb(PIC1_DATA, master);
    outb(PIC2_DATA, slave);
    /* ICW 3 */
    outb(PIC1_DATA, 1 << 2); /* IRQ2 as slave input */
    outb(PIC2_DATA, 2);      /* Slave identity is 2 */
    /* ICW 4 */
    outb(PIC1_DATA, PIC_ICW4_UPM);
    outb(PIC2_DATA, PIC_ICW4_UPM);

    outb(PIC1_DATA, m_mask);
    outb(PIC2_DATA, s_mask);
}

pub fn remap() {
    remap_pic(PIC_OFFSET_MASTER, PIC_OFFSET_SLAVE);
}

pub fn remap_bios() {
    remap_pic(PIC_BIOS_OFFSET_MASTER, PIC_BIOS_OFFSET_SLAVE);
}

pub fn eoi(irq_id: u8) {
    if irq_id >= 8 {
        outb(PIC2_COMMAND, PIC_OCW2_EOI);
    }
    /* Due to chaining, EOI always needs to be commanded to the master */
    outb(PIC1_COMMAND, PIC_OCW2_EOI);
}

pub fn mask(mut irq_id: u8) {
    assert!(irq_id < 16);

    let port = if irq_id >= 8 {
        irq_id -= 8;
        PIC2_DATA
    } else { PIC1_DATA };

    let mask = inb(port) | (1 << irq_id);
    outb(port, mask);
}

pub fn unmask(mut irq_id: u8) {
    assert!(irq_id < 16);

    let port = if irq_id >= 8 {
        irq_id -= 8;
        PIC2_DATA
    } else { PIC1_DATA };

    let mask = inb(port) & !(1 << irq_id);
    outb(port, mask);
}


pub const PIC_OFFSET_MASTER: u8 = 32;
pub const PIC_OFFSET_SLAVE: u8  = PIC_OFFSET_MASTER + 8;

const PIC_BIOS_OFFSET_MASTER: u8 = 0x08;
const PIC_BIOS_OFFSET_SLAVE:  u8 = 0x70;

const PIC1_BASE: u16 = 0x20;
const PIC2_BASE: u16 = 0xa0;

const PIC1_COMMAND: u16 = PIC1_BASE;
const PIC1_DATA: u16    = PIC1_BASE + 1;

const PIC2_COMMAND: u16 = PIC2_BASE;
const PIC2_DATA: u16    = PIC2_BASE + 1;

const PIC_ICW1_IC4:  u8 = 1 << 0; //< ICW4 needed
const PIC_ICW1_SNGL: u8 = 1 << 1; //< 1: Single mode, 0: Cascade mode
const PIC_ICW1_ADI:  u8 = 1 << 2; //< Call address interval (1: 4, 0: 8)
const PIC_ICW1_LTIM: u8 = 1 << 3; //< 1: Level triggered mode, 0: Edge triggered mode
const PIC_ICW1_1:    u8 = 1 << 4; //< Set to 1

const PIC_ICW4_UPM:  u8 = 1 << 0; //< 0: MCS-80/85 mode, 1: 8086/8088 mode
const PIC_ICW4_AEOI: u8 = 1 << 1; //< Enable automatic EOI
const PIC_ICW4_MS:   u8 = 1 << 2; //< 0: Slave, 1: Master
const PIC_ICW4_BUF:  u8 = 1 << 3; //< Buffered mode enable
const PIC_ICW4_SFNM: u8 = 1 << 4; //< Enable "Special Fully Nested Mode"

const PIC_OCW2_EOI:  u8 = 1 << 5; //< End of interrupt
const PIC_OCW2_SL:   u8 = 1 << 6; //< "Specific"
const PIC_OCW2_R:    u8 = 1 << 7; //< Rotate
