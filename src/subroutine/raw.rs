//! simple subroutine that just transfer input bytes into output without any modifications.

use crate::io::{character::RW as ChRW, ChRead, ChWrite, NoSpace};

pub static mut RAW: [Raw; 2] = [Raw::new(), Raw::new()];
pub struct Raw(Option<u8>);

impl Raw {
	pub const fn new() -> Self {
		Self(None)
	}
}

impl ChRead<u8> for Raw {
	fn read_one(&mut self) -> Option<u8> {
		core::mem::take(&mut self.0)
	}
}

impl ChWrite<u8> for Raw {
	fn write_one(&mut self, data: u8) -> Result<(), NoSpace> {
		self.0 = self.0.or(Some(data));
		Ok(())
	}
}

impl ChRW<u8, u8> for Raw {}
