use crate::{
	collection::WrapQueue,
	console::{constants::*, Ascii, AsciiParser},
	io::character::{Read, Write, RW},
};

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

	pub fn window_at(&self, index: usize) -> &[u8] {
		&self.buf[index..self.tail]
	}
}

enum State {
	Prompt,
	Sync,
}

type Buffer = WrapQueue<u8, 256>;

pub static mut SHELL: Shell = Shell::new();
pub struct Shell {
	state: State,
	line_buffer: LineBuffer<64>,
	write_queue: Buffer,
	parser: AsciiParser,
}

const PROMPT: &[u8] = b"sh=> ";

impl Shell {
	pub const fn new() -> Self {
		Self {
			state: State::Prompt,
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
		for ch in self.line_buffer.window_at(0) {
			self.write_queue.push(*ch);
		}
		self.sync_cursor(self.line_buffer.cursor as u8 + PROMPT.len() as u8);
	}

	fn sync_cursor(&mut self, mut cursor: u8) {
		let mut buf = [0, 0, 0];
		let mut buf_cur = buf.len() - 1;

		loop {
			buf[buf_cur] = cursor % 10 + b'0';
			cursor /= 10;
			if cursor == 0 {
				break;
			}
			buf_cur -= 1;
		}

		self.write_const(b"\x1b[");
		for num in &buf[buf_cur..] {
			self.write_queue.push(*num);
		}
		self.write_queue.push(b'G');
	}

	fn write_ctl(&mut self, c: u8) {
		match c {
			BS => {
				self.line_buffer.delc(true);
			}
			CR | LF => {
				self.write_queue.push(b'\n');
				self.state = State::Prompt;
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
}

impl Read<u8> for Shell {
	fn read_one(&mut self) -> Option<u8> {
		self.write_queue.pop()
	}
}

impl Write<u8> for Shell {
	fn write_one(&mut self, data: u8) {
		if let State::Prompt = self.state {
			self.write_const(PROMPT);
			self.sync_cursor(PROMPT.len() as u8);
			self.state = State::Sync;
		}

		let ascii = match self.parser.parse(data) {
			Some(x) => x,
			None => return,
		};

		match ascii {
			Ascii::Text(c) => self.write_text(c),
			Ascii::Control(c) => self.write_ctl(c),
			Ascii::CtlSeq(param, kind) => self.write_ctlseq(param, kind),
		}
		self.sync_line();
		self.parser.reset();
	}
}

impl RW<u8, u8> for Shell {}
