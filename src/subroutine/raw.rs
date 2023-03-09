//! simple subroutine that just transfer input bytes into output without any modifications.

use crate::io::character::{Read, Write, RW};

pub static mut RAW: [Raw; 2] = [Raw::new(), Raw::new()];
pub struct Raw(Option<u8>);

impl Raw {
	pub const fn new() -> Self {
		Self(None)
	}
}

impl Read<u8> for Raw {
	fn read_one(&mut self) -> Option<u8> {
		core::mem::take(&mut self.0)
	}
}

impl Write<u8> for Raw {
	fn write_one(&mut self, data: u8) {
		self.0 = self.0.or(Some(data));
	}
}

impl RW<u8, u8> for Raw {}
