use crate::{
	collection::WrapQueue,
	console::{constants::*, Ascii, AsciiParser},
	io::character::{Read as ChRead, Write as ChWrite, RW as ChRW},
	BOOT_INFO,
};

use core::fmt::{self, Write};

use core::{arch::asm, slice::from_raw_parts};

pub struct LineBuffer<const CAP: usize> {
	buf: [u8; CAP],
	cursor: usize,
	tail: usize,
}

type Result<T> = core::result::Result<T, ()>;

impl<const CAP: usize> LineBuffer<CAP> {
	pub const fn new() -> Self {
		Self {
			buf: [0; CAP],
			tail: 0,
			cursor: 0,
		}
	}

	pub fn full(&self) -> bool {
		self.tail == CAP
	}

	pub fn size(&self) -> usize {
		self.tail
	}

	pub fn empty(&self) -> bool {
		self.tail == 0
	}

	pub fn clear(&mut self) {
		self.tail = 0;
		self.cursor = 0;
	}

	pub fn shift_chars(&mut self, from: usize, left: bool) {
		if left {
			for i in from..self.tail {
				self.buf[i - 1] = self.buf[i];
			}
		} else {
			for i in (from..self.tail).rev() {
				self.buf[i + 1] = self.buf[i];
			}
		}
	}

	pub fn putc(&mut self, c: u8) {
		if self.full() {
			return;
		}

		self.shift_chars(self.cursor, false);
		self.buf[self.cursor] = c;
		self.tail += 1;
		self.cursor += 1;
	}

	pub fn delc(&mut self, left: bool) {
		if self.empty() {
			return;
		}

		if left && self.cursor > 0 {
			self.shift_chars(self.cursor, true);
			self.cursor -= 1;
			self.tail -= 1;
		} else if !left && self.cursor < self.tail {
			self.shift_chars(self.cursor + 1, true);
			self.tail -= 1;
		}
	}

	pub fn cursor_left(&mut self) -> isize {
		if self.cursor == 0 {
			return 0;
		} else {
			self.cursor -= 1;
			return -1;
		}
	}

	pub fn cursor_right(&mut self) -> isize {
		if self.cursor == self.tail {
			return 0;
		} else {
			self.cursor += 1;
			return 1;
		}
	}

	pub fn cursor_head(&mut self) -> isize {
		let ret = -(self.cursor as isize);

		self.cursor = 0;

		ret
	}

	pub fn cursor_tail(&mut self) -> isize {
		let ret = self.tail - self.cursor;

		self.cursor = self.tail;

		ret as isize
	}

	pub fn as_slice(&self) -> &[u8] {
		&self.buf[..self.tail]
	}
}

enum State {
	Prompt,
	Sync,
	Normal,
}

type Buffer = WrapQueue<u8, 4096>;

pub static mut SHELL: Shell = Shell::new();
pub struct Shell {
	state: State,
	current_line: usize,
	line_buffer: LineBuffer<64>,
	write_queue: Buffer,
	parser: AsciiParser,
}

const PROMPT: &[u8] = b"sh=> ";

impl Shell {
	pub const fn new() -> Self {
		Self {
			state: State::Prompt,
			current_line: 0,
			line_buffer: LineBuffer::new(),
			write_queue: Buffer::with(0),
			parser: AsciiParser::new(),
		}
	}

	fn write_text(&mut self, ch: u8) {
		self.line_buffer.putc(ch);
	}

	fn sync_line(&mut self) {
		self.sync_cursor(PROMPT.len() as u8);
		self.write_const(b"\x1b[J");
		for ch in self.line_buffer.as_slice() {
			self.write_queue.push(*ch);
		}
		self.sync_cursor(self.line_buffer.cursor as u8 + PROMPT.len() as u8);
	}

	fn sync_cursor(&mut self, cursor: u8) {
		write!(self, "\x1b[{}G", cursor).unwrap();
	}

	fn write_ctl(&mut self, c: u8) {
		match c {
			BS => {
				self.line_buffer.delc(true);
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
				let delta = self.line_buffer.cursor_right();
				if delta != 0 {
					self.write_parse_result();
				}
			}
			b'D' => {
				let delta = self.line_buffer.cursor_left();
				if delta != 0 {
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

	fn builtin_mem(&mut self) {
		let boot_info = match unsafe { multiboot2::load(BOOT_INFO) } {
			Err(e) => {
				writeln!(self, "mem: missing multiboot2 boot info: {:?}", e).unwrap();
				return;
			}
			Ok(v) => v,
		};

		let mmap = match boot_info.memory_map_tag() {
			None => {
				writeln!(self, "mem: missing memory map tag").unwrap();
				return;
			}
			Some(v) => v,
		};

		for mem in mmap.all_memory_areas() {
			writeln!(
				self,
				"base: {:#x}, size: {:#x}, type: {:?}",
				mem.start_address(),
				mem.size(),
				mem.typ()
			)
			.unwrap();
		}
	}

	fn execute_builtin<'a, T>(&mut self, kind: Builtin, _args: T)
	where
		T: Iterator<Item = &'a [u8]>,
	{
		match kind {
			Builtin::Help => self.builtin_help(),
			Builtin::Clear => self.builtin_clear(),
			Builtin::Halt => self.builtin_halt(),
			Builtin::Mem => self.builtin_mem(),
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
	fn write_one(&mut self, data: u8) {
		if let State::Prompt = self.state {
			self.write_const(PROMPT);
			self.sync_cursor(PROMPT.len() as u8);
			self.state = State::Normal;
		}

		let ascii = match self.parser.parse(data) {
			Some(x) => x,
			None => return,
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
	Mem,
}

impl Builtin {
	pub fn from_slice(slice: &[u8]) -> Option<Self> {
		let value = match slice {
			b"help" => Self::Help,
			b"halt" => Self::Halt,
			b"clear" => Self::Clear,
			b"mem" => Self::Mem,
			_ => return None,
		};

		Some(value)
	}
}
