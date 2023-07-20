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

	pub const FG_BLACK: u8 = 30;
	pub const FG_RED: u8 = 31;
	pub const FG_GREEN: u8 = 32;
	pub const FG_BROWN: u8 = 33;
	pub const FG_BLUE: u8 = 34;
	pub const FG_MAGENTA: u8 = 35;
	pub const FG_CYAN: u8 = 36;
	pub const FG_WHITE: u8 = 37;
	pub const FG_DEFAULT: u8 = 39;

	pub const BG_BLACK: u8 = 40;
	pub const BG_RED: u8 = 41;
	pub const BG_GREEN: u8 = 42;
	pub const BG_BROWN: u8 = 43;
	pub const BG_BLUE: u8 = 44;
	pub const BG_MAGENTA: u8 = 45;
	pub const BG_CYAN: u8 = 46;
	pub const BG_WHITE: u8 = 47;
	pub const BG_DEFAULT: u8 = 49;
}

use constants::*;

use crate::collection::WrapQueue;

#[derive(Debug)]
pub enum Ascii {
	Text(u8),
	Control(u8),
	CtlSeq(u8, u8),
}

enum State {
	Start,
	Escape,
	Csi,
	Param,
}
pub struct AsciiParser {
	state: State,
	buf: WrapQueue<u8, 16>,
	param: u8,
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
	pub const fn new() -> Self {
		Self {
			state: State::Start,
			buf: WrapQueue::with(0),
			param: 0,
		}
	}

	/// continue or start parsing process.
	/// invalid sequence is silently ignored.
	///
	/// Even after `Some(x)` is returned, internal state is not automatically reset.
	/// So, in order to keep parsing properly,
	/// you have to call `reset()` after each `Some(x)` return.
	pub fn parse(&mut self, c: u8) -> Option<Ascii> {
		if self.buf.full() {
			self.reset();
			return None;
		}
		self.buf.push(c);
		match self.state {
			State::Start => self.parse_start(c),
			State::Escape => self.parse_escape(c),
			State::Csi => self.parse_csi(c),
			State::Param => self.parse_param(c),
		}
	}

	/// reset internal buffer and state.
	pub fn reset(&mut self) {
		self.buf.reset();
		self.state = State::Start;
		self.param = 0;
	}

	/// inspect internal buffer
	pub fn as_mut_buf(&mut self) -> &mut WrapQueue<u8, 16> {
		&mut self.buf
	}

	fn parse_start(&mut self, c: u8) -> Option<Ascii> {
		if b' ' <= c && c <= b'~' {
			Some(Ascii::Text(c))
		} else if c == ESC {
			self.state = State::Escape;
			None
		} else {
			Some(Ascii::Control(c))
		}
	}

	fn parse_escape(&mut self, c: u8) -> Option<Ascii> {
		if c == b'[' {
			self.state = State::Csi;
			None
		} else {
			self.handle_invaild(c)
		}
	}

	fn parse_csi(&mut self, c: u8) -> Option<Ascii> {
		if c.is_ascii_digit() {
			let value = self
				.param
				.checked_mul(10)
				.and_then(|x| x.checked_add(c - b'0'));
			match value {
				Some(x) => self.param = x,
				None => return self.handle_invaild(c),
			}
			None
		} else {
			self.state = State::Param;
			self.parse_param(c)
		}
	}

	fn parse_param(&mut self, c: u8) -> Option<Ascii> {
		if is_ctlseq_terminator(c) {
			Some(Ascii::CtlSeq(self.param, c))
		} else {
			self.handle_invaild(c)
		}
	}

	fn handle_invaild(&mut self, c: u8) -> Option<Ascii> {
		self.reset();
		self.parse(c)
	}
}
