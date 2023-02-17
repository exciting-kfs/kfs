//! Simple PMIO helpers

use core::arch::asm;

pub struct Port {
	port: u16,
}

impl Port {
	pub const fn new(port: u16) -> Self {
		Port { port }
	}

	/// read single byte from port
	pub fn read_byte(&self) -> u8 {
		let mut byte: u8;

		unsafe {
			asm!(
				"in al, dx",
				in("dx") self.port,
				out("al") byte,
			)
		};

		byte
	}

	/// write single byte into port
	pub fn write_byte(&self, byte: u8) {
		unsafe {
			asm!(
				"out dx, al",
				in("dx") self.port,
				in("al") byte,
			)
		};
	}
}
