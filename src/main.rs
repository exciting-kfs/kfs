#![feature(used_with_arg)]

#![no_std]
#![no_main]

use core::panic::PanicInfo;
use core::mem::size_of;

#[repr(packed)]
pub struct Multiboot2 {
    magic: u32,
    arch: u32,
    length: u32,
    checksum: u32,
    tag_type: u16,
    tag_flags: u16,
    tag_size: u32,
}

impl Multiboot2 {
    const fn new() -> Self {
        let arch: u32 = 0x0;
        let magic: u32 = 0xE85250D6;
        let length: u32 = size_of::<Multiboot2>() as u32; 
        let checksum: u32 = u32::MAX  - (arch + magic + length) + 1;

        Multiboot2 {
            magic,
            arch,
            length,
            checksum,
            tag_type: 0,
            tag_flags: 0,
            tag_size: 8, 
        }
    }
}

#[used(linker)]
#[link_section = ".multiboot2_header"]
pub static _HEADER: Multiboot2 = Multiboot2::new();

#[panic_handler]
fn software_halt(_info: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn kernel_entry() -> ! {
    loop {}
}