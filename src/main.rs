#![feature(used_with_arg)]
#![feature(asm_const)]
#![feature(naked_functions)]
#![no_std]
#![no_main]

use core::arch::asm;
use core::panic::PanicInfo;

mod multiboot;
use multiboot::Multiboot2;

mod stack;
use stack::TempStack;

mod tty;
use tty::controller::TtyController;
use tty::keyboard::Keyboard;

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

#[used(linker)]
#[link_section = ".multiboot2_header"]
static _MULTIBOOT_HEADER: Multiboot2 = Multiboot2::new();

#[used(linker)]
#[no_mangle]
#[link_section = ".stack.temp"]
static _TEMP_STACK: TempStack = TempStack::new();

#[panic_handler]
fn software_halt(_info: &PanicInfo) -> ! {
	unsafe { asm!("mov eax, 0x2f65", "mov [0xb8000], eax") }
	loop {}
}

#[no_mangle]
#[cfg(target_arch = "x86")]
extern "C" fn kernel_init() -> ! {
	#![naked]
	unsafe {
		asm!(
		"mov eax, _TEMP_STACK", // stack bot의 주소를 가져오는 더 좋은 방법?
		"add eax, {}",
		"mov esp, eax",
		"mov al, 0xa7",
		"out 0x64, al",		// disable second PS/2 port

		"jmp kernel_entry",
		const stack::TEMP_STACK_SIZE,
		options(noreturn),
		)
	}
}

#[no_mangle]
pub extern "C" fn kernel_entry() -> ! {
	let mut keyboard = Keyboard::new();
	let mut tty_cont = TtyController::new();

	tty_cont.get_tty().draw();

	loop {
		keyboard.read();
		if let Some(key_input) = keyboard.get_key_input() {
			tty_cont.input(key_input)
		}
	}
}
