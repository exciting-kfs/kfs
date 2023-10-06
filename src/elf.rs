use core::{mem::size_of, slice::from_raw_parts};

use crate::{mm::user::vma::AreaFlag, syscall::errno::Errno};

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FileHdr {
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

#[derive(Clone)]
pub struct Elf<'a> {
	pub contents: &'a [u8],
	pub file_hdr: &'a FileHdr,
	pub program_hdrs: &'a [ProgramHdr],
}

/// .ELF
const ELF_SIGNATURE: [u8; 4] = [0x7f, 0x45, 0x4c, 0x46];

pub mod elf_type {
	pub const NONE: u16 = 0;
	pub const REL: u16 = 0x01;
	pub const EXEC: u16 = 0x02;
	pub const DYN: u16 = 0x03;
	pub const CORE: u16 = 0x04;
	pub const LOOS: u16 = 0xFE00;
	pub const HIOS: u16 = 0xFEFF;
	pub const LOPROC: u16 = 0xFF00;
	pub const HIPROC: u16 = 0xFFFF;
}

pub mod p_type {
	pub const NULL: u32 = 0x00000000;
	pub const LOAD: u32 = 0x00000001;
	pub const DYNAMIC: u32 = 0x00000002;
	pub const INTERP: u32 = 0x00000003;
	pub const NOTE: u32 = 0x00000004;
	pub const SHLIB: u32 = 0x00000005;
	pub const PHDR: u32 = 0x00000006;
	pub const TLS: u32 = 0x00000007;
	pub const LOOS: u32 = 0x60000000;
	pub const HIOS: u32 = 0x6FFFFFFF;
	pub const LOPROC: u32 = 0x70000000;
	pub const HIPROC: u32 = 0x7FFFFFFF;
}

pub mod p_flags {
	pub const EXECUTE: u32 = 0x1;
	pub const WRITE: u32 = 0x2;
	pub const READ: u32 = 0x4;
	pub const RW: u32 = WRITE | READ;
}

impl<'a> Elf<'a> {
	pub fn new(raw: &'a [u8]) -> Result<Self, Errno> {
		if raw.len() < size_of::<FileHdr>() {
			return Err(Errno::ENOEXEC);
		}

		let file_hdr = unsafe { &*raw.as_ptr().cast::<FileHdr>() };

		if &file_hdr.e_ident[0..4] != &ELF_SIGNATURE {
			return Err(Errno::ENOEXEC);
		}

		if raw.len() < file_hdr.e_phoff + (file_hdr.e_phnum as usize) * size_of::<ProgramHdr>() {
			return Err(Errno::ENOEXEC);
		}

		let program_hdrs = unsafe {
			from_raw_parts(
				((&raw[file_hdr.e_phoff]) as *const u8).cast::<ProgramHdr>(),
				file_hdr.e_phnum as usize,
			)
		};

		for hdr in program_hdrs {
			if raw.len() < hdr.p_offset + hdr.p_filesz {
				return Err(Errno::ENOEXEC);
			}
		}

		Ok(Self {
			contents: raw,
			file_hdr,
			program_hdrs,
		})
	}

	pub fn loadable_sections(&self) -> LoadSectionIter<'a> {
		LoadSectionIter {
			elf: self.clone(),
			idx: 0,
		}
	}

	pub fn get_entry_point(&self) -> usize {
		self.file_hdr.e_entry
	}
}

pub struct LoadSection<'a> {
	pub data: &'a [u8],
	pub vaddr: usize,
	pub mem_size: usize,
	pub flags: AreaFlag,
}

pub struct LoadSectionIter<'a> {
	elf: Elf<'a>,
	idx: usize,
}

fn p_flags_to_area_flag(p_flags: u32) -> AreaFlag {
	match p_flags & (p_flags::READ | p_flags::WRITE) {
		p_flags::READ => AreaFlag::Readable,
		p_flags::WRITE => AreaFlag::Writable,
		p_flags::RW => AreaFlag::Readable | AreaFlag::Writable,
		_ => AreaFlag::empty(),
	}
}

impl<'a> Iterator for LoadSectionIter<'a> {
	type Item = LoadSection<'a>;

	fn next(&mut self) -> Option<Self::Item> {
		let program_hdrs = self.elf.program_hdrs;

		if program_hdrs.len() <= self.idx {
			return None;
		}

		let remain_hdrs = &program_hdrs[self.idx..];

		let load_hdr_idx = remain_hdrs
			.iter()
			.position(|x| x.p_type == p_type::LOAD)
			.unwrap_or(remain_hdrs.len());

		self.idx += load_hdr_idx + 1;

		if remain_hdrs.len() <= load_hdr_idx {
			return None;
		}

		let phdr = &remain_hdrs[load_hdr_idx];

		let data = &self.elf.contents[phdr.p_offset..phdr.p_offset + phdr.p_filesz];
		let vaddr = phdr.p_vaddr;
		let mem_size = phdr.p_memsz;
		let flags = p_flags_to_area_flag(phdr.p_flags);

		Some(LoadSection {
			data,
			vaddr,
			mem_size,
			flags,
		})
	}
}
