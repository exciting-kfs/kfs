use rustc_demangle::demangle;

use crate::{
	collection,
	console::{constants::*, Ascii, AsciiParser},
	io::character::{Read as ChRead, Write as ChWrite, RW as ChRW},
	boot::{BOOT_INFO, SYMTAB, STRTAB}, printk, pr_info,
};

use core::fmt::{self, Write, Debug};

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

		let print_space = |printable: &mut Shell, mut size| {
			let modifier = if size < 1024 {
				"B"
			} else if size < 1024 * 1024 {
				size /= 1024;
				"K"
			} else {
				size /= 1024 * 1024;
				"M"
			};

			write!(printable, "{}{modifier}, ", size).unwrap();
		};

		for mem in mmap.all_memory_areas() {
			write!(self, "base: ").unwrap();
			print_space(self, mem.start_address());

			write!(self, "size: ").unwrap();
			print_space(self, mem.size());
			// 31 34 39
			writeln!(self, "status: {:?}", mem.typ()).unwrap();
		}
	}

	fn builtin_unit_test<'a, I>(&mut self, mut args: I)
	where
		I: Iterator<Item = &'a [u8]> + Debug,
	{
		const PREFIX: &'static str = "kernel_test";
		while let Some(s) = args.next() {
			let s = core::str::from_utf8(s).unwrap_or_default();
			unsafe {
				STRTAB.iter().filter(|name| name.contains(s) && name.contains(PREFIX)).for_each(|name| {
					let index = name.as_ptr() as usize - STRTAB.addr() as usize;

					SYMTAB.get_addr(index).map(|addr| {
						let func: fn() = core::mem::transmute(addr);
						printk!("TEST: {} ", demangle(name));
						func();
						pr_info!("\x1b[32mok!\x1b[0m");
					});
				});
			}
		}
	}

	fn execute_builtin<'a, I>(&mut self, kind: Builtin, args: I)
	where
		I: Iterator<Item = &'a [u8]> + Debug,
	{
		match kind {
			Builtin::Help => self.builtin_help(),
			Builtin::Clear => self.builtin_clear(),
			Builtin::Halt => self.builtin_halt(),
			Builtin::Mem => self.builtin_mem(),
			Builtin::UnitTest => self.builtin_unit_test(args)
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
	UnitTest
}

impl Builtin {
	pub fn from_slice(slice: &[u8]) -> Option<Self> {
		let value = match slice {
			b"help" => Self::Help,
			b"halt" => Self::Halt,
			b"clear" => Self::Clear,
			b"mem" => Self::Mem,
			b"unit_test" => Self::UnitTest,
			_ => return None,
		};

		Some(value)
	}
}
