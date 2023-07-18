use crate::{
	collection,
	console::{ascii_constants::*, Ascii, AsciiParser},
	io::{character::RW as ChRW, ChRead, ChWrite, NoSpace},
};

use core::fmt::{self, Debug, Write};

use core::{arch::asm, slice::from_raw_parts};

type Result<T> = core::result::Result<T, ()>;

type WrapQueue = collection::WrapQueue<u8, 4096>;
type LineBuffer = collection::LineBuffer<64>;

enum State {
	Prompt,
	Sync,
	Normal,
}

pub static mut SHELL: Shell = Shell::new();
pub struct Shell {
	state: State,
	line_buffer: LineBuffer,
	write_queue: WrapQueue,
	parser: AsciiParser,
}

const PROMPT: &[u8] = b"sh=> ";

impl Shell {
	pub const fn new() -> Self {
		Self {
			state: State::Prompt,
			line_buffer: LineBuffer::new(),
			write_queue: WrapQueue::with(0),
			parser: AsciiParser::new(),
		}
	}

	fn write_text(&mut self, ch: u8) {
		self.line_buffer.put_char(ch);
	}

	fn sync_line(&mut self) {
		self.sync_cursor(PROMPT.len() as u8);
		self.write_const(b"\x1b[K");
		for ch in self.line_buffer.as_slice() {
			self.write_queue.push(*ch);
		}
		self.sync_cursor(self.line_buffer.cursor() as u8 + PROMPT.len() as u8);
	}

	fn sync_cursor(&mut self, cursor: u8) {
		write!(self, "\x1b[{}G", cursor).unwrap();
	}

	fn write_ctl(&mut self, c: u8) {
		match c {
			BS => {
				self.line_buffer.backspace();
			}
			CR | LF => {
				self.write_queue.push(b'\n');
				self.execute_line();
				self.write_const(PROMPT);
				self.sync_cursor(PROMPT.len() as u8);
				self.line_buffer.clear();
			}
			_ => (),
		}
	}

	fn write_ctlseq(&mut self, _param: u8, kind: u8) {
		match kind {
			b'C' => {
				if !self.line_buffer.is_cursor_at_end() {
					self.line_buffer.move_cursor_right();
					self.write_parse_result();
				}
			}
			b'D' => {
				if !self.line_buffer.is_cursor_at_begin() {
					self.line_buffer.move_cursor_left();
					self.write_parse_result();
				}
			}
			_ => (),
		}
	}

	fn write_page(&mut self) {
		if let State::Normal = self.state {
			self.state = State::Sync;
			self.write_const(b"\x1b[s");
		}
		self.write_parse_result();
	}

	fn write_const(&mut self, ascii: &[u8]) {
		for c in ascii {
			self.write_queue.push(*c);
		}
	}

	fn write_parse_result(&mut self) {
		while let Some(x) = self.parser.as_mut_buf().pop() {
			self.write_queue.push(x);
		}
	}

	fn builtin_help(&mut self) {
		write!(
			self,
			concat!(
				"sh: minimal debug shell.\n",
				" - help: show this help message.\n",
				" - halt: halt system.\n",
				" - mem: show memory info.\n",
				" - clear: clear output.\n",
			),
		)
		.unwrap();
	}

	fn builtin_clear(&mut self) {
		self.write_queue.push(FF);
	}

	fn builtin_halt(&mut self) {
		unsafe { asm!("hlt") }; // wait what?
	}

	fn execute_builtin<'a, I>(&mut self, kind: Builtin, _args: I)
	where
		I: Iterator<Item = &'a [u8]> + Debug,
	{
		match kind {
			Builtin::Help => self.builtin_help(),
			Builtin::Clear => self.builtin_clear(),
			Builtin::Halt => self.builtin_halt(),
		}
	}

	fn builtin_not_found(&mut self, builtin: &[u8]) {
		write!(
			self,
			concat!(
				"sh: {}: no such command.\n",
				" (try `help` to list available commands.)\n",
			),
			unsafe { core::str::from_utf8_unchecked(builtin) }
		)
		.unwrap();
	}

	fn execute_line(&mut self) {
		let line = self.line_buffer.as_slice();
		// partial self borrowing.
		let line = unsafe { from_raw_parts(line.as_ptr(), line.len()) };

		let mut tokens = line.split(|c| *c == b' ').filter(|elem| elem.len() > 0);

		let token = match tokens.next() {
			Some(x) => x,
			None => return,
		};

		match Builtin::from_slice(token) {
			Some(kind) => self.execute_builtin(kind, tokens),
			None => self.builtin_not_found(token),
		};
	}
}

impl ChRead<u8> for Shell {
	fn read_one(&mut self) -> Option<u8> {
		self.write_queue.pop()
	}
}

impl ChWrite<u8> for Shell {
	fn write_one(&mut self, data: u8) -> core::result::Result<(), NoSpace> {
		if let State::Prompt = self.state {
			self.write_const(PROMPT);
			self.sync_cursor(PROMPT.len() as u8);
			self.state = State::Normal;
		}

		let ascii = match self.parser.parse(data) {
			Some(x) => x,
			None => return Ok(()),
		};

		if let Ascii::CtlSeq(5 | 6, b'~') = ascii {
			self.write_page();
		} else {
			if let State::Sync = self.state {
				self.write_const(b"\x1b[u");
				self.state = State::Normal;
			}
			match ascii {
				Ascii::Text(c) => self.write_text(c),
				Ascii::Control(c) => self.write_ctl(c),
				Ascii::CtlSeq(param, kind) => self.write_ctlseq(param, kind),
			}
			self.sync_line();
		}

		self.parser.reset();
		Ok(())
	}
}

impl ChRW<u8, u8> for Shell {}

impl Write for Shell {
	fn write_str(&mut self, s: &str) -> fmt::Result {
		for byte in s.bytes() {
			self.write_queue.push(byte);
		}
		Ok(())
	}
}

enum Builtin {
	Help,
	Halt,
	Clear,
}

impl Builtin {
	pub fn from_slice(slice: &[u8]) -> Option<Self> {
		let value = match slice {
			b"help" => Self::Help,
			b"halt" => Self::Halt,
			b"clear" => Self::Clear,
			_ => return None,
		};

		Some(value)
	}
}
