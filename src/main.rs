#![no_std]
#![no_main]

/* TODO: Write custom allocator */
use linked_list_allocator::LockedHeap;
#[global_allocator]
static HEAP: LockedHeap = LockedHeap::empty();

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
    output::puts("This is a test\n");

    let mut bcall = bios::BiosCall {
        int_n: 0x13,
        eax: 0,
        edx: 0,
        ..Default::default()
    };
    unsafe { bcall.call(); }

    let blk = BiosBlockDevice::new(0x00).unwrap();
    let rdata = blk.read(0, 512);
    match rdata {
        Ok(data) => {
            output::hexdump(&data);
        },
        Err(_) => {
            println!("Read failure");
        }
    }

    println!("This is a new line");

    loop {}
}

#[inline(never)]
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    //_info.to_string()
    loop {
        atomic::compiler_fence(Ordering::SeqCst);
    }
}
