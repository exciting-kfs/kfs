use crate::{
	collection::WrapQueue,
	console::{Ascii, AsciiParser},
	io::character::{Read, Write, RW},
};

pub struct Dmesg {
	parser: AsciiParser,
	user_ready: bool,
	kern_buf: WrapQueue<u8, 16>,
}

pub static mut DMESG: Dmesg = Dmesg::new();

impl Dmesg {
	pub const fn new() -> Self {
		Self {
			user_ready: false,
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

impl Read<u8> for Dmesg {
	fn read_one(&mut self) -> Option<u8> {
		if !self.kern_buf.empty() {
			self.kern_buf.pop()
		} else {
			if self.user_ready {
				let value = self.parser.as_mut_buf().pop();
				if value.is_none() {
					self.user_ready = false;
					self.parser.reset();
				}
				return value;
			}
			None
		}
	}
}

impl Write<u8> for Dmesg {
	fn write_one(&mut self, data: u8) {
		if self.user_ready {
			return;
		}

		let ascii = match self.parser.parse(data) {
			Some(v) => v,
			None => return,
		};

		if let Ascii::CtlSeq(_, kind) = ascii {
			self.user_ready = match kind {
				b'A' => true,
				b'B' => true,
				b'C' => true,
				b'D' => true,
				b'H' => true,
				b'F' => true,
				_ => false,
			}
		}

		if !self.user_ready {
			self.parser.reset();
		}
	}
}

impl RW<u8, u8> for Dmesg {}
