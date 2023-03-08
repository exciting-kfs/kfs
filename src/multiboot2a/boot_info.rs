use core::{
	ffi::{c_char, CStr},
	fmt::{Display, Formatter, Result},
	marker::PhantomData,
	mem::{size_of, transmute},
	slice::from_raw_parts,
};

use crate::pr_info;

#[repr(C, align(8))]
pub struct BootInfoHeader {
	total_size: u32,
	reserved: u32,
}

#[repr(C, align(8))]
pub struct TagHeader {
	pub kind: u32,
	pub size: u32,
}

// #[repr(C, align(8))]
// pub struct BasicMemory {}

#[repr(C, align(8))]
pub struct ElfSHTag {
	header: TagHeader,
	num: u32,
	entsize: u32,
	shndx: u32,
	entries: PhantomData<ElfSH>,
}

#[repr(C, align(4))]
pub struct ElfSH {
	pub sh_name: u32,
	pub sh_type: u32,
	pub sh_flags: u32,
	pub sh_addr: u32,
	pub sh_offset: u32,
	pub sh_size: u32,
	pub sh_link: u32,
	pub sh_info: u32,
	pub sh_addralign: u32,
	pub sh_entsize: u32,
}

#[repr(C, align(8))]
pub struct MMapTag {
	header: TagHeader,
	entry_size: u32,
	entry_version: u32,
	entries: PhantomData<MMap>,
}

#[derive(Debug)]
pub enum MMapKind {
	Available,
	Acpi,
	Hibernation,
	Defactive,
	Reserved,
}

#[repr(C, align(8))]
pub struct MMap {
	pub base_addr: u64,
	pub length: u64,
	kind: u32,
	reserved: u32,
}

pub struct BootInfo {
	header: &'static BootInfoHeader,
	elf_sh: Option<&'static ElfSHTag>,
	mmap: Option<&'static MMapTag>,
}

pub struct TagIterator {
	curr: &'static TagHeader,
}

mod tag_kind {
	pub const END: u32 = 0;
	pub const MEMORY_MAP: u32 = 6;
	pub const ELF_SECTION_HEADER: u32 = 9;
}

mod section_kind {
	pub const SYMTAB: u32 = 0x2;
}

impl Iterator for TagIterator {
	type Item = &'static TagHeader;

	fn next(&mut self) -> Option<Self::Item> {
		if self.curr.kind == tag_kind::END {
			return None;
		}

		let ret = Some(self.curr);

		let curr = self.curr as *const _ as usize;
		let next = curr + self.curr.size as usize;
		let next_aligned = next + 7 & !7;

		self.curr = unsafe { &*(next_aligned as *const TagHeader) };

		ret
	}
}

impl IntoIterator for &BootInfoHeader {
	type Item = &'static TagHeader;

	type IntoIter = TagIterator;

	fn into_iter(self) -> Self::IntoIter {
		let tag_start = unsafe { (self as *const BootInfoHeader).offset(1) };
		let tag_start = unsafe { &*(tag_start as usize as *const TagHeader) };

		TagIterator { curr: tag_start }
	}
}

impl BootInfo {
	pub fn load_from_header(header: &'static BootInfoHeader) -> Self {
		let mut elf_sh: Option<&ElfSHTag> = None;
		let mut mmap: Option<&MMapTag> = None;
		for tag_header in header.into_iter() {
			match tag_header.kind {
				tag_kind::ELF_SECTION_HEADER => elf_sh = Some(unsafe { transmute(tag_header) }),
				tag_kind::MEMORY_MAP => mmap = Some(unsafe { transmute(tag_header) }),
				_ => (),
			};
		}

		Self {
			header,
			elf_sh,
			mmap,
		}
	}

	pub fn mmap(&self) -> Option<&MMapTag> {
		self.mmap
	}

	pub fn elf_sh(&self) -> Option<&ElfSHTag> {
		self.elf_sh
	}
}

impl MMapTag {
	pub fn entries(&self) -> &[MMap] {
		let entry_start = &self.entries as *const _ as usize as *const MMap;

		let entry_count = (self.header.size as usize - size_of::<Self>()) / size_of::<MMap>();

		unsafe { from_raw_parts(entry_start, entry_count) }
	}
}

impl MMap {
	pub fn identify(&self) -> MMapKind {
		match self.kind {
			1 => MMapKind::Available,
			3 => MMapKind::Acpi,
			4 => MMapKind::Hibernation,
			5 => MMapKind::Defactive,
			_ => MMapKind::Reserved,
		}
	}
}

impl Display for MMap {
	fn fmt(&self, f: &mut Formatter) -> Result {
		write!(
			f,
			"<MMap Base={:#010x}, Size={:#010x}, Type={:?}>",
			self.base_addr,
			self.length,
			self.identify()
		)
	}
}

impl ElfSHTag {
	pub fn entries(&self) -> &[ElfSH] {
		let entry_start = &self.entries as *const _ as usize as *const ElfSH;
		unsafe { from_raw_parts(entry_start, self.num as usize) }
	}

	pub fn lookup_name(&self, idx: usize, offset: isize) -> Option<&str> {
		let sh = self.entries();

		if idx == 0 || sh.len() <= idx {
			return None;
		}

		let strtab = sh[idx].sh_addr as usize as *const c_char;

		let cstr = unsafe { CStr::from_ptr(strtab.offset(offset)) };

		Some(cstr.to_str().ok()?)
	}

	pub fn section_name(&self, section: &ElfSH) -> Option<&str> {
		self.lookup_name(self.shndx as usize, section.sh_name as isize)
	}
}

#[repr(C, align(4))]
pub struct SymtabEntry {
	pub st_name: u32,
	pub st_value: u32,
	pub st_size: u32,
	pub st_info: u8,
	pub st_other: u8,
	pub st_shndx: u16,
}

// Elf32_Word		st_name;
// Elf32_Addr		st_value;
// Elf32_Word		st_size;
// unsigned char	st_info;
// unsigned char	st_other;
// Elf32_Half		st_shndx;
