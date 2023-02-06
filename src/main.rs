#![feature(used_with_arg)]

#![no_std]
#![no_main]

use core::panic::PanicInfo;

mod multiboot;
use multiboot::Multiboot2;

#[used(linker)]
#[link_section = ".multiboot2_header"]
static _MULTIBOOT_HEADER: Multiboot2 = Multiboot2::new();

#[panic_handler]
fn software_halt(_info: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn kernel_entry() -> ! {
    loop {}
}