use core::{cmp::max, ffi::c_char, mem::size_of};

use multiboot2::{BootInformation, ElfSection, ElfSectionsTag};

use crate::{
	mm::{constant::VM_OFFSET, util::phys_to_virt},
	sync::singleton::Singleton,
};

use super::{symtab::SymtabEntry, Error, Strtab, Symtab};

pub static KSYMS: Singleton<KernelSymbol> = Singleton::uninit();

#[derive(Clone)]
pub struct KernelSymbol {
	symtab: Symtab,
	strtab: Strtab,
}

impl KernelSymbol {
	pub fn new(symtab: Symtab, strtab: Strtab) -> Self {
		Self { symtab, strtab }
	}

	pub fn find_name_by_addr(&self, addr: *const usize) -> Option<&'static str> {
		self.symtab
			.get_name_index(addr)
			.and_then(|idx| self.strtab.get_name(idx))
	}
}

/// # Return
///
/// end address of kernel.
pub fn init(bi: &BootInformation, kernel_end: &mut usize) -> Result<(), Error> {
	let elf_tag = bi
		.elf_sections_tag()
		.ok_or_else(|| Error::MissingElfHeader)?;

	let (symtab, strtab, section_end) = parse_elf_tag(&elf_tag)?;
	*kernel_end = section_end;

	let ksyms = KernelSymbol::new(symtab, strtab);
	unsafe { KSYMS.write(ksyms) };

	Ok(())
}

fn parse_elf_tag(tag: &ElfSectionsTag) -> Result<(Symtab, Strtab, usize), Error> {
	let mut strtab = None;
	let mut symtab = None;
	let mut last_end = 0;

	for section in tag.sections() {
		let end = section.end_address() as usize;
		let end = end.checked_sub(VM_OFFSET).unwrap_or(end);

		last_end = max(last_end, end);

		if section.name() == ".symtab" {
			symtab = Some(unsafe { get_symtab(&section) });
		} else if section.name() == ".strtab" {
			strtab = Some(unsafe { get_strtab(&section) });
		}
	}

	if let (Some(symtab), Some(strtab)) = (symtab, strtab) {
		Ok((symtab, strtab, last_end))
	} else {
		Err(Error::MissingSection)
	}
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
