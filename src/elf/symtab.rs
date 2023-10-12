#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Symbol {
	pub st_name: u32,
	pub st_value: usize,
	pub st_size: u32,
	pub st_info: u8,
	pub st_other: u8,
	st_shndx: u16,
}

impl Symbol {
	pub fn get_related_section(&self) -> SectionHdrNdx {
		use SectionHdrNdx::*;
		match self.st_shndx {
			x if x < 0xff00 => Normal(x),
			0xfff1 => Absolute,
			_ => UnknownReserved,
		}
	}
}

#[derive(Debug)]
pub enum SectionHdrNdx {
	Normal(u16),
	Absolute,
	UnknownReserved,
}
