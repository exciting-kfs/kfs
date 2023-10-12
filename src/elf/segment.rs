use crate::mm::user::vma::AreaFlag;
use bitflags::bitflags;

use super::ElfError;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ProgramHdr {
	pub p_type: u32,
	pub p_offset: usize,
	pub p_vaddr: usize,
	pub p_paddr: usize,
	pub p_filesz: usize,
	pub p_memsz: usize,
	pub p_flags: u32,
	pub p_align: u32,
}

pub enum SegmentType {
	Null,
	Load,
	Dynamic,
	Interp,
	Note,
	Shlib,
	Phdr,
	Tls,
	OsPrivate(u32),
	ProcPrivate(u32),
}

impl SegmentType {
	const NULL: u32 = 0x00000000;
	const LOAD: u32 = 0x00000001;
	const DYNAMIC: u32 = 0x00000002;
	const INTERP: u32 = 0x00000003;
	const NOTE: u32 = 0x00000004;
	const SHLIB: u32 = 0x00000005;
	const PHDR: u32 = 0x00000006;
	const TLS: u32 = 0x00000007;
	const LOOS: u32 = 0x60000000;
	const HIOS: u32 = 0x6FFFFFFF;
	const LOPROC: u32 = 0x70000000;
	const HIPROC: u32 = 0x7FFFFFFF;

	pub fn new(raw: u32) -> Result<Self, ElfError> {
		match raw {
			Self::NULL => Ok(Self::Null),
			Self::LOAD => Ok(Self::Load),
			Self::DYNAMIC => Ok(Self::Dynamic),
			Self::INTERP => Ok(Self::Interp),
			Self::NOTE => Ok(Self::Note),
			Self::SHLIB => Ok(Self::Shlib),
			Self::PHDR => Ok(Self::Phdr),
			Self::TLS => Ok(Self::Tls),
			x @ (Self::LOOS..=Self::HIOS) => Ok(Self::OsPrivate(x)),
			x @ (Self::LOPROC..=Self::HIPROC) => Ok(Self::ProcPrivate(x)),
			_ => Err(ElfError::InvalidSegmentType),
		}
	}
}

bitflags! {
	#[derive(Clone, Copy, PartialEq, Eq)]
	pub struct SegmentFlag: u32 {
		const EXECUTE = 0x1;
		const WRITE = 0x2;
		const READ = 0x4;
	}
}

impl SegmentFlag {
	pub const RW: Self = Self::WRITE.union(Self::READ);
}

impl Into<AreaFlag> for SegmentFlag {
	fn into(self) -> AreaFlag {
		let mut result = AreaFlag::empty();

		if self.contains(Self::WRITE) {
			result |= AreaFlag::Writable;
		}

		if self.contains(Self::READ) {
			result |= AreaFlag::Readable;
		}

		result
	}
}
