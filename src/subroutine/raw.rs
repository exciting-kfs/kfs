use crate::io::character::{Read, Write, RW};

pub static mut RAW: [Raw; 2] = [Raw::new(), Raw::new()];
pub struct Raw(Option<u8>);

const PROMPT: &[u8] = b"sh=> ";

impl Raw {
	pub const fn new() -> Self {
		Self(None)
	}
}

impl Read<u8> for Raw {
	fn read_one(&mut self) -> Option<u8> {
		if let Some(x) = self.0 {
			self.0 = None;
			Some(x)
		} else {
			None
		}
	}
}

impl Write<u8> for Raw {
	fn write_one(&mut self, data: u8) {
		if let None = self.0 {
			self.0 = Some(data)
		}
	}
}

impl RW<u8, u8> for Raw {}
