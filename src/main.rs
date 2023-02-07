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
	let mut al: u8 = 0;
	let mut x: u16 = 0;
	let mut y: u16 = 0;

	loop {
		if can_read() {
			al = read_char_from_keyboard();
		}
		handle_input(al, &mut x, &mut y);
	}
}

fn can_read() -> bool {
	let mut eax: u32 = 0;
	unsafe {
		asm!(
			"in al, 0x64",
			"add ax , 0x2f20", // res + sp
			"mov [0xb8000], ax",
			inout("eax") eax,
			options(nostack)
		)
	}
	eax & 0x01 == 1
}

fn read_char_from_keyboard() -> u8 {
	let mut ax: u16 = 0;
	unsafe {
		asm!(
			"in al, 0x60",
			"add ax , 0x3000", // res + sp
			"mov [0xb8002], ax",
			inout("ax") ax
		)
	}
	ax as u8
}

fn handle_input(al: u8, x: &mut u16, y: &mut u16) {
	let attribute = 0x2f;

	match al {
		ARROW_PRESS_LEFT => move_cursor(x, y, 0, -1),
		ARROW_PRESS_TOP => move_cursor(x, y, -1, 0),
		ARROW_PRESS_RIGHT => move_cursor(x, y, 0, 1),
		ARROW_PRESS_DOWN => move_cursor(x, y, 1, 0),
		ARROW_RELEASE_LEFT => put_char(*x, *y, '1', attribute),
		ARROW_RELEASE_TOP => put_char(*x, *y, '2', attribute),
		ARROW_RELEASE_RIGHT => put_char(*x, *y, '3', attribute),
		ARROW_RELEASE_DOWN => put_char(*x, *y, '4', attribute),
		_ => put_char(*x, *y, ' ', attribute),
	}
}

fn move_cursor(x: &mut u16, y: &mut u16, ox: i32, oy: i32) {
	let px = (*x as i32 + ox) % SCREEN_HEIGHT as i32;
	let py = (*y as i32 + oy) % SCREEN_WITDH as i32;
	*x = if px < 0 { 0 } else { px as u16 };
	*y = if py < 0 { 0 } else { py as u16 };
	put_cursor(*x, *y);
}

fn put_cursor(x: u16, y: u16) {
	unsafe {
		asm!(
			"mov dl, cl",
			"mul dl",
			"add bx, ax",		// bx = x * width + y

			"mov dx, 0x03D4",	// dx = 0x03d4
			"mov al, 0x0F",		// 뭔가 컨트롤 명령어?
			"out dx, al",

			"inc dl",		// dx = 0x03d5
			"mov al, bl",		// write bl ?
			"out dx, al",

			"dec dl",		// dx = 0x03d4
			"mov al, 0x0E",		// ?
			"out dx, al",

			"inc dl",		// dx = 0x03d5
			"mov al, bh",		// write bh ?
			"out dx, al",

			in("cl") SCREEN_WITDH as i8,
			in("ax") x,
			in("bx") y
		)
	}
}

fn put_char(x: u16, y: u16, c: char, attribute: u8) {
	let eax: u32 = 0xb8000 + (x as u32) * SCREEN_WITDH + (y as u32) * 2;
	let ebx: u32 = (c as u32) + ((attribute as u32) << 8);
	unsafe {
		asm!(
			"mov [eax], ebx",
			in("eax") eax,
			in("ebx") ebx
		)
	}
}
