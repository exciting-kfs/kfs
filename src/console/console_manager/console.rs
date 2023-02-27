use super::ascii::{self, constants::*, Ascii, AsciiParser};
use super::cursor::{Cursor, Result as CursorResult};

use crate::collection::WrapQueue;
use crate::driver::vga::text_vga::{self, Attr as VGAAttr, Char as VGAChar, Color};
use crate::input::key_event::{Code, CursorCode};
use crate::printk;

use crate::driver::vga::text_vga::{HEIGHT as WINDOW_HEIGHT, WIDTH as WINDOW_WIDTH, WINDOW_SIZE};
pub const BUFFER_HEIGHT: usize = WINDOW_HEIGHT * 4;
pub const BUFFER_WIDTH: usize = WINDOW_WIDTH;
pub const BUFFER_SIZE: usize = BUFFER_HEIGHT * BUFFER_WIDTH;

type ConsoleCursor = Cursor<WINDOW_HEIGHT, WINDOW_WIDTH>;
type ConsoleBuffer = WrapQueue<VGAChar, BUFFER_SIZE>;

pub trait IConsole {
	fn update(&mut self, ascii: &[u8]);
	fn draw(&self);
}

pub struct Console {
	buf: ConsoleBuffer,
	window_start: usize,
	window_start_backup: usize,
	cursor: ConsoleCursor,
	cursor_backup: ConsoleCursor,
	attr: VGAAttr,
	parser: AsciiParser,
}

impl Console {
	pub fn new() -> Self {
		Self::buffer_reserved(0)
	}

	pub fn buffer_reserved(n: usize) -> Self {
		let mut buf = WrapQueue::from_fn(|_| VGAChar::new(b' '));
		buf.reserve(n);

		Console {
			buf,
			window_start: 0,
			window_start_backup: 0,
			cursor: Cursor::new(),
			cursor_backup: Cursor::new(),
			attr: VGAAttr::default(),
			parser: AsciiParser::new(),
		}
	}

	pub fn put_char(&mut self, ch: u8) {
		let mut window = self
			.buf
			.window_mut(self.window_start, WINDOW_SIZE)
			.expect("buffer overflow");

		let ch = VGAChar::styled(self.attr, ch);
		window[self.cursor.to_idx()] = ch;
	}

	// pub fn put_char_absolute(&mut self, ch: u8, pos: &ConsoleCursor) {
	// 	let ch = VGAChar::styled(self.attr, ch);
	// 	*self.buf.at_mut(pos.y * BUFFER_WIDTH + pos.x).unwrap() = ch;
	// }

	pub fn put_empty_line(&mut self) {
		let ch = VGAChar::styled(self.attr, b' ');
		self.buf.push_n(ch, BUFFER_WIDTH);
	}

	pub fn delete_char(&mut self) {
		self.put_char(b' ');
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

	// TODO: fn form_feed(&mut self) {}

	fn line_feed(&mut self, lines: usize) {
		let minimum_buf_size = self.window_start + WINDOW_SIZE + BUFFER_WIDTH * lines;

		let extend_size = match self.buf.full() {
			true => BUFFER_WIDTH * lines,
			false => minimum_buf_size
				.checked_sub(self.buf.size())
				.unwrap_or_default(),
		};

		if extend_size > 0 {
			self.buf
				.push_n(VGAChar::styled(self.attr, b' '), extend_size);
		}

		self.window_start =
			(self.window_start + BUFFER_WIDTH * lines).min(BUFFER_SIZE - WINDOW_SIZE);
	}

	fn line_up(&mut self, lines: usize) {
		self.window_start = self
			.window_start
			.checked_sub(BUFFER_WIDTH * lines)
			.unwrap_or_default();
	}

	fn line_down(&mut self, lines: usize) {
		self.window_start =
			(self.window_start + BUFFER_WIDTH * lines).min(self.buf.size() - WINDOW_SIZE);
	}

	fn carriage_return(&mut self) {
		self.cursor.move_abs_x(0).unwrap();
	}

	fn cursor_left(&mut self, n: u8) {
		self.cursor.move_rel_partial(0, -(n.max(1) as isize));
	}

	fn cursor_right(&mut self, n: u8) {
		self.cursor.move_rel_partial(0, n.max(1) as isize);
	}

	fn cursor_down(&mut self, n: u8) {
		self.cursor.move_rel_partial(n.max(1) as isize, 0);
	}

	fn cursor_up(&mut self, n: u8) {
		self.cursor.move_rel_partial(-(n.max(1) as isize), 0);
	}

	fn cursor_home(&mut self) {
		self.cursor.move_abs_x(0).unwrap();
	}

	fn cursor_end(&mut self) {
		self.cursor.move_abs_x(BUFFER_WIDTH as isize - 1).unwrap();
	}

	fn cursor_save(&mut self) {
		self.cursor_backup = self.cursor.clone();
		self.window_start_backup = self.window_start;
	}

	fn cursor_restore(&mut self) {
		self.cursor = self.cursor_backup.clone();
		self.window_start = self.window_start_backup;
	}

	fn handle_text(&mut self, ch: u8) {
		if let Ok(_) = self.cursor.check_rel(0, 1) {
			self.put_char(ch);
			self.cursor.move_rel_partial(0, 1);
		}
	}

	fn handle_ctl(&mut self, ctl: u8) {
		// TODO: FF HT VT
		match ctl {
			BS => {
				if let Ok(_) = self.cursor.check_rel(0, -1) {
					self.delete_char();
					self.cursor.move_rel_partial(0, -1);
				}
			}
			CR | LF => {
				if let Err(_) = self.cursor.check_rel(1, 0) {
					self.line_feed(1);
				} else {
					self.cursor.move_rel_partial(1, 0);
				}
				self.carriage_return();
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
			5 => self.line_up(WINDOW_HEIGHT / 2),
			6 => self.line_down(WINDOW_HEIGHT / 2),
			_ => (),
		}
	}

	fn handle_ctlseq(&mut self, param: u8, kind: u8) {
		match kind {
			b'~' => self.handle_key(param),
			b'A' => self.cursor_up(param),
			b'B' => self.cursor_down(param),
			b'C' => self.cursor_right(param),
			b'D' => self.cursor_left(param),
			b'H' => self.cursor_home(),
			b'F' => self.cursor_end(),
			b'm' => self.handle_color(param),
			b's' => self.cursor_save(),
			b'u' => self.cursor_restore(),
			_ => (),
		};
	}
}

impl IConsole for Console {
	fn draw(&self) {
		let window = self
			.buf
			.window(self.window_start, WINDOW_SIZE)
			.expect("buffer overflow");
		text_vga::put_slice_iter(window);
		text_vga::put_cursor(self.cursor.to_idx());
	}

	fn update(&mut self, ascii: &[u8]) {
		for c in ascii {
			self.write(*c);
		}
	}
}
