use super::cursor::{Cursor, MoveResult};

use crate::collection::WrapQueue;
use crate::driver::vga::text_vga::{self, Attr as VGAAttr, Char as VGAChar};
use crate::input::key_event::{Code, CursorCode};

pub const BUFFER_HEIGHT: usize = 100;
pub const BUFFER_WIDTH: usize = 80;

pub const BUFFER_SIZE: usize = BUFFER_HEIGHT * BUFFER_WIDTH;

pub trait IConsole {
	fn update(&mut self, ascii: &[u8]);
	fn draw(&self);
}

enum AsciiEscape {
	Start,
	Escape,

	Function,
	Csi,

	PageUp,
	PageDown,
	Delete,
}

pub struct Console {
	state: AsciiEscape,
	buf: WrapQueue<VGAChar, BUFFER_SIZE>,
	window_start: usize,
	cursor: Cursor,
	attr: VGAAttr,
}

impl Console {
	pub fn new() -> Self {
		let buf = WrapQueue::from_fn(|_| VGAChar::new(b'\0'));

		Console {
			state: AsciiEscape::Start,
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
			state: AsciiEscape::Start,
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

	pub fn move_cursor(&mut self, code: CursorCode) {
		let home = -(self.cursor.x as isize);
		let end = (BUFFER_WIDTH - self.cursor.x - 1) as isize;
		let up = -(text_vga::HEIGHT as isize) + 1;
		let down = text_vga::HEIGHT as isize - 1;

		let res = match code {
			CursorCode::Up => self.cursor.relative_move(-1, 0),
			CursorCode::Down => self.cursor.relative_move(1, 0),
			CursorCode::Left => self.cursor.relative_move(0, -1),
			CursorCode::Right => self.cursor.relative_move(0, 1),
			CursorCode::PageUp => self.cursor.relative_move(up, 0),
			CursorCode::PageDown => self.cursor.relative_move(down, 0),
			CursorCode::Home => self.cursor.relative_move(0, home),
			CursorCode::End => self.cursor.relative_move(0, end),
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

	// yeah.. i know... i will refactor it ASAP. for now it's just for test.

	fn parse_start(&mut self, c: u8) {
		if c.is_ascii_graphic() {
			self.put_char(c);
			self.move_cursor(CursorCode::Right);
			return;
		}

		if c == b'\n' {
			self.move_cursor(CursorCode::Home);
			self.move_cursor(CursorCode::Down);
			return;
		}

		if c == b'\x7f' {
			self.delete_char();
			self.move_cursor(CursorCode::Left);
			return;
		}

		if c == b'\x1b' {
			self.state = AsciiEscape::Escape;
			return;
		}
	}

	fn parse_esc(&mut self, c: u8) {
		if c == b'[' {
			self.state = AsciiEscape::Csi;
			return;
		}

		if c == b'O' {
			self.state = AsciiEscape::Function;
			return;
		}

		// unknown escape sequence
		self.state = AsciiEscape::Start;
	}

	fn dumb_puti(&mut self, n: u8) {
		if n == 0 {
			return;
		}

		let c = (n % 10) + b'0';

		self.dumb_puti(n / 10);

		self.put_char(c);
		self.move_cursor(CursorCode::Right);
	}

	fn parse_fn(&mut self, c: u8) {
		if b'P' <= c && c <= b'[' {
			let offset = c - b'P' + 1;

			self.put_char(b'F');
			self.move_cursor(CursorCode::Right);

			self.dumb_puti(offset);
		}

		self.state = AsciiEscape::Start;
	}

	fn parse_csi(&mut self, c: u8) {
		if c == b'A' {
			self.move_cursor(CursorCode::Up);
			self.state = AsciiEscape::Start;
			return;
		}

		if c == b'B' {
			self.move_cursor(CursorCode::Down);
			self.state = AsciiEscape::Start;
			return;
		}

		if c == b'C' {
			self.move_cursor(CursorCode::Right);
			self.state = AsciiEscape::Start;
			return;
		}

		if c == b'D' {
			self.move_cursor(CursorCode::Left);
			self.state = AsciiEscape::Start;
			return;
		}

		if c == b'H' {
			self.move_cursor(CursorCode::Home);
			self.state = AsciiEscape::Start;
			return;
		}

		if c == b'F' {
			self.move_cursor(CursorCode::End);
			self.state = AsciiEscape::Start;
			return;
		}

		if c == b'5' {
			self.state = AsciiEscape::PageUp;
			return;
		}

		if c == b'6' {
			self.state = AsciiEscape::PageDown;
			return;
		}

		if c == b'3' {
			self.state = AsciiEscape::Delete;
			return;
		}

		self.state = AsciiEscape::Start;
	}

	fn parse_pgdn(&mut self, c: u8) {
		if c == b'~' {
			self.move_cursor(CursorCode::PageDown);
		}
		self.state = AsciiEscape::Start;
	}

	fn parse_pgup(&mut self, c: u8) {
		if c == b'~' {
			self.move_cursor(CursorCode::PageUp);
		}
		self.state = AsciiEscape::Start;
	}

	fn parse_del(&mut self, c: u8) {
		if c == b'~' {
			self.delete_char();
		}
		self.state = AsciiEscape::Start;
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

	fn update(&mut self, ascii: &[u8]) {
		for c in ascii {
			match self.state {
				AsciiEscape::Start => self.parse_start(*c),
				AsciiEscape::Escape => self.parse_esc(*c),
				AsciiEscape::Function => self.parse_fn(*c),
				AsciiEscape::Csi => self.parse_csi(*c),
				AsciiEscape::PageUp => self.parse_pgup(*c),
				AsciiEscape::PageDown => self.parse_pgdn(*c),
				AsciiEscape::Delete => self.parse_del(*c),
			}
		}
	}
}
