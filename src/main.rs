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

use crate::config::Config;
use crate::exec::ExecFile;
use crate::io::output;
use crate::io::serial;
use crate::storage::block::bios::BiosBlockDevice;
use crate::storage::block::BlockDevice;
use crate::storage::fs::fat::FATFilesystem;
use crate::storage::fs::Filesystem;

extern "C" { static mut __lboot_end: u8; }
#[no_mangle]
pub extern "C" fn ruststart(boot_drive: u32) -> ! {
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
    /* Currently, enabling interrupts breaks BIOS calls */
    //intr::init();
    //println!("Interrupts enabled");

    let blk = Rc::new(RefCell::new(BiosBlockDevice::new(boot_drive as u8).unwrap())) as Rc<RefCell<dyn BlockDevice>>;
    let fs = FATFilesystem::init(&blk, 0);
    let cfg_file = match fs.borrow().find_file(None, "RLBOOT/RLBOOT.CFG") {
        Ok(file) => file,
        Err(_) => {
            println!("Could not find config file!");
            loop {};
        }
    };

    let config = match Config::load(&cfg_file) {
        Some(cfg) => cfg,
        None => {
            println!("Error loading config");
            loop {};
        }
    };

    println!("{}", config);

    println!("Loading kernel {}", config.kernel_path);
    let exec_file = match fs.borrow().find_file(None, &config.kernel_path) {
        Ok(file) => file,
        Err(_) => {
            println!("Could not find kernel `{}`", config.kernel_path);
            loop {};
        }
    };

    let mut exec = match ExecFile::open(exec_file) {
        Ok(exec) => exec,
        Err(e) => {
            println!("Could not load kernel as executable: {:?}", e);
            loop {};
        }
    };

    if exec.prepare(&config).is_err() {
        println!("Could not prepare kernel");
    }

    if exec.load(&config).is_err() {
        println!("Could not prepare kernel");
    }

    /*let mut port = serial::create_port(serial::SerialPortBase::COM1, &serial::SerialConfig {
        baud: 115200,
        rxfifo_sz: 32,
        txfifo_sz: 32,
        use_rts: false,
        use_dtr: false,
    });

    let _ = write!(port, "This is a test!\n");*/

    println!("This is a new line");

    loop {}
}

#[inline(never)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
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

