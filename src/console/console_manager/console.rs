use super::cursor::{Cursor, MoveResult};
use super::key_record::KeyRecord;

use crate::collection::WrapQueue;
use crate::driver::vga::text_vga::{self, Attr as VGAAttr, Char as VGAChar};
use crate::input::{
	key_event::{Code, Key, KeyState},
	keyboard::KeyboardEvent,
};
use crate::printkln;

pub const BUFFER_HEIGHT: usize = 100;
pub const BUFFER_WIDTH: usize = 80;

pub const BUFFER_SIZE: usize = BUFFER_HEIGHT * BUFFER_WIDTH;

pub trait IConsole {
	fn update(&mut self, ev: &KeyboardEvent, record: &KeyRecord);
	fn draw(&self);
}

pub struct Console {
	buf: WrapQueue<VGAChar, BUFFER_SIZE>,
	window_start: usize,
	cursor: Cursor,
	attr: VGAAttr,
}

impl Console {
	pub fn new() -> Self {
		let buf = WrapQueue::from_fn(|_| VGAChar::new(b'\0'));

		Console {
			buf,
			window_start: 0,
			cursor: Cursor::new(0, 0),
			attr: VGAAttr::default(),
		}
	}

	pub fn buffer_reserved(n: usize) -> Self {
		let mut buf = WrapQueue::from_fn(|_| VGAChar::new(b'\0'));
		buf.reserve(n);

		Console {
			buf,
			window_start: 0,
			cursor: Cursor::new(0, 0),
			attr: VGAAttr::default(),
		}
	}

	pub fn put_char(&mut self, ch: u8) {
		let mut window = self
			.buf
			.window_mut(self.window_start, text_vga::WIDTH * text_vga::HEIGHT)
			.expect("buffer overflow");

		let ch = VGAChar::styled(self.attr, ch);

		window[self.cursor.to_idx()] = ch;
	}

	pub fn put_char_absolute(&mut self, ch: u8, pos: &Cursor) {
		let ch = VGAChar::styled(self.attr, ch);
		*self.buf.at_mut(pos.y * BUFFER_WIDTH + pos.x).unwrap() = ch;
	}

	pub fn put_empty_line(&mut self) {
		let ch = VGAChar::styled(self.attr, b'\0');
		self.buf.push_n(ch, BUFFER_WIDTH);
	}

	pub fn delete_char(&mut self) {
		self.put_char(b'\0');
	}

	pub fn change_color(&mut self, color: Code) {
		self.attr = VGAAttr::form_u8(color as u8);

		for i in 0..self.buf.size() {
			let ch = self.buf.at_mut(i).unwrap();
			*ch = VGAChar::styled(self.attr, (ch.0 & 0xff) as u8);
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

		if let MoveResult::AdjustWindowStart(dy) = res {
			self.adjust_window_start(dy)
		}
	}

	pub fn sync_window_start(&mut self, y: usize) {
		self.window_start =
			usize::checked_sub(text_vga::WIDTH * y, text_vga::WINDOW_SIZE).unwrap_or_default();
	}

	pub fn adjust_window_start(&mut self, dy: isize) {
		let orig = self.window_start as isize;
		let delta = dy * text_vga::WIDTH as isize;
		let window_size = (text_vga::HEIGHT * text_vga::WIDTH) as isize;
		let max_window_start = (BUFFER_SIZE as isize - window_size) as isize;

		self.window_start = (orig + delta).clamp(0, max_window_start) as usize;

		let overflow = (self.window_start as isize + window_size) - self.buf.size() as isize;
		(0..overflow).for_each(|_| self.buf.push(text_vga::Char::styled(self.attr, b'\0')));
	}
}

impl IConsole for Console {
	fn draw(&self) {
		let window = self
			.buf
			.window(self.window_start, text_vga::WIDTH * text_vga::HEIGHT)
			.expect("buffer overflow");
		text_vga::put_slice_iter(window);
		text_vga::put_cursor(self.cursor.y, self.cursor.x);
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

		// FIXME hmm....
		static mut I: usize = 0;

		printkln!("kernel_entry: {}", I);
		unsafe {
			I += 1;
		}
	}
}
