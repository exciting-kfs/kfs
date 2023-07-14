//! Translate key code to ascii code
//! and do basic line discipline.

use alloc::collections::VecDeque;
use alloc::sync::Arc;
use kfs_macro::context;

use crate::config::CONSOLE_COUNTS;
use crate::console::console_manager::console::SyncConsole;
use crate::console::CONSOLE_MANAGER;
use crate::file::FileOps;
use crate::input::key_event::*;
use crate::input::keyboard::KEYBOARD;
use crate::io::{BlkRead, BlkWrite, ChRead, ChWrite, NoSpace};
use crate::sync::locked::Locked;

#[rustfmt::skip]
static ALPHA_LOWER: [u8; 26] = [
	b'a', b'b', b'c', b'd', b'e',
	b'f', b'g', b'h', b'i', b'j',
	b'k', b'l', b'm', b'n', b'o',
	b'p', b'q', b'r', b's', b't',
	b'u', b'v', b'w', b'x', b'y',
	b'z',
];

#[rustfmt::skip]
static ALPHA_UPPER: [u8; 26] = [
	b'A', b'B', b'C', b'D', b'E',
	b'F', b'G', b'H', b'I', b'J',
	b'K', b'L', b'M', b'n', b'O',
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
	console: SyncConsole,
	line_buffer: VecDeque<u8>,
	record: VecDeque<u8>,
}

impl TTY {
	pub const fn new(console: SyncConsole, echo: bool, icanon: bool) -> Self {
		Self {
			icanon,
			echo,
			console,
			line_buffer: VecDeque::new(),
			record: VecDeque::new(),
		}
	}

	/// echo back given characters.
	/// if character is non-printable,
	///   then escape with caret-notation to make it printable.
	///
	/// # caret-notation
	///
	/// represent non printable ascii `(0..=31, 127)` with `^('@' + ascii)` and MSB(bit 8) cleared.
	///
	/// ## examples
	/// - 0  (NUL) => `^@`
	/// - 1b (ESC) => `^[`
	/// - 7f (DEL) => `^?`
	fn do_echo(&mut self, mut c: u8) -> Result<(), NoSpace> {
		let mut console = self.console.lock();
		if !is_printable(c) {
			console.write_one(b'^')?;
			c = (b'@' + c) & !(1 << 7);
		}
		console.write_one(c)
	}
}

/// from keyboard
impl ChWrite<Code> for TTY {
	// irq_disabled
	fn write_one(&mut self, data: Code) -> Result<(), NoSpace> {
		let buf = convert(data);

		if let None = buf {
			return Ok(());
		}

		for c in buf.unwrap().iter().map(|b| *b) {
			if self.icanon {
				self.line_buffer.push_back(c);
			} else {
				self.record.push_back(c); // TODO alloc Error
			}

			if self.echo {
				self.do_echo(c)?
			}
		}

		if data == Code::Enter {
			self.record.append(&mut self.line_buffer);
		}

		Ok(())
	}
}

/// from process
impl ChWrite<u8> for TTY {
	fn write_one(&mut self, data: u8) -> Result<(), NoSpace> {
		self.console.lock().write_one(data)
	}
}

/// to process
impl ChRead<u8> for TTY {
	fn read_one(&mut self) -> Option<u8> {
		self.record.pop_front()
	}
}

impl BlkWrite for TTY {}
impl BlkRead for TTY {}

impl FileOps for Locked<TTY> {
	#[context(irq_disabled)]
	fn read(&self, buf: &mut [u8]) -> usize {
		// dead lock
		self.lock().read(buf)
	}

	#[context(irq_disabled)]
	fn write(&self, buf: &[u8]) -> usize {
		self.lock().write(buf)
	}
}

pub type SyncTTY = Arc<Locked<TTY>>;

pub fn open(id: usize) -> Option<SyncTTY> {
	if id >= CONSOLE_COUNTS {
		None
	} else {
		Some(unsafe { CONSOLE_MANAGER.assume_init_ref().get_tty(id) })
	}
}

fn is_printable(c: u8) -> bool {
	b' ' <= c && c <= b'~'
}
