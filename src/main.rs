#![feature(exclusive_range_pattern)]
#![no_std]
#![no_main]

mod driver;
mod raw_io;

use core::arch::asm;
use core::panic::PanicInfo;

use driver::vga::text_vga;
use text_vga::Char as VGAChar;
use text_vga::Attr as VGAAttr;
use text_vga::Color as Color;

use driver::ps2::keyboard;

#[panic_handler]
fn panic_handler_impl(_info: &PanicInfo) -> ! {
	unsafe { asm!("mov eax, 0x2f65", "mov [0xb8000], eax") }
	loop {}
}

#[no_mangle]
pub extern "C" fn kernel_entry() -> ! {
	let cyan    = VGAChar::styled(VGAAttr::new(false, Color::Cyan, false, Color::Cyan), b'\0');
	let magenta = VGAChar::styled(VGAAttr::new(false, Color::Magenta, false, Color::Magenta), b'\0');

	text_vga::clear();

	loop {	
		if let Some(event) = keyboard::get_key_event() {
			text_vga::clear();
			text_vga::putc(24, 79, cyan);

			if event.key == keyboard::Key::A {
				let attr = match event.state {
					keyboard::KeyState::Pressed => VGAAttr::new(false, Color::Cyan, false, Color::Black),
					keyboard::KeyState::Released => VGAAttr::new(false, Color::Magenta, false, Color::Black),
				};
				text_vga::putc(0, 1, VGAChar::styled(attr, b'A'));
			}
			for _ in 0..100000 {}
		} else {
			text_vga::putc(24, 79, magenta);
			for _ in 0..100000 {}
		}
		
	}
}
