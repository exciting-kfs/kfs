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
	let mut keyboard = Keyboard::new();
	let mut tty_cont = TtyController::new();

	tty_cont.get_tty_forground().draw();

	loop {
		keyboard.read();
		if let Some(key_input) = keyboard.get_key_input() {
			tty_cont.input(key_input)
		}
	}
}
