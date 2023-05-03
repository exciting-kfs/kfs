mod kernel_symbol;
mod strtab;
mod symtab;

use core::cmp::max;
use core::mem::align_of;
use core::ops::Range;
use core::{ffi::c_char, mem::size_of, mem::MaybeUninit};

use multiboot2::{ElfSection, ElfSectionsTag, MemoryMapTag};

use crate::mm::constant::VM_OFFSET;
use crate::mm::util::{next_align_64, phys_to_virt};

use self::kernel_symbol::KernelSymbol;
use self::{
	strtab::Strtab,
	symtab::{Symtab, SymtabEntry},
};

const MULTIBOOT2_MAGIC: u32 = 0x36d76289;

/// Singleton object
pub static mut BOOT_INFO: MaybeUninit<BootInfo> = MaybeUninit::uninit();
pub struct BootInfo {
	pub ksyms: KernelSymbol,
	pub mem_info: PMemory,
}
pub struct PMemory {
	pub linear: Range<u64>,
	pub kernel_end: u64,
}

impl PMemory {
	pub unsafe fn alloc_n<T>(&mut self, n: usize) -> *mut T {
		let begin = next_align_64(self.kernel_end, align_of::<T>() as u64);
		let end = begin + size_of::<T>() as u64 * n as u64;

		let limit = self.linear.end;

		assert!(end <= limit);

		self.kernel_end = end;

		phys_to_virt(begin as usize) as *mut T
	}
}

#[derive(Debug)]
pub enum Error {
	InSufficientMemory,
	WrongMagic,
	FailedToLoadHeader,
	MissingSection,
	MissingElfHeader,
	MissingMemoryMap,
	MissingLinearMemory,
}

impl BootInfo {
	pub fn init(bi_header: usize, magic: u32) -> Result<(), Error> {
		if !check_magic(magic) {
			return Err(Error::WrongMagic);
		}

		let bi = unsafe { multiboot2::load(bi_header) }.map_err(|_| Error::FailedToLoadHeader)?;

		let elf_tag = bi
			.elf_sections_tag()
			.ok_or_else(|| Error::MissingElfHeader)?;

		let (symtab, strtab, kernel_end) = parse_elf_tag(&elf_tag)?;

		let ksyms = KernelSymbol::new(symtab, strtab);

		let mut header_end = bi.end_address() as usize;
		if header_end >= VM_OFFSET {
			header_end -= VM_OFFSET;
		}

		let kernel_end = max(kernel_end as u64, header_end as u64);

		let mmap_tag = bi.memory_map_tag().ok_or_else(|| Error::MissingMemoryMap)?;
		let mem_info = PMemory {
			linear: parse_memory_map(mmap_tag)?,
			kernel_end,
		};

		unsafe { BOOT_INFO.write(BootInfo { mem_info, ksyms }) };

		Ok(())
	}
}

fn parse_memory_map(tag: &MemoryMapTag) -> Result<Range<u64>, Error> {
	let linear = tag
		.memory_areas()
		.find(|x| x.start_address() == (1024 * 1024))
		.ok_or_else(|| Error::MissingLinearMemory)?;

	Ok(linear.start_address()..linear.end_address())
}

unsafe fn get_symtab(symtab: &ElfSection) -> Symtab {
	let addr = phys_to_virt(symtab.start_address() as usize) as *const SymtabEntry;
	let count = symtab.size() as usize / size_of::<SymtabEntry>();

	Symtab::new(addr, count)
}

unsafe fn get_strtab(strtab: &ElfSection) -> Strtab {
	let addr = phys_to_virt(strtab.start_address() as usize) as *const c_char;
	let size = strtab.size() as usize / size_of::<c_char>();

	Strtab::new(addr, size)
}

fn parse_elf_tag(tag: &ElfSectionsTag) -> Result<(Symtab, Strtab, usize), Error> {
	let mut strtab = None;
	let mut symtab = None;
	let mut kernel_end = 0;

	for section in tag.sections() {
		let end_addr = section.end_address() as usize;
		let section_addr = end_addr.checked_sub(VM_OFFSET).unwrap_or(end_addr);

		kernel_end = max(kernel_end, section_addr);
		if section.name() == ".symtab" {
			symtab = Some(unsafe { get_symtab(&section) });
		} else if section.name() == ".strtab" {
			strtab = Some(unsafe { get_strtab(&section) });
		}
	}

	if let (Some(symtab), Some(strtab)) = (symtab, strtab) {
		Ok((symtab, strtab, kernel_end))
	} else {
		Err(Error::MissingSection)
	}
}

fn check_magic(magic: u32) -> bool {
	magic == MULTIBOOT2_MAGIC
}
