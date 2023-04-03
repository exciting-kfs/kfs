use crate::{
	collection::WrapQueue,
	console::{Ascii, AsciiParser},
	io::character::{Read as CRead, Write as CWrite, RW},
};

use core::fmt::{Error, Result, Write};

pub struct Dmesg {
	parser: AsciiParser,
	kern_buf: WrapQueue<u8, 4096>,
}

pub static mut DMESG: Dmesg = Dmesg::new();

impl Dmesg {
	pub const fn new() -> Self {
		Self {
			parser: AsciiParser::new(),
			kern_buf: WrapQueue::with(0),
		}
	}

	pub fn write(&mut self, data: u8) {
		if !self.kern_buf.full() {
			self.kern_buf.push(data);
		}
	}
}

impl CRead<u8> for Dmesg {
	fn read_one(&mut self) -> Option<u8> {
		self.kern_buf.pop()
	}
}

impl CWrite<u8> for Dmesg {
	fn write_one(&mut self, data: u8) {
		let ascii = match self.parser.parse(data) {
			Some(v) => v,
			None => return,
		};

		if let Ascii::CtlSeq(param, kind) = ascii {
			let is_cursor_sequence = match (param, kind) {
				(_, b'A' | b'B' | b'C' | b'D' | b'H' | b'F') => true,
				(5 | 6, b'~') => true,
				_ => false,
			};

			if is_cursor_sequence {
				let buffer = self.parser.as_mut_buf();
				while let Some(x) = buffer.pop() {
					self.kern_buf.push(x);
				}
			}
		}

		self.parser.reset();
	}
}

impl RW<u8, u8> for Dmesg {}

impl Write for Dmesg {
	fn write_str(&mut self, s: &str) -> Result {
		let prefix = "\x1b[u".bytes().into_iter();
		let string = s.bytes().into_iter();
		let suffix = "\x1b[s".bytes().into_iter();

		for ch in prefix.chain(string).chain(suffix) {
			// FIXME BOOM hmm..?
			// if self.kern_buf.full() {
			// 	return Err(Error);
			// }
			self.kern_buf.push(ch);
		}

		Ok(())
	}
}
