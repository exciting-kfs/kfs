// 공유자원이라 락이 필요할 듯 한데... 나중에 고민.
use core::arch::asm;

use super::position::Position;
use super::tty::BUFFER_HEIGHT;

const VGA_TEXT_START: u32 = 0xb8000;
// const VGA_TEXT_END: u32 = 0xb8fa0;
pub const SCREEN_WITDH: usize = 80;
pub const SCREEN_HEIGHT: usize = 25;

#[derive(Clone, Copy)]
pub struct Screen;

pub trait IScreen {
	fn draw(screen_pos: usize, buf: &[[u8; SCREEN_WITDH]; BUFFER_HEIGHT], attr: u8);
	fn putc(c: char, attr: u8, pos: Position); // print char at cursor
	fn put_cursor(pos: Position);
}

impl IScreen for Screen {
	fn draw(mut line: usize, buf: &[[u8; SCREEN_WITDH]; BUFFER_HEIGHT], attr: u8) {
		let mut index = 0;

		while line < BUFFER_HEIGHT {
			Screen::print_line(index, &buf[line], attr);
			line += 1;
			index += 1;
		}

		line = 0;
		while index < SCREEN_HEIGHT as u8 {
			Screen::print_line(index, &buf[line], attr);
			index += 1;
		}
	}

	fn putc(c: char, attr: u8, pos: Position) {
		let eax: u32 = VGA_TEXT_START + (pos.0 * SCREEN_WITDH as u8 * 2 + pos.1 * 2) as u32;
		let ebx: u32 = (c as u32) + ((attr as u32) << 8);
		unsafe {
			asm!(
				"mov [eax], ebx",
				in("eax") eax,
				in("ebx") ebx
			)
		}
	}

	fn put_cursor(pos: Position) {
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
				in("ax") pos.0 as i16,
				in("bx") pos.1 as i16
			)
		}
	}
}

impl Screen {
	pub fn print_line(index: u8, buf: &[u8; 80], attr: u8) {
		let mut i = 0;

		while i < SCREEN_WITDH {
			let pos = Position(index, i as u8);
			Screen::putc(buf[i as usize] as char, attr, pos);
			i += 1;
		}
	}
}
