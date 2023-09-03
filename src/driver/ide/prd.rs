/// Physical Region Descriptor.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PRD {
	paddr: u32,
	conf: u32,
}

pub const PRD_MAX_BYTE: usize = u16::MAX as usize;

impl PRD {
	pub const fn new(paddr: usize, byte_count: u16) -> Self {
		let conf = byte_count as u32;
		Self {
			paddr: (paddr) as u32,
			conf,
		}
	}

	pub fn set_eot(&mut self, eot: bool) {
		if eot {
			self.conf |= 1 << 31;
		} else {
			self.conf &= 0x7fff_fff;
		}
	}
}
