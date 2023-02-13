#![no_std]
#![no_main]

mod console;
mod driver;
mod input;
mod raw_io;

use core::arch::asm;
use core::panic::PanicInfo;

use driver::vga::text_vga;

use text_vga::Attr as VGAAttr;
use text_vga::Char as VGAChar;
use text_vga::Color;

use input::key_event::Key;
use input::keyboard::Keyboard;

use console::ConsoleManager;

#[panic_handler]
fn panic_handler_impl(_info: &PanicInfo) -> ! {
	unsafe { asm!("mov eax, 0x2f65", "mov [0xb8000], eax") }
	loop {}
}

#[no_mangle]
pub extern "C" fn kernel_entry() -> ! {
	let cyan = VGAChar::styled(VGAAttr::new(false, Color::Cyan, false, Color::Cyan), b'\0');
	let magenta = VGAChar::styled(
		VGAAttr::new(false, Color::Magenta, false, Color::Magenta),
		b'\0',
	);

	let mut keyboard = Keyboard::new();
	let mut console_manager = ConsoleManager::new();

	text_vga::clear();
	text_vga::enable_cursor(0, 11);

	loop {
		if let Some(event) = keyboard.get_keyboard_event() {
			text_vga::putc(24, 79, cyan);
			console_manager.update(event);
		}
		text_vga::putc(24, 79, magenta);
		for _ in 0..90000 {}
	}
}
