#![no_std]
#![no_main]

use core::sync::atomic;
use core::sync::atomic::Ordering;
use core::panic::PanicInfo;

#[no_mangle]
pub extern "C" fn ruststart() -> ! {
    loop {}
}

#[inline(never)]
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        atomic::compiler_fence(Ordering::SeqCst);
    }
}
