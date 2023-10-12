use bitflags::bitflags;

use super::ElfError;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct SectionHdr {
	pub sh_name: u32,
	sh_type: u32,
	sh_flags: u32,
	pub sh_addr: usize,
	pub sh_offset: usize,
	pub sh_size: u32,
	pub sh_link: u32,
	pub sh_info: u32,
	pub sh_addralign: u32,
	pub sh_entsize: u32,
}

impl SectionHdr {
	pub fn get_type(&self) -> Result<SectionType, ElfError> {
		SectionType::new(self.sh_type)
	}

	pub fn get_flags(&self) -> SectionFlag {
		SectionFlag::from_bits_truncate(self.sh_flags)
	}

	pub fn has_flag(&self, flag: SectionFlag) -> bool {
		self.get_flags().contains(flag)
	}
}

pub enum SectionType {
	Null,
	Progbits,
	Symtab,
	Strtab,
	Rela,
	Hash,
	Dynamic,
	Note,
	Nobits,
	Rel,
	Shlib,
	Dynsym,
	Num,
	ProcPrivate(u32),
	UserPrivate(u32),
}

impl SectionType {
	const NULL: u32 = 0;
	const PROGBITS: u32 = 1;
	const SYMTAB: u32 = 2;
	const STRTAB: u32 = 3;
	const RELA: u32 = 4;
	const HASH: u32 = 5;
	const DYNAMIC: u32 = 6;
	const NOTE: u32 = 7;
	const NOBITS: u32 = 8;
	const REL: u32 = 9;
	const SHLIB: u32 = 10;
	const DYNSYM: u32 = 11;
	const NUM: u32 = 12;
	const LOPROC: u32 = 1879048192;
	const HIPROC: u32 = 2147483647;
	const LOUSER: u32 = 2147483648;
	const HIUSER: u32 = 4294967295;

	pub fn new(raw: u32) -> Result<Self, ElfError> {
		match raw {
			Self::NULL => Ok(Self::Null),
			Self::PROGBITS => Ok(Self::Progbits),
			Self::SYMTAB => Ok(Self::Symtab),
			Self::STRTAB => Ok(Self::Strtab),
			Self::RELA => Ok(Self::Rela),
			Self::HASH => Ok(Self::Hash),
			Self::DYNAMIC => Ok(Self::Dynamic),
			Self::NOTE => Ok(Self::Note),
			Self::NOBITS => Ok(Self::Nobits),
			Self::REL => Ok(Self::Rel),
			Self::SHLIB => Ok(Self::Shlib),
			Self::DYNSYM => Ok(Self::Dynsym),
			Self::NUM => Ok(Self::Num),
			x @ (Self::LOPROC..=Self::HIPROC) => Ok(Self::ProcPrivate(x)),
			x @ (Self::LOUSER..=Self::HIUSER) => Ok(Self::UserPrivate(x)),
			_ => Err(ElfError::InvalidSectionType),
		}
	}
}

bitflags! {
	#[derive(Clone, Copy, PartialEq, Eq)]
	pub struct SectionFlag: u32 {
		const WRITE = 0x1;
		const ALLOC = 0x2;
		const EXECINSTR = 0x4;
	}
}
