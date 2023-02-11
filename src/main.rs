#![feature(exclusive_range_pattern)]
#![no_std]
#![no_main]

use core::arch::asm;
use core::panic::PanicInfo;

mod tty;
use tty::controller::TtyController;
use tty::keyboard::Keyboard;

mod vga;
use vga::TextVGA;
use vga::Char as VGAChar;
use vga::Attr as VGAAttr;
use vga::Color as Color;

mod pmio;

mod ps2;
use ps2::keyboard;

const SCREEN_WITDH: u32 = 80;
const SCREEN_HEIGHT: u32 = 25;

const ARROW_PRESS_LEFT: u8 = 0x4b;
const ARROW_PRESS_TOP: u8 = 0x48;
const ARROW_PRESS_RIGHT: u8 = 0x4d;
const ARROW_PRESS_DOWN: u8 = 0x50;
const ARROW_RELEASE_LEFT: u8 = 0xcb;
const ARROW_RELEASE_TOP: u8 = 0xc8;
const ARROW_RELEASE_RIGHT: u8 = 0xcd;
const ARROW_RELEASE_DOWN: u8 = 0xd0;

#[panic_handler]
fn panic_handler_impl(_info: &PanicInfo) -> ! {
	unsafe { asm!("mov eax, 0x2f65", "mov [0xb8000], eax") }
	loop {}
}

#[no_mangle]
pub extern "C" fn kernel_entry() -> ! {
	let cyan    = VGAChar::styled(VGAAttr::new(false, Color::Cyan, false, Color::Cyan), b'\0');
	let magenta = VGAChar::styled(VGAAttr::new(false, Color::Magenta, false, Color::Magenta), b'\0');

	TextVGA::clear();

	loop {	
		if let Some(event) = keyboard::get_key_event() {
			TextVGA::clear();
			TextVGA::putc(24, 79, cyan);

			if event.key == keyboard::Key::A {
				let attr = match event.state {
					keyboard::KeyState::Pressed => VGAAttr::new(false, Color::Cyan, false, Color::White),
					keyboard::KeyState::Released => VGAAttr::new(false, Color::Magenta, false, Color::White),
				};
				TextVGA::putc(0, 1, VGAChar::styled(attr, b'A'));
			}
			for _ in 0..500000 {}
		} else {
			TextVGA::putc(24, 79, magenta);
			for _ in 0..500000 {}
		}
		
	}
}
