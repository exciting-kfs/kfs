#![no_std]
#![no_main]

mod collection;
mod console;
mod driver;
mod input;
mod io;
mod printk;
mod subroutine;
mod util;

use core::{fmt::Write, panic::PanicInfo};

use driver::vga::text_vga;

use text_vga::{Attr as VGAAttr, Char as VGAChar, Color};

use console::{CONSOLE_COUNTS, CONSOLE_MANAGER};

use input::{
	key_event::{Code, KeyState},
	keyboard::{Keyboard, KEYBOARD},
};

use collection::{Window, WrapQueue};

#[panic_handler]
fn panic_handler_impl(info: &PanicInfo) -> ! {
	unsafe { CONSOLE_MANAGER.get().set_foreground(CONSOLE_COUNTS - 1) };

	printk_panic!("{}", info);

	loop {}
}

#[no_mangle]
pub extern "C" fn kernel_entry() -> ! {
	let cyan = VGAChar::styled(VGAAttr::new(false, Color::Cyan, false, Color::Cyan), b'\0');
	let magenta = VGAChar::styled(
		VGAAttr::new(false, Color::Magenta, false, Color::Magenta),
		b'\0',
	);

	text_vga::clear();
	text_vga::enable_cursor(0, 11);

	loop {
		if let Some(event) = unsafe { KEYBOARD.get_keyboard_event() } {
			if event.key == Code::Backtick && event.pressed() {
				static mut I: usize = 0;
				unsafe {
					pr_warn!("BACKTICK PRESSED {} TIMES!!", I);
					I += 1;
				}
			}
			text_vga::putc(24, 79, cyan);
			unsafe {
				CONSOLE_MANAGER.get().update(event);
				CONSOLE_MANAGER.get().draw();
			};
		}
		text_vga::putc(24, 79, magenta);
	}
}
