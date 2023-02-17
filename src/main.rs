#![no_std]
#![no_main]

mod collection;
mod console;
mod driver;
mod input;
mod printk;
mod raw_io;
mod util;

use core::panic::PanicInfo;

use driver::vga::text_vga;

use text_vga::{Attr as VGAAttr, Char as VGAChar, Color};

use console::CONSOLE_MANAGER;

use input::{
	key_event::{Code, KeyState},
	keyboard::{Keyboard, KeyboardEvent},
};

use collection::{Window, WrapQueue};

#[panic_handler]
fn panic_handler_impl(_info: &PanicInfo) -> ! {
	if let Some(location) = _info.location() {
		printkln!(
			"PANIC: {}: ({}, {})",
			location.file(),
			location.line(),
			location.column()
		);
	}

	let mut keyboard = Keyboard::new();
	loop {
		if let Some(event) = keyboard.get_keyboard_event() {
			unsafe { CONSOLE_MANAGER.get().panic(event) }
		}
	}
}

#[no_mangle]
pub extern "C" fn kernel_entry() -> ! {
	let cyan = VGAChar::styled(VGAAttr::new(false, Color::Cyan, false, Color::Cyan), b'\0');
	let magenta = VGAChar::styled(
		VGAAttr::new(false, Color::Magenta, false, Color::Magenta),
		b'\0',
	);

	let mut keyboard = Keyboard::new();

	text_vga::clear();
	text_vga::enable_cursor(0, 11);

	loop {
		if let Some(event) = keyboard.get_keyboard_event() {
			if b'`' == event.ascii {
				panic!("I hate backtick!!!");
			}
			text_vga::putc(24, 79, cyan);
			unsafe { CONSOLE_MANAGER.get().update(event) };
		}
		text_vga::putc(24, 79, magenta);
		for _ in 0..50000 {}
	}
}
