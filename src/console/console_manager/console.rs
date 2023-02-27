use super::ascii::{self, constants::*, Ascii, AsciiParser};
use super::cursor::{Cursor, MoveResult};

use crate::collection::WrapQueue;
use crate::driver::vga::text_vga::{self, Attr as VGAAttr, Char as VGAChar, Color};
use crate::input::key_event::{Code, CursorCode};
use crate::printkln;

pub const BUFFER_HEIGHT: usize = 100;
pub const BUFFER_WIDTH: usize = 80;

pub const BUFFER_SIZE: usize = BUFFER_HEIGHT * BUFFER_WIDTH;

pub trait IConsole {
	fn update(&mut self, ascii: &[u8]);
	fn draw(&self);
}

pub struct Console {
	buf: WrapQueue<VGAChar, BUFFER_SIZE>,
	window_start: usize,
	cursor: Cursor,
	attr: VGAAttr,
	parser: AsciiParser,
}

impl Console {
	pub fn new() -> Self {
		Self::buffer_reserved(0)
	}

	pub fn buffer_reserved(n: usize) -> Self {
		let mut buf = WrapQueue::from_fn(|_| VGAChar::new(b'\0'));
		buf.reserve(n);

		Console {
			buf,
			window_start: 0,
			cursor: Cursor::new(0, 0),
			attr: VGAAttr::default(),
			parser: AsciiParser::new(),
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

	fn set_fg_color(&mut self, color: Color) {
		self.attr.set_fg(color);
	}

	fn set_bg_color(&mut self, color: Color) {
		self.attr.set_bg(color);
	}

	fn reset_fg_color(&mut self) {
		self.attr.reset_fg();
	}

	fn reset_bg_color(&mut self) {
		self.attr.reset_bg();
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

	fn handle_text(&mut self, ch: u8) {
		self.put_char(ch);
		self.move_cursor(CursorCode::Right);
	}

	fn handle_ctl(&mut self, ctl: u8) {
		// TODO: FF HT VT
		match ctl {
			BS | DEL => {
				self.move_cursor(CursorCode::Left);
				self.delete_char();
			}
			CR | LF => {
				self.move_cursor(CursorCode::Home);
				self.move_cursor(CursorCode::Down);
			}
			_ => (),
		}
	}

	fn handle_color(&mut self, color: u8) {
		match color {
			FG_BLACK => self.set_fg_color(Color::Black),
			FG_RED => self.set_fg_color(Color::Red),
			FG_GREEN => self.set_fg_color(Color::Green),
			FG_BROWN => self.set_fg_color(Color::Brown),
			FG_BLUE => self.set_fg_color(Color::Blue),
			FG_MAGENTA => self.set_fg_color(Color::Magenta),
			FG_CYAN => self.set_fg_color(Color::Cyan),
			FG_WHITE => self.set_fg_color(Color::White),
			FG_DEFAULT => self.reset_fg_color(),
			BG_BLACK => self.set_bg_color(Color::Black),
			BG_RED => self.set_bg_color(Color::Red),
			BG_GREEN => self.set_bg_color(Color::Green),
			BG_BROWN => self.set_bg_color(Color::Brown),
			BG_BLUE => self.set_bg_color(Color::Blue),
			BG_MAGENTA => self.set_bg_color(Color::Magenta),
			BG_CYAN => self.set_bg_color(Color::Cyan),
			BG_WHITE => self.set_bg_color(Color::White),
			BG_DEFAULT => self.reset_bg_color(),
			_ => (),
		}
	}

	pub fn write(&mut self, c: u8) {
		if let Some(v) = self.parser.parse(c) {
			match v {
				Ascii::Text(ch) => self.handle_text(ch),
				Ascii::Control(ctl) => self.handle_ctl(ctl),
				Ascii::CtlSeq(p, k) => self.handle_ctlseq(p, k),
			}
			self.parser.reset();
		}
	}

	fn handle_key(&mut self, key: u8) {
		match key {
			3 => self.delete_char(),
			5 => self.move_cursor(CursorCode::PageUp),
			6 => self.move_cursor(CursorCode::PageDown),
			_ => (),
		}
	}

	fn handle_ctlseq(&mut self, param: u8, kind: u8) {
		match kind {
			b'~' => self.handle_key(param),
			b'A' => self.move_cursor(CursorCode::Up),
			b'B' => self.move_cursor(CursorCode::Down),
			b'C' => self.move_cursor(CursorCode::Right),
			b'D' => self.move_cursor(CursorCode::Left),
			b'H' => self.move_cursor(CursorCode::Home),
			b'F' => self.move_cursor(CursorCode::End),
			b'm' => self.handle_color(param),
			_ => (),
		}
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
			self.write(*c);
		}
	}
}
