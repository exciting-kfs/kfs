//! Translate key code to ascii code
//! and do basic line discipline.

use crate::input::key_event::*;
use crate::input::keyboard::KEYBOARD;

#[rustfmt::skip]
static ALPHA_LOWER: [u8; 26] = [
	b'a', b'b', b'c', b'd', b'e',
	b'f', b'g', b'h', b'i', b'j',
	b'k', b'l', b'n', b'm', b'o',
	b'p', b'q', b'r', b's', b't',
	b'u', b'v', b'w', b'x', b'y',
	b'z',
];

#[rustfmt::skip]
static ALPHA_UPPER: [u8; 26] = [
	b'A', b'B', b'C', b'D', b'E',
	b'F', b'G', b'H', b'I', b'J',
	b'K', b'L', b'N', b'M', b'O',
	b'P', b'Q', b'R', b'S', b'T',
	b'U', b'V', b'W', b'X', b'Y',
	b'Z',
];

#[rustfmt::skip]
static SYMBOL_PLAIN: [u8; 22] = [
	b'0',	b'1',	b'2',	b'3',	b'4',
	b'5',	b'6',	b'7',	b'8',	b'9',
	b'`',	b'-',	b'=',	b'[',	b']',
	b'\\',	b';',	b'\'',	b',',	b'.',
	b'/',	b' ',
];

#[rustfmt::skip]
static SYMBOL_SHIFT: [u8; 22] = [
	b')',	b'!',	b'@',	b'#',	b'$',
	b'%',	b'^',	b'&',	b'*',	b'(',
	b'~',	b'_',	b'+',	b'{',	b'}',
	b'|',	b':',	b'"',	b'<',	b'>',
	b'?',	b' ',
];

#[rustfmt::skip]
static FUNCTION: [&[u8]; 12] = [
	b"\x1b[11~", b"\x1b[12~", b"\x1b[13~", b"\x1b[14~",
	b"\x1b[15~", b"\x1b[17~", b"\x1b[18~", b"\x1b[19~",
	b"\x1b[20~", b"\x1b[21~", b"\x1b[23~", b"\x1b[24~",
];

// TODO: implement KEYPAD_NUMLOCK
#[rustfmt::skip]
static KEYPAD_PLAIN: [u8; 16] = [
	b'0', b'1', b'2', b'3',
	b'4', b'5', b'6', b'7',
	b'8', b'9', b'-', b'+',
	b'.', b'/', b'*', b'\n',
];

#[rustfmt::skip]
static CURSOR: [&[u8]; 8] = [
	b"\x1b[A",	b"\x1b[B",
	b"\x1b[D",	b"\x1b[C",
	b"\x1b[5~",	b"\x1b[6~",
	b"\x1b[H",	b"\x1b[F",
];

fn convert(code: Code) -> Option<&'static [u8]> {
	match code.identify() {
		KeyKind::Alpha(code) => convert_alpha(code),
		KeyKind::Symbol(code) => convert_symbol(code),
		KeyKind::Function(code) => convert_function(code),
		KeyKind::Keypad(code) => convert_keypad(code),
		KeyKind::Cursor(code) => convert_cursor(code),
		KeyKind::Control(code) => convert_control(code),
		KeyKind::Modifier(..) => None,
		KeyKind::Toggle(..) => None,
	}
}

fn convert_alpha(code: AlphaCode) -> Option<&'static [u8]> {
	let kbd = unsafe { &KEYBOARD };
	let table = match kbd.shift_pressed() ^ kbd.pressed(Code::Capslock) {
		true => &ALPHA_UPPER,
		false => &ALPHA_LOWER,
	};

	let idx = code.index() as usize;
	Some(&table[idx..=idx])
}

fn convert_symbol(code: SymbolCode) -> Option<&'static [u8]> {
	let kbd = unsafe { &KEYBOARD };
	let table = match kbd.shift_pressed() {
		true => &SYMBOL_SHIFT,
		false => &SYMBOL_PLAIN,
	};

	let idx = code.index() as usize;
	Some(&table[idx..=idx])
}

fn convert_function(code: FunctionCode) -> Option<&'static [u8]> {
	Some(&FUNCTION[code.index() as usize])
}

fn convert_cursor(code: CursorCode) -> Option<&'static [u8]> {
	Some(&CURSOR[code.index() as usize])
}

fn convert_control(code: ControlCode) -> Option<&'static [u8]> {
	match code {
		ControlCode::Backspace => Some(b"\x08"),
		ControlCode::Delete => Some(b"\x1b[3~"),
		ControlCode::Tab => Some(b"\t"),
		ControlCode::Enter => Some(b"\n"),
		ControlCode::Escape => Some(b"\x1b"),
		_ => None,
	}
}

fn convert_keypad(code: KeypadCode) -> Option<&'static [u8]> {
	let idx = code.index() as usize;
	Some(&KEYPAD_PLAIN[idx..=idx])
}

pub struct TTY {
	icanon: bool,
	echo: bool,
	buf: Option<&'static [u8]>,
	cursor: usize,
	carrot: bool,
}

impl TTY {
	pub const fn new(echo: bool) -> Self {
		Self {
			icanon: false,
			echo,
			buf: None,
			cursor: 0,
			carrot: false,
		}
	}

	pub fn write(&mut self, code: Code) {
		self.buf = convert(code);
	}

	pub fn read_echo(&mut self) -> Option<u8> {
		if !self.echo {
			return None;
		}

		let buf = self.buf?;

		if buf.len() <= self.cursor {
			self.cursor = 0;
			return None;
		}

		let c = buf[self.cursor];

		if b' ' <= c && c <= b'~' {
			self.cursor += 1;
			return Some(c);
		}

		if !self.carrot {
			self.carrot = true;
			return Some(b'^');
		} else {
			self.cursor += 1;
			self.carrot = false;
			return Some(b'@' + c & !(1 << 7)); // TODO 주석으로 설명이 필요할 듯.
		}
	}

	pub fn read_task(&mut self) -> Option<u8> {
		let buf = self.buf?;

		if buf.len() <= self.cursor {
			self.clear();
			return None;
		}

		let c = buf[self.cursor];
		self.cursor += 1;

		Some(c)
	}

	fn clear(&mut self) {
		self.buf = None;
		self.carrot = false;
		self.cursor = 0;
	}
}
