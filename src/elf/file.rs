use super::ElfError;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ElfHdr {
	pub e_ident: [u8; 16],
	pub e_type: u16,
	pub e_machine: u16,
	pub e_version: u32,
	pub e_entry: usize,
	pub e_phoff: usize,
	pub e_shoff: usize,
	pub e_flags: u32,
	pub e_ehsize: u16,
	pub e_phentsize: u16,
	pub e_phnum: u16,
	pub e_shentsize: u16,
	pub e_shnum: u16,
	pub e_shstrndx: u16,
}

pub enum ElfType {
	None,
	Rel,
	Exec,
	Dyn,
	Core,
	OsPrivate(u16),
	ProcPrivate(u16),
}

impl ElfType {
	const NONE: u16 = 0;
	const REL: u16 = 0x01;
	const EXEC: u16 = 0x02;
	const DYN: u16 = 0x03;
	const CORE: u16 = 0x04;
	const LOOS: u16 = 0xFE00;
	const HIOS: u16 = 0xFEFF;
	const LOPROC: u16 = 0xFF00;
	const HIPROC: u16 = 0xFFFF;

	pub fn new(raw: u16) -> Result<Self, ElfError> {
		match raw {
			Self::NONE => Ok(Self::None),
			Self::REL => Ok(Self::Rel),
			Self::EXEC => Ok(Self::Exec),
			Self::DYN => Ok(Self::Dyn),
			Self::CORE => Ok(Self::Core),
			x @ (Self::LOOS..=Self::HIOS) => Ok(Self::OsPrivate(x)),
			x @ (Self::LOPROC..=Self::HIPROC) => Ok(Self::ProcPrivate(x)),
			_ => Err(ElfError::InvalidElfType),
		}
	}
}
