// 공유자원이라 락이 필요할 듯 한데... 나중에 고민.
use core::arch::asm;

use super::position::Position;
use super::screen_char::{ColorCode, ScreenChar};
use super::tty::BUFFER_HEIGHT;

const VGA_TEXT_START: u32 = 0xb8000;
pub const SCREEN_WITDH: usize = 80;
pub const SCREEN_HEIGHT: usize = 24;

#[derive(Clone, Copy)]
pub struct Screen;

pub trait IScreen {
	fn draw(buf: &[[ScreenChar; SCREEN_WITDH]; BUFFER_HEIGHT], line: usize);
	fn putc(pos: Position, ch: ScreenChar); // print char at cursor
	fn put_cursor(pos: Position);
	fn line_clear(pos: Position, attr: ColorCode);
}

impl IScreen for Screen {
	fn draw(buf: &[[ScreenChar; SCREEN_WITDH]; BUFFER_HEIGHT], mut buf_x: usize) {
		let mut screen_x = 0;
		let mut addr = VGA_TEXT_START;

		while buf_x < BUFFER_HEIGHT && screen_x < SCREEN_HEIGHT as u8 {
			Screen::print_line(&buf[buf_x], &mut addr);
			buf_x += 1;
			screen_x += 1;
		}

		buf_x = 0;
		while screen_x < SCREEN_HEIGHT as u8 {
			Screen::print_line(&buf[buf_x], &mut addr);
			screen_x += 1;
		}
	}

	fn putc(pos: Position, ch: ScreenChar) {
		let addr: u32 = Screen::vga_addr(pos);
		Screen::putc_addr(addr, ch);
	}

	fn put_cursor(pos: Position) {
		unsafe {
			asm!(
				"mov dl, cl",
				"mul dl",		// ax = al * dl
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

	fn line_clear(pos: Position, attr: ColorCode) {
		let mut screen_y = pos.1;
		let mut addr = Screen::vga_addr(pos);
		let ch = ScreenChar::new(attr, '\0');
		while screen_y < SCREEN_WITDH as u8 {
			Screen::putc_addr(addr, ch);
			screen_y += 1;
			addr += 2;
		}
	}
}

impl Screen {
	fn vga_addr(pos: Position) -> u32 {
		VGA_TEXT_START + pos.0 as u32 * SCREEN_WITDH as u32 * 2 + pos.1 as u32 * 2
	}

	fn print_line(line: &[ScreenChar; SCREEN_WITDH], addr: &mut u32) {
		let mut screen_y = 0;

		while screen_y < SCREEN_WITDH {
			Screen::putc_addr(*addr, line[screen_y]);
			screen_y += 1;
			*addr += 2
		}
	}

	fn putc_addr(addr: u32, ch: ScreenChar) {
		unsafe {
			asm!(
				"mov [eax], bx",
				in("eax") addr,
				in("bx") ch.to_u16()
			)
		}
	}
}
