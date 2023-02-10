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
	fn draw(buf: &[[char; SCREEN_WITDH]; BUFFER_HEIGHT], line: usize, attr: u8);
	fn putc(pos: Position, c: char, attr: u8); // print char at cursor
	fn put_cursor(pos: Position);
}

impl IScreen for Screen {
	fn draw(buf: &[[char; SCREEN_WITDH]; BUFFER_HEIGHT], mut buf_x: usize, attr: u8) {
		let mut screen_x = 0;

		while buf_x < BUFFER_HEIGHT && screen_x < SCREEN_HEIGHT as u8 {
			Screen::print_line(&buf[buf_x], screen_x, attr);
			buf_x += 1;
			screen_x += 1;
		}

		buf_x = 0;
		while screen_x < SCREEN_HEIGHT as u8 {
			Screen::print_line(&buf[buf_x], screen_x, attr);
			screen_x += 1;
		}
	}

	fn putc(pos: Position, c: char, attr: u8) {
		let eax: u32 = VGA_TEXT_START + pos.0 as u32 * SCREEN_WITDH as u32 * 2 + pos.1 as u32 * 2;
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
	pub fn print_line(buf: &[char; SCREEN_WITDH], screen_x: u8, attr: u8) {
		let mut screen_y = 0;

		while screen_y < SCREEN_WITDH as u8 {
			let pos = Position(screen_x, screen_y);
			Screen::putc(pos, buf[screen_y as usize] as char, attr);
			screen_y += 1;
		}
	}
}
