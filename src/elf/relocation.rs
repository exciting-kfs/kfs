#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Relocation {
	r_offset: usize,
	r_info: u32,
}

impl Relocation {
	const TYPE_R_386_32: u32 = 1;
	const TYPE_R_386_PC32: u32 = 2;
	const TYPE_R_386_PLT32: u32 = 4;

	pub fn get_offset(&self) -> usize {
		self.r_offset
	}

	pub fn get_symbol_index(&self) -> usize {
		(self.r_info >> 8) as usize
	}

	pub fn get_type(&self) -> RelocationType {
		let raw_type = self.r_info & 0xff;

		use RelocationType::*;
		match raw_type {
			Self::TYPE_R_386_32 => R_386_32,
			Self::TYPE_R_386_PC32 => R_386_PC32,
			Self::TYPE_R_386_PLT32 => R_386_PLT32,
			_ => UNKNOWN,
		}
	}
}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug)]
pub enum RelocationType {
	R_386_32,
	R_386_PC32,
	R_386_PLT32,
	UNKNOWN,
}
