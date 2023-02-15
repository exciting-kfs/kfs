use crate::driver::vga::text_vga::{self, Attr as VGAAttr, Char as VGAChar};
use crate::input::key_event::{Code, Key, KeyState};
use crate::input::keyboard::KeyboardEvent;
use crate::printkln;

use super::cursor::{Cursor, MoveResult};
use super::key_record::KeyRecord;

pub const BUFFER_HEIGHT: usize = 100;
pub const BUFFER_WIDTH: usize = 80;

pub trait IConsole {
	fn update(&mut self, ev: &KeyboardEvent, record: &KeyRecord);
	fn draw(&self);
}

#[derive(Clone, Copy)]
pub struct Console {
	buf: [[VGAChar; BUFFER_WIDTH]; BUFFER_HEIGHT],
	pub vga_top: usize,
	pub buf_top: usize,
	pub cursor: Cursor,
	attr: VGAAttr,
}

impl Console {
	pub const fn new() -> Self {
		let default_char = VGAChar::new(0);
		Console {
			buf: [[default_char; BUFFER_WIDTH]; BUFFER_HEIGHT],
			buf_top: 0,
			vga_top: 0,
			cursor: Cursor::new(0, 0),
			attr: VGAAttr::default(),
		}
	}

	pub fn put_char(&mut self, c: u8) {
		let ch = VGAChar::styled(self.attr, c);
		let y = self.cursor.y + self.vga_top;
		let x = self.cursor.x;

		self.buf[y][x] = ch;
	}

	pub fn put_char_cursor(&mut self, c: u8, pos: Cursor) {
		let ch = VGAChar::styled(self.attr, c);
		let y = pos.y;
		let x = pos.x;

		self.buf[y][x] = ch;
	}

	pub fn delete_char(&mut self) {
		self.put_char(0);
	}

	pub fn change_color(&mut self, color: Code) {
		let color = color as u16;
		self.attr = VGAAttr::form_u8(color as u8);

		for y in 0..BUFFER_HEIGHT {
			for x in 0..BUFFER_WIDTH {
				let ch = self.buf[y][x];
				let ch = VGAChar(color << 8 | ch.0 & 0x00ff);
				self.buf[y][x] = ch;
			}
		}
	}

	pub fn move_cursor(&mut self, code: Code) {
		let home = -(self.cursor.x as isize);
		let end = (BUFFER_WIDTH - self.cursor.x - 1) as isize;
		let up = -(text_vga::HEIGHT as isize) + 1;
		let down = text_vga::HEIGHT as isize - 1;

		let res = match code {
			Code::Home => self.cursor.relative_move(0, home),
			Code::ArrowUp => self.cursor.relative_move(-1, 0),
			Code::PageUp => self.cursor.relative_move(up, 0),
			Code::ArrowLeft => self.cursor.relative_move(0, -1),
			Code::ArrowRight => self.cursor.relative_move(0, 1),
			Code::End => self.cursor.relative_move(0, end),
			Code::ArrowDown => self.cursor.relative_move(1, 0),
			Code::PageDown => self.cursor.relative_move(down, 0),
			_ => MoveResult::Pass,
		};

		if let MoveResult::AdjustTop(dy) = res {
			self.adjust_vga_top(dy)
		}
	}

	fn _draw(&self) {
		text_vga::draw(&self.buf, self.vga_top);
		text_vga::put_cursor(self.cursor.y, self.cursor.x);

	}

	fn adjust_vga_top(&mut self, dy: isize) {
		let vga_height: isize = text_vga::HEIGHT as isize;
		let buf_height: isize = BUFFER_HEIGHT as isize;
		let mut vga_top: isize = self.vga_top as isize;
		let s = self.buf_top as isize;
		let e = s - vga_height + 1 + buf_height;
		let top;

		if vga_top < s {
			vga_top += buf_height;
		}

		let y = dy + vga_top as isize;

		if dy < 0 && y < s {
			top = s;
		} else if dy > 0 && y >= e {
			top = e - 1;
		} else {
			top = y;
		}

		self.vga_top = top as usize % BUFFER_HEIGHT;
	}
}

impl IConsole for Console {
	fn draw(&self) {
		self._draw();
	}

	fn update(&mut self, ev: &KeyboardEvent, record: &KeyRecord) {
		if let (Key::Control(c), KeyState::Pressed) = (ev.key, ev.state) {
			match c {
				Code::Home
				| Code::ArrowUp
				| Code::PageUp
				| Code::ArrowLeft
				| Code::ArrowRight
				| Code::End
				| Code::ArrowDown
				| Code::PageDown => self.move_cursor(c),
				Code::Delete => self.delete_char(),
				Code::Backspace => {
					self.move_cursor(Code::ArrowLeft);
					self.delete_char();
				}
				_ => {}
			}
		}

		if let Code::None = record.printable {
			return;
		}

		if record.alt {
			self.change_color(record.printable);
		} else {
			self.put_char(ev.ascii);
			self.move_cursor(Code::ArrowRight);
		}

		static mut I: usize = 0;

		printkln!("kernel_entry: {}", I);
		unsafe {
			I += 1;
		}
	}
}
