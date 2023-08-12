use core::fmt::Display;

use crate::io::pmio::Port;

use super::header::HeaderCommon;

#[derive(Debug, Clone)]
pub struct BDF {
	pub(super) bus: u8,
	pub(super) dev: u8,
	pub(super) func: u8,
}

static INDEX: Port = Port::new(0xcf8);
static DATA: Port = Port::new(0xcfc);

impl BDF {
	fn index(&self) -> u32 {
		const ENABLE: u32 = 0x8000_0000;

		let bus = self.bus as u32;
		let dev = self.dev as u32;
		let func = self.func as u32;

		ENABLE | bus << 16 | dev << 11 | func << 8
	}

	pub fn read_u32(&self, offset: u8) -> u32 {
		let address = self.index() | offset as u32 & 0xfc;

		INDEX.write_u32(address);
		DATA.read_u32()
	}

	pub fn write_u32(&self, offset: u8, data: u32) {
		let address = self.index() | offset as u32 & 0xfc;

		INDEX.write_u32(address);
		DATA.write_u32(data)
	}

	pub fn set_busmaster(&self, on: bool) {
		let h = HeaderCommon::get(self).expect("invalid BDF");
		let c = match on {
			true => h.command | 0x04,
			false => h.command & 0xfb,
		};

		let data = (h.status as u32) << 16 | c as u32;
		self.write_u32(0x4, data);
	}
}

impl Display for BDF {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(f, "BDF:{:x}:{:x}:{:x}", self.bus, self.dev, self.func)
	}
}
