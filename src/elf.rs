pub mod kobject;
pub mod syscall;

mod file;
mod relocation;
mod section;
mod segment;
mod strtab;
mod symtab;

pub use file::{ElfHdr, ElfType};
pub use relocation::Relocation;
pub use section::{SectionFlag, SectionHdr, SectionType};
pub use segment::{ProgramHdr, SegmentFlag, SegmentType};
pub use strtab::StringTable;
pub use symtab::{SectionHdrNdx, Symbol};

use crate::{mm::user::vma::AreaFlag, syscall::errno::Errno};
use core::{mem::size_of, slice::from_raw_parts};

pub struct Elf<'a> {
	pub raw: &'a [u8],
	pub elf_hdr: &'a ElfHdr,
	pub program_hdrs: &'a [ProgramHdr],
	pub section_hdrs: &'a [SectionHdr],
	pub symbol_table: &'a [Symbol],
	pub string_table: StringTable<'a>,
}

#[derive(Debug)]
pub enum ElfError {
	OutOfMemory,
	OutOfBound,
	InvalidElfType,
	InvalidSectionType,
	InvalidSegmentType,
	InvalidRelatedSection,
	SectionNotFound,
	SymbolNotFound,
	StringNotFound,
}

impl Into<Errno> for ElfError {
	fn into(self) -> Errno {
		match self {
			ElfError::OutOfMemory => Errno::ENOMEM,
			_ => Errno::ENOEXEC,
		}
	}
}

/// .ELF
const ELF_SIGNATURE: [u8; 4] = [0x7f, 0x45, 0x4c, 0x46];

// pub struct ElfType(u16);

fn check_size_and_deref_array<T>(
	raw: &[u8],
	base_offset: usize,
	count: usize,
) -> Result<&'_ [T], ElfError> {
	if raw.len() < base_offset + size_of::<T>() * count {
		return Err(ElfError::OutOfBound);
	}

	Ok(unsafe { from_raw_parts(((&raw[base_offset]) as *const u8).cast::<T>(), count) })
}

fn check_size_and_deref<T>(raw: &[u8], base_offset: usize) -> Result<&'_ T, ElfError> {
	if raw.len() < base_offset + size_of::<T>() {
		return Err(ElfError::OutOfBound);
	}

	Ok(unsafe { &*((&raw[base_offset]) as *const u8).cast::<T>() })
}

impl<'a> Elf<'a> {
	pub fn new(raw: &'a [u8]) -> Result<Self, ElfError> {
		let elf_hdr = check_size_and_deref::<ElfHdr>(raw, 0)?;

		if &elf_hdr.e_ident[0..4] != &ELF_SIGNATURE {
			return Err(ElfError::OutOfBound);
		}

		let program_hdrs = check_size_and_deref_array::<ProgramHdr>(
			raw,
			elf_hdr.e_phoff,
			elf_hdr.e_phnum as usize,
		)?;

		let section_hdrs = check_size_and_deref_array::<SectionHdr>(
			raw,
			elf_hdr.e_shoff,
			elf_hdr.e_shnum as usize,
		)?;

		let string_table = StringTable::new(
			section_hdrs
				.iter()
				.find(|hdr| matches!(hdr.get_type(), Ok(SectionType::Strtab)))
				.ok_or(ElfError::SectionNotFound)
				.and_then(|hdr| {
					check_size_and_deref_array::<u8>(raw, hdr.sh_offset, hdr.sh_size as usize)
				})
				.unwrap_or(&[]),
		);

		let symbol_table = section_hdrs
			.iter()
			.find(|hdr| matches!(hdr.get_type(), Ok(SectionType::Symtab)))
			.ok_or(ElfError::SectionNotFound)
			.and_then(|hdr| {
				check_size_and_deref_array::<Symbol>(
					raw,
					hdr.sh_offset,
					hdr.sh_size as usize / size_of::<Symbol>(),
				)
			})
			.unwrap_or(&[]);

		Ok(Self {
			raw,
			elf_hdr,
			program_hdrs,
			section_hdrs,
			symbol_table,
			string_table,
		})
	}

	pub fn loadable_sections(&'a self) -> LoadSectionIter<'a> {
		LoadSectionIter {
			raw: self.raw,
			program_hdrs: self.program_hdrs,
			idx: 0,
		}
	}

	pub fn get_entry_point(&self) -> usize {
		self.elf_hdr.e_entry
	}
}

pub struct LoadSection<'a> {
	pub data: &'a [u8],
	pub vaddr: usize,
	pub mem_size: usize,
	pub flags: AreaFlag,
}

pub struct LoadSectionIter<'a> {
	raw: &'a [u8],
	program_hdrs: &'a [ProgramHdr],
	idx: usize,
}

impl<'a> Iterator for LoadSectionIter<'a> {
	type Item = LoadSection<'a>;

	fn next(&mut self) -> Option<Self::Item> {
		let program_hdrs = self.program_hdrs;

		if program_hdrs.len() <= self.idx {
			return None;
		}

		let remain_hdrs = &program_hdrs[self.idx..];

		let load_hdr_idx = remain_hdrs
			.iter()
			.position(|x| matches!(SegmentType::new(x.p_type), Ok(SegmentType::Load)))
			.unwrap_or(remain_hdrs.len());

		self.idx += load_hdr_idx + 1;

		if remain_hdrs.len() <= load_hdr_idx {
			return None;
		}

		let phdr = &remain_hdrs[load_hdr_idx];

		let data = &self.raw[phdr.p_offset..phdr.p_offset + phdr.p_filesz];
		let vaddr = phdr.p_vaddr;
		let mem_size = phdr.p_memsz;
		let flags = SegmentFlag::from_bits_truncate(phdr.p_flags).into();

		Some(LoadSection {
			data,
			vaddr,
			mem_size,
			flags,
		})
	}
}
