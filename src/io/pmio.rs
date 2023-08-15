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
		let byte: u8;

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

	pub fn read_u16(&self) -> u16 {
		let data: u16;

		unsafe {
			asm!(
				"in ax, dx",
				in("dx") self.port,
				out("ax") data,
			)
		};

		data
	}

	pub fn write_u16(&self, data: u16) {
		unsafe {
			asm!(
				"out dx, ax",
				in("dx") self.port,
				in("ax") data,
			)
		};
	}

	/// read 4 byte from port
	pub fn read_u32(&self) -> u32 {
		let data: u32;

		unsafe {
			asm!(
				"in eax, dx",
				in("dx") self.port,
				out("eax") data,
			)
		};

		data
	}

	/// write 4 byte into port
	pub fn write_u32(&self, data: u32) {
		unsafe {
			asm!(
				"out dx, eax",
				in("dx") self.port,
				in("eax") data,
			)
		};
	}

	pub const fn add(&self, offset: u16) -> Self {
		Self {
			port: self.port + offset,
		}
	}
}
