#![allow(dead_code)]

use core::alloc::Layout;
use core::mem;
use core::ptr::NonNull;

use super::ioport::{outb, inb};
use crate::data::fifo::FIFO;
use crate::intr::{interrupt_enable, interrupt_register, interrupts_enabled, InterruptID};

#[derive(Copy, Clone)]
pub struct SerialConfig {
    pub baud: u32,        //< Desired baud rate
    pub rxfifo_sz: usize, //< Size of RX FIFO
    pub txfifo_sz: usize, //< size of TX FIFO
    pub use_rts: bool,    //< Enable RTS/CTS flow control
    pub use_dtr: bool,    //< Enable DTR/DSR flow control
}

/// Structure of data that the serial port interrupt has access to
struct IntrData {
    port: u16,           //< Base IO port
    mcr_val: u8,         //< Current value of MCR register, for flow control
    fifo_low: usize,     //< Number of FIFO elements under/over which to utilize flow control
    rxfifo: FIFO<u8>,
    txfifo: FIFO<u8>,
    /* Using separate booleans for now, but may use bit flags in the future */
    status_overrun: bool, //< HW or FIFO overrun has occured
    status_framing: bool, //< UART reported a framing error
}

pub struct SerialPort {
    cfg: SerialConfig,
    port: u16,                //< Base IO port
    int_id: InterruptID,      //< Interrupt ID used by serial port
    idata: NonNull<IntrData>, //< Pointer to interrupt-accessible data
}

unsafe fn serial_int_handler(idata: *mut IntrData) {
    /* NOTE: Currently does not support concurrent use of COM1/3 or COM2/4. */
    let port = (*idata).port;
    let iir = inb(port + SERIAL_REG_IIR);

    match (iir >> SERIALREG_IIR_INTID_POS) & SERIALREG_IIR_INTID_MSK {
        SERIALREG_IIR_INTID_TXEMPTY => {
            match (*idata).txfifo.dequeue() {
                Some(byte) => outb(port + SERIAL_REG_DATA, byte),
                None       => { /* TX buffer empty */ }
            }
        },
        SERIALREG_IIR_INTID_RXAVAIL => {
            let data = inb(port + SERIAL_REG_DATA);
            if (*idata).rxfifo.enqueue(data).is_err() {
                (*idata).status_overrun = true;
            }
        },
        SERIALREG_IIR_INTID_RXLINESTATUS => {
            let lsr = inb(port + SERIAL_REG_LSR);
            if (lsr & (1 << SERIALREG_LSR_OE_POS)) != 0 {
                (*idata).status_overrun = true;
            }
            if (lsr & (1 << SERIALREG_LSR_FE_POS)) != 0 {
                (*idata).status_framing = true;
            }
        },
        _ => { /* Unhandled interrupt, we shouldn't be here */ }
    }
}

impl SerialPort {
    fn init(&mut self) {
        outb(self.port + SERIAL_REG_IER, 0);

        {
            /* Assumes standard baud rate, otherwise we might not necessarially
             * find the closest one. */
            let div = (115200 / self.cfg.baud) as u16;

            outb(self.port + SERIAL_REG_LCR, 1 << SERIALREG_LCR_DLAB_POS);
            outb(self.port + SERIAL_REG_DLL, div as u8);
            outb(self.port + SERIAL_REG_DLM, (div >> 8) as u8);
        }

        /* 8N1 */
        outb(self.port + SERIAL_REG_LCR, SERIALREG_LCR_WORDLEN_8BIT << SERIALREG_LCR_WORDLEN_POS);

        /* Disable FIFO to simplify SW FIFO */
        outb(self.port + SERIAL_REG_FCR, (0 << SERIALREG_FCR_FIFOEN_POS)    |
                                         (1 << SERIALREG_FCR_RXFIFORST_POS) |
                                         (1 << SERIALREG_FCR_TXFIFORST_POS) |
                                         (0 << SERIALREG_FCR_TRIGLVL_POS));

        /* Clear outputs */
        outb(self.port + SERIAL_REG_MCR, (1 << SERIALREG_MCR_DTR_POS)  |
                                         (1 << SERIALREG_MCR_RTS_POS)  |
                                         (1 << SERIALREG_MCR_OUT1_POS) |
                                         (1 << SERIALREG_MCR_OUT2_POS));

        unsafe {
            let idata = self.idata.as_ptr();
            let _ = interrupt_register(self.int_id, move |_id, _err| {
                serial_int_handler(idata);
            });
        }
        interrupt_enable(self.int_id);
        outb(self.port + SERIAL_REG_IER, (1 << SERIALREG_IER_RXAVAIL_POS) |
                                         (1 << SERIALREG_IER_TXEMPTY_POS) |
                                         (1 << SERIALREG_IER_RXLINESTATUS_POS));

        unsafe {
            (*self.idata.as_ptr()).mcr_val = inb(self.port + SERIAL_REG_MCR);
        }
    }

