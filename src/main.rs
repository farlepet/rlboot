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
use core::sync::atomic;
use core::sync::atomic::Ordering;
use core::panic::PanicInfo;
use core::ptr::addr_of;
use core::fmt::Write;

#[macro_use]
mod io;
mod bios;
mod storage;

use crate::io::output;
use crate::storage::block::bios::BiosBlockDevice;
use crate::storage::block::BlockDevice;
use crate::storage::fs::fat::FATFilesystem;
use crate::storage::fs::Filesystem;

extern "C" { static mut __lboot_end: u8; }
#[no_mangle]
pub extern "C" fn ruststart() -> ! {
    unsafe {
        /* Assuming fully populated conventional memory. Could also use INT 12.
         * Realistically, it's unlikely this will ever be used on a system with
         * less than 1 MiB of RAM. */
        let heap_size: usize = 0x80000 - addr_of!(__lboot_end) as usize;
        let heap_start = addr_of!(__lboot_end) as *mut u8;
        HEAP.lock().init(heap_start, heap_size);
    }

    output::init();

    let blk = Rc::new(RefCell::new(BiosBlockDevice::new(0x00).unwrap())) as Rc<RefCell<dyn BlockDevice>>;
    let fs = FATFilesystem::init(&blk, 0);
    match fs.borrow().find_file(None, "RLBOOT/RLBOOT.CFG") {
        Ok(file) => {
            match file.read(0, file.get_size()) {
                Ok(data) => {
                    output::hexdump(&data, 0, 0);
                },
                Err(_) => {
                    println!("File read failure");
                }
            }
        },
        Err(_) => {
            println!("File open failure");
        }
    }

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
        atomic::compiler_fence(Ordering::SeqCst);
    }
}

