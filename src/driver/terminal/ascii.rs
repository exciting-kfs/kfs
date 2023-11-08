pub mod constants {
	pub const ESC: u8 = b'\x1b';

	pub const ETX: u8 = b'\x03'; // ^C
	pub const EOF: u8 = b'\x04'; // ^D (EOT)
	pub const NAK: u8 = b'\x15'; // ^U
	pub const FS: u8 = b'\x1c'; // ^\

	pub const BS: u8 = b'\x08';
	pub const HT: u8 = b'\x09';
	pub const LF: u8 = b'\x0a';
	pub const VT: u8 = b'\x0b';
	pub const FF: u8 = b'\x0c';
	pub const CR: u8 = b'\x0d';
	pub const DEL: u8 = b'\x7f';

	pub const FG_BLACK: u16 = 30;
	pub const FG_RED: u16 = 31;
	pub const FG_GREEN: u16 = 32;
	pub const FG_BROWN: u16 = 33;
	pub const FG_BLUE: u16 = 34;
	pub const FG_MAGENTA: u16 = 35;
	pub const FG_CYAN: u16 = 36;
	pub const FG_WHITE: u16 = 37;
	pub const FG_DEFAULT: u16 = 39;

	pub const BG_BLACK: u16 = 40;
	pub const BG_RED: u16 = 41;
	pub const BG_GREEN: u16 = 42;
	pub const BG_BROWN: u16 = 43;
	pub const BG_BLUE: u16 = 44;
	pub const BG_MAGENTA: u16 = 45;
	pub const BG_CYAN: u16 = 46;
	pub const BG_WHITE: u16 = 47;
	pub const BG_DEFAULT: u16 = 49;

	pub const RESET_COLOR: u16 = 0;
}

use core::mem;

use alloc::vec::Vec;
use constants::*;

#[derive(Debug)]
pub enum Ascii {
	Text(u8),
	Control(u8),
	CtlSeq(u8, Vec<u16>),
}

enum State {
	Start,
	Escape,
	Param { index: u8 },
}

pub struct AsciiParser {
	state: State,
	buf: [u8; 5],
	params: Vec<u16>,
}

impl Default for AsciiParser {
	fn default() -> Self {
		AsciiParser::new()
	}
}

fn is_ctlseq_terminator(c: u8) -> bool {
	0x40 <= c && c <= 0x7f
}

/// very simple ascii escape sequence parser.
/// this only recognizes
///  - Text: normal ascii characters
///  - Control: c0 or maybe 8bit c1
///  - CtlSeq: escape sequence started by `CSI`("\x1b[") with single parameter.
impl AsciiParser {
	pub fn new() -> Self {
		Self {
			state: State::Start,
			buf: [0; 5],
			params: Vec::new(),
		}
	}

	/// continue or start parsing process.
	/// invalid sequence is silently ignored.
	pub fn parse(&mut self, c: u8) -> Option<Ascii> {
		use State::*;
		match self.state {
			Start => self.parse_start(c),
			Escape => self.parse_escape(c),
			Param { index } => self.parse_csi(index, c),
		}
	}

	fn parse_error(&mut self) -> (State, Option<Ascii>) {
		self.state = State::Start;
		self.params = Vec::new();

		(State::Start, None)
	}

	fn parse_start(&mut self, c: u8) -> Option<Ascii> {
		let (next_state, ret) = match c {
			ESC => (State::Escape, None),
			b' '..=b'~' => (State::Start, Some(Ascii::Text(c))),
			_ => (State::Start, Some(Ascii::Control(c))),
		};

		self.state = next_state;

		ret
	}

	fn parse_escape(&mut self, c: u8) -> Option<Ascii> {
		let (next_state, ret) = match c {
			b'[' => (State::Param { index: 0 }, None),
			_ => (State::Start, None),
		};

		self.state = next_state;

		ret
	}

	fn parse_csi(&mut self, index: u8, c: u8) -> Option<Ascii> {
		let (next_state, ret) = match c {
			b'0'..=b'9' => self.parse_param(index, c),
			b';' => (self.parse_param_sep(index), None),
			x if is_ctlseq_terminator(x) => self.parse_ctlseq_terminator(index, c),
			_ => self.parse_error(),
		};

		self.state = next_state;

		ret
	}

	fn parse_ctlseq_terminator(&mut self, index: u8, c: u8) -> (State, Option<Ascii>) {
		if (index as usize) < self.buf.len() {
			self.parse_param_sep(index);
		} else {
			return self.parse_error();
		}

		(
			State::Start,
			Some(Ascii::CtlSeq(c, mem::take(&mut self.params))),
		)
	}

	fn parse_param(&mut self, index: u8, c: u8) -> (State, Option<Ascii>) {
		if index as usize >= self.buf.len() {
			return self.parse_error();
		}

		self.buf[index as usize] = c;

		(State::Param { index: index + 1 }, None)
	}

	fn parse_param_sep(&mut self, index: u8) -> State {
		use core::str;

		let string = str::from_utf8(&self.buf[0..index as usize]).unwrap();
		let param = match string.len() {
			0 => Ok(0),
			_ => string.parse(),
		};

		match param {
			Ok(x) => {
				self.params.push(x);
				State::Param { index: 0 }
			}
			Err(_) => self.parse_error().0,
		}
	}
}