    fn tx_ready(&self) -> bool {
        (inb(self.port + SERIAL_REG_LSR) & (1 << SERIALREG_LSR_THRE_POS)) != 0
    }

    fn write_u8(&mut self, data: u8) {
        let idata = self.idata.as_ptr();

        if interrupts_enabled() {
            unsafe {
                /* TODO: Forward error */
                let _ = (*idata).txfifo.enqueue(data);
            }

            if self.tx_ready() {
                /* Last interrupt occured while FIFO empty, or this is the first
                 * write. */
                match unsafe { (*idata).txfifo.dequeue() } {
                    Some(val) => {
                        outb(self.port + SERIAL_REG_DATA, val);
                    },
                    None => { /* This shouldn't happen */ }
                }
            }
        }
    }

    fn read_u8(&self) -> Option<u8> {
        let idata = self.idata.as_ptr();
        unsafe { (*idata).txfifo.dequeue() }
    }
}

impl crate::io::output::IOOutput for SerialPort {
    fn write(&mut self, data: &[u8]) {
        for ch in data {
            self.write_u8(*ch);
        }
    }
}

impl core::fmt::Write for SerialPort {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for ch in s.bytes() {
            self.write_u8(ch);
        }
        Ok(())
    }
}

pub fn create_port(port: SerialPortBase, cfg: &SerialConfig) -> SerialPort {
    let layout = Layout::from_size_align(mem::size_of::<IntrData>(), mem::align_of::<IntrData>()).unwrap();
    let buffer = unsafe {
        alloc::alloc::alloc(layout)
    };

    let mut sport = SerialPort {
        cfg: *cfg,
        port: port as u16,
        int_id:
            if (port == SerialPortBase::COM1) || (port == SerialPortBase::COM3) {
                InterruptID::COM1
            } else {
                InterruptID::COM2
            },
        idata: NonNull::new(buffer as *mut IntrData).unwrap(),
    };

    unsafe {
        *sport.idata.as_ptr() = IntrData {
            port: sport.port,
            mcr_val: 0,
            fifo_low: cfg.rxfifo_sz / 4,
            rxfifo: FIFO::new(cfg.rxfifo_sz),
            txfifo: FIFO::new(cfg.txfifo_sz),
            status_overrun: false,
            status_framing: false,
        };
    }

    sport.init();

    sport
}

#[derive(PartialEq, Clone, Copy)]
#[repr(u16)]
pub enum SerialPortBase {
    COM1 = 0x03f8,
    COM2 = 0x02f8,
    COM3 = 0x03e8,
    COM4 = 0x02e8,
}

const SERIAL_REG_DATA: u16 = 0; //< Rx/Tx buffer
const SERIAL_REG_IER: u16  = 1; //< Interrupt enable register
const SERIAL_REG_IIR: u16  = 2; //< Interrupt identification
const SERIAL_REG_FCR: u16  = 2; //< FIFO control register
const SERIAL_REG_LCR: u16  = 3; //< Line control
const SERIAL_REG_MCR: u16  = 4; //< Modem control
const SERIAL_REG_LSR: u16  = 5; //< Line status
const SERIAL_REG_MSR: u16  = 6; //< Modem status
const SERIAL_REG_SCR: u16  = 7; //< Scratch register
/* DLAB = 1 */
const SERIAL_REG_DLL: u16  = 0; //< Divisor latch low
const SERIAL_REG_DLM: u16  = 1; //< Divisor latch high

/*
 * 8250 UART controller registers
 */

const SERIALREG_IER_RXAVAIL_POS: u8          =   0; //< Received data available
const SERIALREG_IER_TXEMPTY_POS: u8          =   1; //< Transmit holding register empty
const SERIALREG_IER_RXLINESTATUS_POS: u8     =   2; //< Receiver line status
const SERIALREG_IER_MODEMSTATUS_POS: u8      =   3; //< Modem status

