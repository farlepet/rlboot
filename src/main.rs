#![no_std]
#![no_main]
#![feature(panic_info_message)]

extern crate alloc;

use alloc::rc::Rc;
/* TODO: Write custom allocator */
use linked_list_allocator::LockedHeap;
#[global_allocator]
static HEAP: LockedHeap = LockedHeap::empty();

use core::cell::RefCell;
use core::panic::PanicInfo;
use core::ptr::addr_of;
use core::fmt::Write;

#[macro_use]
mod io;
mod data;
mod bios;
mod intr;
mod storage;
mod config;
mod exec;
mod errors;

use crate::config::Config;
use crate::exec::ExecFile;
use crate::io::output;
use crate::storage::{
    block::{bios::BiosBlockDevice, BlockDevice},
    fs::{Filesystem, fat::FATFilesystem},
};

extern "C" {
    static mut __lboot_end: u8;
    static mut __lboot_bss_begin: u8;
    static mut __lboot_bss_end: u8;
}

fn init_data() {
    unsafe {
        /* Clear BSS */
        let bss = addr_of!(__lboot_bss_begin) as *mut u8;
        let bss_size = addr_of!(__lboot_bss_end) as usize - addr_of!(__lboot_bss_begin) as usize;
        bss.write_bytes(0x00, bss_size);
    }
}

#[no_mangle]
pub extern "C" fn ruststart(boot_drive: u32) -> ! {
    init_data();

    unsafe {
        /* Assuming fully populated conventional memory. Could also use INT 12.
         * Realistically, it's unlikely this will ever be used on a system with
         * less than 1 MiB of RAM. */
        let heap_size: usize = 0x80000 - addr_of!(__lboot_end) as usize;
        let heap_start = addr_of!(__lboot_end) as *mut u8;
        HEAP.lock().init(heap_start, heap_size);
    }

    output::init();

    println!("RLBoot v{} -- (c) 2024 Peter Farley\n", env!("CARGO_PKG_VERSION"));

    println!("Heap size: {} KiB", HEAP.lock().size() / 1024);

    intr::init();
    println!("Interrupts enabled");

    let blk = match BiosBlockDevice::new(boot_drive as u8) {
        Ok(bbd) => Rc::new(RefCell::new(bbd)) as Rc<RefCell<dyn BlockDevice>>,
        Err(e) => {
            println!("Could not create block device: {}", e);
            loop {}
        },
    };
    println!("Block device created");
    let fs = match FATFilesystem::init(&blk, 0) {
        Ok(fs) => fs,
        Err(e) => {
            println!("Could not open filesystem: {}", e);
            loop {}
        }
    };
    println!("Filesystem created");

    let cfg_file = match fs.borrow().find_file(None, "RLBOOT/RLBOOT.CFG") {
        Ok(file) => file,
        Err(e) => {
            println!("Could not find config file: {}", e);
            loop {}
        }
    };
    println!("Config file opened");

    let config = match Config::load(&cfg_file) {
        Ok(cfg) => cfg,
        Err(e) => {
            println!("Error loading config: {}", e);
            loop {}
        }
    };

    println!("{}", config);

    println!("Loading kernel {}", config.kernel_path);
    let exec_file = match fs.borrow().find_file(None, &config.kernel_path) {
        Ok(file) => file,
        Err(e) => {
            println!("Could not find kernel `{}`: {}", config.kernel_path, e);
            loop {}
        }
    };

    let mut exec = match ExecFile::open(exec_file) {
        Ok(exec) => exec,
        Err(e) => {
            println!("Could not load kernel as executable: {:}", e);
            loop {}
        }
    };

    if let Err(e) = exec.prepare(&config) {
        println!("Could not prepare kernel: {}", e);
        loop {}
    }

    if let Err(e) = exec.load(&config) {
        println!("Could not load kernel: {}", e);
        loop {}
    }

    /*let mut port = serial::create_port(serial::SerialPortBase::COM1, &serial::SerialConfig {
        baud: 115200,
        rxfifo_sz: 32,
        txfifo_sz: 32,
        use_rts: false,
        use_dtr: false,
    });

    let _ = write!(port, "This is a test!\n");*/

    loop {}
}

#[allow(unused_variables)]
#[inline(never)]
#[panic_handler]
fn cust_panic(info: &PanicInfo) -> ! {
    #[cfg(feature = "verbose_panic")]
    {
        match info.message() {
            Some(msg) => println!("panic(): {}", msg),
            None      => println!("panic()")
        }
        match info.location() {
            Some(msg) => println!("  Occured at {}", msg),
            None      => {}
        }
    }
    loop {
        //atomic::compiler_fence(Ordering::SeqCst);
    }
}