const SERIALREG_IIR_NOTPENDING_POS: u8       =   0; //< 0 if interrupt pending
const SERIALREG_IIR_INTID_POS: u8            =   1; //< Interrupt ID
const SERIALREG_IIR_INTID_MSK: u8            =0x03;
const SERIALREG_IIR_INTID_MODEMSTATUS: u8    =   0; //< Modem status interrupt
const SERIALREG_IIR_INTID_TXEMPTY: u8        =   1; //< TX holding register empty
const SERIALREG_IIR_INTID_RXAVAIL: u8        =   2; //< RX data available
const SERIALREG_IIR_INTID_RXLINESTATUS: u8   =   3; //< RX line status
const SERIALREG_IIR_INTID_TIMEOUTPENDING: u8 =   6; //< Timeout interrupt pending

const SERIALREG_FCR_FIFOEN_POS: u8           =   0; //< FIFO Enable
const SERIALREG_FCR_RXFIFORST_POS: u8        =   1; //< RX FIFO Reset
const SERIALREG_FCR_TXFIFORST_POS: u8        =   2; //< TX FIFO Reset
const SERIALREG_FCR_DMAMODE_POS: u8          =   3; //< DMA MODE
const SERIALREG_FCR_64BFIFO_POS: u8          =   5; //< 64 Byte FIFO
const SERIALREG_FCR_TRIGLVL_POS: u8          =   6; //< RX FIFO trigger level
const SERIALREG_FCR_TRIGLVL_MSK: u8          =0x03;

const SERIALREG_LCR_WORDLEN_POS: u8          =   0; //< Word length
const SERIALREG_LCR_WORDLEN_MSK: u8          =0x03;
const SERIALREG_LCR_WORDLEN_5BIT: u8         =0x00; //< 5 bit words
const SERIALREG_LCR_WORDLEN_6BIT: u8         =0x01; //< 6 bit words
const SERIALREG_LCR_WORDLEN_7BIT: u8         =0x02; //< 7 bit words
const SERIALREG_LCR_WORDLEN_8BIT: u8         =0x03; //< 8 bit words
const SERIALREG_LCR_STOPBITS_POS: u8         =   2; //< 1 or 1.5/2 stop bits
const SERIALREG_LCR_PARITY_POS: u8           =   3; //< Parity enable
const SERIALREG_LCR_EVENPARITY_POS: u8       =   4; //< Even parity
const SERIALREG_LCR_STICKPARITY_POS: u8      =   5; //< Stick parity
const SERIALREG_LCR_SETBREAK_POS: u8         =   6; //< Break control
const SERIALREG_LCR_DLAB_POS: u8             =   7; //< Divisor Latch Access Bit (DLAB)

const SERIALREG_MCR_DTR_POS: u8              =   0; //< Set Data Terminal Ready (DTR)
const SERIALREG_MCR_RTS_POS: u8              =   1; //< Set Request To Send (RTS)
const SERIALREG_MCR_OUT1_POS: u8             =   2; //< OUT1 control (RI in loopback)
const SERIALREG_MCR_OUT2_POS: u8             =   3; //< OUT2 control (DCD in loopback)
const SERIALREG_MCR_LOOP_POS: u8             =   4; //< Enable loopback

const SERIALREG_LSR_DR_POS: u8               =   0; //< Data Ready
const SERIALREG_LSR_OE_POS: u8               =   1; //< Overrun Error
const SERIALREG_LSR_PE_POS: u8               =   2; //< Parity Error
const SERIALREG_LSR_FE_POS: u8               =   3; //< Framing Error
const SERIALREG_LSR_BI_POS: u8               =   4; //< Break Interrupt
const SERIALREG_LSR_THRE_POS: u8             =   5; //< Transmitter Holding Register Empty
const SERIALREG_LSR_TEMT_POS: u8             =   6; //< Data Holding Register Empty

const SERIALREG_MSR_DCTS_POS: u8             =   0; //< Change in CTS
const SERIALREG_MSR_DDSR_POS: u8             =   1; //< Change in DSR
const SERIALREG_MSR_TERI_POS: u8             =   2; //< RI De-asserted
const SERIALREG_MSR_DDCD_POS: u8             =   3; //< Change in DCD
const SERIALREG_MSR_CTS_POS: u8              =   4; //< CTS asserted
const SERIALREG_MSR_DSR_POS: u8              =   5; //< DSR asserteed
const SERIALREG_MSR_RI_POS: u8               =   6; //< RI asserted
const SERIALREG_MSR_DCD_POS: u8              =   7; //< DCD asserted

